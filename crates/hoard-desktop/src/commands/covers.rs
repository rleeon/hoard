//! Game cover-art cache.
//!
//! The UI shows each game's cover in the Library, the Dashboard grid and the
//! Map. Fetching it from a CDN on every paint adds network latency and breaks
//! offline, so we cache the bytes on disk under the app cache dir and serve
//! them from there. First sight of a given game downloads once; every
//! subsequent call (this session or a later launch) reads the local file. The
//! frontend receives the raw bytes as an `ArrayBuffer` (via
//! `tauri::ipc::Response`) and wraps them in an object URL: no base64 bloat,
//! no canvas-tainting cross-origin draws.
//!
//! **Covers are keyed by a [`CoverKey`], not by a Steam app id.** They used to
//! be a bare `u32`, which quietly meant "if it isn't on Steam it has no cover
//! and you can't even give it one": the pencil in `Cover.svelte` only appears
//! for a resolved id. That's 2,316 games in the Ludusavi catalog (10% of it),
//! Minecraft Java among them, plus every emulator. The key is now a string:
//! the app id for a Steam game, `slug-<game-slug>` for everything else. Cache
//! filenames for Steam games are unchanged, so no existing cache is orphaned.
//!
//! **Vertical art first.** A game cover is 2:3 by convention (that's what
//! Steam, GOG and the Epic launcher all show), and that's the shape the UI
//! frames. Steam publishes exactly that as `library_600x900_2x.jpg`; the old
//! `header.jpg` is a 460×215 landscape capsule, and framing it as a poster
//! center-crops ~70% of the art away. So we ask for the portrait first and
//! only fall back to the header when a game truly has no vertical art. Where
//! that portrait *lives* is the fiddly part: for newer store items the URL
//! is unguessable and has to be read out of the store's asset manifest; see
//! [`fetch_portrait`].
//!
//! Games with no Steam presence get their art from [`index_lookup`]: a small
//! `slug -> URL` index we host, pointing at each game's art on its own
//! publisher's CDN (Microsoft Store, GOG and so on). We host the *index*, never
//! the images: an index is a few KB of text we're free to publish, while a folder
//! of box art is redistributing somebody else's copyrighted work.
//!
//! A missing game, a 404, or being offline surfaces as an `Err`, which the
//! JS side catches and falls back to the initial-letter placeholder.
//!
//! Users can override any game's cover with a custom image stored locally.
//! Custom covers are saved as `{key}_custom.{ext}` in the same cache dir and
//! take priority over any downloaded art.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

const CUSTOM_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif"];

/// Which game a cover belongs to.
///
/// `Steam(id)` keeps the historic filenames (`{id}.jpg`, `{id}_600x900.jpg`),
/// so every cover cached by an older build is still a hit after this change.
/// `Slug(s)` is everything Steam doesn't know about (Minecraft Java, GOG
/// classics, emulators) and is where the hosted index comes in.
enum CoverKey {
    Steam(u32),
    Slug(String),
}

impl CoverKey {
    /// Parse the key the frontend sent. All-digits is a Steam app id; anything
    /// else is a slug, with the `slug-` prefix stripped. The prefix is what
    /// keeps the two apart: the catalog really does contain games slugged `2`
    /// and `3`, which would otherwise read as app ids.
    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        if let Ok(id) = raw.parse::<u32>() {
            return Some(CoverKey::Steam(id));
        }
        let slug = raw.strip_prefix("slug-").unwrap_or(raw);
        if slug.is_empty() {
            return None;
        }
        Some(CoverKey::Slug(slug.to_string()))
    }

    /// Filename stem for this game's files in the cache dir. Sanitised because
    /// the slug reaches us from the frontend and ends up in a path: anything
    /// that isn't `[a-z0-9-_]` becomes `_`, so no key can walk out of the
    /// covers directory or collide with a Steam id.
    fn stem(&self) -> String {
        match self {
            CoverKey::Steam(id) => id.to_string(),
            CoverKey::Slug(slug) => {
                let safe: String = slug
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                            c.to_ascii_lowercase()
                        } else {
                            '_'
                        }
                    })
                    .collect();
                format!("slug-{safe}")
            }
        }
    }
}

/// Every filename stem a game's files may be under, newest naming first.
///
/// The second entry is the bare Steam app id, which is what *all* covers were
/// filed under before keys existed. Without it this change would silently
/// orphan every cached cover on every machine, including the custom ones people
/// picked by hand, which is the kind of loss that reads as a bug.
async fn stems_for(app: &AppHandle, cover: &CoverKey) -> Vec<String> {
    let mut stems = vec![cover.stem()];
    if let CoverKey::Slug(slug) = cover {
        if let Some(id) = catalog_app_id(app, slug).await {
            stems.push(id.to_string());
        }
    }
    stems
}

/// The catalogue's Steam app id for a slug, asked of the service (see
/// [`crate::commands::catalog::game_facts`]). Remembered for the session: a shelf
/// paint asks for dozens of covers, each with the same question.
async fn catalog_app_id(app: &AppHandle, slug: &str) -> Option<u32> {
    static KNOWN: LazyLock<Mutex<HashMap<String, Option<u32>>>> = LazyLock::new(Default::default);
    if let Some(id) = KNOWN.lock().unwrap().get(slug) {
        return *id;
    }
    let state = app.state::<crate::state::AppState>();
    let id = crate::commands::catalog::game_facts(&state, slug)
        .await
        .steam_app_id
        .map(|v| v as u32);
    KNOWN.lock().unwrap().insert(slug.to_string(), id);
    id
}

