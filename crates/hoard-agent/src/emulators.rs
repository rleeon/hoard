//! Catálogo de emuladores y localización de sus carpetas de save.
//!
//! Un emulador reutiliza el sistema de ficheros del host, así que el motor ya
//! sabe respaldarlo en cuanto alguien señala la carpeta. Lo que no puede hacer
//! solo es *encontrarla*: no hay tienda, ni `install_dir`, ni entrada en el
//! manifiesto. Este módulo suple eso con un catálogo curado — carpetas de save
//! nativas y nombres de proceso típicos — y con dos sondeos que lo estiran
//! hasta donde vive la gente de verdad:
//!
//! 1. [`resolve_save_paths`] — las plantillas del catálogo expandidas y
//!    filtradas a lo que existe en este host (instalación normal).
//! 2. [`portable_save_paths`] — el mismo emulador **descomprimido** en otra
//!    unidad, que guarda junto al ejecutable en vez de en la carpeta de
//!    usuario.
//! 3. [`split_per_title`] — bajar de la raíz de saves de una consola a la
//!    carpeta de CADA juego, cuando el árbol intermedio lleva un identificador
//!    que no significa nada en la otra máquina.
//!
//! Vive en el agente y no en el desktop porque a partir del sondeo de unidades
//! esto es **detección**, y la detección la comparten los dos frontends: el
//! diálogo de "añadir emulador" y `hoard scan` preguntan lo mismo.
//!
//! El catálogo apunta a **saves nativos** (memory cards, carpetas por título),
//! nunca a savestates: dependen de la versión exacta del emulador y no
//! sobreviven a un viaje entre máquinas.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::expand_path;

/// Forma del árbol de saves de una consola, cuando ofrecer la raíz entera es
/// un error. Ver [`split_per_title`] para el porqué de cada una.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleLayout {
    /// `…/nand/user/save/<cuenta>/<uuid-de-perfil>/<title-id>/` — la estirpe
    /// de yuzu (y sus forks). El uuid de perfil se genera en la primera
    /// ejecución, así que es distinto en cada instalación.
    SwitchNand,
    /// `…/sdmc/Nintendo 3DS/<id0>/<id1>/title/<hi>/<lo>/data/` — Citra y
    /// Azahar. `id0`/`id1` derivan de las claves de la consola emulada, o sea
    /// que también son por instalación.
    Citra3ds,
}

/// Entrada del catálogo. Se mantiene separada del tipo que sale por el cable
/// para que las *plantillas* (multiplataforma, placeholders estilo Ludusavi)
/// vivan aquí y se expandan a rutas reales en el momento de preguntar.
pub struct EmulatorDef {
    /// Id estable; la UI forma el slug sintético del juego como `emu-<id>`.
    pub id: &'static str,
    pub display_name: &'static str,
    /// Consola / plataforma, como sublínea en el selector.
    pub system: &'static str,
    /// Nombres de ejecutable que marcan al emulador como "en marcha". El
    /// agente casa cualquiera de ellos (sin distinguir mayúsculas, nombre
    /// exacto), así que se listan las variantes de todos los SO y builds.
    pub processes: &'static [&'static str],
    /// Plantillas de carpeta de save nativa. Se expanden con [`expand_path`] y
    /// se filtran a las que existen; vacío (o todas ausentes) significa que el
    /// usuario elige la carpeta a mano — lo normal en emuladores portables que
    /// guardan junto a la ROM.
    pub save_templates: &'static [&'static str],
    /// Forma del árbol por título, si ofrecer la raíz entera rompe el sync.
    pub title_layout: Option<TitleLayout>,
}

