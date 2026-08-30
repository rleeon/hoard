//! DETECCIÓN — carpetas contenedoras: emuladores de Steam / repacks.
//!
//! Un "wrapper" es una carpeta que agrupa **un subdirectorio por juego**,
//! normalmente nombrado con el AppID de Steam:
//!
//! ```text
//! %APPDATA%/Goldberg SteamEmu Saves/413150/remote/...
//! %PUBLIC%/Documents/Steam/CODEX/1091500/remote/...
//! ```
//!
//! Sin esta etapa esas carpetas sólo las alcanzaba el walk genérico de fase 4,
//! que no sabe que el subdirectorio es un AppID ni que el save real está en
//! `remote/`. De ahí salieron dos bugs reales: `GSE Saves` acabó rastreado con
//! el slug del nombre de usuario de Windows, y un save quedó rotulado con el
//! nombre de un instalador. Aquí el AppID se resuelve contra el catálogo, así
//! que el juego sale con su nombre y su carátula, y la carpeta ofrecida es la
//! de los saves, no el contenedor.
//!
//! El contenedor importa además por una razón de sync: junto a `remote/`
//! conviven `remotecache.vdf`, logros, estadísticas y contadores de tiempo
//! jugado que **cambian en cada sesión y son distintos en cada máquina**.
//! Rastrear el padre convierte eso en un conflicto permanente entre dispositivos
//! sin que ningún save se haya movido.

use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::{expand_path, expand_path_in_prefix_as_user};

/// Un wrapper conocido: dónde vive y cómo se llama en la UI.
struct Wrapper {
    /// Plantilla estilo Ludusavi, resuelta igual en el host y dentro de un
    /// prefijo Wine.
    template: &'static str,
    /// Etiqueta para el log y para el nombre de un hallazgo sin AppID.
    label: &'static str,
}

/// Los emuladores de Steam y repacks que agrupan saves por AppID.
///
/// Todas son convenciones de Windows; en Linux los mismos juegos corren bajo
/// Proton y estas rutas viven dentro del prefijo, que es justo por lo que
/// [`discover_wrappers_in_prefix`] existe.
const WRAPPERS: &[Wrapper] = &[
    Wrapper {
        template: "<winAppData>/Goldberg SteamEmu Saves",
        label: "Goldberg",
    },
    Wrapper {
        template: "<winAppData>/GSE Saves",
        label: "Goldberg (GSE)",
    },
    Wrapper {
        template: "<winPublic>/Documents/Steam/CODEX",
        label: "CODEX",
    },
    Wrapper {
        template: "<winPublic>/Documents/Steam/RUNE",
        label: "RUNE",
    },
    Wrapper {
        template: "<winDocuments>/Steam/TENOKE",
        label: "TENOKE",
    },
    Wrapper {
        template: "<winPublic>/Documents/EMPRESS",
        label: "EMPRESS",
    },
    Wrapper {
        template: "<winPublic>/Documents/OnlineFix",
        label: "Online-Fix",
    },
    Wrapper {
        template: "<winPublic>/Documents/CPY_SAVES",
        label: "CPY",
    },
    Wrapper {
        template: "<winAppData>/SmartSteamEmu",
        label: "SmartSteamEmu",
    },
    Wrapper {
        template: "<winAppData>/SKIDROW",
        label: "SKIDROW",
    },
    Wrapper {
        template: "<winLocalAppData>/SKIDROW",
        label: "SKIDROW",
    },
    Wrapper {
        template: "<winPublic>/Documents/3DMGAME",
        label: "3DM",
    },
    Wrapper {
        template: "<winAppData>/FLT",
        label: "Fairlight",
    },
    Wrapper {
        template: "<winAppData>/ALi",
        label: "ALi",
    },
    Wrapper {
        template: "<winProgramData>/Steam/RLD!",
        label: "RELOADED",
    },
    // El genérico va el último: `%PUBLIC%/Documents/Steam` contiene a CODEX y
    // RUNE como subcarpetas, y quien mira primero gana (ver `is_app_id`, que
    // descarta esos nombres por no ser numéricos).
    Wrapper {
        template: "<winPublic>/Documents/Steam",
        label: "Steam emu",
    },
];

/// Subcarpetas de un wrapper que son configuración del propio emulador, no
/// un juego. `saves`/`remote` aparecen cuando el emulador guarda en plano en
/// vez de por AppID; ahí el contenedor entero ES el save y lo trata el walk
/// normal, no esta etapa.
const WRAPPER_SYSTEM_DIRS: &[&str] = &["settings", "remote", "saves", "stats", "storage"];

