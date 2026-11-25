//! `hoard sync backup`: the app's "Back up now", from the terminal.
//!
//! It is not `hoard backup`. That one uploads a folder from this process, for a
//! machine with no service or a folder nobody watches. This one asks the
//! service to back up a save it already syncs, so there is a single uploader
//! and the engine's own rules apply: the file types ticked for the save, its
//! second folders, the plan's per-save cap, the budget for copies made by hand.
//!
//! Fire and forget, like the button. The result arrives as a notification (a
//! copy someone asked for always notifies) and in `hoard sync run`.

use anyhow::{bail, Result};
use hoard_agent::prefs::Prefs;
use hoard_agent::session;
use hoard_agent::state::CliState;
use hoard_core::ipc::{DaemonStatus, Payload, Request};
use serde::Serialize;

use crate::commands::link;
use crate::output;

#[derive(Serialize)]
pub struct BackupRequested {
    /// The saves the service was asked to back up, one by one.
    pub save_ids: Vec<String>,
    /// `--all` went out as the engine's sweep instead: every save re-checked,
    /// the changed ones backed up.
    pub sweep: bool,
}

pub async fn run(save_id: Option<String>, all: bool) -> Result<()> {
    let mut client = link::required("backup").await?;
    let status = match link::ask(&mut client, Request::Status).await? {
        Payload::Status(status) => status,
        other => bail!("the service answered a {other:?} to a status request"),
    };
    if !status.engine.running {
        let why = status
            .engine
            .last_error
            .as_deref()
            .unwrap_or("no reason given");
        return Err(output::err(
            "engine_down",
            format!(
                "the Hoard service is up but its sync engine isn't ({why}); `hoard sync` shows more"
            ),
        ));
    }

    session::set_context_offline();
    let (state, _) = CliState::load_default()?;

    let out = match save_id {
        Some(id) => {
            let id = id.trim().to_string();
            check_watched(&status, &state, &id)?;
            link::ask(
                &mut client,
                Request::BackupNow {
                    save_id: id.clone(),
                },
            )
            .await?;
            BackupRequested {
                save_ids: vec![id],
                sweep: false,
            }
        }
        None if all => {
            let (prefs, _) = Prefs::load_for_cli_edit()?;
            if prefs.manual_only {
                // In "everything by hand" the engine holds every upload nobody asked
                // for, the sweep's included: there it would find the changes and
                // leave them on disk. Asking save by save is the one way through.
                let ids: Vec<String> = status
                    .slots
                    .iter()
                    .map(|slot| slot.save_id.clone())
                    .filter(|id| state.saves.contains_key(id))
                    .collect();
                for id in &ids {
                    link::ask(
                        &mut client,
                        Request::BackupNow {
                            save_id: id.clone(),
                        },
                    )
                    .await?;
                }
                BackupRequested {
                    save_ids: ids,
                    sweep: false,
                }
            } else {
                let window_secs = prefs.automatic_backup_interval_secs;
                link::ask(&mut client, Request::SweepAll { window_secs }).await?;
                BackupRequested {
                    save_ids: Vec::new(),
                    sweep: true,
                }
            }
        }
        None => bail!("say which save (`hoard sync backup <save_id>`) or pass --all"),
    };

    output::emit(&out, |out| {
        if out.sweep {
            println!(
                "the Hoard service is checking every save and backing up the ones that changed"
            );
        } else if out.save_ids.is_empty() {
            println!("the Hoard service isn't syncing any save on this machine");
            return;
        } else {
            for id in &out.save_ids {
                let name = status
                    .slots
                    .iter()
                    .find(|slot| &slot.save_id == id)
                    .map(|slot| slot.display_name.as_str())
                    .unwrap_or(id);
                println!("backing up {name}");
            }
        }
        println!(
            "only what changed goes up; you'll get a notification, or watch it with `hoard sync run`"
        );
    })
}

/// The service only backs up what it watches. A save it doesn't would be a
/// silent no-op, so say why instead.
fn check_watched(status: &DaemonStatus, state: &CliState, id: &str) -> Result<()> {
    if status.slots.iter().any(|slot| slot.save_id == id) {
        return Ok(());
    }
    let why = match state.saves.get(id) {
        None => "it isn't tracked on this machine (see `hoard saves`)".to_string(),
        Some(save) if save.paused => {
            format!("it's paused; `hoard save resume {id}` turns it back on")
        }
        Some(save) => format!(
            "its folder isn't being watched ({}); `hoard doctor` checks why",
            save.local_path.display()
        ),
    };
    Err(output::err(
        "not_tracked",
        format!("the Hoard service isn't syncing save {id}: {why}"),
    ))
}