/// Find a custom cover file for the given game, returning its path if it
/// exists. Checks multiple image extensions since the user can pick any format.
fn find_custom_cover(dir: &std::path::Path, stems: &[String]) -> Option<PathBuf> {
    for stem in stems {
        for ext in CUSTOM_EXTENSIONS {
            let path = dir.join(format!("{stem}_custom.{ext}"));
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

/// Returns the bytes of a game's cover image, reading from the on-disk cache.
/// Priority: the user's custom cover (`{key}_custom.*`), then the vertical
/// 2:3 art (`{key}_600x900.jpg`), then, for Steam games only, the landscape
/// capsule (`{key}.jpg`). Each tier downloads and persists on first miss.
///
/// `size` is the longer side of the frame the art will fill, in device pixels.
/// Most frames are list icons of 32 to 74 px, and the webview keeps every image
/// decoded at its own resolution, not the frame's: a 600×900 cover costs 2.2 MB
/// of memory whether it is drawn as a poster or as a 36 px square. With a size,
/// the answer is a copy scaled to the first of [`SIZES`] that covers the frame.
#[tauri::command]
pub async fn cover_bytes(
    app: tauri::AppHandle,
    key: String,
    size: Option<u32>,
) -> Result<Response, String> {
    let cover = CoverKey::parse(&key).ok_or_else(|| format!("cover: bad key {key:?}"))?;
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");

    let bytes = source_bytes(&app, &cover, &dir).await?;
    let Some(side) = size.and_then(thumb_side) else {
        return Ok(Response::new(bytes));
    };
    let stem = cover.stem();
    let served = tokio::task::spawn_blocking(move || sized_copy(&dir, &stem, side, bytes))
        .await
        .map_err(|e| e.to_string())?;
    Ok(Response::new(served))
}

/// The art as stored, in the order [`cover_bytes`] describes.
async fn source_bytes(
    app: &AppHandle,
    cover: &CoverKey,
    dir: &std::path::Path,
) -> Result<Vec<u8>, String> {
    let stems = stems_for(app, cover).await;
    let stem = stems[0].clone();

    // Fast path 1: user has set a custom cover for this game.
    if let Some(custom) = tokio::task::spawn_blocking({
        let dir = dir.to_path_buf();
        let stems = stems.clone();
        move || find_custom_cover(&dir, &stems)
    })
    .await
    .map_err(|e| e.to_string())?
    {
        if let Ok(bytes) = tokio::fs::read(&custom).await {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }

    // Fast path 2: the 2:3 portrait, the shape the UI actually frames. Tried
    // under the legacy app-id name too, so an existing cache survives the move
    // to keys instead of every user re-downloading their whole shelf.
    let portrait = dir.join(format!("{stem}{PORTRAIT_SUFFIX}.jpg"));
    for s in &stems {
        if let Ok(bytes) = tokio::fs::read(dir.join(format!("{s}{PORTRAIT_SUFFIX}.jpg"))).await {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }

    let marker = dir.join(format!("{stem}{PORTRAIT_SUFFIX}.none"));
    let known_artless = marker_still_stands(&marker).await;

    // Which game is this, in Steam's terms? Four sources, most trustworthy
    // first, and the order is the whole point:
    //
    //   1. the app id detection already resolved on this machine;
    //   2. the Ludusavi catalog's exact slug -> app id mapping;
    //   3. our own curated index, for games Steam doesn't have at all;
    //   4. Steam's fuzzy store search on the de-slugified name.
    //
    // (4) is last because it is a guess and it guesses confidently: searching
    // "minecraft" returns Minecraft *Dungeons*, so a fuzzy hit that outranked
    // the curated index would put the wrong box art on the shelf.
    let slug = match &cover {
        CoverKey::Steam(_) => None,
        CoverKey::Slug(slug) => Some(slug.clone()),
    };
    let app_id = match &cover {
        CoverKey::Steam(id) => Some(*id),
        CoverKey::Slug(slug) => catalog_app_id(app, slug).await,
    };

    // Tier 3: a game with no Steam app id of its own. The hosted index is the
    // only automatic source, and it only ever points at 2:3 art, so there is no
    // landscape fallback to be had.
    if app_id.is_none() && !known_artless {
        let slug = slug.clone().unwrap_or_default();
        match index_cover(&slug, &dir).await {
            Fetch::Bytes(bytes) => {
                let _ = tokio::fs::create_dir_all(&dir).await;
                let _ = tokio::fs::write(&portrait, &bytes).await;
                let _ = tokio::fs::remove_file(&marker).await;
                return Ok(bytes);
            }
            // Index in hand, game not in it: fall through to the fuzzy
            // search, which is all that's left.
            Fetch::Missing => {}
            Fetch::Unavailable => return Err(format!("cover {slug}: index unreachable")),
        }
    }

    // Tier 4: nothing knows this game by id, so guess from its name.
    let app_id = match app_id {
        Some(id) => id,
        None => {
            let slug = slug.unwrap_or_default();
            if known_artless {
                return Err(format!("cover {slug}: no art anywhere"));
            }
            match steam_store_search_app_id(&deslugify(&slug)).await {
                Some(id) => id,
                None => {
                    let _ = tokio::fs::create_dir_all(&dir).await;
                    let _ = tokio::fs::write(&marker, LOOKUP_STRATEGY.to_string()).await;
                    // The one row that turns into work: this slug is what goes
                    // in `covers.json`. Reported here and not at the top of the
                    // function because only here do we know every source came
                    // back empty, and it rides the marker, so a game already
                    // known to be artless is not re-reported for 30 days.
                    hoard_agent::telemetry::no_cover(&slug, "none");
                    return Err(format!("cover {slug}: no art anywhere"));
                }
            }
        }
    };

    let landscape = dir.join(format!("{stem}.jpg"));

    // No portrait on disk. Unless we already learned this game has none, ask
    // the CDN for it. The marker matters: without it, every game that only
    // ships a header would re-ask Steam on each launch, forever.
    if !known_artless {
        match fetch_portrait(app_id).await {
            Fetch::Bytes(bytes) => {
                let _ = tokio::fs::create_dir_all(&dir).await;
                let _ = tokio::fs::write(&portrait, &bytes).await;
                let _ = tokio::fs::remove_file(&marker).await;
                // A landscape capsule cached by an older build is now dead
                // weight; the portrait supersedes it.
                let _ = tokio::fs::remove_file(&landscape).await;
                return Ok(bytes);
            }
            // Steam answered "no such asset". Remember it, stamped with the
            // strategy that concluded it, and fall through to the header.
            Fetch::Missing => {
                let _ = tokio::fs::create_dir_all(&dir).await;
                let _ = tokio::fs::write(&marker, LOOKUP_STRATEGY.to_string()).await;
            }
            // Offline or a transient error: don't write the marker, or one
            // flaky launch would pin this game to landscape art for good.
            Fetch::Unavailable => {}
        }
    }

    // Fast path 3: landscape capsule already on disk.
    for s in &stems {
        if let Ok(bytes) = tokio::fs::read(dir.join(format!("{s}.jpg"))).await {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }

    // Miss. Try the legacy capsule path first: it's present for the vast majority
    // of (older) apps and served straight from the CDN.
    let legacy = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/header.jpg");
    let bytes = match fetch_image(&legacy).await {
        Some(b) => b,
        // Newer / unreleased games (e.g. Europa Universalis V) only publish
        // assets under a hashed `store_item_assets` path, where the legacy URL
        // 404s. Ask Steam's appdetails API for the real header image and fetch
        // that instead.
        None => {
            let url = match appdetails_header_url(app_id).await {
                Some(url) => url,
                None => {
                    // On Steam and still nothing to show: neither the vertical
                    // capsule nor the header. Rare enough to be worth telling
                    // apart from the games that simply aren't on Steam.
                    report_no_cover(&cover, "steam");
                    return Err(format!("steam cover {app_id}: no header image"));
                }
            };
            match fetch_image(&url).await {
                Some(bytes) => bytes,
                // A dead URL from a live manifest is a fetch failure, not
                // "this game has no art", so don't file it as one.
                None => return Err(format!("steam cover {app_id}: header fetch failed")),
            }
        }
    };

    // Best-effort write: a failed cache write just means we re-fetch next time.
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = tokio::fs::write(&landscape, &bytes).await;
    Ok(bytes)
}

// ---- scaled copies

/// The frame sizes a scaled copy is kept at, in device pixels of the frame's
/// longer side. A 36 px list icon on a 1x screen lands on 96; the dashboard's
/// posters (220 to 720 px wide) mostly ask for more than 768 and get the art
/// as stored, which at 600×900 is about what they draw anyway.
const SIZES: [u32; 4] = [96, 192, 384, 768];

const THUMB_QUALITY: u8 = 85;

/// Past this a custom cover is not worth decoding just to shrink it: an 8K
/// square is 256 MB of pixels. It is served as stored, as it always was.
const MAX_DECODE_SIDE: u32 = 8192;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

/// The step a request for `size` px lands on, or `None` past the largest one,
/// where only the original will do.
fn thumb_side(size: u32) -> Option<u32> {
    if size == 0 {
        return None;
    }
    SIZES.iter().copied().find(|&s| s >= size)
}

/// `bytes` scaled for a frame whose longer side is `side` px, read from disk
/// when this exact art was scaled before. The file name carries a fingerprint
/// of the source, so a new custom cover or a better download can never have an
/// old copy served in its place, and there is nothing to invalidate by hand.
///
/// Returns `bytes` untouched when they are already about that small or can't
/// be decoded: the frame shows what it always showed, only heavier.
fn sized_copy(dir: &Path, stem: &str, side: u32, bytes: Vec<u8>) -> Vec<u8> {
    let prefix = format!("{stem}_s{side}_");
    let name = format!("{prefix}{:016x}", fingerprint(&bytes));
    for ext in ["jpg", "png"] {
        if let Ok(cached) = std::fs::read(dir.join(format!("{name}.{ext}"))) {
            if !cached.is_empty() {
                return cached;
            }
        }
    }
    let Some((small, ext)) = make_thumb(&bytes, side) else {
        return bytes;
    };
    drop_stale_copies(dir, &prefix);
    // Best effort, like every other write in here: a copy that didn't land is
    // made again next time.
    let _ = hoard_agent::atomic_write::write_atomic(&dir.join(format!("{name}.{ext}")), &small);
    small
}

/// Delete this game's copies at this size, scaled from art it no longer has.
fn drop_stale_copies(dir: &Path, prefix: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix(prefix)) else {
            continue;
        };
        // Exactly `<16 hex>.<ext>`: the prefix alone also matches a game whose
        // slug happens to end in `_s96`.
        let rest = rest.as_bytes();
        if rest.len() == 20 && rest[16] == b'.' && rest[..16].iter().all(u8::is_ascii_hexdigit) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Decode, scale and re-encode. `None` means "serve the source": already small
/// enough for the frame, too big to be worth decoding, or not an image we read.
fn make_thumb(bytes: &[u8], side: u32) -> Option<(Vec<u8>, &'static str)> {
    let reader = || {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_DECODE_SIDE);
        limits.max_image_height = Some(MAX_DECODE_SIDE);
        limits.max_alloc = Some(MAX_DECODE_BYTES);
        reader.limits(limits);
        Some(reader)
    };
    // The header alone says whether there is anything to gain.
    let (w, h) = reader()?.into_dimensions().ok()?;
    let (tw, th) = thumb_dims(w, h, side)?;
    let small = reader()?
        .decode()
        .ok()?
        .resize_exact(tw, th, image::imageops::FilterType::CatmullRom);

    let mut out = Vec::new();
    if small.color().has_alpha() {
        small
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .ok()?;
        Some((out, "png"))
    } else {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, THUMB_QUALITY)
            .encode_image(&small.to_rgb8())
            .ok()?;
        Some((out, "jpg"))
    }
}

/// Target dimensions for a frame of `side` px, or `None` when the source is
/// within a quarter of that already. The *shorter* side is the one scaled to
/// `side`: the frame crops the art to fill, so both of its sides have to be
/// covered whatever the art's shape.
fn thumb_dims(w: u32, h: u32, side: u32) -> Option<(u32, u32)> {
    let short = w.min(h);
    if short == 0 || short <= side + side / 4 {
        return None;
    }
    let scale = f64::from(side) / f64::from(short);
    let fit = |n: u32| ((f64::from(n) * scale).round() as u32).max(1);
    Some((fit(w), fit(h)))
}

/// FNV-1a over the whole file: stable across builds, which `DefaultHasher`
/// does not promise, and plenty to tell one cached image from the next.
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Cache-name suffix for the vertical art, kept apart from the landscape
/// capsule cached as `{app_id}.jpg` by every build up to 1.0.4.
const PORTRAIT_SUFFIX: &str = "_600x900";

/// How we currently look a portrait up. **Bump this whenever `fetch_portrait`
/// learns a new place to look.**
///
/// A "this game has no vertical art" marker is only as true as the search that
/// produced it, and that bit me the first day: a build that only tried the
/// guessable URLs marked Europa Universalis V and Surviving Mars: Relaunched
/// as artless, and the very next build (the one that reads the store manifest
/// and *can* find their art) never asked again, because the marker was
/// already there. Stamping the marker with the strategy makes the old verdicts
/// expire on their own instead of outliving the code that reached them.
///
/// 3: added the hosted index, so every "artless" verdict reached before it
/// existed has to be re-asked, which is the whole population this release is
/// for.
const LOOKUP_STRATEGY: u32 = 3;

/// Steam publishes vertical art for older games later on, so a negative
/// verdict also expires with time, not just with a better search.
const MARKER_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 3600);

/// `true` if a "no vertical art" marker is still worth believing: written by
/// the search we run today, and recent enough. Anything else (missing, an older
/// strategy, stale, unreadable) means ask Steam again.
async fn marker_still_stands(marker: &std::path::Path) -> bool {
    let Ok(body) = tokio::fs::read_to_string(marker).await else {
        return false;
    };
    if body.trim().parse::<u32>() != Ok(LOOKUP_STRATEGY) {
        return false;
    }
    tokio::fs::metadata(marker)
        .await
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|age| age < MARKER_TTL).unwrap_or(true))
        .unwrap_or(false)
}

/// Report a game we found no art for, if we know it by slug.
///
/// A bare Steam app id is skipped on purpose: the index is keyed by slug, so a
/// row we can't name is a row nobody can act on.
fn report_no_cover(cover: &CoverKey, source: &str) {
    if let CoverKey::Slug(slug) = cover {
        hoard_agent::telemetry::no_cover(slug, source);
    }
}

/// Where the `slug -> cover URL` index lives. Static file on the marketing
/// site, so publishing a cover for a newly-popular game is a web deploy, not
/// an app release, which is the whole point of hosting an index instead of baking
/// the list into the binary.
const COVER_INDEX_URL: &str = "https://hoard.services/covers.json";

/// How long a downloaded index is trusted before we re-fetch it. A week: the
/// list changes when we add games to it, which is rare, and a stale entry
/// still points at working art.
const INDEX_TTL: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);

