//! Phantom-hours filter for folder-to-process correlation signals.

use std::collections::{HashMap, HashSet};

/// Keeps only the correlation signals trustworthy enough to count as playtime.
/// `candidates` are `(proc_name_lower, save_id, game_slug)` tuples for saves
/// with no manifest whose folder has a valid correlation observation;
/// `configured` are the process names games with a manifest already declare.
///
/// Two vetoes, both learned the hard way from one process racking up hours for
/// Ark, Minecraft, Offworld and REPO because something in the background
/// rewrote their save folders while it happened to be running:
///
///  (a) a process already configured for another game with a manifest belongs
///      to that game, not to whichever folder it brushed against;
///  (b) a process bound to several different `game_slug`s is background noise,
///      not "you are playing" any of them.
///
/// Returns the accepted `(proc_name_lower, save_id)` pairs, one save per
/// process, so a game with several folders does not double its hours. The
/// observations themselves stay untouched for folder detection, which the user
/// can review.
pub fn accept_correlation_signals<'a>(
    candidates: &[(String, &'a str, &'a str)],
    configured: &HashSet<String>,
) -> Vec<(String, &'a str)> {
    let mut slugs_per_proc: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (pname, _, slug) in candidates {
        slugs_per_proc.entry(pname).or_default().insert(slug);
    }
    let mut out: Vec<(String, &'a str)> = Vec::new();
    let mut taken: HashSet<&str> = HashSet::new();
    for (pname, save_id, _) in candidates {
        if configured.contains(pname) {
            continue; // (a)
        }
        if slugs_per_proc.get(pname.as_str()).map_or(0, |s| s.len()) != 1 {
            continue; // (b)
        }
        if taken.insert(pname.as_str()) {
            out.push((pname.clone(), save_id));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The phantom-correlation bug (D.4): one game's process ended up
    /// correlated with the save folders of four games nobody had played,
    /// because a background sync rewrote them while it ran. A process bound to
    /// more than one game is noise and must give hours to none of them.
    #[test]
    fn correlation_rejects_shared_process_phantom_hours() {
        let configured: HashSet<String> = ["rustclient.exe".to_string()].into_iter().collect();
        let candidates = vec![
            ("rustclient.exe".to_string(), "ark", "ark-survival-ascended"),
            ("rustclient.exe".to_string(), "mc", "minecraft-java"),
            ("rustclient.exe".to_string(), "off", "offworld-trading"),
            ("rustclient.exe".to_string(), "repo", "r-e-p-o"),
        ];
        let accepted = accept_correlation_signals(&candidates, &configured);
        assert!(
            accepted.is_empty(),
            "a process shared by several games must not accrue hours: {accepted:?}"
        );
    }

    #[test]
    fn correlation_accepts_exclusive_off_catalog_game() {
        // The legitimate case correlation exists to rescue: a game with no
        // manifest whose own exe wrote its save. Exclusive to one game and not
        // configured anywhere else, so it counts.
        let configured: HashSet<String> = HashSet::new();
        let candidates = vec![("eu5.exe".to_string(), "eu5-save", "europa-universalis-5")];
        let accepted = accept_correlation_signals(&candidates, &configured);
        assert_eq!(accepted, vec![("eu5.exe".to_string(), "eu5-save")]);
    }

    #[test]
    fn correlation_one_save_per_process_no_double_count() {
        // One game with two tracked folders. The process is exclusive to that
        // slug, but it may only be injected once; marking both would double the
        // same game's hours.
        let configured: HashSet<String> = HashSet::new();
        let candidates = vec![
            ("eu5.exe".to_string(), "save-a", "eu5"),
            ("eu5.exe".to_string(), "save-b", "eu5"),
        ];
        let accepted = accept_correlation_signals(&candidates, &configured);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].0, "eu5.exe");
    }

    #[test]
    fn correlation_rejects_configured_process_of_another_game() {
        // Even if it had dirtied only one foreign folder, the process is
        // declared by its own game's manifest. It belongs to that game, not to
        // the folder it touched.
        let configured: HashSet<String> = ["rustclient.exe".to_string()].into_iter().collect();
        let candidates = vec![("rustclient.exe".to_string(), "ark", "ark-survival-ascended")];
        let accepted = accept_correlation_signals(&candidates, &configured);
        assert!(accepted.is_empty());
    }
}
