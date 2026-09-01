//! `hoard doctor`: checks this machine's tracked saves and reports what looks
//! wrong, with the command that fixes each one.
//!
//! Thin wrapper, as usual: the rules live in `hoard_agent::doctor` so the desktop
//! can show the same findings.

use anyhow::Result;
use serde::Serialize;

use hoard_agent::doctor::{self, Finding, Severity};
use hoard_agent::session;
use hoard_agent::state::CliState;

use crate::output;

#[derive(Serialize)]
pub struct DoctorOut {
    pub checked: usize,
    pub findings: Vec<Finding>,
}

pub async fn run() -> Result<()> {
    // Purely local, like `hoard saves`: no network, works offline.
    session::set_context_offline();
    let (state, _) = CliState::load_default()?;

    let out = DoctorOut {
        checked: state.saves.len(),
        findings: doctor::diagnose(&state),
    };

    // Findings are advice, not a failed command: exit stays 0 so a script that
    // runs `doctor` before something else doesn't abort over a warning.
    output::emit(&out, |out| {
        if out.checked == 0 {
            println!("no saves tracked on this machine — nothing to check.");
            return;
        }
        if out.findings.is_empty() {
            println!("{} save(s) checked · nothing looks wrong.", out.checked);
            return;
        }
        for f in &out.findings {
            let tag = match f.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "warn ",
                Severity::Notice => "yours",
            };
            println!("{tag}  {} / {}", f.game_slug, f.label);
            println!("       {}", f.path);
            println!("       {}", f.detail);
            if let Some(p) = &f.suggested_path {
                println!("       suggested: {p}");
            }
            println!("       fix: {}", f.command);
            println!();
        }
        let errors = out
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        // Counted apart so the summary doesn't read as "9 things are wrong"
        // when most of them are choices the user made on purpose.
        let yours = out
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Notice)
            .count();
        println!(
            "{} save(s) checked · {} finding(s), {} error(s), {} you set up yourself.",
            out.checked,
            out.findings.len(),
            errors,
            yours
        );
    })
}
