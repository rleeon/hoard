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
            // Juegos nativos (no-Proton) que escriben en el "Saved Games" estilo
            // Windows dentro del HOME (Unity/Unreal multiplataforma, varios
            // indies). Sin esto, sólo se miraba dentro de prefijos Wine.
            "<home>/Saved Games",
        ],
        Os::Mac => &["<macAppSupport>", "<macPreferences>", "<home>/Documents"],
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

/// Roots adicionales que SOLO recorre el escaneo profundo (Linux): el gaming
/// confinado en sandboxes y los emuladores, que el tick periódico no mira por
/// coste. Cubre:
///
/// - **Flatpak**: datos por-app en `~/.var/app/<id>/{config,data,.local/share,
///   .config}` — Steam Deck, Heroic/Lutris/Bottles Flatpak, emuladores
///   EmuDeck/RetroDECK.
/// - **Snap**: `~/snap/<app>/{common,current}/.local/share` y `/.config`.
/// - **EmuDeck / RetroDECK**: `~/Emulation/saves`, `~/Emulation/storage`, y
///   las copias en microSD `/run/media/<user>/<label>/Emulation/saves`.
///
/// Todos filtrados a los que existen; vacío en SO que no sean Linux.
pub fn deep_save_roots(os: Os) -> Vec<PathBuf> {
    if !matches!(os, Os::Linux) {
        return Vec::new();
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>| {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
    };

    // Flatpak: one entry per installed app id under ~/.var/app.
    if let Ok(entries) = std::fs::read_dir(home.join(".var/app")) {
        for app in entries.flatten().map(|e| e.path()) {
            for sub in ["config", "data", ".local/share", ".config"] {
                push(app.join(sub), &mut out, &mut seen);
            }
        }
    }

    // Snap: per-app data lives under ~/snap/<app>/{common,current}.
    if let Ok(entries) = std::fs::read_dir(home.join("snap")) {
        for app in entries.flatten().map(|e| e.path()) {
            for rev in ["common", "current"] {
                push(app.join(rev).join(".local/share"), &mut out, &mut seen);
                push(app.join(rev).join(".config"), &mut out, &mut seen);
            }
        }
    }

    // EmuDeck / RetroDECK conventional save roots, local and on microSD.
    push(home.join("Emulation/saves"), &mut out, &mut seen);
    push(home.join("Emulation/storage"), &mut out, &mut seen);
    if let Ok(mounts) = std::fs::read_dir("/run/media") {
        for user in mounts.flatten().map(|e| e.path()) {
            if let Ok(vols) = std::fs::read_dir(&user) {
                for vol in vols.flatten().map(|e| e.path()) {
                    push(vol.join("Emulation/saves"), &mut out, &mut seen);
                    push(vol.join("Emulation/storage"), &mut out, &mut seen);
                }
            }
        }
    }

    out
}

/// Nombres de usuario Windows reales dentro de un prefijo Wine/Proton.
///
/// Lista los directorios bajo `drive_c/users/` que son usuarios reales —
/// Proton usa `steamuser`, los prefijos genéricos (`wine`, PlayOnLinux,
/// lanzadores `.desktop`) usan el login del host (`$USER`). Excluye `Public`
/// (no es un perfil de usuario) y entradas no-directorio. Vacío si el prefijo
/// no existe o no tiene `drive_c/users/`.
pub fn prefix_windows_users(prefix: &Path) -> Vec<String> {
    let users_dir = prefix.join("drive_c/users");
    let entries = match std::fs::read_dir(&users_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.eq_ignore_ascii_case("Public") {
            continue;
        }
        out.push(name);
    }
    out
}

/// Subdirectorios de usuario dentro de un prefijo Wine/Proton donde caen
/// los saves, para todos los usuarios reales del prefijo. Mismo naming
/// Windows que `pathexpand::expand_placeholder_in_prefix`. `prefix` apunta al
/// directorio que contiene `drive_c/` directamente.
pub fn prefix_user_roots(prefix: &Path) -> Vec<PathBuf> {
    prefix_windows_users(prefix)
        .iter()
        .flat_map(|user| prefix_user_roots_for(prefix, user))
        .collect()
}

/// Subdirectorios de save de un usuario Windows concreto dentro de un prefijo.
pub fn prefix_user_roots_for(prefix: &Path, user: &str) -> Vec<PathBuf> {
    let userhome = prefix.join("drive_c/users").join(user);
    [
        "AppData/Roaming",
        "AppData/Local",
        "AppData/LocalLow",
        "Documents",
        "Saved Games",
    ]
    .iter()
    .map(|sub| userhome.join(sub))
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