/// Un save encontrado dentro de un wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperHit {
    /// AppID de Steam cuando la subcarpeta es numérica, que es lo normal.
    pub app_id: Option<u64>,
    /// La carpeta que de verdad tiene los saves (ya estrechada).
    pub path: PathBuf,
    /// Nombre de la subcarpeta, para nombrar el hallazgo si no hay AppID.
    pub folder: String,
    pub wrapper: &'static str,
}

/// Wrappers en las rutas nativas del host. Vacío fuera de Windows: las
/// plantillas son `<win*>` y `expand_path` no las resuelve en otros SO.
pub fn discover_wrappers(os: Os) -> Vec<WrapperHit> {
    let mut out = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for w in WRAPPERS {
        for root in expand_path(w.template, os) {
            collect(&root, w.label, &mut out, &mut seen);
        }
    }
    out
}

/// Los mismos wrappers **dentro de un prefijo Wine/Proton**, que es donde
/// caen en Linux y en la Steam Deck: el repack corre bajo Proton y escribe en
/// el `drive_c` del prefijo, no en el home nativo.
pub fn discover_wrappers_in_prefix(prefix_root: &Path, user: &str) -> Vec<WrapperHit> {
    let mut out = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for w in WRAPPERS {
        for root in expand_path_in_prefix_as_user(w.template, prefix_root, user) {
            collect(&root, w.label, &mut out, &mut seen);
        }
    }
    out
}

/// Lista los juegos de un wrapper. `seen` evita que el wrapper genérico
/// `.../Documents/Steam` vuelva a ofrecer lo que CODEX y RUNE ya dieron.
fn collect(root: &Path, label: &'static str, out: &mut Vec<WrapperHit>, seen: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(folder) = name.to_str() else {
            continue;
        };
        let lower = folder.to_lowercase();
        if WRAPPER_SYSTEM_DIRS.contains(&lower.as_str())
            || crate::junkdirs::is_cache_dir_name(folder)
        {
            continue;
        }
        let container = entry.path();
        // Un subdirectorio ya cubierto por un wrapper más específico
        // (CODEX/RUNE dentro de `Documents/Steam`) no se repite.
        if seen.iter().any(|s| s == &container) {
            continue;
        }
        let path = resolve_game_container_dir(&container);
        if !dir_non_empty(&path) || !holds_anything_but_bookkeeping(&path) {
            continue;
        }
        seen.push(container);
        out.push(WrapperHit {
            app_id: folder.parse::<u64>().ok().filter(|_| is_app_id(folder)),
            path,
            folder: folder.to_string(),
            wrapper: label,
        });
    }
}

/// `true` si el nombre de carpeta es un AppID de Steam: sólo dígitos.
fn is_app_id(name: &str) -> bool {
    !name.is_empty() && name.len() <= 10 && name.bytes().all(|b| b.is_ascii_digit())
}

/// Estrecha una carpeta CONTENEDORA a la que de verdad guarda los saves.
///
/// Dos formas cubren lo que hay ahí dentro:
///
/// * `remote/` — el layout de Steam Cloud, que todos los emuladores copian.
/// * un único subdirectorio con nombre de save (`Saves`, `SaveData`, la forma
///   `Saved/SaveGames` de Unreal…) cuando el contenedor envuelve el árbol
///   propio del juego.
///
/// Si no hay ninguna de las dos, se devuelve el contenedor: muchos juegos y
/// emuladores escriben directamente ahí. Y si hay **varias** candidatas no se
/// adivina — acertar a medias es peor que ofrecer el contenedor, que el
/// usuario ve y puede corregir.
pub fn resolve_game_container_dir(dir: &Path) -> PathBuf {
    let remote = dir.join("remote");
    if remote.is_dir() {
        return remote;
    }
    let nested = crate::junkdirs::save_dirs_under(dir);
    if nested.len() == 1 && nested[0] != dir {
        return nested[0].clone();
    }
    dir.to_path_buf()
}

fn dir_non_empty(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut r| r.next().is_some())
}

/// Files the emulator writes for itself. None of them is a saved game:
/// achievements, stats, the Steam Cloud cache and the subscribed-groups lists
/// all appear on their own the first time the game runs, saved or not.
const WRAPPER_BOOKKEEPING_FILES: &[&str] = &[
    "achievements.json",
    "remotecache.vdf",
    "stats.txt",
    "stats.bin",
    "leaderboards.json",
    "subscribed_groups.json",
    "subscribed_groups_clans.json",
    "time.txt",
];

