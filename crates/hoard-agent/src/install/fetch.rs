//! Downloading and applying a component: which release file this machine needs,
//! how it is checked to be ours, and how it gets put in place.
//!
//! It lives in the agent rather than in a frontend because every road in uses
//! it: Hoard Setup, `hoard install`, `hoard upgrade` and the service's own
//! updater. They are the same operation, and writing it four times is how one
//! of them ends up putting a different Hoard on the machine than the others.
//!
//! None of this opens a window or needs one: naming a file, checking its
//! signature and moving it into place is ordinary business logic.
//!
//! ## Names, not listings
//!
//! Every file a release carries for us has a name fixed by CI and rebuilt here
//! from the version, the system and the architecture ([`core_asset_name`],
//! [`app_asset_name`], [`setup_asset_name`]). Nothing asks GitHub's API what the
//! release contains. The API allows 60 unauthenticated requests an hour per
//! address, and an office or a classroom behind one address goes through that
//! in a morning; every install that hit the limit failed with a 403 that said
//! nothing about why. Downloads by URL and the redirect behind
//! `releases/latest` ([`latest_version`]) have no such quota.
//!
//! The listing survives in one place: releases before these names existed
//! called the app after the bundler (`Hoard_1.2.1_amd64.AppImage`), and pinning
//! one of those (`hoard upgrade --version 1.2.1`) has to find it by suffix.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::Delivery;

/// Repo de las releases. Mismo que resuelven `install.sh` / `install.ps1`.
const REPO: &str = "rleeon/hoard";

/// Downloads from somewhere other than GitHub: a mirror, or a test server.
/// Whatever it serves still has to verify against [`MINISIGN_PUBKEY`], so
/// pointing it elsewhere cannot get anything installed that CI did not sign.
pub const RELEASES_URL_ENV: &str = "HOARD_RELEASES_URL";

fn releases_url() -> String {
    std::env::var(RELEASES_URL_ENV)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| v.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases"))
}

/// The minisign public key CI signs everything published with (ADR 0017; the
/// signing job is isolated from the building one so no third-party dependency ever
/// sees the private key).
///
/// A binary without a valid signature is not installed. It is the only real
/// defence between "I download an executable off the internet" and "I run it":
/// GitHub's TLS says the file arrived intact, not that we published it.
pub const MINISIGN_PUBKEY: &str = "RWSeOL1nHXZI9oa+WOdrc6yVasLPeBurvGWnERo4tN9F+YIQn7ipx3eO";

/// Un fichero publicado en la release.
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(default)]
    tag_name: String,
}

fn client(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(concat!("hoard/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

/// A release's files, through the API. `None` as the version means the latest
/// published one. Only for releases older than the fixed names; see the module
/// header.
pub async fn release_assets(version: Option<&str>) -> Result<(String, Vec<Asset>)> {
    let url = match version {
        Some(v) => format!(
            "https://api.github.com/repos/{REPO}/releases/tags/v{}",
            v.trim_start_matches('v')
        ),
        None => format!("https://api.github.com/repos/{REPO}/releases/latest"),
    };
    let resp = client(Duration::from_secs(30))?
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("asking GitHub for {url}"))?;
    if !resp.status().is_success() {
        bail!(
            "GitHub answered {} for {url}; the release may not be published yet",
            resp.status()
        );
    }
    let rel: Release = resp.json().await.context("parsing the release")?;
    Ok((rel.tag_name.trim_start_matches('v').to_string(), rel.assets))
}

/// The latest published version, read from where `releases/latest` redirects.
pub async fn latest_version() -> Result<String> {
    let url = format!("{}/latest", releases_url());
    let no_follow = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("hoard/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    if let Ok(resp) = no_follow.get(&url).send().await {
        let found = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|l| l.to_str().ok())
            .and_then(version_from_tag_url);
        if let Some(version) = found {
            return Ok(version);
        }
    }
    // A mirror that serves files but doesn't redirect, or a GitHub hiccup: the
    // API still knows, quota permitting.
    let (version, _) = release_assets(None)
        .await
        .context("finding out which release is the latest")?;
    Ok(version)
}

/// `.../releases/tag/v1.2.3` → `1.2.3`.
fn version_from_tag_url(location: &str) -> Option<String> {
    let tag = location.trim_end_matches('/').rsplit('/').next()?;
    let version = tag.strip_prefix('v').unwrap_or(tag);
    version
        .chars()
        .next()
        .filter(char::is_ascii_digit)
        .map(|_| version.to_string())
}

/// Where a release's file is downloaded from.
pub fn asset_url(version: &str, name: &str) -> String {
    format!(
        "{}/download/v{}/{name}",
        releases_url(),
        version.trim_start_matches('v')
    )
}

fn os_name() -> Option<&'static str> {
    match std::env::consts::OS {
        "linux" => Some("linux"),
        "macos" => Some("macos"),
        "windows" => Some("windows"),
        _ => None,
    }
}

fn arch_name() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("x86_64"),
        "aarch64" => Some("aarch64"),
        _ => None,
    }
}

/// How a release names the app for this machine. `None` for a route that has
/// no file of ours (a package from before, a Flatpak).
pub fn app_asset_name(delivery: Delivery, version: &str) -> Option<String> {
    let v = version.trim_start_matches('v');
    let arch = arch_name()?;
    let (os, ext) = match delivery {
        Delivery::AppImage => ("linux", "AppImage"),
        Delivery::Exe => ("windows", "exe"),
        Delivery::Dmg => ("macos", "dmg"),
        _ => return None,
    };
    Some(format!("hoard-desktop-{v}-{os}-{arch}.{ext}"))
}

/// How a release names Hoard Setup for this machine. No version in it: the
/// download page links `releases/latest/download/<this>`.
pub fn setup_asset_name() -> Option<String> {
    let arch = arch_name()?;
    match os_name()? {
        "windows" => Some(format!("HoardSetup-{arch}.exe")),
        "linux" => Some(format!("HoardSetup-{arch}")),
        "macos" => Some(format!("HoardSetup-{arch}.zip")),
        _ => None,
    }
}

