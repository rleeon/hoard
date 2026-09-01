//! `hoard cloud`: the Hoard Cloud account from the terminal. Parity with what the
//! desktop offers on its account screen: export, the black box (storage, archive,
//! reactivate), Pro entitlements and the playtime recap. All the logic lives in
//! [`hoard_agent::cloud_account`]; here we only resolve the Cloud session and
//! print.

use anyhow::{anyhow, bail, Result};
use clap::Subcommand;

use hoard_agent::cloud_account::{self, CloudError};

use super::link;

#[derive(Subcommand)]
pub enum CloudCommand {
    /// Freeable footprint per game + quota figures (the "free space" view)
    Storage,
    /// Request a full-account export; the server builds a ZIP in the background
    Export,
    /// Show the latest export job's status and download link
    ExportStatus,
    /// Archive a game: frees quota now, keeps it downloadable for 7 days
    Archive {
        /// Save id (UUID), see `hoard cloud storage`
        save_id: String,
    },
    /// Reactivate an archived game (needs room in the plan, within 7 days)
    Reactivate {
        /// Save id (UUID)
        save_id: String,
    },
    /// Show Pro entitlements: plan plus per-feature (screen, wrapple) state
    Entitlements,
    /// Sync + show this account's cross-device playtime recap
    Playtime,
}

/// Map a [`CloudError`] to a printable anyhow error.
fn err(e: CloudError) -> anyhow::Error {
    anyhow!(e.message())
}

pub async fn run(cmd: CloudCommand) -> Result<()> {
    // The token is lent by the service (ADR 0021, Slice 4c): the CLI does not
    // rotate. With no service the one on disk is used as-is, and if it has already
    // expired the error says so, with the hint that `hoard sync` is what renews.
    let active = link::resolve_session().await?;
    let Some(sess) = active.cloud else {
        bail!("este comando requiere sesión Hoard Cloud — inicia sesión con `hoard login`");
    };
    let base = sess.server_url.as_str();
    let token = sess.access.as_str();

    match cmd {
        CloudCommand::Storage => {
            let sg = cloud_account::storage_games(base, token)
                .await
                .map_err(err)?;
            println!(
                "plan {} · {} / {} usados{}",
                sg.plan,
                fmt_size(sg.used_bytes as i64),
                fmt_size(sg.limit_bytes as i64),
                if sg.over_bytes > 0 {
                    format!(" · {} por encima", fmt_size(sg.over_bytes as i64))
                } else {
                    String::new()
                }
            );
            if sg.games.is_empty() {
                println!("(sin partidas)");
                return Ok(());
            }
            println!(
                "{:<38} {:<24} {:<12} {:>10}",
                "ID", "GAME", "LABEL", "FREEABLE"
            );
            for g in sg.games {
                let label = if g.archived { "archivada" } else { &g.label };
                println!(
                    "{:<38} {:<24} {:<12} {:>10}",
                    g.save_id,
                    g.game_slug,
                    label,
                    fmt_size(g.freeable_bytes)
                );
            }
        }
        CloudCommand::Export => {
            let job = cloud_account::export_all(base, token).await.map_err(err)?;
            println!("export lanzado: job {} ({})", job.job_id, job.status);
            println!("sondea el estado con `hoard cloud export-status`");
        }
        CloudCommand::ExportStatus => {
            let st = cloud_account::export_status(base, token)
                .await
                .map_err(err)?;
            match st.status {
                None => println!("(nunca has solicitado un export)"),
                Some(status) => {
                    println!("estado: {status}");
                    if let Some(sz) = st.size_bytes {
                        println!("tamaño: {}", fmt_size(sz));
                    }
                    if let Some(url) = st.download_url {
                        println!("descarga: {url}");
                    }
                    if let Some(exp) = st.expires_at {
                        println!("caduca: {exp}");
                    }
                    if let Some(e) = st.error {
                        println!("error: {e}");
                    }
                }
            }
        }
        CloudCommand::Archive { save_id } => {
            let r = cloud_account::archive_save(base, token, &save_id)
                .await
                .map_err(err)?;
            println!(
                "archivada {} · liberados {} · se purga el {}",
                r.save_id,
                fmt_size(r.freed_bytes),
                r.purge_after
            );
        }
        CloudCommand::Reactivate { save_id } => {
            cloud_account::reactivate_save(base, token, &save_id)
                .await
                .map_err(err)?;
            println!("reactivada {save_id}");
        }
        CloudCommand::Entitlements => {
            let ent = cloud_account::entitlements(base, token)
                .await
                .map_err(err)?;
            println!("plan: {}", ent.plan);
            println!("screen:  {}", fmt_feature(&ent.features.screen));
            println!("wrapple: {}", fmt_feature(&ent.features.wrapple));
        }
        CloudCommand::Playtime => {
            use hoard_agent::cloud_account::PlaytimeUploadBody;
            use hoard_agent::playtime::PlaytimeStore;

            let _ = PlaytimeStore::migrate_legacy_into_current_context();
            let path = PlaytimeStore::default_path()?;
            let store = PlaytimeStore::load(&path);
            let dev = hoard_agent::logship::device_identity();
            let body = PlaytimeUploadBody {
                device_fp: dev.fingerprint,
                authoritative: store.is_authoritative(),
                rows: store.upload_rows(),
            };
            // Push best-effort, then read the server's cross-device aggregate.
            let _ = cloud_account::push_playtime(base, "/v1/cloud/playtime", token, &body).await;
            let sum = cloud_account::fetch_playtime(base, "/v1/cloud/playtime", token)
                .await
                .map_err(err)?;
            println!("total: {}", fmt_hours(sum.total_secs));
            if sum.by_game.is_empty() {
                println!("(sin partidas registradas)");
                return Ok(());
            }
            let mut games: Vec<(&String, &u64)> = sum.by_game.iter().collect();
            games.sort_by(|a, b| b.1.cmp(a.1));
            for (slug, secs) in games {
                println!("{:<28} {}", slug, fmt_hours(*secs));
            }
        }
    }
    Ok(())
}

fn fmt_feature(f: &cloud_account::FeatureState) -> String {
    use cloud_account::FeatureState::*;
    match f {
        Entitled => "entitled".into(),
        TrialAvailable { days } => format!("trial disponible ({days}d)"),
        Trial { expires_at } => format!("trial (caduca {expires_at})"),
        TrialExpired => "bloqueada".into(),
    }
}

fn fmt_hours(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn fmt_size(b: i64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2}G", b / GB)
    } else if b >= MB {
        format!("{:.2}M", b / MB)
    } else if b >= KB {
        format!("{:.2}K", b / KB)
    } else {
        format!("{}B", b as i64)
    }
}
