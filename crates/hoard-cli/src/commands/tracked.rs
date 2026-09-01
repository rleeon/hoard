//! `hoard saves`: the saves this machine tracks, meaning what `daemon` and `sync`
//! watch. Purely local: it reads `contexts/<id>.json` of the active context (Cloud
//! or self-host) and never touches the network, so it works offline and is
//! instant.

use anyhow::Result;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use hoard_agent::session;
use hoard_agent::state::CliState;

use crate::output;

/// One tracked save as agents and scripts see it. Declared here on purpose:
/// `SaveState` is the engine's own struct and must stay free to change.
#[derive(Serialize)]
pub struct SaveRow {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    /// Absolute and untruncated. The table clips this, JSON must not.
    pub local_path: String,
    pub paused: bool,
    pub last_version_num: Option<i64>,
    /// RFC3339, or null when this save has never been backed up.
    pub last_backup_at: Option<String>,
    pub preset: Option<String>,
}

#[derive(Serialize)]
pub struct SavesOut {
    pub saves: Vec<SaveRow>,
    pub state_file: String,
}

pub async fn run() -> Result<()> {
    // No network: pin the active account's context via the stored JWT/URL.
    session::set_context_offline();
    let (state, path) = CliState::load_default()?;

    // Stable order by game name + label for deterministic output.
    let mut rows: Vec<_> = state.saves.iter().collect();
    rows.sort_by(|(_, a), (_, b)| {
        a.game_slug
            .cmp(&b.game_slug)
            .then_with(|| a.label.cmp(&b.label))
    });

    let out = SavesOut {
        saves: rows
            .into_iter()
            .map(|(id, s)| SaveRow {
                save_id: id.clone(),
                game_slug: s.game_slug.clone(),
                label: s.label.clone(),
                local_path: s.local_path.display().to_string(),
                paused: s.paused,
                last_version_num: s.last_version_num,
                last_backup_at: s.last_backup_at.and_then(|t| t.format(&Rfc3339).ok()),
                preset: s.preset.clone(),
            })
            .collect(),
        state_file: path.display().to_string(),
    };

    output::emit(&out, |out| {
        if out.saves.is_empty() {
            println!(
                "you don't track any save on this machine.\n\
                 Add one with `hoard track \"<game>\"`."
            );
            return;
        }
        println!(
            "{:<24}  {:<10}  {:>6}  {:<20}  {:<8}  PATH",
            "GAME", "LABEL", "VER", "LAST", "STATE"
        );
        for s in &out.saves {
            let ver = s
                .last_version_num
                .map(|v| format!("v{v}"))
                .unwrap_or_else(|| "—".to_string());
            let last = s
                .last_backup_at
                .as_deref()
                .map(|t| t.chars().take(19).collect::<String>().replace('T', " "))
                .unwrap_or_else(|| "—".to_string());
            let state_label = if s.paused { "paused" } else { "active" };
            println!(
                "{:<24}  {:<10}  {:>6}  {:<20}  {:<8}  {}",
                truncate(&s.game_slug, 24),
                truncate(&s.label, 10),
                ver,
                last,
                state_label,
                s.local_path
            );
        }
        println!("\n{} save(s) · {}", out.saves.len(), out.state_file);
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}