/// The file isn't in that release. Its own type so a caller can fall back to
/// the old names instead of failing.
#[derive(Debug)]
pub struct NotPublished {
    pub name: String,
}

impl std::fmt::Display for NotPublished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not published in that release", self.name)
    }
}

impl std::error::Error for NotPublished {}

async fn get(client: &reqwest::Client, url: &str, name: &str) -> Result<bytes::Bytes> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("downloading {name}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(NotPublished {
            name: name.to_string(),
        }
        .into());
    }
    Ok(resp
        .error_for_status()
        .with_context(|| format!("downloading {name}"))?
        .bytes()
        .await?)
}

/// Downloads `name` from release `version` and its `.minisig`, checks one
/// against the other and leaves the file in `dest_dir`.
///
/// It fails closed: no signature published, or one that does not match, and
/// nothing gets written.
pub async fn download_named(version: &str, name: &str, dest_dir: &Path) -> Result<PathBuf> {
    let url = asset_url(version, name);
    let client = client(Duration::from_secs(600))?;
    let bytes = get(&client, &url, name).await?;
    let sig_name = format!("{name}.minisig");
    let sig = get(&client, &format!("{url}.minisig"), &sig_name)
        .await
        .with_context(|| format!("{name} has no published signature. Refusing to install it."))?;
    let sig = String::from_utf8_lossy(&sig);
    verify(&bytes, &sig).with_context(|| format!("verifying {name}"))?;
    write_into(dest_dir, name, &bytes)
}

fn write_into(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// The app for `delivery` at `version`, verified, in `dest_dir`.
pub async fn download_app(delivery: Delivery, version: &str, dest_dir: &Path) -> Result<PathBuf> {
    let name = app_asset_name(delivery, version).with_context(|| {
        format!(
            "there is no {} build of the app for this machine",
            delivery.as_str()
        )
    })?;
    match download_named(version, &name, dest_dir).await {
        Err(err) if err.is::<NotPublished>() => {
            // A release from before the fixed names. Only the AppImage and the
            // `.dmg` existed then under a name we can still recognise.
            let (released, assets) = release_assets(Some(version)).await?;
            let asset = asset_for(delivery, &assets).with_context(|| {
                format!(
                    "release {released} publishes no {} build",
                    delivery.as_str()
                )
            })?;
            download_verified(asset, &assets, dest_dir).await
        }
        other => other,
    }
}

/// The app's file in a release listing, by the suffix the bundler gave it.
/// For releases older than [`app_asset_name`].
pub fn asset_for(delivery: Delivery, assets: &[Asset]) -> Option<&Asset> {
    let suffix = match delivery {
        Delivery::AppImage => ".AppImage",
        Delivery::Dmg => ".dmg",
        _ => return None,
    };
    let candidates: Vec<&Asset> = assets
        .iter()
        .filter(|a| a.name.ends_with(suffix) && !is_our_installer(a))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    pick_for_arch(&candidates)
}

/// Is this asset the graphical installer rather than a copy of the app?
///
/// Both ship in every release and their suffixes collide; taking the installer
/// for the app would have the updater run it as an update, and the user would
/// get a window asking them to install what they already have. CI publishes
/// ours as `HoardSetup-<arch>[.ext]`; the check is the contract.
fn is_our_installer(asset: &Asset) -> bool {
    asset.name.to_ascii_lowercase().starts_with("hoardsetup")
}

/// The tokens the bundlers use to name our architecture.
fn arch_tokens() -> &'static [&'static str] {
    match std::env::consts::ARCH {
        "x86_64" => &["x86_64", "amd64", "x64"],
        "aarch64" => &["aarch64", "arm64"],
        _ => &[],
    }
}

/// Every architecture token we know how to recognise, ours or not.
const KNOWN_ARCH_TOKENS: &[&str] = &["x86_64", "amd64", "x64", "aarch64", "arm64"];

/// Out of several files of the same format, the one for this architecture.
///
/// If no candidate carries an architecture token, the release does not
/// distinguish and the first one will do. If they do carry one but none is ours,
/// `None` comes back on purpose: "there is no build for your architecture" is a
/// useful answer, and installing somebody else's is not.
fn pick_for_arch<'a>(candidates: &[&'a Asset]) -> Option<&'a Asset> {
    let ours = arch_tokens();
    if let Some(hit) = candidates
        .iter()
        .find(|a| ours.iter().any(|t| contains_token(&a.name, t)))
    {
        return Some(hit);
    }
    let any_tagged = candidates
        .iter()
        .any(|a| KNOWN_ARCH_TOKENS.iter().any(|t| contains_token(&a.name, t)));
    if any_tagged {
        return None;
    }
    candidates.first().copied()
}