/// The parsed index, loaded at most once per process. `None` means "not loaded
/// yet, or the last attempt failed": a failure must not be memoised, or one
/// offline launch would blank every non-Steam cover until the app restarts.
static COVER_INDEX: tokio::sync::Mutex<Option<std::sync::Arc<serde_json::Value>>> =
    tokio::sync::Mutex::const_new(None);

/// Fetch a non-Steam game's cover through the hosted index.
///
/// [`Fetch::Missing`] means the index loaded and this slug isn't in it: a real
/// verdict, worth a marker. An unreachable index or a dead URL is
/// [`Fetch::Unavailable`]: try again next launch.
async fn index_cover(slug: &str, dir: &std::path::Path) -> Fetch {
    match index_lookup(slug, dir).await {
        Some(url) => match fetch(&url).await {
            Fetch::Bytes(b) => Fetch::Bytes(b),
            // The index pointed somewhere dead. That's our list being wrong,
            // not the game having no art, so don't burn it in with a marker.
            Fetch::Missing | Fetch::Unavailable => Fetch::Unavailable,
        },
        None => match load_index(dir).await {
            // Index in hand and the slug isn't there: a real "no art" verdict.
            Some(_) => Fetch::Missing,
            None => Fetch::Unavailable,
        },
    }
}