/// Conjunto curado. Conservador a propósito: una ruta sugerida incorrecta es
/// peor que ninguna (el usuario acabaría respaldando una carpeta vacía), así
/// que sólo se listan carpetas que el emulador usa para saves **nativos** en
/// una instalación por defecto.
pub const CATALOG: &[EmulatorDef] = &[
    EmulatorDef {
        id: "pcsx2",
        display_name: "PCSX2",
        system: "PlayStation 2",
        processes: &[
            "pcsx2-qt.exe",
            "pcsx2-qtx64.exe",
            "pcsx2-qtx64-avx2.exe",
            "pcsx2.exe",
            "pcsx2",
        ],
        save_templates: &[
            "<winDocuments>/PCSX2/memcards",
            "<xdgConfig>/PCSX2/memcards",
            "<home>/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "rpcs3",
        display_name: "RPCS3",
        system: "PlayStation 3",
        processes: &["rpcs3.exe", "rpcs3"],
        save_templates: &[
            "<xdgConfig>/rpcs3/dev_hdd0/home/00000001/savedata",
            "<home>/.config/rpcs3/dev_hdd0/home/00000001/savedata",
            "<home>/.var/app/net.rpcs3.RPCS3/config/rpcs3/dev_hdd0/home/00000001/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "duckstation",
        display_name: "DuckStation",
        system: "PlayStation 1",
        processes: &[
            "duckstation-qt-x64-ReleaseLTCG.exe",
            "duckstation-nogui-x64-ReleaseLTCG.exe",
            "duckstation-qt",
            "duckstation",
        ],
        // Windows moved to Local AppData; the README keeps Documents only for
        // "old installs", so it stays listed but never first — an install that
        // predates the move still has the folder, and existence filtering
        // picks whichever is real. Linux is the data dir, not config: the
        // official migration command moves the Flatpak tree *into*
        // `~/.local/share`, and the Flatpak itself has been seen under both
        // `config/` and `data/`, so both are offered and the one that exists
        // wins.
        save_templates: &[
            "<winLocalAppData>/DuckStation/memcards",
            "<winDocuments>/DuckStation/memcards",
            "<xdgData>/duckstation/memcards",
            "<home>/.var/app/org.duckstation.DuckStation/data/duckstation/memcards",
            "<home>/.var/app/org.duckstation.DuckStation/config/duckstation/memcards",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "shadps4",
        display_name: "shadPS4",
        system: "PlayStation 4",
        processes: &["shadPS4.exe", "shadps4"],
        save_templates: &[
            "<winAppData>/shadPS4/savedata",
            "<xdgData>/shadPS4/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "vita3k",
        display_name: "Vita3K",
        system: "PlayStation Vita",
        processes: &["Vita3K.exe", "Vita3K", "vita3k"],
        save_templates: &[
            "<winAppData>/Vita3K/Vita3K/ux0/user/00/savedata",
            "<xdgConfig>/Vita3K/Vita3K/ux0/user/00/savedata",
            "<home>/.local/share/Vita3K/Vita3K/ux0/user/00/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "ppsspp",
        display_name: "PPSSPP",
        system: "PSP",
        processes: &[
            "PPSSPPWindows64.exe",
            "PPSSPPWindows.exe",
            "PPSSPPSDL",
            "ppsspp-qt",
            "ppsspp",
        ],
        save_templates: &[
            "<winDocuments>/PPSSPP/PSP/SAVEDATA",
            "<xdgConfig>/ppsspp/PSP/SAVEDATA",
            "<home>/.config/ppsspp/PSP/SAVEDATA",
            "<home>/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "dolphin",
        display_name: "Dolphin",
        system: "GameCube / Wii",
        processes: &["Dolphin.exe", "dolphin-emu", "dolphin-emu-qt2"],
        save_templates: &[
            "<winDocuments>/Dolphin Emulator/GC",
            "<winDocuments>/Dolphin Emulator/Wii",
            "<xdgData>/dolphin-emu/GC",
            "<xdgData>/dolphin-emu/Wii",
            "<home>/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/GC",
            "<home>/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/Wii",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "cemu",
        display_name: "Cemu",
        system: "Wii U",
        processes: &["Cemu.exe", "cemu"],
        save_templates: &[
            "<winAppData>/Cemu/mlc01/usr/save",
            "<home>/.local/share/Cemu/mlc01/usr/save",
            "<home>/.var/app/info.cemu.Cemu/data/Cemu/mlc01/usr/save",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "ryujinx",
        display_name: "Ryujinx",
        system: "Switch",
        processes: &["Ryujinx.exe", "Ryujinx.Ava.exe", "Ryujinx"],
        // `bis/user/save/<save-data-id>`: el id lo asigna el propio emulador y
        // sólo su base de datos interna sabe a qué título corresponde, así que
        // NO se puede partir por título mirando el nombre de la carpeta. Se
        // ofrece la raíz, como siempre.
        save_templates: &[
            "<winAppData>/Ryujinx/bis/user/save",
            "<xdgConfig>/Ryujinx/bis/user/save",
            "<home>/.local/share/Ryujinx/bis/user/save",
            "<home>/.var/app/org.ryujinx.Ryujinx/config/Ryujinx/bis/user/save",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "yuzu",
        display_name: "yuzu",
        system: "Switch",
        processes: &["yuzu.exe", "yuzu-cmd.exe", "yuzu"],
        save_templates: &[
            "<winAppData>/yuzu/nand/user/save",
            "<home>/.local/share/yuzu/nand/user/save",
            "<home>/.var/app/org.yuzu_emu.yuzu/data/yuzu/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "eden",
        display_name: "Eden",
        system: "Switch",
        processes: &["eden.exe", "eden"],
        save_templates: &[
            "<winAppData>/eden/nand/user/save",
            "<home>/.local/share/eden/nand/user/save",
            "<home>/.var/app/dev.eden_emu.eden/data/eden/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "suyu",
        display_name: "Suyu",
        system: "Switch",
        processes: &["suyu.exe", "suyu"],
        save_templates: &[
            "<winAppData>/suyu/nand/user/save",
            "<home>/.local/share/suyu/nand/user/save",
            "<home>/.var/app/dev.suyu_emu.suyu/data/suyu/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "citron",
        display_name: "Citron",
        system: "Switch",
        processes: &["citron.exe", "citron"],
        save_templates: &[
            "<winAppData>/citron/nand/user/save",
            "<home>/.local/share/citron/nand/user/save",
            "<home>/.var/app/org.citron_emu.citron/data/citron/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "sudachi",
        display_name: "Sudachi",
        system: "Switch",
        processes: &["sudachi.exe", "sudachi"],
        save_templates: &[
            "<winAppData>/sudachi/nand/user/save",
            "<home>/.local/share/sudachi/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "citra",
        display_name: "Citra / Azahar",
        system: "Nintendo 3DS",
        processes: &[
            "citra-qt.exe",
            "azahar.exe",
            "lime3ds.exe",
            "citra.exe",
            "citra-qt",
            "lime3ds",
            "citra",
        ],
        save_templates: &[
            "<winAppData>/Citra/sdmc",
            "<winAppData>/Azahar/sdmc",
            "<winAppData>/Lime3DS/sdmc",
            "<xdgData>/citra-emu/sdmc",
            "<xdgData>/azahar-emu/sdmc",
            "<xdgData>/lime3ds-emu/sdmc",
            "<home>/.var/app/org.azahar_emu.Azahar/data/azahar-emu/sdmc",
        ],
        title_layout: Some(TitleLayout::Citra3ds),
    },
    EmulatorDef {
        id: "xemu",
        display_name: "xemu",
        system: "Xbox",
        processes: &["xemu.exe", "xemu"],
        save_templates: &[
            "<winAppData>/xemu/xemu/eeprom.bin",
            "<xdgData>/xemu/xemu",
            "<home>/.var/app/app.xemu.xemu/data/xemu/xemu",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "flycast",
        display_name: "Flycast",
        system: "Dreamcast",
        processes: &["flycast.exe", "flycast"],
        // No Windows template on purpose: the standalone build ships as a zip
        // with no installer and locates its own folder from the executable
        // path, so nothing ever lands in `%APPDATA%\flycast`. Offering it made
        // the dialog point at a folder that cannot exist. Windows installs are
        // found by `portable_save_paths`, which reuses the `flycast`/`data`
        // pair from the Linux template below — that row is load-bearing for
        // Windows detection even though it never expands there.
        save_templates: &[
            "<xdgData>/flycast/data",
            "<home>/.var/app/org.flycast.Flycast/data/flycast/data",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "retroarch",
        display_name: "RetroArch",
        system: "Multi-system",
        processes: &["retroarch.exe", "retroarch"],
        save_templates: &[
            "<winAppData>/RetroArch/saves",
            "<xdgConfig>/retroarch/saves",
            "<home>/.config/retroarch/saves",
            "<home>/.var/app/org.libretro.RetroArch/config/retroarch/saves",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "mgba",
        display_name: "mGBA",
        system: "Game Boy Advance",
        // Guarda junto a la ROM por defecto, así que no hay plantilla fiable.
        processes: &["mGBA.exe", "mgba-qt", "mgba"],
        save_templates: &[],
        title_layout: None,
    },
    EmulatorDef {
        id: "melonds",
        display_name: "melonDS",
        system: "Nintendo DS",
        processes: &["melonDS.exe", "melonDS", "melonds"],
        save_templates: &[],
        title_layout: None,
    },
    EmulatorDef {
        id: "project64",
        display_name: "Project64",
        system: "Nintendo 64",
        processes: &["Project64.exe"],
        // Project64 is portable by design: the manual puts auto saves in the
        // `Save` subfolder of the program folder, and nothing is written to
        // `%APPDATA%`. The template is kept anyway because it is the only
        // source of the `Project64`/`Save` pair that `portable_save_paths`
        // reanchors onto a real install — delete it and Windows detection goes
        // to zero. What is still wrong is the fallback: with no folder found,
        // `resolve_save_paths` offers this path, which will never exist. That
        // needs the entry to be able to say "portable only", not a different
        // template.
        save_templates: &["<winAppData>/Project64/Save"],
        title_layout: None,
    },
];

/// Busca una entrada del catálogo por su id.
pub fn find(id: &str) -> Option<&'static EmulatorDef> {
    CATALOG.iter().find(|d| d.id == id)
}

/// Expande las plantillas de una entrada contra este SO y conserva las
/// carpetas que existen, deduplicadas y en orden. Si no existe ninguna pero
/// alguna plantilla expande a una ruta concreta, devuelve esa única
/// mejor-apuesta para que el diálogo tenga algo que enseñar (el usuario puede
/// corregirla antes de añadir).
pub fn resolve_save_paths(def: &EmulatorDef) -> Vec<String> {
    let os = Os::current();
    let mut existing: Vec<String> = Vec::new();
    let mut first_guess: Option<String> = None;
    for tmpl in def.save_templates {
        for path in expand_path(tmpl, os) {
            let s = path.to_string_lossy().into_owned();
            if first_guess.is_none() {
                first_guess = Some(s.clone());
            }
            if path.is_dir() && !existing.contains(&s) {
                existing.push(s);
            }
        }
    }
    if existing.is_empty() {
        first_guess.into_iter().collect()
    } else {
        existing
    }
}

// ─── Instalaciones portables ────────────────────────────────────────────────

/// Carpetas donde una instalación portable guarda lo que una instalada pondría
/// en la carpeta de usuario. `""` es la propia raíz de la instalación.
const PORTABLE_USER_DIRS: &[&str] = &["", "user"];

/// Parte una plantilla anclada en la carpeta de usuario en (carpeta de la app,
/// cola por debajo). Devuelve `None` para plantillas sin cola —la carpeta de
/// la app *es* la de saves, no hay nada que reanclar— y para las que no salen
/// de un root reanclable (Documentos, Saved Games, una raíz de wrapper).
fn app_dir_and_tail(template: &str) -> Option<(&str, &str)> {
    // Sólo los roots "de aplicación": una plantilla de Documentos no tiene
    // equivalente portable, y `<home>` es demasiado ancho para deducir nada.
    const REANCHORABLE: &[&str] = &["<winAppData>/", "<xdgData>/", "<xdgConfig>/"];
    let rest = REANCHORABLE.iter().find_map(|p| template.strip_prefix(p))?;
    let (app_dir, tail) = rest.split_once('/')?;
    if app_dir.is_empty() || tail.is_empty() {
        return None;
    }
    Some((app_dir, tail))
}

/// ¿El nombre de este directorio nombra plausiblemente una instalación de
/// `app_dir`? Coincidencia exacta, o el nombre con un sufijo: las builds que
/// se descomprimen llegan como `RetroArch-Win64` o `Azahar-2120` mucho más a
/// menudo que con el nombre pelado.
fn looks_like_install_of(dir_name: &str, app_dir: &str) -> bool {
    let d = dir_name.to_lowercase();
    let a = app_dir.to_lowercase();
    if d == a {
        return true;
    }
    let Some(rest) = d.strip_prefix(&a) else {
        return false;
    };
    // Exige un separador tras el nombre para que "eden" no case con "edenring".
    matches!(
        rest.as_bytes().first(),
        Some(b'-' | b'_' | b' ' | b'.' | b'0'..=b'9')
    )
}

/// Carpetas de save de este emulador **descomprimido** en algún sitio, en vez
/// de instalado.
///
/// El catálogo localiza cada emulador por su carpeta de datos por usuario
/// (`%APPDATA%\RetroArch` y compañía), que es donde la deja un instalador — y
/// está en C: viva donde viva el ejecutable. Sólo que muchísima gente no
/// instala: RetroArch, la estirpe de Citra y la de yuzu se distribuyen como
/// una carpeta que descomprimes donde quieras, y en ese modo guardan sus datos
/// **junto al ejecutable**. Quien tiene sus emuladores en `D:\Emulators` no
/// tiene ningún `%APPDATA%\RetroArch`, el escaneo mira el único sitio donde no
/// están, y la app parece rota justo para el público con más emuladores.
///
/// La distribución interna de una instalación portable es la misma que la de
/// la carpeta de datos, sólo que colgando de otro sitio: o directo bajo la
/// raíz (los `saves/` de RetroArch) o bajo un `user/` junto al ejecutable (las
/// estirpes de Citra y yuzu). Así que se reutiliza la **cola** de cada
/// plantilla, y para dar por buena una candidata se exigen **las dos cosas**:
/// que la carpeta se llame como el emulador y que esa cola exista de verdad.
/// La cola sola no es prueba de nada — hay montones de carpetas de juego con
/// algo llamado `saves` dentro.
///
/// Acotado a propósito: un listado por unidad más uno por carpeta-colección,
/// sin recorrer nada. Un barrido completo de un disco de juegos leería decenas
/// de miles de directorios para encontrar un puñado de aciertos.
pub fn portable_save_paths(def: &EmulatorDef) -> Vec<PathBuf> {
    let tails: Vec<(&str, &str)> = def
        .save_templates
        .iter()
        .filter_map(|t| app_dir_and_tail(t))
        .collect();
    if tails.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for root in crate::roots::portable_install_roots(Os::current()) {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let dir = entry.path();
            let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            for (app_dir, tail) in &tails {
                if !looks_like_install_of(name, app_dir) {
                    continue;
                }
                for user_dir in PORTABLE_USER_DIRS {
                    let candidate = if user_dir.is_empty() {
                        dir.join(tail)
                    } else {
                        dir.join(user_dir).join(tail)
                    };
                    if candidate.is_dir() && seen.insert(candidate.clone()) {
                        out.push(candidate);
                    }
                }
            }
        }
    }
    out
}

// ─── Partición por título ───────────────────────────────────────────────────

/// Una carpeta de save de un juego concreto dentro del árbol de una consola.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleSave {
    /// Identificador del título tal cual lo nombra la carpeta (16 hex en
    /// Switch, `<hi>/<lo>` en 3DS). Es lo único que las dos instalaciones
    /// llaman igual.
    pub title_id: String,
    pub path: PathBuf,
}

/// ¿Es este nombre un id de título de Switch? 16 dígitos hexadecimales.
/// Casar por forma evita ofrecer como juegos las copias de seguridad y los
/// directorios de trabajo que el emulador deja al lado.
fn is_switch_title_id(name: &str) -> bool {
    name.len() == 16 && name.chars().all(|c| c.is_ascii_hexdigit())
}

/// ¿Tiene contenido esta carpeta? El emulador crea una por cada título que se
/// haya lanzado alguna vez, aunque nunca haya guardado nada.
fn has_any_file(dir: &Path) -> bool {
    let mut stack = vec![dir.to_path_buf()];
    let mut budget = 64; // suficiente para decidir; no es un recorrido completo
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            budget -= 1;
            if budget < 0 {
                return false;
            }
            match entry.file_type() {
                Ok(t) if t.is_file() => return true,
                Ok(t) if t.is_dir() => stack.push(entry.path()),
                _ => {}
            }
        }
    }
    false
}

/// Baja de la raíz de saves de una consola a la carpeta de cada juego.
///
/// Ofrecer la raíz entera como una sola carpeta de save mete dentro del árbol
/// sincronizado un identificador que **se genera en cada instalación** — el
/// uuid del perfil en la estirpe de yuzu, el par `id0`/`id1` derivado de las
/// claves de consola en Citra. La copia que llega a la otra máquina queda
/// entonces colgando de un perfil que su emulador no ha visto nunca, y el
/// emulador informa de un save sin perfil asociado. Nada en ese mensaje apunta
/// a la sincronización, que es por lo que se lee como un fallo del emulador o
/// como que uno lo ha configurado mal.
///
/// La carpeta del título es la parte en la que las dos instalaciones sí están
/// de acuerdo, así que es la que se ofrece, una por juego.
///
/// **Lo importante es el fallback.** Las bifurcaciones y las versiones varían,
/// y una suposición de distribución que falle deja al usuario sin detección
/// ninguna — que es peor que el problema del identificador. Así que una forma
/// que no se reconozca cae a ofrecer la raíz tal cual, y sólo un árbol que
/// encaje del todo se parte por título.
pub fn split_per_title(root: &Path, layout: TitleLayout) -> Vec<TitleSave> {
    match layout {
        TitleLayout::SwitchNand => split_switch_nand(root),
        TitleLayout::Citra3ds => split_citra_sdmc(root),
    }
}

/// `<raíz>/<cuenta>/<uuid-de-perfil>/<title-id>/`. Dos niveles opacos y luego
/// el título; se aceptan también árboles con un solo nivel intermedio, que es
/// como quedan algunas builds.
fn split_switch_nand(root: &Path) -> Vec<TitleSave> {
    let mut out = Vec::new();
    for level1 in read_dirs(root) {
        for level2 in read_dirs(&level1) {
            let Some(name) = level2.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if is_switch_title_id(name) && has_any_file(&level2) {
                out.push(TitleSave {
                    title_id: name.to_string(),
                    path: level2.clone(),
                });
                continue;
            }
            // Un nivel más abajo: <cuenta>/<perfil>/<title-id>.
            for level3 in read_dirs(&level2) {
                let Some(name) = level3.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if is_switch_title_id(name) && has_any_file(&level3) {
                    out.push(TitleSave {
                        title_id: name.to_string(),
                        path: level3,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

/// `<sdmc>/Nintendo 3DS/<id0>/<id1>/title/<hi>/<lo>/data/`. El save vive en
/// `data`; se ofrece esa carpeta y el título se nombra `<hi><lo>`.
fn split_citra_sdmc(root: &Path) -> Vec<TitleSave> {
    let mut out = Vec::new();
    let base = if root.join("Nintendo 3DS").is_dir() {
        root.join("Nintendo 3DS")
    } else {
        root.to_path_buf()
    };
    for id0 in read_dirs(&base) {
        for id1 in read_dirs(&id0) {
            let titles = id1.join("title");
            if !titles.is_dir() {
                continue;
            }
            for hi in read_dirs(&titles) {
                for lo in read_dirs(&hi) {
                    let data = lo.join("data");
                    if !data.is_dir() || !has_any_file(&data) {
                        continue;
                    }
                    let hi_name = hi.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    let lo_name = lo.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    out.push(TitleSave {
                        title_id: format!("{hi_name}{lo_name}"),
                        path: data,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

// ─── La raíz de un emulador, vista desde la detección ───────────────────────

/// The emulator whose save root this path **is**, if any.
///
/// The catalog can't answer this by comparing expanded paths: the roots that
/// reach detection are the ones the templates missed. rpcs3 on macOS lives
/// under `~/Library/Application Support`, and under RetroDECK it lives inside
/// `~/retrodeck` — neither is `<xdgConfig>`, and both are still rpcs3.
///
/// So the match is on the **tail** of the template, which is the part that
/// belongs to the emulator instead of to the host: `rpcs3/dev_hdd0/home/
/// <profile>/savedata` identifies rpcs3 wherever the tree was rooted. A
/// numeric template segment matches any numeric segment of the same width,
/// because those are per-install: `00000001` is only the *first* rpcs3
/// profile, and the account of someone on their second is `00000002`.
pub fn save_root_at(path: &Path) -> Option<&'static EmulatorDef> {
    CATALOG.iter().find(|def| {
        def.save_templates
            .iter()
            .filter_map(|t| template_tail(t))
            .any(|tail| path_ends_with_tail(path, tail))
    })
}

/// La raíz de emulador **por encima** de `path`, con la carpeta de título por
/// la que se entró.
///
/// El barrido no siempre aterriza en la raíz: baja hasta donde hay ficheros,
/// así que lo que trae es `…/savedata/BLUS30443` o algo aún más hondo. Sin
/// esto, esa carpeta se atribuye sola y vuelve a salir nombrada por el árbol
/// del emulador en vez de por el emulador.
///
/// `None` si `path` no cuelga de ninguna raíz conocida, o si ES la raíz —para
/// eso está [`save_root_at`], y devolver ambas cosas por el mismo sitio haría
/// que quien pregunta tuviera que desempatar.
pub fn save_root_above(path: &Path) -> Option<(&'static EmulatorDef, PathBuf)> {
    let mut title = path;
    while let Some(parent) = title.parent() {
        if let Some(def) = save_root_at(parent) {
            return Some((def, title.to_path_buf()));
        }
        title = parent;
    }
    None
}

/// La parte de una plantilla por debajo de su placeholder de raíz.
fn template_tail(template: &str) -> Option<&str> {
    let (_, tail) = template.strip_prefix('<')?.split_once(">/")?;
    Some(tail).filter(|t| !t.is_empty())
}

/// ¿Termina `path` en esta cola de plantilla? Compara por componentes y sin
/// distinguir mayúsculas (macOS y Windows no las distinguen, y las plantillas
/// están escritas con la caja del proyecto de cada emulador).
fn path_ends_with_tail(path: &Path, tail: &str) -> bool {
    let want: Vec<&str> = tail.split('/').filter(|s| !s.is_empty()).collect();
    let have: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    if want.is_empty() || have.len() < want.len() {
        return false;
    }
    have[have.len() - want.len()..]
        .iter()
        .zip(&want)
        .all(|(h, w)| segment_matches(h, w))
}

/// Un segmento de la plantilla contra uno real. Los identificadores de
/// cuenta/perfil se comparan por FORMA, no por valor: ver [`save_root_at`].
fn segment_matches(have: &str, want: &str) -> bool {
    if want.chars().all(|c| c.is_ascii_digit()) {
        return have.len() == want.len() && have.chars().all(|c| c.is_ascii_digit());
    }
    have.eq_ignore_ascii_case(want)
}

/// Las carpetas por título que hay dentro de la raíz de saves de `def`.
///
/// Vacío significa "esta raíz no se puede partir", y es una respuesta
/// legítima: una raíz recién creada no tiene ningún título dentro todavía.
/// Quien pregunta decide qué hacer con eso, pero lo que **no** puede hacer es
/// ofrecer la raíz entera como si fuera un save — ver [`split_per_title`].
pub fn titles_in(def: &EmulatorDef, root: &Path) -> Vec<TitleSave> {
    if let Some(layout) = def.title_layout {
        return split_per_title(root, layout);
    }
    // Sin distribución conocida, la forma genérica: una carpeta por título y
    // nada suelto en la raíz. El `has_direct_file` es la línea que separa un
    // contenedor de un save de verdad — RetroArch deja sus `.srm` sueltos en
    // `saves/`, así que esa carpeta ES el save y no hay nada que partir.
    if has_direct_file(root) {
        return Vec::new();
    }
    let mut out: Vec<TitleSave> = Vec::new();
    for dir in read_dirs(root) {
        if !has_any_file(&dir) {
            continue;
        }
        let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        out.push(TitleSave {
            title_id: name.to_string(),
            path: dir.clone(),
        });
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out
}

/// `true` si la raíz tiene ficheros **suyos**, sin bajar. Distingue el save
/// plano (RetroArch) del contenedor de carpetas por título (rpcs3).
pub fn has_direct_file(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    read.flatten()
        .any(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
}

/// Subdirectorios inmediatos de `dir`, ordenados. Vacío si no se puede leer:
/// aquí un error sólo significa "no hay nada que ofrecer por debajo".
fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Las raíces que producción rastreó como si fueran un save, cada una con
    /// el emulador que debería haberlas reclamado. Las rutas son las de los
    /// casos vistos: rpcs3 en macOS y en RetroDECK (ninguna de las dos es la
    /// que expande la plantilla), RetroArch, Ryujinx, Dolphin y Yuzu.
    #[test]
    fn an_emulator_save_root_is_recognised_wherever_it_was_installed() {
        let cases: &[(&str, Option<&str>)] = &[
            // rpcs3, el de las 224 líneas de "nothing to back up". El slug que
            // salía de aquí era `dev-hdd0`.
            (
                "/Users/u/Library/Application Support/rpcs3/dev_hdd0/home/00000001/savedata",
                Some("rpcs3"),
            ),
            // El mismo árbol dentro de RetroDECK.
            (
                "/home/u/retrodeck/saves/ps3/rpcs3/dev_hdd0/home/00000001/savedata",
                Some("rpcs3"),
            ),
            // Y con el perfil que NO es el primero: el id es por instalación,
            // así que se compara la forma, no el valor.
            (
                "/home/u/.config/rpcs3/dev_hdd0/home/00000002/savedata",
                Some("rpcs3"),
            ),
            (
                "/home/u/.var/app/org.libretro.RetroArch/config/retroarch/saves",
                Some("retroarch"),
            ),
            ("/home/u/.config/retroarch/saves", Some("retroarch")),
            (
                "/home/u/.local/share/Ryujinx/bis/user/save",
                Some("ryujinx"),
            ),
            ("/home/u/.local/share/dolphin-emu/GC", Some("dolphin")),
            ("/home/u/.local/share/dolphin-emu/Wii", Some("dolphin")),
            ("/home/u/.local/share/yuzu/nand/user/save", Some("yuzu")),
            ("/home/u/.local/share/Cemu/mlc01/usr/save", Some("cemu")),
            // Y lo que NO puede reclamar ningún emulador: la carpeta de UN
            // título dentro de la raíz, y un save cualquiera de un juego.
            (
                "/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443",
                None,
            ),
            (
                "/home/u/.local/share/Steam/steamapps/common/Stellaris",
                None,
            ),
            ("/home/u/Documents/My Games/Skyrim/Saves", None),
        ];
        for (raw, expected) in cases {
            let got = save_root_at(Path::new(raw)).map(|d| d.id);
            assert_eq!(got, *expected, "{raw}");
        }
    }

    /// Y desde dentro se llega a la raíz por arriba, que es como el barrido la
    /// encuentra de verdad: baja hasta donde hay ficheros, no para en la raíz.
    #[test]
    fn a_folder_inside_a_save_root_finds_the_root_above_it() {
        let deep = Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443/sub");
        let (def, title) = save_root_above(deep).expect("cuelga de la raíz de rpcs3");
        assert_eq!(def.id, "rpcs3");
        // La carpeta del TÍTULO, no la hoja donde aterrizó el barrido.
        assert_eq!(
            title,
            Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443")
        );

        // La raíz misma no cuelga de sí misma: eso lo contesta `save_root_at`.
        let root = Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata");
        assert!(save_root_above(root).is_none());
    }

    /// Partir la raíz: una fila por título cuando hay títulos, y **nada**
    /// cuando la raíz está vacía —que es el caso de las 224 líneas: rpcs3
    /// instalado y el `savedata` del primer perfil sin estrenar.
    #[test]
    fn a_container_root_splits_per_title_and_an_empty_one_offers_nothing() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("savedata");
        fs::create_dir_all(&root).unwrap();
        let rpcs3 = find("rpcs3").unwrap();

        // Vacía: no hay título que ofrecer, y la raíz NO vale como save.
        assert!(titles_in(rpcs3, &root).is_empty());
        assert!(!has_direct_file(&root));

        // Con dos títulos dentro, uno por fila.
        for title in ["BLUS30443-AUTOSAVE", "NPUB30493-SAVEDATA01"] {
            let d = root.join(title);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("PARAM.SFO"), b"x").unwrap();
        }
        // Y una carpeta vacía, que no es un título: no hay nada que respaldar.
        fs::create_dir_all(root.join("EMPTY00000")).unwrap();

        let titles = titles_in(rpcs3, &root);
        assert_eq!(
            titles
                .iter()
                .map(|t| t.title_id.as_str())
                .collect::<Vec<_>>(),
            ["BLUS30443-AUTOSAVE", "NPUB30493-SAVEDATA01"]
        );
    }

    /// La otra mitad: una raíz con los ficheros sueltos dentro NO es un
    /// contenedor. RetroArch deja sus `.srm` en `saves/`, así que esa carpeta
    /// ES el save y partirla la destrozaría.
    #[test]
    fn a_flat_save_root_is_not_a_container() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("saves");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Chrono Trigger.srm"), b"x").unwrap();
        let retroarch = find("retroarch").unwrap();

        assert!(has_direct_file(&root));
        assert!(
            titles_in(retroarch, &root).is_empty(),
            "no hay títulos que partir: los saves son los ficheros"
        );
    }

    #[test]
    fn every_catalog_id_is_unique() {
        let mut seen = HashSet::new();
        for def in CATALOG {
            assert!(seen.insert(def.id), "id duplicado: {}", def.id);
        }
    }

    #[test]
    fn install_names_match_with_a_suffix_but_not_a_longer_word() {
        assert!(looks_like_install_of("RetroArch", "RetroArch"));
        assert!(looks_like_install_of("retroarch-win64", "RetroArch"));
        assert!(looks_like_install_of("Azahar-2120", "Azahar"));
        assert!(looks_like_install_of("eden 0.1", "eden"));
        // El caso que obliga al separador: "edenring" no es una build de Eden.
        assert!(!looks_like_install_of("edenring", "eden"));
        assert!(!looks_like_install_of("Elden Ring", "eden"));
    }

    #[test]
    fn only_app_rooted_templates_have_a_portable_equivalent() {
        assert_eq!(
            app_dir_and_tail("<winAppData>/RetroArch/saves"),
            Some(("RetroArch", "saves"))
        );
        assert_eq!(
            app_dir_and_tail("<xdgData>/citra-emu/sdmc"),
            Some(("citra-emu", "sdmc"))
        );
        // Sin cola: la carpeta de la app ES la de saves.
        assert_eq!(app_dir_and_tail("<winAppData>/RetroArch"), None);
        // Documentos y Saved Games no se reanclan.
        assert_eq!(app_dir_and_tail("<winDocuments>/PCSX2/memcards"), None);
        assert_eq!(app_dir_and_tail("<home>/.config/retroarch/saves"), None);
    }

    #[test]
    fn portable_only_emulators_keep_a_reanchorable_template() {
        // Flycast and Project64 write next to their executable on Windows, so
        // neither has a correct `%APPDATA%` path to offer. Detection there
        // runs entirely through `portable_save_paths`, which needs some
        // template with an app-rooted shape to borrow the folder name and
        // tail from. Drop the last one and Windows detection silently goes to
        // zero, which is why these two rows cannot be trimmed to nothing.
        for id in ["flycast", "project64"] {
            let def = find(id).unwrap();
            assert!(
                def.save_templates
                    .iter()
                    .any(|t| app_dir_and_tail(t).is_some()),
                "{id} lost the template that feeds portable detection"
            );
        }
    }

    #[test]
    fn a_switch_nand_tree_splits_into_one_entry_per_title() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let profile = root.join("0000000000000000/78b4e1c9a0f24d3b8e5f6a7c9d0e1f2a");
        for title in ["0100152000022000", "01007ef00011e000"] {
            let t = profile.join(title);
            fs::create_dir_all(&t).unwrap();
            fs::write(t.join("save.dat"), b"x").unwrap();
        }
        // Vacía: el emulador la crea por cada título lanzado, no es un save.
        fs::create_dir_all(profile.join("0100abcd00099000")).unwrap();
        // Ni carpeta de trabajo ni copia: la forma no es un id de título.
        fs::create_dir_all(profile.join("backup")).unwrap();

        let found = split_per_title(root, TitleLayout::SwitchNand);
        let ids: Vec<&str> = found.iter().map(|t| t.title_id.as_str()).collect();
        assert_eq!(ids, vec!["0100152000022000", "01007ef00011e000"]);
    }

    #[test]
    fn an_unrecognised_shape_yields_nothing_so_the_caller_keeps_the_root() {
        // La forma que obliga al fallback: save/0000/save.bin, un solo nivel.
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("0000")).unwrap();
        fs::write(root.join("0000/save.bin"), b"x").unwrap();

        assert!(split_per_title(root, TitleLayout::SwitchNand).is_empty());
    }

    #[test]
    fn a_citra_sdmc_tree_splits_at_the_data_folder() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let data = root
            .join("Nintendo 3DS")
            .join("00000000000000000000000000000000")
            .join("11111111111111111111111111111111")
            .join("title/00040000/00055d00/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("00000001.sav"), b"x").unwrap();

        let found = split_per_title(root, TitleLayout::Citra3ds);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title_id, "0004000000055d00");
        assert_eq!(found[0].path, data);
    }

    #[test]
    fn an_empty_title_folder_is_not_offered() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let t = root.join("0000000000000000/78b4e1c9a0f24d3b8e5f6a7c9d0e1f2a/0100152000022000");
        fs::create_dir_all(&t).unwrap();

        assert!(split_per_title(root, TitleLayout::SwitchNand).is_empty());
    }
}