/// Does `token` appear as a piece of the name rather than inside another word?
///
/// The boundaries are checked instead of splitting on separators, and that is not
/// a matter of style: the most important token, `x86_64`, has a `_` inside it, so
/// splitting on `_` destroys it before it can be looked for. A boundary is
/// anything non-alphanumeric, or the start or end of the name.
fn contains_token(name: &str, token: &str) -> bool {
    let hay = name.to_ascii_lowercase();
    let needle = token.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(&needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// [`download_named`] for an asset out of a listing.
pub async fn download_verified(
    asset: &Asset,
    assets: &[Asset],
    dest_dir: &Path,
) -> Result<PathBuf> {
    let sig_name = format!("{}.minisig", asset.name);
    let sig = assets
        .iter()
        .find(|a| a.name == sig_name)
        .with_context(|| {
            format!(
                "{} has no published signature ({sig_name}). Refusing to install it.",
                asset.name
            )
        })?;
    let client = client(Duration::from_secs(600))?;
    let bytes = get(&client, &asset.url, &asset.name).await?;
    let sig_text = get(&client, &sig.url, &sig_name).await?;
    verify(&bytes, &String::from_utf8_lossy(&sig_text))
        .with_context(|| format!("verifying {}", asset.name))?;
    write_into(dest_dir, &asset.name, &bytes)
}

/// Comprueba `bytes` contra la firma minisign `sig_text` con [`MINISIGN_PUBKEY`].
pub fn verify(bytes: &[u8], sig_text: &str) -> Result<()> {
    use minisign_verify::{PublicKey, Signature};

    let pubkey = PublicKey::from_base64(MINISIGN_PUBKEY)
        .map_err(|e| anyhow::anyhow!("the embedded public key is unusable: {e}"))?;
    let signature = Signature::decode(sig_text)
        .map_err(|e| anyhow::anyhow!("the signature file is malformed: {e}"))?;
    pubkey
        .verify(bytes, &signature, false)
        .map_err(|e| anyhow::anyhow!("signature does not match: {e}"))?;
    Ok(())
}

/// Puts the already-verified app in place and returns where it landed.
///
/// No route here asks for privileges any more. The AppImage and the Windows
/// executable go into the user's own directories and the `.app` into
/// `/Applications` (or `~/Applications`), so an update applies with nobody in
/// front of the machine, the same way on all three systems. The native packages
/// are only ever removed now ([`super::legacy`]).
pub async fn apply_desktop(delivery: Delivery, path: &Path) -> Result<PathBuf> {
    match delivery {
        Delivery::AppImage => place_appimage(path),
        Delivery::Exe => place_exe(path),
        Delivery::Dmg => install_dmg(path).await,
        Delivery::Managed => {
            bail!("this install is managed by your package manager; nothing to do")
        }
        Delivery::Deb | Delivery::Rpm | Delivery::Nsis | Delivery::Msi => bail!(
            "Hoard no longer installs {} packages; Hoard Setup moves this machine off it",
            delivery.as_str()
        ),
    }
}

/// The Windows app: one executable, renamed into place so a running copy is
/// parked instead of blocking the write.
#[cfg(windows)]
fn place_exe(downloaded: &Path) -> Result<PathBuf> {
    let dest = super::layout::app_path()?;
    let dir = dest.parent().context("the app path has no parent")?;
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let staging = dest.with_extension("new");
    let _ = std::fs::remove_file(&staging);
    std::fs::copy(downloaded, &staging)
        .with_context(|| format!("staging the app at {}", staging.display()))?;
    // A program copied out of Downloads brings its "from the internet" mark
    // along, and Windows would ask about it on every start from the menu.
    let mut mark = staging.clone().into_os_string();
    mark.push(":Zone.Identifier");
    let _ = std::fs::remove_file(mark);
    replace_binary(&staging, &dest)?;
    Ok(dest)
}

#[cfg(not(windows))]
fn place_exe(path: &Path) -> Result<PathBuf> {
    bail!(
        "{} is a Windows program and this isn't Windows",
        path.display()
    )
}

/// Mounts the `.dmg`, copies the `.app` into `/Applications`, unmounts.
///
/// Mounted with `-nobrowse` so a Finder window doesn't open on top of ours, and
/// unmounted whatever happens above: a volume nobody detaches sits on the
/// user's desktop until they reboot.
#[cfg(target_os = "macos")]
async fn install_dmg(path: &Path) -> Result<PathBuf> {
    use tokio::process::Command;

    let out = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-plist"])
        .arg(path)
        .output()
        .await
        .context("running `hdiutil attach`")?;
    if !out.status.success() {
        bail!(
            "`hdiutil attach` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let mount = mount_point(&String::from_utf8_lossy(&out.stdout)).with_context(|| {
        format!(
            "{} mounted but no volume came back from hdiutil",
            path.display()
        )
    })?;

    let result = copy_app_out(&mount).await;

    // Whatever happened above, the volume gets detached. `-force` because
    // Spotlight may still be indexing it and a polite unmount would fail.
    let _ = Command::new("hdiutil")
        .args(["detach", "-force"])
        .arg(&mount)
        .output()
        .await;

    result
}

/// The mount point inside the plist `hdiutil attach -plist` prints.
///
/// No plist parser: we look for the `<string>` that starts with `/Volumes/`,
/// the only field in that output shaped like this. A whole XML dependency to
/// read one path would be expensive for the same answer.
#[cfg(target_os = "macos")]
fn mount_point(plist: &str) -> Option<PathBuf> {
    plist
        .split("<string>")
        .skip(1)
        .filter_map(|rest| rest.split("</string>").next())
        .map(str::trim)
        .find(|s| s.starts_with("/Volumes/"))
        .map(PathBuf::from)
}

/// Copies the volume's single `.app` into `/Applications`, or into the home
/// directory when that folder isn't ours.
///
/// `ditto` rather than `cp -R`: it preserves extended attributes and code
/// signatures, and a bundle copied without them is a bundle Gatekeeper refuses
/// to open. The destination is removed first instead of copied over, because an
/// update that leaves stray files from the previous version inside the bundle
/// produces failures that look nothing like their cause.
#[cfg(target_os = "macos")]
async fn copy_app_out(mount: &Path) -> Result<PathBuf> {
    use tokio::process::Command;

    let app = std::fs::read_dir(mount)
        .with_context(|| format!("listing {}", mount.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .with_context(|| format!("no .app inside {}", mount.display()))?;
    let name = app
        .file_name()
        .context("the .app bundle has no name")?
        .to_owned();

    // `/Applications` is the right answer and isn't always writable (an account
    // without admin rights). `~/Applications` always is, and Launchpad looks
    // there too, so the fallback is a complete install and not a consolation
    // prize.
    let mut targets = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        targets.push(home.join("Applications"));
    }

    let mut last = String::from("no destination directory");
    for dir in targets {
        if std::fs::create_dir_all(&dir).is_err() && !dir.is_dir() {
            last = format!("{} is not writable", dir.display());
            continue;
        }
        let dest = dir.join(&name);
        if dest.exists() {
            if let Err(e) = std::fs::remove_dir_all(&dest) {
                last = format!("replacing {}: {e}", dest.display());
                continue;
            }
        }
        let out = Command::new("ditto")
            .arg(&app)
            .arg(&dest)
            .output()
            .await
            .context("running `ditto`")?;
        if out.status.success() {
            return Ok(dest);
        }
        last = format!(
            "copying to {}: {}",
            dest.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    bail!("could not install the app bundle: {last}")
}

/// Off macOS a `.dmg` is installable by nobody. We never get here:
/// [`super::resolve_delivery`] only picks it on macOS, and this exists so
/// [`apply_desktop`]'s match stays total on the other platforms.
#[cfg(not(target_os = "macos"))]
async fn install_dmg(path: &Path) -> Result<PathBuf> {
    bail!(
        "{} is a macOS disk image and this isn't macOS",
        path.display()
    )
}

/// `pkexec` ran nothing because its password prompt closed without an answer.
/// A Cancel does that, and so does GNOME Shell when it cannot draw the prompt at
/// all: "Failed to show modal dialog. Dismissing authentication request", which
/// is how the 1.2.0 Linux installer failed its first attempt on a machine where
/// the second one went through. A type of its own, so a window can say it in the
/// user's language next to the button that asks again.
#[derive(Debug)]
pub struct PromptDismissed;

impl std::fmt::Display for PromptDismissed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the password prompt closed before it was answered; try again")
    }
}

impl std::error::Error for PromptDismissed {}

/// Runs a package manager with privileges, to take an old package off:
/// `pkexec` first (it draws its own dialog and does not depend on a terminal),
/// `sudo -n` otherwise. Somebody at a terminal with no desktop of their own (an
/// SSH session, a text console) gets a plain `sudo`, which asks right there:
/// pkexec could only ask in text, and over SSH polkit 127 fails before asking
/// ("No session for cookie", its helper now runs as a systemd service that
/// can't find the session).
///
/// Neither route may sit waiting for a human, which is what would happen inside
/// `curl ... | sh`, where stdin is the script itself: `sudo` always carries `-n`,
/// so with no cached credential it fails instantly. `noninteractive` rules out
/// `pkexec` as well: `$DISPLAY` can be set with no polkit agent listening (SSH
/// with X11 forwarding), and then it waits for a dialog nobody draws.
pub(super) async fn elevated_argv(argv: &[String], noninteractive: bool) -> Result<()> {
    let argv: Vec<String> = argv.to_vec();

    let is_root = {
        #[cfg(unix)]
        {
            // SAFETY: `geteuid` no toma argumentos y no falla.
            unsafe { libc::geteuid() == 0 }
        }
        #[cfg(not(unix))]
        {
            false
        }
    };

    let (program, args): (String, Vec<String>) = if is_root {
        (argv[0].clone(), argv[1..].to_vec())
    } else if !noninteractive && which("sudo") && terminal_without_desktop() {
        ("sudo".into(), argv)
    } else if which("pkexec") && !noninteractive {
        ("pkexec".into(), argv)
    } else if which("sudo") {
        let mut a = vec!["-n".to_string()];
        a.extend(argv);
        ("sudo".into(), a)
    } else if noninteractive {
        bail!(
            "this package needs privileges and nothing here can grant them without asking \
             (run `hoard install` yourself from a terminal, or use `--headless`)"
        )
    } else {
        bail!("no way to get the privileges this package needs (no pkexec, no sudo)")
    };

    let status = tokio::process::Command::new(&program)
        .args(&args)
        .status()
        .await
        .with_context(|| format!("running `{program}`"))?;
    if !status.success() {
        // 126 is pkexec saying the prompt was dismissed; dpkg and rpm have no
        // use for that code, so here it cannot be theirs.
        if program == "pkexec" && status.code() == Some(126) {
            return Err(PromptDismissed.into());
        }
        bail!("`{program} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

/// An X11 forwarded over SSH sets `DISPLAY` with no polkit agent behind it.
fn terminal_without_desktop() -> bool {
    use std::io::IsTerminal;
    let set = |key: &str| std::env::var_os(key).is_some_and(|v| !v.is_empty());
    std::io::stdin().is_terminal()
        && (set("SSH_CONNECTION") || !(set("DISPLAY") || set("WAYLAND_DISPLAY")))
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(name).is_file()))
        .unwrap_or(false)
}

/// Puts the AppImage in the home and gives it a menu entry.
///
/// No `sudo` and no package manager: it is the route that works where the other
/// two cannot (SteamOS, Bazzite, Arch). The engine does NOT go inside it, since
/// the installer put it on a stable path, which is what lets this machine's sync
/// start at boot even though the app is an AppImage.
fn place_appimage(downloaded: &Path) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("no HOME in the environment")?;
    let bin = home.join(".local").join("bin");
    std::fs::create_dir_all(&bin).with_context(|| format!("creating {}", bin.display()))?;
    let dest = bin.join("hoard-desktop");

    // It writes to a temporary and renames over, never a direct `copy`: if the app
    // is open, and updating from the app itself is the normal case, the kernel
    // refuses to write over its own executable with `ETXTBSY`. A `rename` over a
    // running binary is fine: the live process keeps its inode and the name comes
    // to point at the new one. And it is atomic into the bargain, so a failure
    // halfway does not leave a truncated AppImage where a working one was.
    let staging = bin.join(".hoard-desktop.new");
    let _ = std::fs::remove_file(&staging);
    std::fs::copy(downloaded, &staging)
        .with_context(|| format!("staging the AppImage at {}", staging.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("making {} executable", staging.display()))?;
    }
    if let Err(e) = std::fs::rename(&staging, &dest) {
        let _ = std::fs::remove_file(&staging);
        return Err(e).with_context(|| format!("installing the AppImage at {}", dest.display()));
    }
    write_desktop_entry(&home, &dest)?;
    Ok(dest)
}

/// The name the menu entry asks the icon theme for. The `.deb` and the `.rpm`
/// install theirs under this one, and it is what `hoardd` hands the notification
/// server, so an AppImage install answers to the same name as a packaged one.
const MENU_ICON_NAME: &str = "hoard-desktop";

const MENU_ICON: &[u8] = include_bytes!("../../../hoard-desktop/icons/128x128@2x.png");

/// Where the menu icon goes: the user's own `hicolor` tree, the one place a
/// launcher searches that needs no privileges to write.
pub(super) fn menu_icon_path(home: &Path) -> PathBuf {
    home.join(".local")
        .join("share")
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps")
        .join(format!("{MENU_ICON_NAME}.png"))
}

/// The menu entry. Without it the AppImage exists but cannot be launched from
/// anywhere but a terminal, and in gaming mode that is not existing.
fn write_desktop_entry(home: &Path, exe: &Path) -> Result<()> {
    write_menu_icon(home)?;
    let dir = home.join(".local").join("share").join("applications");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Hoard\n\
         Comment=Game save sync\n\
         Exec=\"{}\" %u\n\
         Icon={MENU_ICON_NAME}\n\
         StartupWMClass=hoard-desktop\n\
         MimeType=x-scheme-handler/hoard;\n\
         Terminal=false\n\
         Categories=Utility;Game;\n",
        exe.display()
    );
    let path = dir.join("dev.hoard.desktop.desktop");
    std::fs::write(&path, entry).with_context(|| format!("writing {}", path.display()))?;
    // The `.deb` registered `hoard://` system-wide, and it goes away with the
    // move to the AppImage. Refreshing the cache is what makes this entry the
    // handler; best-effort, since the login's loopback doesn't need it.
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    Ok(())
}

/// Puts the icon where the entry above can find it.
///
/// The AppImage carries its icons inside itself, where no launcher looks, and
/// the entry used to name one that nothing had installed: the menu drew a blank
/// page beside "Hoard" on every AppImage install (Arch, CachyOS, SteamOS).
fn write_menu_icon(home: &Path) -> Result<()> {
    let path = menu_icon_path(home);
    let dir = path.parent().context("the icon path has no parent")?;
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    std::fs::write(&path, MENU_ICON).with_context(|| format!("writing {}", path.display()))?;

    // Toolkits decide whether to rescan a theme by the mtime of its top
    // directory, and a file added three levels down does not move it. Without
    // this a launcher that is already running keeps the blank icon until the
    // next login.
    if let Some(theme) = dir.parent().and_then(Path::parent) {
        if let Ok(handle) = std::fs::File::open(theme) {
            let _ = handle.set_modified(std::time::SystemTime::now());
        }
    }
    Ok(())
}

/// Points the app's start at login at `app`, keeping its arguments.
///
/// The app writes that entry itself, with the path it ran from, so after a
/// `.deb` or `.rpm` is taken off it still names `/usr/bin/hoard-desktop`. The
/// app then stopped starting at login until somebody opened it by hand.
#[cfg(target_os = "linux")]
pub fn repoint_app_autostart(app: &Path) {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let entry = home.join(".config").join("autostart").join("Hoard.desktop");
    let Ok(text) = std::fs::read_to_string(&entry) else {
        return;
    };
    if let Some(text) = repointed_autostart(&text, app) {
        let _ = std::fs::write(&entry, text);
    }
}

#[cfg(target_os = "linux")]
fn repointed_autostart(text: &str, app: &Path) -> Option<String> {
    const APP: &str = "hoard-desktop";
    if text.contains("flatpak") {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        let new = body.strip_prefix("Exec=").and_then(|value| {
            let at = value.rfind(APP)?;
            let args = value[at + APP.len()..].trim_start_matches('"').trim();
            // The same shape the app writes, so its own rewrite at the next
            // start changes nothing.
            let exec = format!("Exec={} {args}", app.display());
            Some(exec.trim_end().to_string())
        });
        match new {
            Some(new) => {
                out.push_str(&new);
                out.push_str(&line[body.len()..]);
            }
            None => out.push_str(line),
        }
    }
    (out != text).then_some(out)
}

// ---- the core: `hoardd` plus `hoard`, with no shell installer
//
// `hoard upgrade` used to relieve the core by piping `curl ... | sh` into an
// interpreter. That works for a person typing in a terminal and for nothing else:
// the service has no terminal, a `curl` that is not installed turns "it updates
// itself" into "it does not update and does not say so", and the script ends by
// calling `hoard install`, meaning a process we have just replaced on disk. What
// follows is the same operation written where it can be supervised: download the
// tarball, check its signature, and replace the two binaries.
//
// None of this asks for privileges: the core only ever lives in
// [`super::layout::core_dir`], which is the user's own.

/// How a release names this machine's core tarball. Fixed by CI and also
/// built by `install.sh` and `install.ps1`.
pub fn core_asset_name(version: &str) -> Option<String> {
    Some(format!(
        "hoard-{}-{}-{}.tar.gz",
        version.trim_start_matches('v'),
        os_name()?,
        arch_name()?
    ))
}

/// The core tarball inside an already-listed release.
pub fn core_asset_for<'a>(version: &str, assets: &'a [Asset]) -> Option<&'a Asset> {
    let want = core_asset_name(version)?;
    assets.iter().find(|a| a.name == want)
}