/// The cover URL this index has for `slug`, if any.
async fn index_lookup(slug: &str, dir: &std::path::Path) -> Option<String> {
    let index = load_index(dir).await?;
    index
        .get("covers")?
        .get(slug)?
        .as_str()
        .map(|s| s.to_string())
}

/// The index, from memory, else disk (while fresh), else the network.
async fn load_index(dir: &std::path::Path) -> Option<std::sync::Arc<serde_json::Value>> {
    let mut slot = COVER_INDEX.lock().await;
    if let Some(index) = slot.as_ref() {
        return Some(index.clone());
    }

    // Kept alongside the cover images rather than in the app's data dir: it's
    // derived, and losing it costs exactly one download.
    let cached = dir.join("index.json");
    if index_cache_is_fresh(&cached).await {
        if let Some(parsed) = tokio::fs::read(&cached)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        {
            let index = std::sync::Arc::new(parsed);
            *slot = Some(index.clone());
            return Some(index);
        }
    }

    let body = match fetch(COVER_INDEX_URL).await {
        Fetch::Bytes(b) => b,
        // Offline. A stale copy beats no covers at all, so fall back to
        // whatever is on disk regardless of age.
        Fetch::Missing | Fetch::Unavailable => {
            let parsed = tokio::fs::read(&cached)
                .await
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())?;
            let index = std::sync::Arc::new(parsed);
            *slot = Some(index.clone());
            return Some(index);
        }
    };

    let parsed: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let _ = tokio::fs::create_dir_all(dir).await;
    let _ = tokio::fs::write(&cached, &body).await;
    let index = std::sync::Arc::new(parsed);
    *slot = Some(index.clone());
    Some(index)
}

