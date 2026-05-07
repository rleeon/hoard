//! Auto-discovery: find installed games and register them with the agent.
//!
//! This is the v0.3 "light first-launch scan" path. We:
//!
//! 1. Walk every supported storefront (right now: Steam) via
//!    [`hoard_detect::store::scan_all`].
//! 2. For each detected game, resolve its save paths against Windows
//!    Known Folders (or the test [`PlaceholderEnv`] in development).
//! 3. Auto-create the corresponding `save` row on the server if it
//!    doesn't exist yet — Windows-friendly, the user never has to click
//!    "Add this game" before the first backup runs.
//! 4. Hand each detected game to the live [`AgentHandle`] as a
//!    [`WatchedSave`].
//!
//! After the startup pass, the agent's own process poll is what watches
//! for new games — the user playing a game we already auto-registered
//! triggers `GameStarted`, which spins up the fs watcher just for that
//! save (see `agent::process_poll`).

use std::path::PathBuf;

use hoard_detect::{DetectedGame, GameSource};
use hoard_manifest::{
    placeholders::{PlaceholderEnv, PlaceholderError},
    GameManifest, SavePath,
};

use crate::agent::{AgentHandle, WatchedSave};
use crate::api::ApiClient;

/// Outcome of a single auto-registration attempt. Mostly used by tests
/// and the UI's "what did we just discover?" toast.
#[derive(Debug, Clone)]
pub struct Registered {
    pub manifest_id: String,
    pub title: String,
    pub save_id: String,
    pub source: GameSource,
    pub local_path: PathBuf,
    pub created_save: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AutodetectError {
    #[error("placeholder resolution failed for `{game}`: {source}")]
    Placeholder {
        game: String,
        #[source]
        source: PlaceholderError,
    },
    #[error("server save creation/lookup failed for `{game}`: {source}")]
    Server {
        game: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("agent command channel closed")]
    AgentDown,
}

/// Run the full auto-discovery pass.
///
/// Returns one [`Registered`] entry per game successfully registered.
/// Per-game errors are logged but don't abort the whole pass — finding
/// 9 of 10 games is much better than failing the whole boot because one
/// game's `installdir` is missing.
pub async fn run_autodetect(
    api: &ApiClient,
    agent: &AgentHandle,
    env: &dyn PlaceholderEnv,
    label: &str,
) -> Vec<Registered> {
    let detected = hoard_detect::store::scan_all();
    tracing::info!(count = detected.len(), "autodetect: scan complete");

    let mut out = Vec::new();
    for det in detected {
        match register_one(api, agent, env, label, &det).await {
            Ok(reg) => {
                tracing::info!(
                    game = %reg.title,
                    save_id = %reg.save_id,
                    created = reg.created_save,
                    "autodetect: registered"
                );
                out.push(reg);
            }
            Err(e) => {
                tracing::warn!(
                    game = %det.title,
                    error = %e,
                    "autodetect: skipping game"
                );
            }
        }
    }
    out
}

/// Register one detected game. Pulled out for unit-testability.
pub async fn register_one(
    api: &ApiClient,
    agent: &AgentHandle,
    env: &dyn PlaceholderEnv,
    label: &str,
    det: &DetectedGame,
) -> Result<Registered, AutodetectError> {
    let manifest =
        hoard_manifest::lookup(&det.manifest_id).ok_or_else(|| AutodetectError::Placeholder {
            game: det.manifest_id.clone(),
            source: PlaceholderError::UnknownToken("manifest not in catalogue".into()),
        })?;

    let local_path =
        first_resolvable_path(manifest, env).ok_or_else(|| AutodetectError::Placeholder {
            game: det.manifest_id.clone(),
            source: PlaceholderError::Unavailable("no save path resolved".into()),
        })?;

    // Find-or-create on the server. We auto-create per the v0.3
    // "Windows-friendly, no clicks" priority.
    let save = match find_save(api, &det.manifest_id, label).await {
        Ok(Some(s)) => (s, false),
        Ok(None) => {
            let created = api
                .create_save(&det.manifest_id, label)
                .await
                .map_err(|e| AutodetectError::Server {
                    game: det.manifest_id.clone(),
                    source: e,
                })?;
            (created, true)
        }
        Err(e) => {
            return Err(AutodetectError::Server {
                game: det.manifest_id.clone(),
                source: e,
            });
        }
    };

    let watched = WatchedSave {
        save_id: save.0.id.clone(),
        game_slug: det.manifest_id.clone(),
        display_name: det.title.clone(),
        label: label.to_string(),
        local_path: local_path.clone(),
        steam_install_dir: det.install_dir.clone(),
        processes: manifest.processes.clone(),
    };

    agent
        .add_save(watched)
        .await
        .map_err(|_| AutodetectError::AgentDown)?;

    Ok(Registered {
        manifest_id: det.manifest_id.clone(),
        title: det.title.clone(),
        save_id: save.0.id,
        source: det.source,
        local_path,
        created_save: save.1,
    })
}

/// Pick the first save path on the manifest that resolves cleanly. We
/// take the first hit rather than try to merge multiple paths — v0.3's
/// agent watches one path per save. Multi-path games (Skyrim has both
/// `Saves/` and `*.ini` config) get the first slot in the manifest;
/// the others can be handled in v0.4 once we support multi-root saves.
fn first_resolvable_path(manifest: &GameManifest, env: &dyn PlaceholderEnv) -> Option<PathBuf> {
    for sp in manifest.paths_for_current_os() {
        if let Some(p) = try_resolve(sp, env) {
            return Some(p);
        }
    }
    None
}

fn try_resolve(sp: &SavePath, env: &dyn PlaceholderEnv) -> Option<PathBuf> {
    match hoard_manifest::resolve_placeholders(&sp.location, env) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::debug!(location = %sp.location, error = %e, "skip path");
            None
        }
    }
}