/// The two binaries that make up the core. The order does not matter here; what
/// matters is that both are present before anything is touched (see
/// [`apply_core`]).
const CORE_BINARIES: [&str; 2] = ["hoard", "hoardd"];

/// Pulls `hoard` and `hoardd` out of the already-verified tarball and leaves them
/// in `dir`, replacing whatever was there.
///
/// Returns the paths written.
///
/// All or nothing: first both are extracted to files beside them, and only once
/// both are whole on disk are they renamed over. Half an update, a new `hoard`
/// talking to an old `hoardd`, is worse than none, because the handshake tolerates
/// it and the mismatch says nothing.
pub async fn apply_core(tarball: &Path, dir: &Path) -> Result<Vec<PathBuf>> {
    let bytes = tokio::fs::read(tarball)
        .await
        .with_context(|| format!("reading {}", tarball.display()))?;
    let staged = extract_core(&bytes, dir).await?;

    let mut written = Vec::new();
    for (name, temp) in staged {
        let dest = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        replace_binary(&temp, &dest).with_context(|| format!("replacing {}", dest.display()))?;
        written.push(dest);
    }
    Ok(written)
}

/// Extracts both binaries to `<dir>/.<name>.new`. It fails when either is missing,
/// and cleans up whatever it had written before failing.
async fn extract_core(tarball: &[u8], dir: &Path) -> Result<Vec<(&'static str, PathBuf)>> {
    use async_compression::tokio::bufread::GzipDecoder;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let decoder = GzipDecoder::new(tokio::io::BufReader::new(std::io::Cursor::new(tarball)));
    let mut archive = tokio_tar::Archive::new(decoder);
    let mut entries = archive.entries().context("reading the core tarball")?;

    let mut found: Vec<(&'static str, PathBuf)> = Vec::new();
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading a tarball entry")?;
        let path = entry
            .path()
            .context("a tarball entry has no path")?
            .into_owned();
        // The tarball carries a root directory (`hoard-<ver>-<platform>/`), but
        // that is not assumed: only the last component is looked at, so a change
        // in packaging does not break the update silently.
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.trim_end_matches(".exe"));
        let Some(name) = CORE_BINARIES.iter().find(|w| Some(**w) == stem) else {
            continue;
        };
        if found.iter().any(|(n, _)| n == name) {
            continue;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .await
            .with_context(|| format!("reading {name} out of the tarball"))?;

        let temp = dir.join(format!(".{name}.new"));
        let _ = std::fs::remove_file(&temp);
        let mut out = tokio::fs::File::create(&temp)
            .await
            .with_context(|| format!("creating {}", temp.display()))?;
        out.write_all(&buf).await?;
        out.flush().await?;
        drop(out);
        make_executable(&temp)?;
        found.push((name, temp));
    }

    if found.len() != CORE_BINARIES.len() {
        for (_, temp) in &found {
            let _ = std::fs::remove_file(temp);
        }
        let missing: Vec<&str> = CORE_BINARIES
            .iter()
            .copied()
            .filter(|w| !found.iter().any(|(n, _)| n == w))
            .collect();
        bail!(
            "the release tarball is missing {}; refusing to install half of the core",
            missing.join(" and ")
        );
    }
    Ok(found)
}