async fn index_cache_is_fresh(path: &std::path::Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|age| age < INDEX_TTL).unwrap_or(false))
        .unwrap_or(false)
}

/// Fetch a game's vertical cover from Steam, in two tiers.
///
/// 1. **The flat legacy path**, `apps/<id>/library_600x900_2x.jpg`. One
///    request, and it answers for most of the catalog. Note the `_2x`: plain
///    `library_600x900.jpg` lies, it serves a 300×450 scaled copy, and the
///    poster is about 300 CSS px wide, so on any HiDPI screen that's visibly soft.
/// 2. **The store's asset manifest** for everything else. Recent releases
///    (Europa Universalis V, Surviving Mars: Relaunched and friends) publish each
///    asset type under its own hashed directory, so *every* guessable URL 404s and
///    the old code silently fell back to the landscape header, which is why
///    those games showed up as widescreen rectangles. See
///    [`library_capsule_url`].
///
/// [`Fetch::Missing`] only when Steam positively says there's no vertical art:
/// a network error is not proof, and the caller writes a permanent marker on
/// that verdict.
async fn fetch_portrait(app_id: u32) -> Fetch {
    let flat = format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/library_600x900_2x.jpg"
    );
    match fetch(&flat).await {
        Fetch::Bytes(b) => return Fetch::Bytes(b),
        // Offline: stop here rather than dragging the store API into it.
        Fetch::Unavailable => return Fetch::Unavailable,
        Fetch::Missing => {}
    }
    match library_capsule_url(app_id).await {
        Capsule::At(url) => fetch(&url).await,
        Capsule::Absent => Fetch::Missing,
        Capsule::Unavailable => Fetch::Unavailable,
    }
}

/// What Steam's asset manifest says about a game's vertical art.
enum Capsule {
    /// The manifest points at a 600×900 capsule, here.
    At(String),
    /// The manifest came back and this game has no vertical art at all.
    Absent,
    /// The manifest was unreachable: no verdict either way.
    Unavailable,
}

/// Ask Steam's store service where a game's vertical library capsule lives.
///
/// Newer store items keep every asset in a per-asset hashed directory
/// (`store_item_assets/steam/apps/<id>/<40-hex>/library_600x900_2x.jpg`), and
/// that hash appears in no other public endpoint: `appdetails` exposes only
/// the header and the small capsules, each under a *different* hash. So
/// `GetItems` is the one public answer to "where is this game's 600×900 art",
/// and it also says whether the game has one at all.
async fn library_capsule_url(app_id: u32) -> Capsule {
    let input = serde_json::json!({
        "ids": [{ "appid": app_id }],
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_assets": true },
    })
    .to_string();
    let resp = reqwest::Client::new()
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
        .query(&[("input_json", input.as_str())])
        .send()
        .await;
    let json: serde_json::Value = match resp {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(j) => j,
            Err(_) => return Capsule::Unavailable,
        },
        // A 4xx here means we asked wrong, not that the art is missing; either
        // way there's nothing to learn, and a marker would be a lie.
        _ => return Capsule::Unavailable,
    };
    match assets_of(&json).map(capsule_path) {
        Some(Some(path)) => Capsule::At(format!(
            "https://shared.cloudflare.steamstatic.com/store_item_assets/{path}"
        )),
        // Manifest present, no library capsule in it: the game really has none.
        Some(None) => Capsule::Absent,
        None => Capsule::Unavailable,
    }
}

