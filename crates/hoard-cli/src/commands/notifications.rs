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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use clap::ValueEnum;
    use serde_json::Value;

    /// Spelled out field by field on purpose: a switch added to the wire stops
    /// this from compiling, which is the moment to give it a name here too.
    const ALL_OFF: NotificationSwitches = NotificationSwitches {
        backup_success: false,
        backup_problems: false,
        restore_success: false,
        restore_problems: false,
        game_activity: false,
        updates: false,
    };

    /// What the service is told when somebody types `arg`.
    fn sent(arg: SwitchArg) -> String {
        match serde_json::to_value(NotificationSwitch::from(arg)).unwrap() {
            Value::String(name) => name,
            other => panic!("a switch goes over the wire as a name, not {other}"),
        }
    }

    fn keys(value: Value) -> BTreeSet<String> {
        value.as_object().unwrap().keys().cloned().collect()
    }

    /// `hoard notifications off backup-success` has to turn off that switch and
    /// not its neighbour: the name typed and the name sent are the same words.
    #[test]
    fn each_switch_typed_is_the_switch_sent() {
        for arg in SwitchArg::value_variants() {
            let typed = arg.to_possible_value().unwrap();
            assert_eq!(typed.get_name().replace('-', "_"), sent(*arg));
        }
    }

    #[test]
    fn every_switch_the_service_keeps_can_be_flipped_from_here() {
        let sendable: BTreeSet<String> = SwitchArg::value_variants()
            .iter()
            .map(|arg| sent(*arg))
            .collect();
        let kept = keys(serde_json::to_value(ALL_OFF).unwrap());
        assert_eq!(sendable, kept);
    }

    /// `--json` declares its own shape so the wire can change, but each switch
    /// has to come out under its own name and carry its own state.
    #[test]
    fn the_json_output_reports_each_switch_under_its_own_name() {
        let names = keys(serde_json::to_value(ALL_OFF).unwrap());
        for on in &names {
            let mut wire = serde_json::to_value(ALL_OFF).unwrap();
            wire[on.as_str()] = Value::Bool(true);
            let switches: NotificationSwitches = serde_json::from_value(wire).unwrap();

            let shown = serde_json::to_value(SwitchesOut::from(switches)).unwrap();
            assert_eq!(keys(shown.clone()), names);
            for (name, state) in shown.as_object().unwrap() {
                assert_eq!(state, &Value::Bool(name == on), "{name} with {on} on");
            }
        }
    }
}