/// Copies `hoard` and `hoardd` from `src` into `dest`, all or nothing, the same
/// way [`apply_core`] does from a tarball.
///
/// This is `hoard install` run out of the folder the terminal installer just
/// unpacked: the binaries are already here and already checked against the
/// release's SHA-256, so there is nothing to download.
pub fn place_core_from(src: &Path, dest: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    let mut staged = Vec::new();
    for name in CORE_BINARIES {
        let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
        let from = src.join(&file);
        let temp = dest.join(format!(".{name}.new"));
        let _ = std::fs::remove_file(&temp);
        if let Err(err) = std::fs::copy(&from, &temp)
            .with_context(|| format!("copying {}", from.display()))
            .and_then(|_| make_executable(&temp))
        {
            for (_, t) in &staged {
                let _ = std::fs::remove_file(t);
            }
            let _ = std::fs::remove_file(&temp);
            return Err(err);
        }
        staged.push((file, temp));
    }
    let mut written = Vec::new();
    for (file, temp) in staged {
        let to = dest.join(file);
        replace_binary(&temp, &to).with_context(|| format!("replacing {}", to.display()))?;
        written.push(to);
    }
    Ok(written)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod +x {}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Puts `src` where `dest` was, with `dest` running.
///
/// It is the only place this can be done properly, and each system does it for a
/// different reason:
///
/// - Unix: a `rename` over a live process's executable is legal. The process keeps
///   its inode (it carries on running the old binary until it restarts) and the
///   name comes to point at the new one. Writing *inside* the file is not: the
///   kernel answers `ETXTBSY`, which is what broke a direct copy.
/// - Windows: an open `.exe` cannot be replaced, but it can be renamed. So the old
///   one is moved aside and the new one takes its name; the one moved aside is
///   deleted on the next pass, once nobody holds it open.
pub(super) fn replace_binary(src: &Path, dest: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // Sweeping the previous attempt. Best-effort: if the old process is still
        // alive it will still refuse to be deleted, and that is fine.
        let parked = dest.with_extension("old");
        let _ = std::fs::remove_file(&parked);
        if dest.exists() {
            std::fs::rename(dest, &parked).with_context(|| {
                format!(
                    "parking the running {}; is another copy still starting?",
                    dest.display()
                )
            })?;
        }
    }
    std::fs::rename(src, dest)
        .with_context(|| format!("moving {} into place as {}", src.display(), dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.to_string(),
            url: format!("https://example.invalid/{name}"),
        }
    }

    /// How the bundlers spell *this* machine's architecture, so a fixture
    /// describes a release that actually has a file for the runner.
    fn bundler_arch() -> &'static str {
        match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            _ => "amd64",
        }
    }

    #[test]
    fn the_latest_version_comes_out_of_the_redirect() {
        assert_eq!(
            version_from_tag_url("https://github.com/rleeon/hoard/releases/tag/v1.2.2"),
            Some("1.2.2".into())
        );
        assert_eq!(
            version_from_tag_url("/rleeon/hoard/releases/tag/1.3.0/"),
            Some("1.3.0".into())
        );
        // A redirect somewhere else (a login page, a mirror's index) is not a version.
        assert_eq!(version_from_tag_url("https://github.com/login"), None);
    }

    #[test]
    fn every_file_of_ours_has_a_name_built_from_the_version() {
        let arch = std::env::consts::ARCH;
        assert_eq!(
            app_asset_name(Delivery::AppImage, "v1.2.2"),
            Some(format!("hoard-desktop-1.2.2-linux-{arch}.AppImage"))
        );
        assert_eq!(
            app_asset_name(Delivery::Exe, "1.2.2"),
            Some(format!("hoard-desktop-1.2.2-windows-{arch}.exe"))
        );
        assert_eq!(
            app_asset_name(Delivery::Dmg, "1.2.2"),
            Some(format!("hoard-desktop-1.2.2-macos-{arch}.dmg"))
        );
        for old in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::Nsis,
            Delivery::Msi,
            Delivery::Managed,
        ] {
            assert_eq!(app_asset_name(old, "1.2.2"), None, "{old:?}");
        }
    }

    /// Hoard Setup's name has to stay out of [`asset_for`]'s way: an old
    /// updater looking for the app must never pick the installer.
    #[test]
    fn the_installer_is_never_taken_for_the_app() {
        let setup = setup_asset_name().expect("supported target");
        assert!(setup.starts_with("HoardSetup-"));
        let arch = bundler_arch();
        let assets = vec![
            asset(&setup),
            asset(&format!("HoardSetup-{}.AppImage", std::env::consts::ARCH)),
            asset(&format!("Hoard_1.2.1_{arch}.AppImage")),
            asset(&format!("HoardSetup-{}.dmg", std::env::consts::ARCH)),
            asset(&format!("Hoard_1.2.1_{arch}.dmg")),
        ];
        for delivery in [Delivery::AppImage, Delivery::Dmg] {
            let picked = asset_for(delivery, &assets).expect("the app is there");
            assert!(
                picked.name.starts_with("Hoard_"),
                "{delivery:?}: {}",
                picked.name
            );
        }
    }

    #[test]
    fn a_signature_is_never_mistaken_for_the_artifact() {
        let assets = vec![
            asset("Hoard_1.2.1_amd64.AppImage.minisig"),
            asset("Hoard_1.2.1_amd64.AppImage.sha256"),
        ];
        assert!(asset_for(Delivery::AppImage, &assets).is_none());
    }

    /// Only the two formats that existed under the bundler's names can be
    /// found that way. Everything else is either gone or never was ours.
    #[test]
    fn only_the_appimage_and_the_dmg_have_old_names() {
        let assets = vec![
            asset("Hoard_1.2.1_amd64.deb"),
            asset("Hoard_1.2.1_x64-setup.exe"),
            asset("Hoard_1.2.1_x64_en-US.msi"),
        ];
        for d in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::Nsis,
            Delivery::Msi,
            Delivery::Exe,
            Delivery::Managed,
        ] {
            assert!(asset_for(d, &assets).is_none(), "{d:?}");
        }
    }

    /// And if the release only carries another architecture, better to say
    /// there is nothing than to install the wrong build.
    #[test]
    fn a_release_for_another_arch_only_reports_nothing() {
        let other = if cfg!(target_arch = "x86_64") {
            "Hoard_1.2.0_aarch64.AppImage"
        } else {
            "Hoard_1.2.0_amd64.AppImage"
        };
        assert!(asset_for(Delivery::AppImage, &[asset(other)]).is_none());
    }

    #[test]
    fn an_untagged_release_still_resolves() {
        assert_eq!(
            asset_for(Delivery::AppImage, &[asset("Hoard.AppImage")]).map(|a| a.name.as_str()),
            Some("Hoard.AppImage")
        );
    }

    /// The token is a piece of the name, not a substring: `x64` must not match
    /// inside `x86_64` or the other way round.
    #[test]
    fn arch_tokens_match_whole_parts_only() {
        // The token with a `_` inside it is the one that broke the first version
        // of this.
        assert!(contains_token("hoard-1.2.0-linux-x86_64.tar.gz", "x86_64"));
        assert!(contains_token("Hoard_1.2.0_x64-setup.exe", "x64"));
        assert!(contains_token("Hoard_1.2.0_amd64.deb", "amd64"));
        assert!(contains_token("Hoard_1.2.0_aarch64.dmg", "aarch64"));
        assert!(!contains_token("Hoard_1.2.0_x86_64.deb", "x64"));
        assert!(!contains_token("Hoard_prearm64x.deb", "arm64"));
    }

    /// It fails closed: junk does not verify against a signature.
    #[test]
    fn a_bogus_signature_does_not_verify() {
        assert!(verify(b"payload", "untrusted comment: nope\nnot-a-signature\n").is_err());
    }

    #[test]
    fn the_core_tarball_is_named_after_this_machine() {
        let name = core_asset_name("1.1.2").expect("this target should be supported");
        assert!(name.starts_with("hoard-1.1.2-"), "{name}");
        assert!(name.ends_with(".tar.gz"), "{name}");
        assert!(name.contains(std::env::consts::ARCH), "{name}");
        // The tag's `v` does not travel in the file name.
        assert_eq!(core_asset_name("v1.1.2"), Some(name));
    }

    /// A tarball laid out the way CI publishes it: a root directory with both
    /// binaries inside. It is built with the same pieces that read it
    /// ([`tokio_tar`] plus `async-compression`'s gzip), so the test does not depend
    /// on a packer other than the one we will actually meet.
    async fn core_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use async_compression::tokio::write::GzipEncoder;
        use tokio::io::AsyncWriteExt;

        let mut tar = tokio_tar::Builder::new(Vec::new());
        for (name, body) in entries {
            let mut header = tokio_tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, name, *body).await.unwrap();
        }
        let raw = tar.into_inner().await.unwrap();

        let mut gz = GzipEncoder::new(Vec::new());
        gz.write_all(&raw).await.unwrap();
        gz.shutdown().await.unwrap();
        gz.into_inner()
    }

    #[tokio::test]
    async fn extracting_the_core_writes_both_halves_next_to_the_old_ones() {
        let dir = tempdir();
        let tarball = core_tarball(&[
            ("hoard-9.9.9-linux-x86_64/hoard", b"new-cli"),
            ("hoard-9.9.9-linux-x86_64/hoardd", b"new-engine"),
        ])
        .await;
        let staged = extract_core(&tarball, &dir).await.unwrap();
        assert_eq!(staged.len(), 2);
        for (name, temp) in &staged {
            assert!(temp.exists(), "{name} was not written");
            assert_eq!(temp.file_name().unwrap(), format!(".{name}.new").as_str());
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn half_a_core_is_refused_and_leaves_nothing_behind() {
        let dir = tempdir();
        // The terminal only: a new `hoard` against an old `hoardd` is the silent
        // mismatch this whole module exists not to create.
        let tarball = core_tarball(&[("hoard-9.9.9-linux-x86_64/hoard", b"new-cli")]).await;
        let err = extract_core(&tarball, &dir).await.unwrap_err();
        assert!(err.to_string().contains("hoardd"), "{err}");
        assert!(
            !dir.join(".hoard.new").exists(),
            "the partial write was kept"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn applying_the_core_replaces_what_was_there() {
        let dir = tempdir();
        let exe = std::env::consts::EXE_SUFFIX;
        std::fs::write(dir.join(format!("hoard{exe}")), b"old-cli").unwrap();
        std::fs::write(dir.join(format!("hoardd{exe}")), b"old-engine").unwrap();

        let tarball = core_tarball(&[
            ("hoard-9.9.9-linux-x86_64/hoard", b"new-cli"),
            ("hoard-9.9.9-linux-x86_64/hoardd", b"new-engine"),
        ])
        .await;
        let tar_path = dir.join("core.tar.gz");
        std::fs::write(&tar_path, &tarball).unwrap();

        let written = apply_core(&tar_path, &dir).await.unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            std::fs::read(dir.join(format!("hoard{exe}"))).unwrap(),
            b"new-cli"
        );
        assert_eq!(
            std::fs::read(dir.join(format!("hoardd{exe}"))).unwrap(),
            b"new-engine"
        );
        // No leftovers: the temporaries were consumed by the rename.
        assert!(!dir.join(".hoard.new").exists());
        assert!(!dir.join(".hoardd.new").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `hoard install` out of the unpacked tarball: both binaries move, or
    /// neither does.
    #[test]
    fn placing_the_core_from_a_folder_needs_both_halves() {
        let src = tempdir().join("src");
        let dest = src.parent().unwrap().join("dest");
        std::fs::create_dir_all(&src).unwrap();
        let exe = std::env::consts::EXE_SUFFIX;
        std::fs::write(src.join(format!("hoard{exe}")), b"cli").unwrap();

        assert!(place_core_from(&src, &dest).is_err());
        assert!(!dest.join(format!("hoard{exe}")).exists());
        assert!(!dest.join(".hoard.new").exists());

        std::fs::write(src.join(format!("hoardd{exe}")), b"engine").unwrap();
        let written = place_core_from(&src, &dest).unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            std::fs::read(dest.join(format!("hoardd{exe}"))).unwrap(),
            b"engine"
        );
        std::fs::remove_dir_all(src.parent().unwrap()).ok();
    }

    /// The entry and the icon it names have to agree, and the icon has to exist:
    /// `Icon=hoard` with no such file anywhere is how AppImage installs got a
    /// blank page in the menu.
    #[test]
    fn the_menu_entry_names_an_icon_that_was_installed() {
        let home = tempdir();
        let exe = home.join(".local").join("bin").join("hoard-desktop");
        write_desktop_entry(&home, &exe).unwrap();

        let entry = std::fs::read_to_string(
            home.join(".local/share/applications/dev.hoard.desktop.desktop"),
        )
        .unwrap();
        let named = entry
            .lines()
            .find_map(|l| l.strip_prefix("Icon="))
            .expect("the entry has an Icon line");
        // It also takes over `hoard://` from the package it replaces.
        assert!(entry.contains("MimeType=x-scheme-handler/hoard;"));
        assert!(entry.contains("%u"));

        let icon = menu_icon_path(&home);
        assert_eq!(icon.file_stem().unwrap(), named);
        assert!(icon.starts_with(home.join(".local/share/icons/hicolor")));
        let bytes = std::fs::read(&icon).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        // The directory name is a promise to the theme about the size inside.
        assert_eq!(&bytes[16..24], [0, 0, 1, 0, 0, 0, 1, 0]);

        // Updating rewrites both over what is already there.
        write_desktop_entry(&home, &exe).unwrap();
        std::fs::remove_dir_all(&home).ok();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_login_entry_follows_the_app_and_keeps_its_arguments() {
        let app = Path::new("/home/u/.local/bin/hoard-desktop");
        let packaged = "[Desktop Entry]\nType=Application\nName=Hoard\n\
                        Exec=/usr/bin/hoard-desktop --silent\nTerminal=false";
        let moved = repointed_autostart(packaged, app).unwrap();
        assert!(moved.contains("\nExec=/home/u/.local/bin/hoard-desktop --silent\n"));
        assert!(moved.ends_with("Terminal=false"));

        // Already right: nothing to write.
        assert_eq!(repointed_autostart(&moved, app), None);
        assert_eq!(
            repointed_autostart("Exec=\"/opt/hoard-desktop\"\n", app).as_deref(),
            Some("Exec=/home/u/.local/bin/hoard-desktop\n")
        );
        // Somebody else's entry, or the Flatpak's.
        assert_eq!(repointed_autostart("Exec=/usr/bin/other --x\n", app), None);
        assert_eq!(
            repointed_autostart("Exec=flatpak run services.hoard.saves\n", app),
            None
        );
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hoard-core-install-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