/// The `assets` object of the single item in a `GetItems` response.
fn assets_of(json: &serde_json::Value) -> Option<&serde_json::Value> {
    json.get("response")?
        .get("store_items")?
        .as_array()?
        .first()?
        .get("assets")
}

/// Resolve an asset manifest to the CDN-relative path of the vertical capsule.
///
/// `asset_url_format` is a template (`steam/apps/<id>/${FILENAME}?t=<epoch>`) and
/// the capsule entry is the filename to substitute in, itself possibly
/// prefixed by that asset's hash directory. Prefers the 2x (the true 600×900)
/// and falls back to the 1x for games that only ship one.
fn capsule_path(assets: &serde_json::Value) -> Option<String> {
    let format = assets.get("asset_url_format")?.as_str()?;
    let file = assets
        .get("library_capsule_2x")
        .or_else(|| assets.get("library_capsule"))?
        .as_str()?;
    Some(format.replace("${FILENAME}", file))
}

/// Returns `true` if the game has a user-set custom cover on disk.
#[tauri::command]
pub async fn has_custom_cover(app: tauri::AppHandle, key: String) -> Result<bool, String> {
    let cover = CoverKey::parse(&key).ok_or_else(|| format!("cover: bad key {key:?}"))?;
    let stems = stems_for(&app, &cover).await;
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    tokio::task::spawn_blocking(move || find_custom_cover(&dir, &stems).is_some())
        .await
        .map_err(|e| e.to_string())
}

/// Copy a user-selected image into the cover cache as a custom cover for the
/// given game. The file is stored as `{key}_custom.{ext}` preserving the
/// original extension. Any previous custom cover for this game is replaced.
#[tauri::command]
pub async fn set_custom_cover(
    app: tauri::AppHandle,
    key: String,
    source_path: String,
) -> Result<(), String> {
    let stem = CoverKey::parse(&key)
        .ok_or_else(|| format!("cover: bad key {key:?}"))?
        .stem();
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(format!("source file does not exist: {source_path}"));
    }

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();

    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    // Remove any previous custom cover (different extension).
    for old_ext in CUSTOM_EXTENSIONS {
        let old = dir.join(format!("{stem}_custom.{old_ext}"));
        let _ = tokio::fs::remove_file(&old).await;
    }

    let dest = dir.join(format!("{stem}_custom.{ext}"));
    tokio::fs::copy(src, &dest)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a user's custom cover, reverting to the downloaded art (or, for a