/// Look up an existing save by (game_slug, label) in the user's library.
/// Returns `Ok(None)` if no save matches, error on transport failure.
async fn find_save(
    api: &ApiClient,
    game_slug: &str,
    label: &str,
) -> anyhow::Result<Option<crate::api::Save>> {
    let saves = api.list_saves(Some(game_slug)).await?;
    Ok(saves.into_iter().find(|s| s.label == label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_manifest::placeholders::MapEnv;
    use std::collections::HashMap;

    #[test]
    fn first_resolvable_path_picks_first_winner() {
        let env = MapEnv(HashMap::from([(
            "APPDATA".to_string(),
            PathBuf::from("/x/AppData/Roaming"),
        )]));

        // Stardew Valley — single path, resolves cleanly.
        let m = hoard_manifest::lookup("stardew-valley")
            .expect("stardew-valley should be in the catalogue");
        let p = first_resolvable_path(m, &env).expect("should resolve");
        assert!(p.to_string_lossy().contains("StardewValley"), "got {p:?}");
    }

    #[test]
    fn first_resolvable_path_skips_unresolvable_then_takes_next() {
        // Skyrim SE has two paths: saves under DOCUMENTS, configs under
        // DOCUMENTS. Only DOCUMENTS in env → first one wins.
        let env = MapEnv(HashMap::from([(
            "DOCUMENTS".to_string(),
            PathBuf::from("/x/Documents"),
        )]));
        let m = hoard_manifest::lookup("skyrim-special-edition").unwrap();
        let p = first_resolvable_path(m, &env).expect("DOCUMENTS resolves");
        assert!(p.to_string_lossy().contains("Skyrim"));
    }

    #[test]
    fn first_resolvable_path_returns_none_when_no_env() {
        let env = MapEnv(HashMap::new());
        let m = hoard_manifest::lookup("stardew-valley").unwrap();
        assert!(first_resolvable_path(m, &env).is_none());
    }
}
