//! `hoard notifications`: the same switches as the app's Settings and the Decky
//! plugin, on the same service.
//!
//! It only moves them. The notices themselves never come through here: the service
//! sends them, and a terminal is not where anybody is looking at 3 a.m.

use anyhow::{bail, Result};
use clap::{Subcommand, ValueEnum};
use hoard_core::ipc::{NotificationSwitch, NotificationSwitches, Payload, Request};
use serde::Serialize;

use crate::commands::link;
use crate::output;

#[derive(Subcommand)]
pub enum NotificationsCommand {
    /// Turn notifications on
    On {
        #[arg(required = true, value_enum)]
        switches: Vec<SwitchArg>,
    },
    /// Turn notifications off
    Off {
        #[arg(required = true, value_enum)]
        switches: Vec<SwitchArg>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SwitchArg {
    /// A backup finished uploading (one you asked for always notifies)
    BackupSuccess,
    /// A backup failed, was left incomplete or needs you
    BackupProblems,
    /// A save was downloaded from the cloud
    RestoreSuccess,
    /// A download keeps failing, or conflicting files were kept aside
    RestoreProblems,
    /// A game opened or closed
    GameActivity,
    /// An update needs you to install it
    Updates,
}

impl From<SwitchArg> for NotificationSwitch {
    fn from(arg: SwitchArg) -> Self {
        match arg {
            SwitchArg::BackupSuccess => Self::BackupSuccess,
            SwitchArg::BackupProblems => Self::BackupProblems,
            SwitchArg::RestoreSuccess => Self::RestoreSuccess,
            SwitchArg::RestoreProblems => Self::RestoreProblems,
            SwitchArg::GameActivity => Self::GameActivity,
            SwitchArg::Updates => Self::Updates,
        }
    }
}

/// Declared here rather than serialising the wire type: `--json` is a public
/// contract, and the IPC's fields should be free to change.
#[derive(Serialize)]
pub struct SwitchesOut {
    pub backup_success: bool,
    pub backup_problems: bool,
    pub restore_success: bool,
    pub restore_problems: bool,
    pub game_activity: bool,
    pub updates: bool,
}

impl From<NotificationSwitches> for SwitchesOut {
    fn from(s: NotificationSwitches) -> Self {
        Self {
            backup_success: s.backup_success,
            backup_problems: s.backup_problems,
            restore_success: s.restore_success,
            restore_problems: s.restore_problems,
            game_activity: s.game_activity,
            updates: s.updates,
        }
    }
}

pub async fn run(command: Option<NotificationsCommand>) -> Result<()> {
    let mut client = link::ensure("notifications").await?;
    let (switches, enabled) = match command {
        None => (Vec::new(), false),
        Some(NotificationsCommand::On { switches }) => (switches, true),
        Some(NotificationsCommand::Off { switches }) => (switches, false),
    };
    let mut current = None;
    for switch in switches {
        current = Some(
            ask(
                &mut client,
                Request::SetNotificationSwitch {
                    switch: switch.into(),
                    enabled,
                },
            )
            .await?,
        );
    }
    let current = match current {
        Some(current) => current,
        None => ask(&mut client, Request::NotificationSwitches).await?,
    };
    output::emit(&SwitchesOut::from(current), print)
}

async fn ask(
    client: &mut hoardd::client::Client,
    request: Request,
) -> Result<NotificationSwitches> {
    match link::ask(client, request).await? {
        Payload::NotificationSwitches(switches) => Ok(switches),
        other => bail!("unexpected answer about the notifications: {other:?}"),
    }
}

fn print(s: &SwitchesOut) {
    let rows = [
        ("backup-success", s.backup_success),
        ("backup-problems", s.backup_problems),
        ("restore-success", s.restore_success),
        ("restore-problems", s.restore_problems),
        ("game-activity", s.game_activity),
        ("updates", s.updates),
    ];
    for (name, on) in rows {
        println!("{name:<18} {}", if on { "on" } else { "off" });
    }
}
