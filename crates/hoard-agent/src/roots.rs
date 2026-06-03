//! DETECCIÓN — enumeración de roots de usuario (fase 0, ADR 0020).
//!
//! Lista los directorios raíz donde los juegos guardan saves, por SO,
//! derivada de los placeholders que `pathexpand` ya sabe expandir
//! (`<winAppData>`, `<winLocalAppDataLow>`, `<xdgData>`, …). Es la base del
//! scan automático catalog-free: el walk por señales (fase 1+) debe
//! recorrer ESTOS roots, no sólo `install_dir` + `drive_c/users/steamuser`.
//!
//! NOTA DE INTEGRACIÓN: este módulo es la cimentación de la fase 0. Todavía
//! NO está cableado en `detection::detect_all` — recorrer todo el HOME por
//! cada slug sin resolver sería I/O explosiva, así que el cableado real
//! espera a la fase 4 (atribución), que asocia candidatos sueltos a juegos.
//! Aquí sólo se provee la lista de roots, deduplicada y filtrada a los que
//! existen en el host.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::expand_path;

/// Templates de roots de usuario por SO (placeholders estilo Ludusavi).
fn root_templates(os: Os) -> &'static [&'static str] {
    match os {
        Os::Windows => &[
            "<winAppData>",         // Roaming
            "<winLocalAppData>",    // Local
            "<winLocalAppDataLow>", // LocalLow — Unity Application.persistentDataPath
            "<winSavedGames>",
            "<home>/Documents",
            "<home>/Documents/My Games",
        ],
        Os::Linux => &[
            "<xdgData>",   // ~/.local/share
            "<xdgConfig>", // ~/.config
            "<home>/.local/state",
            "<home>/Documents",
        ],
        Os::Mac => &[
            "<macAppSupport>",
            "<macPreferences>",
            "<home>/Documents",
        ],
    }
}

/// Roots de usuario nativos que existen en este host, deduplicados.
pub fn user_save_roots(os: Os) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for tmpl in root_templates(os) {
        for p in expand_path(tmpl, os) {
            if seen.insert(p.clone()) && p.is_dir() {
                out.push(p);
            }
        }
    }
    out
}

/// Subdirectorios de usuario dentro de un prefijo Wine/Proton donde caen
/// los saves. Mismo naming Windows que `pathexpand::expand_placeholder_in_prefix`.
/// `prefix` apunta al `pfx/` de una entrada de compatdata.
pub fn prefix_user_roots(prefix: &Path) -> Vec<PathBuf> {
    let steamuser = prefix.join("drive_c/users/steamuser");
    [
        "AppData/Roaming",
        "AppData/Local",
        "AppData/LocalLow",
        "Documents",
        "Saved Games",
    ]
    .iter()
    .map(|sub| steamuser.join(sub))
    .filter(|p| p.is_dir())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_non_empty_per_os() {
        for os in [Os::Windows, Os::Linux, Os::Mac] {
            assert!(!root_templates(os).is_empty());
        }
    }

    #[test]
    fn user_save_roots_runs_and_dedups() {
        // No panics; result is deduplicated (existence depends on host).
        let roots = user_save_roots(Os::current());
        let mut seen = HashSet::new();
        for r in &roots {
            assert!(seen.insert(r.clone()), "duplicate root: {r:?}");
        }
    }

    #[test]
    fn prefix_user_roots_filters_missing() {
        // A bogus prefix has none of the steamuser subdirs.
        let roots = prefix_user_roots(Path::new("/nonexistent/prefix/pfx"));
        assert!(roots.is_empty());
    }
}