/// `true` if the folder holds anything that could be a saved game.
///
/// [`dir_non_empty`] wasn't enough, and Stellaris is the case that showed it: a
/// Goldberg repack leaves `GSE Saves/281990/achievements.json` **with no
/// `remote/`**, because the real saves live where the game has always put them
/// (`Documents/Paradox Interactive/Stellaris/save games`). That folder isn't
/// empty, so it was offered as the game's save on every sweep — a log line
/// every ten minutes, forever, about a directory with no game in it.
///
/// Deliberately conservative: it only rejects when **everything** there is
/// known emulator bookkeeping. One unknown file, one subdirectory, anything off
/// the list, and the folder passes — missing a real save is far worse than one
/// spurious offer.
fn holds_anything_but_bookkeeping(p: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(p) else {
        return false;
    };
    for entry in read.flatten() {
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            return true;
        }
        let name = entry.file_name();
        let lower = name.to_string_lossy().to_lowercase();
        if !WRAPPER_BOOKKEEPING_FILES.contains(&lower.as_str()) {
            return true;
        }
    }
    // Nothing worth offering — including the empty case. `dir_non_empty` also
    // rejects that one, and the two agreeing is the point: a caller that ever
    // drops the other check doesn't silently start offering empty folders.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn container_narrows_to_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("413150");
        touch(&app.join("remote/save.dat"));
        // Justo la basura que hace divergir dos máquinas si se rastrea el padre.
        touch(&app.join("remotecache.vdf"));
        touch(&app.join("playtime.txt"));
        assert_eq!(resolve_game_container_dir(&app), app.join("remote"));
    }

    /// The Stellaris case: the repack leaves the achievements file and nothing
    /// else, because the real saves go where the game has always written them.
    #[test]
    fn a_folder_with_only_emulator_bookkeeping_is_not_a_save() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("281990");
        touch(&app.join("achievements.json"));
        assert!(
            dir_non_empty(&app),
            "not empty — which is why it used to pass"
        );
        assert!(
            !holds_anything_but_bookkeeping(&app),
            "emulator bookkeeping only: not a save"
        );

        // One unknown file and the folder passes again: rejecting too much
        // costs a save, rejecting too little costs a log line.
        touch(&app.join("campaign01.sav"));
        assert!(holds_anything_but_bookkeeping(&app));
    }

    #[test]
    fn container_narrows_to_a_single_save_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("1091500");
        touch(&app.join("Saved/SaveGames/slot.sav"));
        // La hija más específica gana sobre `Saved`.
        assert_eq!(
            resolve_game_container_dir(&app),
            app.join("Saved").join("SaveGames")
        );
    }

    #[test]
    fn container_stays_put_when_ambiguous_or_flat() {
        let tmp = tempfile::tempdir().unwrap();
        // Plano: el save cuelga directamente del contenedor.
        let flat = tmp.path().join("flat");
        touch(&flat.join("game.sav"));
        assert_eq!(resolve_game_container_dir(&flat), flat);

        // Ambiguo: dos candidatas, no se adivina.
        let ambiguous = tmp.path().join("ambiguous");
        touch(&ambiguous.join("saves/a.sav"));
        touch(&ambiguous.join("savedata/b.sav"));
        assert_eq!(resolve_game_container_dir(&ambiguous), ambiguous);
    }

    #[test]
    fn collect_reads_appids_and_skips_emulator_plumbing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("GSE Saves");
        touch(&root.join("413150/remote/save.dat"));
        touch(&root.join("settings/user.ini")); // config del emulador
        touch(&root.join("MyGame/saves/x.sav")); // sin AppID, pero es un juego
        std::fs::create_dir_all(root.join("empty")).unwrap(); // vacío: se ignora

        let mut out = Vec::new();
        let mut seen = Vec::new();
        collect(&root, "GSE", &mut out, &mut seen);
        out.sort_by(|a, b| a.folder.cmp(&b.folder));

        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0].app_id, Some(413150));
        assert_eq!(out[0].path, root.join("413150").join("remote"));
        assert_eq!(out[1].app_id, None);
        assert_eq!(out[1].folder, "MyGame");
        assert_eq!(out[1].path, root.join("MyGame").join("saves"));
    }

    #[test]
    fn a_non_numeric_folder_is_not_an_appid() {
        assert!(is_app_id("413150"));
        assert!(!is_app_id("CODEX"));
        assert!(!is_app_id(""));
        // Un nombre absurdamente largo no es un AppID aunque sea numérico.
        assert!(!is_app_id("12345678901234"));
    }
}
