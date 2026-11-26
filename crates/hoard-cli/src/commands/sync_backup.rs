//! `hoard sync backup`: the app's "Back up now", from the terminal.
//!
//! It is not `hoard backup`. That one uploads a folder from this process, for a
//! machine with no service or a folder nobody watches. This one asks the
//! service to back up a save it already syncs, so there is a single uploader
//! and the engine's own rules apply: the file types ticked for the save, its
//! second folders, the plan's per-save cap, the budget for copies made by hand.
//!
//! Fire and forget, like the button. If the save changed since its last version,
//! a new one goes up and a notification says so (a copy someone asked for
//! notifies even with success notices off); if it didn't, nothing is uploaded
//! and nothing is said. `--all` is the button pressed on every save, not
//! automatic mode's sweep: the sweep's copies are automatic ones, which count
//! against the other budget and stay quiet, and in "everything by hand" the
//! engine holds them altogether.

use anyhow::{bail, Result};
use hoard_agent::session;
use hoard_agent::state::CliState;
use hoard_core::ipc::{DaemonStatus, Payload, Request};
use serde::Serialize;

use crate::commands::link;
use crate::output;

#[derive(Serialize)]
pub struct BackupRequested {
    /// The saves the service was asked to back up.
    pub save_ids: Vec<String>,
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

    let save_ids = match save_id {
        Some(id) => {
            let id = id.trim().to_string();
            check_watched(&status, &state, &id)?;
            vec![id]
        }
        // Every slot the service watches that is one of this account's saves:
        // hours-only games have a slot too, and nothing to back up.
        None if all => status
            .slots
            .iter()
            .map(|slot| slot.save_id.clone())
            .filter(|id| state.saves.contains_key(id))
            .collect(),
        None => bail!("say which save (`hoard sync backup <save_id>`) or pass --all"),
    };
    for id in &save_ids {
        link::ask(
            &mut client,
            Request::BackupNow {
                save_id: id.clone(),
            },
        )
        .await?;
    }
    let out = BackupRequested { save_ids };

    output::emit(&out, |out| {
        if out.save_ids.is_empty() {
            println!("the Hoard service isn't syncing any save on this machine");
            return;
        }
        for id in &out.save_ids {
            let name = status
                .slots
                .iter()
                .find(|slot| &slot.save_id == id)
                .map(|slot| slot.display_name.as_str())
                .unwrap_or(id);
            println!("backing up {name}");
        }
        if out.save_ids.len() == 1 {
            println!(
                "if it changed since its last version, a new one goes up and you'll get \
                 a notification; if not, nothing is uploaded"
            );
        } else {
            println!(
                "each one that changed since its last version gets a new one and a \
                 notification; the rest are left as they are"
            );
        }
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
