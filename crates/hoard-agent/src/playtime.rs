//! Playtime: real hours played, attributed by local day.
//!
//! The agent's process poll ([`crate::agent::process_poll`]) already knows which
//! tracked saves have their game process alive on each tick. This module
//! accumulates that time: every tick it adds the elapsed interval to the games
//! still alive, grouped by local day and by game. It is the source of truth for
//! "hours played" that the recap consumes through `list_playtime`, and the data
//! is entirely local; nothing leaves the machine.
//!
//! The accumulation model is anchors: for each live save we keep the instant of
//! the last attribution (`anchors`, in memory only). On the next tick, if the game
//! is still alive, we add `now - anchor` to the day's bucket and move the anchor to
//! `now`. A game that stops being alive loses its anchor, and its final tail, at
//! most one poll interval, is not counted, which is a bounded error and irrelevant
//! at the scale of hours. The step is capped (`max_step_secs`) so a suspend and
//! resume does not count hours of a sleeping machine as play time.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The local offset, captured once. The desktop seeds it from `run()` before
/// starting any threads (see `lib.rs`), because `time`'s `current_local_offset`
/// refuses to read the environment in an already multi-threaded process. If
/// nobody seeds it (the headless CLI) we fall back to UTC; the recap is a desktop
/// feature, so the CLI loses nothing visible.
static LOCAL_OFFSET: OnceLock<time::UtcOffset> = OnceLock::new();

/// Seeds the local offset. Idempotent; only the first value counts.
pub fn set_local_offset(off: time::UtcOffset) {
    let _ = LOCAL_OFFSET.set(off);
}

fn local_offset() -> time::UtcOffset {
    // Unseeded (the headless CLI, or a desktop that did not call
    // `set_local_offset`) we fall back to UTC. `current_local_offset` is not used
    // here: it needs `time`'s `local-offset` feature and, on top of that, fails in
    // a multi-threaded process, which is why the desktop captures the offset
    // before its threads and seeds it.
    *LOCAL_OFFSET.get_or_init(|| time::UtcOffset::UTC)
}

/// The `YYYY-MM-DD` local day key for an epoch-ms instant. It uses the same local
/// clock as the UI (same machine), so the buckets line up with the heatmap's
/// per-day binning.
fn local_day_key(now_ms: u64) -> String {
    let secs = (now_ms / 1000) as i64;
    let dt = time::OffsetDateTime::from_unix_timestamp(secs)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
        .to_offset(local_offset());
    let d = dt.date();
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

#[cfg(test)]
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Cubetas de tiempo jugado persistidas en disco, más anclas en memoria.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaytimeStore {
    /// Local day `YYYY-MM-DD` to seconds played that day, across all games.
    #[serde(default)]
    days: BTreeMap<String, u64>,
    /// `game_slug` → segundos jugados acumulados (histórico).
    #[serde(default)]
    by_game: BTreeMap<String, u64>,
    /// The cross breakdown of day to (`game_slug` to seconds): what was played
    /// each day, and at what. A newer field, only filled in from here on, so for
    /// older days the sum of its games can be less than `days[day]` (see
    /// [`Self::upload_rows`], which adds a remainder row to balance it).
    #[serde(default)]
    daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    /// Segundos jugados acumulados (histórico).
    #[serde(default)]
    total_secs: u64,

    /// Sólo memoria: `save_id` → instante (epoch-ms) de la última atribución.
    #[serde(skip)]
    anchors: HashMap<String, u64>,
    #[serde(skip)]
    dirty: bool,
    #[serde(skip)]
    last_flush_ms: u64,

    /// In memory only: this store was read from a file that existed.
    ///
    /// It tells "this machine has played nothing" from "this machine does not know
    /// what it has played". They look the same, both being an empty store, and
    /// they are not: the second happens when the file is missing, and then the
    /// hours are somewhere else, or gone, rather than being zero.
    ///
    /// The difference matters because the server replaces this device's rows with
    /// whatever we send. Uploading an empty one without knowing whether it is
    /// genuinely empty is how a history gets lost: see [`Self::is_authoritative`].
    #[serde(skip)]
    from_disk: bool,
}

/// The serialisable shape the `list_playtime` command returns to the UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaytimeSummary {
    pub days: BTreeMap<String, u64>,
    pub by_game: BTreeMap<String, u64>,
    /// The cross breakdown of day to (`game_slug` to seconds): what was played
    /// each day and for how long. Feeds the recap's per-day detail (a click on a
    /// square).
    #[serde(default)]
    pub daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    pub total_secs: u64,
}

/// One atomic row of play time to upload to the cloud: "this day, this game, this
/// many seconds". The server upserts them by `(user_id, device_fp, day,
/// game_slug)`. The recap reads the aggregate back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeRow {
    pub day: String,
    pub game_slug: String,
    pub secs: u64,
}

impl PlaytimeStore {
    /// The store's path for the active sync context (the signed-in account or
    /// self-hosted server). Each account has its own history, just as the `saves`
    /// live in `contexts/<id>.json`, so one account's recap does not show, or
    /// upload, another's hours on the same machine.
    pub fn default_path() -> Result<PathBuf> {
        Self::path_for_context(&crate::state::current_context_id())
    }