/// game with none, to the initial-letter tile).
#[tauri::command]
pub async fn remove_custom_cover(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let cover = CoverKey::parse(&key).ok_or_else(|| format!("cover: bad key {key:?}"))?;
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    // Both names, or "restore" would drop the new custom cover and surface one
    // the user set years ago under the app-id filename.
    for stem in stems_for(&app, &cover).await {
        for ext in CUSTOM_EXTENSIONS {
            let path = dir.join(format!("{stem}_custom.{ext}"));
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
    Ok(())
}

/// Outcome of asking the CDN for one image. The distinction that matters is
/// between "this asset does not exist" (final, worth remembering) and "we
/// couldn't reach Steam" (temporary, must be retried).
enum Fetch {
    Bytes(Vec<u8>),
    /// The CDN answered, and the answer was no (404 / empty body).
    Missing,
    /// Offline, DNS down, 5xx: no verdict about the asset itself.
    Unavailable,
}

/// GET an image URL, classifying the outcome. See [`Fetch`].
async fn fetch(url: &str) -> Fetch {
    let resp = match reqwest::get(url).await {
        Ok(r) => r,
        Err(_) => return Fetch::Unavailable,
    };
    if resp.status().is_client_error() {
        return Fetch::Missing;
    }
    if !resp.status().is_success() {
        return Fetch::Unavailable;
    }
    match resp.bytes().await {
        Ok(b) if !b.is_empty() => Fetch::Bytes(b.to_vec()),
        Ok(_) => Fetch::Missing,
        Err(_) => Fetch::Unavailable,
    }
}

/// GET an image URL, returning its bytes on a 2xx with a non-empty body.
async fn fetch_image(url: &str) -> Option<Vec<u8>> {
    match fetch(url).await {
        Fetch::Bytes(b) => Some(b),
        Fetch::Missing | Fetch::Unavailable => None,
    }
}

/// Resolve a game's real header-image URL via Steam's appdetails API. Covers
/// games whose store assets live only under the hashed `store_item_assets`
/// path, for which the legacy `apps/<id>/header.jpg` returns 404.
async fn appdetails_header_url(app_id: u32) -> Option<String> {
    let id = app_id.to_string();
    let resp = reqwest::Client::new()
        .get("https://store.steampowered.com/api/appdetails")
        .query(&[
            ("appids", id.as_str()),
            ("filters", "basic"),
            ("l", "english"),
        ])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get(&id)?
        .get("data")?
        .get("header_image")?
        .as_str()
        .map(|s| s.to_string())
}

/// Resolve a game slug to its Steam app id so the UI can fetch a cover.
///
/// Covers depend on a Steam app id, but a save tracked on another device
/// arrives here with only its `game_slug`, and this machine never detected it, so
/// the local detection report has no id for it. Two layered sources, cheapest
/// first:
///   1. The Ludusavi catalog, keyed by the exact slug, which the service
///      answers (offline, instant). Resolves the long tail of catalogued games
///      (Victoria 3, Europa Universalis and the rest).
///   2. Steam's store search, queried with the de-slugified name. This catches
///      games Ludusavi doesn't list at all (e.g. Rust, which has no documented
///      save path) but that still exist on Steam. Best-effort and network-bound;
///      the JS side memoises the answer for the session so it runs at most once
///      per slug.
///
/// Returns `None` when neither source knows the game; the UI keeps the
/// initial-letter tile.
#[tauri::command]
pub async fn steam_app_id_for_slug(app: AppHandle, slug: String) -> Option<u32> {
    if let Some(id) = catalog_app_id(&app, &slug).await {
        return Some(id);
    }
    steam_store_search_app_id(&deslugify(&slug)).await
}

/// Turn a slug back into a search term: `europa-universalis-v` → `europa
/// universalis v`. Good enough for Steam's fuzzy, case-insensitive search.
fn deslugify(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ask Steam's public store search for the app id of the best match for
/// `term`. Returns the top-ranked result's id (Steam orders by relevance, so
/// the canonical game wins over demos and soundtracks). Any failure (offline, a
/// non-200, an empty result set) resolves to `None`.
async fn steam_store_search_app_id(term: &str) -> Option<u32> {
    if term.is_empty() {
        return None;
    }
    let resp = reqwest::Client::new()
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", term), ("l", "english"), ("cc", "us")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let first = json.get("items")?.as_array()?.first()?;
    first.get("id")?.as_u64().map(|v| v as u32)
}

#[cfg(test)]
mod tests {
    use super::{assets_of, capsule_path, marker_still_stands, CoverKey, LOOKUP_STRATEGY};

    #[test]
    fn a_numeric_key_is_a_steam_app_id() {
        assert!(matches!(
            CoverKey::parse("1245620"),
            Some(CoverKey::Steam(1245620))
        ));
        assert_eq!(CoverKey::parse("1245620").unwrap().stem(), "1245620");
    }

    #[test]
    fn a_prefixed_key_is_a_slug() {
        let key = CoverKey::parse("slug-minecraft-java-edition").unwrap();
        assert!(matches!(key, CoverKey::Slug(ref s) if s == "minecraft-java-edition"));
        assert_eq!(key.stem(), "slug-minecraft-java-edition");
    }

    #[test]
    fn numeric_slugs_survive_the_prefix() {
        // The catalog really does list games slugged `2` and `3`. Without the
        // prefix they would parse as Steam app ids and go looking for the art
        // of whatever app happens to hold that number.
        let key = CoverKey::parse("slug-2").unwrap();
        assert!(matches!(key, CoverKey::Slug(ref s) if s == "2"));
        assert_eq!(key.stem(), "slug-2");
    }

    #[test]
    fn a_stem_cannot_escape_the_covers_dir() {
        let key = CoverKey::parse("slug-../../etc/passwd").unwrap();
        assert_eq!(key.stem(), "slug-______etc_passwd");
        assert!(!key.stem().contains('/'));
        assert!(!key.stem().contains(".."));
    }

    #[test]
    fn an_empty_key_resolves_to_nothing() {
        assert!(CoverKey::parse("").is_none());
        assert!(CoverKey::parse("   ").is_none());
        assert!(CoverKey::parse("slug-").is_none());
    }

    /// Payload de `GetItems` para Elden Ring (1245620): layout plano, los
    /// ficheros cuelgan directos de `apps/<id>/`.
    const FLAT: &str = r#"{"response":{"store_items":[{"assets":{
        "asset_url_format": "steam/apps/1245620/${FILENAME}?t=1784684281",
        "header": "header.jpg",
        "library_capsule": "library_600x900.jpg",
        "library_capsule_2x": "library_600x900_2x.jpg"
    }}]}}"#;

    /// Surviving Mars: Relaunched (3215050). The case that started all of this:
    /// every asset under its own hashed directory, so NO guessable URL exists and
    /// without the manifest we ended up on the landscape header.
    const HASHED: &str = r#"{"response":{"store_items":[{"assets":{
        "asset_url_format": "steam/apps/3215050/${FILENAME}?t=1781089207",
        "header": "80132dfeee2f6463f4c71821edf426af6e8fed97/header.jpg",
        "library_capsule": "23c899254b69d740a0de3d3cc10a370b2316c51a/library_600x900.jpg",
        "library_capsule_2x": "23c899254b69d740a0de3d3cc10a370b2316c51a/library_600x900_2x.jpg"
    }}]}}"#;

    fn path_of(payload: &str) -> Option<String> {
        let json: serde_json::Value = serde_json::from_str(payload).unwrap();
        capsule_path(assets_of(&json).unwrap())
    }

    #[test]
    fn resolves_the_flat_layout() {
        assert_eq!(
            path_of(FLAT).unwrap(),
            "steam/apps/1245620/library_600x900_2x.jpg?t=1784684281"
        );
    }

    #[test]
    fn resolves_the_hashed_layout() {
        assert_eq!(
            path_of(HASHED).unwrap(),
            "steam/apps/3215050/23c899254b69d740a0de3d3cc10a370b2316c51a/library_600x900_2x.jpg?t=1781089207"
        );
    }

    #[test]
    fn falls_back_to_the_1x_when_thats_all_there_is() {
        let payload = HASHED.replace("library_capsule_2x", "library_hero_2x");
        assert!(path_of(&payload)
            .unwrap()
            .ends_with("library_600x900.jpg?t=1781089207"));
    }

    #[tokio::test]
    async fn a_marker_from_an_older_search_is_not_believed() {
        // The real 28 Jul case: a build that only tried the guessable URLs marked
        // Europa Universalis V as having no vertical art, and the next build, which
        // DOES know how to find it, never asked again.
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("3450310_600x900.none");

        // Marcador vacio: el que escribian los builds de ese dia.
        tokio::fs::write(&marker, b"").await.unwrap();
        assert!(!marker_still_stands(&marker).await);

        // Marcador de una estrategia anterior.
        tokio::fs::write(&marker, (LOOKUP_STRATEGY - 1).to_string())
            .await
            .unwrap();
        assert!(!marker_still_stands(&marker).await);

        // El de hoy si vale: sin esto volveriamos a preguntar en cada arranque
        // por cada juego que de verdad no tiene caratula vertical.
        tokio::fs::write(&marker, LOOKUP_STRATEGY.to_string())
            .await
            .unwrap();
        assert!(marker_still_stands(&marker).await);
    }

    #[tokio::test]
    async fn a_cached_index_answers_without_the_network() {
        // Guards the shape contract with `web/static/covers.json`: the lookup
        // reads `covers.<slug>`, so a flat `{slug: url}` file, the obvious thing to
        // write by hand, would silently resolve nothing.
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("index.json"),
            br#"{"version":1,"covers":{"minecraft-java-edition":"https://example.test/mc.png"}}"#,
        )
        .await
        .unwrap();

        assert_eq!(
            super::index_lookup("minecraft-java-edition", dir.path()).await,
            Some("https://example.test/mc.png".to_string())
        );
        assert_eq!(
            super::index_lookup("not-in-the-list", dir.path()).await,
            None
        );
    }

    #[tokio::test]
    async fn no_marker_means_ask() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!marker_still_stands(&dir.path().join("nada.none")).await);
    }

    #[test]
    fn no_library_capsule_means_no_path() {
        // Sin capsule vertical el caller escribe el marcador `.none` y se
        // queda con el header: hay que distinguirlo de un fallo de red.
        let payload = HASHED.replace("library_capsule", "library_hero");
        assert!(path_of(&payload).is_none());
    }

    fn encoded(img: image::DynamicImage, format: image::ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), format)
            .unwrap();
        out
    }

    fn dims(bytes: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory(bytes).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn a_frame_lands_on_the_first_step_that_covers_it() {
        assert_eq!(super::thumb_side(36), Some(96));
        assert_eq!(super::thumb_side(96), Some(96));
        assert_eq!(super::thumb_side(97), Some(192));
        assert_eq!(super::thumb_side(768), Some(768));
        assert_eq!(super::thumb_side(769), None);
        assert_eq!(super::thumb_side(0), None);
    }

    #[test]
    fn the_short_side_is_the_one_scaled() {
        assert_eq!(super::thumb_dims(600, 900, 96), Some((96, 144)));
        // The landscape header: in a square frame its height runs out first.
        assert_eq!(super::thumb_dims(460, 215, 96), Some((205, 96)));
        // Within a quarter of the frame, the original is served as stored.
        assert_eq!(super::thumb_dims(600, 900, 480), None);
        assert_eq!(super::thumb_dims(600, 900, 768), None);
    }

    #[test]
    fn a_poster_becomes_a_jpeg_and_transparency_stays_png() {
        let poster = encoded(
            image::DynamicImage::new_rgb8(600, 900),
            image::ImageFormat::Jpeg,
        );
        let (small, ext) = super::make_thumb(&poster, 96).unwrap();
        assert_eq!((ext, dims(&small)), ("jpg", (96, 144)));

        let logo = encoded(
            image::DynamicImage::new_rgba8(900, 900),
            image::ImageFormat::Png,
        );
        let (small, ext) = super::make_thumb(&logo, 192).unwrap();
        assert_eq!((ext, dims(&small)), ("png", (192, 192)));

        assert!(super::make_thumb(&poster, 768).is_none());
        assert!(super::make_thumb(b"not an image", 96).is_none());
    }

    #[test]
    fn a_copy_is_reused_until_the_art_changes() {
        let dir = tempfile::tempdir().unwrap();
        let names = || {
            let mut v: Vec<String> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            v.sort();
            v
        };
        let first = encoded(
            image::DynamicImage::new_rgb8(600, 900),
            image::ImageFormat::Jpeg,
        );
        let copy = super::sized_copy(dir.path(), "slug-x", 96, first.clone());
        assert_eq!(dims(&copy), (96, 144));
        let made = names();
        assert_eq!(made.len(), 1);

        // Served from disk, not scaled again.
        std::fs::write(dir.path().join(&made[0]), b"from disk").unwrap();
        assert_eq!(
            super::sized_copy(dir.path(), "slug-x", 96, first),
            b"from disk"
        );

        // A game whose stem happens to end like a size step is none of its business.
        let neighbour = format!("slug-x_s96_s96_{:016x}.jpg", 1);
        std::fs::write(dir.path().join(&neighbour), b"x").unwrap();

        // New art for the same game (a custom cover, say): new copy, old one gone.
        let second = encoded(
            image::DynamicImage::new_rgb8(900, 1350),
            image::ImageFormat::Jpeg,
        );
        super::sized_copy(dir.path(), "slug-x", 96, second);
        let now = names();
        assert_eq!(now.len(), 2);
        assert!(now.contains(&neighbour));
        assert!(!now.contains(&made[0]));
    }
}