    /// The store's path for a given context: `playtime/<ctx>.json`.
    pub fn path_for_context(ctx: &str) -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?
            .join("playtime")
            .join(format!("{ctx}.json")))
    }

    /// The path of the pre-partition monolithic file (one global history for the
    /// machine). Only used to migrate it once into the active context.
    fn legacy_path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("playtime.json"))
    }

    /// Adopts the pre-partition monolithic `playtime.json`, once.
    ///
    /// Before 1.0 the recap's history lived in a single global file for the
    /// machine. When the sync state was partitioned per account
    /// (`contexts/<id>.json`) playtime was left out, so every account that opened
    /// the recap saw, and re-uploaded, the whole machine's history. On the first
    /// start after the update we move that legacy file into the *active* context
    /// (the account signed in at boot, which is the user's main one): their recap
    /// survives and any other account starts from an empty, uncontaminated
    /// history.
    ///
    /// Idempotent through `fs::rename`: once the legacy file is gone the call does
    /// nothing, so it can never be re-adopted into another context on a later
    /// switch. If the active context somehow already had its own file, which it
    /// should not on a first update, the legacy one is set aside as `.bak` rather
    /// than overwriting it.
    pub fn migrate_legacy_into_current_context() -> Result<()> {
        let legacy = Self::legacy_path()?;
        if !legacy.exists() {
            return Ok(());
        }
        // Don't bury the history in the signed-out `default` bucket, where no
        // account would ever surface it. Wait until a real context (a cloud
        // account or a self-hosted server) is active; the agent re-runs this
        // after login, and the recap commands run it too.
        let ctx = crate::state::current_context_id();
        if ctx == "default" {
            return Ok(());
        }
        let target = Self::path_for_context(&ctx)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        if target.exists() {
            // The active context already has its own playtime, so we do not
            // overwrite it. The legacy file is set aside so it stops being adopted
            // on future switches.
            let bak = legacy.with_extension("pre-partition.bak");
            let _ = std::fs::rename(&legacy, &bak);
            return Ok(());
        }
        std::fs::rename(&legacy, &target)
            .with_context(|| format!("adopting legacy playtime into {}", target.display()))?;
        Ok(())
    }

    /// Loads the store; a missing or corrupt file produces an empty one (hours
    /// can be accumulated again, they are not critical).
    ///
    /// That empty one is marked NOT authoritative; see [`Self::from_disk`].
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(mut store) => {
                    store.from_disk = true;
                    store
                }
                Err(e) => {
                    // Broken JSON is not "zero hours": it is a history we cannot
                    // read. We carry on with an empty store so time can still
                    // accumulate, but with no right to delete anything server-side.
                    tracing::warn!(
                        error = %e,
                        path = %path.display(),
                        "playtime: store ilegible; se sigue sin él, pero no se declarará autoritativo"
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    /// Can this store speak for this machine's past?
    ///
    /// Only if it came off a file that existed and could be read. A newborn store,
    /// from a clean install, a deleted `AppData` or a new account, knows nothing
    /// about the earlier days, so the server must not take its silence for "those
    /// days did not happen".
    ///
    /// This is the client half of the fix; the other half is in the `/v1/playtime`
    /// route, which without this flag only deletes the days the upload mentions.
    pub fn is_authoritative(&self) -> bool {
        self.from_disk
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serializing playtime store")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Accumulates time for the saves alive this tick. `running` are
    /// `(save_id, game_slug)` pairs. `max_step_secs` caps the per-anchor step,
    /// which is the suspend-and-resume guard.
    pub fn accrue(&mut self, running: &[(String, String)], now: u64, max_step_secs: u64) {
        let live: HashSet<&str> = running.iter().map(|(id, _)| id.as_str()).collect();
        for (id, slug) in running {
            let prev = self.anchors.insert(id.clone(), now);
            let Some(prev) = prev else {
                // First observation: anchor only, attribute nothing, because we do
                // not know how long it had been alive before we saw it.
                continue;
            };
            if now <= prev {
                continue;
            }
            let mut secs = (now - prev) / 1000;
            if secs == 0 {
                continue;
            }
            if secs > max_step_secs {
                secs = max_step_secs;
            }
            let day = local_day_key(now);
            *self.days.entry(day.clone()).or_insert(0) += secs;
            *self.by_game.entry(slug.clone()).or_insert(0) += secs;
            *self
                .daily_by_game
                .entry(day)
                .or_default()
                .entry(slug.clone())
                .or_insert(0) += secs;
            self.total_secs += secs;
            self.dirty = true;
        }
        // Forget the anchors of games that are no longer alive.
        self.anchors.retain(|id, _| live.contains(id.as_str()));
    }

    /// Persists when there are changes and at least 30 s have passed since the
    /// last flush, so the JSON is not written on every tick.
    pub fn flush_if_due(&mut self, path: Option<&Path>, now: u64) {
        if !self.dirty {
            return;
        }
        if now.saturating_sub(self.last_flush_ms) < 30_000 {
            return;
        }
        self.flush(path, now);
    }

    /// Persists now (the agent calls it when a game stops, so the recap is fresh
    /// the moment you leave).
    pub fn flush(&mut self, path: Option<&Path>, now: u64) {
        if !self.dirty {
            return;
        }
        if let Some(p) = path {
            if let Err(e) = self.save(p) {
                tracing::debug!(error = %e, "agent: failed to persist playtime store");
                return;
            }
        }
        self.dirty = false;
        self.last_flush_ms = now;
    }

    /// A serialisable copy for the UI.
    pub fn summary(&self) -> PlaytimeSummary {
        PlaytimeSummary {
            days: self.days.clone(),
            by_game: self.by_game.clone(),
            daily_by_game: self.daily_by_game.clone(),
            total_secs: self.total_secs,
        }
    }

    /// `(day, game, seconds)` rows to upload to the cloud. It emits the real
    /// breakdown from `daily_by_game` and, per day, a remainder row under the
    /// `__other__` slug carrying `days[day]` minus the sum of the games when that
    /// is positive: that way the days from before this breakdown existed, whose
    /// total lives only in `days`, still balance in the server's aggregate without
    /// inventing which game they went to.
    pub fn upload_rows(&self) -> Vec<PlaytimeRow> {
        let mut rows = Vec::new();
        for (day, total) in &self.days {
            let per_game = self.daily_by_game.get(day);
            let mut attributed = 0u64;
            if let Some(games) = per_game {
                for (slug, secs) in games {
                    if *secs == 0 {
                        continue;
                    }
                    attributed += *secs;
                    rows.push(PlaytimeRow {
                        day: day.clone(),
                        game_slug: slug.clone(),
                        secs: *secs,
                    });
                }
            }
            let remainder = total.saturating_sub(attributed);
            if remainder > 0 {
                rows.push(PlaytimeRow {
                    day: day.clone(),
                    game_slug: "__other__".into(),
                    secs: remainder,
                });
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> PlaytimeStore {
        PlaytimeStore::default()
    }

    #[test]
    fn first_observation_only_anchors() {
        let mut s = store();
        s.accrue(&[("a".into(), "game".into())], 10_000, 60);
        assert_eq!(s.total_secs, 0, "no time on the very first sighting");
        assert!(s.anchors.contains_key("a"));
    }

    #[test]
    fn accrues_between_ticks() {
        let mut s = store();
        s.accrue(&[("a".into(), "game".into())], 10_000, 60);
        // 12 s después.
        s.accrue(&[("a".into(), "game".into())], 22_000, 60);
        assert_eq!(s.total_secs, 12);
        assert_eq!(*s.by_game.get("game").unwrap(), 12);
        assert_eq!(s.days.values().sum::<u64>(), 12);
    }

    #[test]
    fn caps_implausible_jumps() {
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 40);
        // An hour later (machine asleep): only the cap counts.
        s.accrue(&[("a".into(), "g".into())], 3_600_000, 40);
        assert_eq!(s.total_secs, 40);
    }

    #[test]
    fn drops_anchor_when_game_stops() {
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 60);
        s.accrue(&[("a".into(), "g".into())], 10_000, 60);
        assert_eq!(s.total_secs, 10);
        // Tick sin juegos vivos: olvida el ancla.
        s.accrue(&[], 20_000, 60);
        assert!(!s.anchors.contains_key("a"));
        // Si vuelve, no atribuye el hueco (re-ancla).
        s.accrue(&[("a".into(), "g".into())], 30_000, 60);
        assert_eq!(s.total_secs, 10);
    }

    #[test]
    fn upload_rows_split_and_remainder() {
        let mut s = store();
        // A day with a real breakdown: 12 s on "g".
        s.accrue(&[("a".into(), "g".into())], 10_000, 60);
        s.accrue(&[("a".into(), "g".into())], 22_000, 60);
        // Simulates a historical day with no breakdown (only in `days`).
        s.days.insert("2020-01-01".into(), 100);
        let rows = s.upload_rows();
        // La fila real "g".
        let g = rows.iter().find(|r| r.game_slug == "g").expect("row for g");
        assert_eq!(g.secs, 12);
        // The historical day is dumped whole as the remainder.
        let other = rows
            .iter()
            .find(|r| r.day == "2020-01-01" && r.game_slug == "__other__")
            .expect("remainder row");
        assert_eq!(other.secs, 100);
        // A day with a complete breakdown produces no remainder.
        assert!(
            !rows
                .iter()
                .any(|r| r.day != "2020-01-01" && r.game_slug == "__other__"),
            "fully-attributed day must not emit a remainder"
        );
    }

    #[test]
    fn round_trips_to_disk() {
        let tmp = std::env::temp_dir().join(format!("hoard-playtime-{}.json", now_ms()));
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 60);
        s.accrue(&[("a".into(), "g".into())], 5_000, 60);
        s.flush(Some(&tmp), 5_000);
        let loaded = PlaytimeStore::load(&tmp);
        assert_eq!(loaded.total_secs, 5);
        assert_eq!(*loaded.by_game.get("g").unwrap(), 5);
        let _ = std::fs::remove_file(&tmp);
    }
}
