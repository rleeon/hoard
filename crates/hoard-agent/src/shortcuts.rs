//! Non-Steam games added to Steam, and the Proton prefix each one runs in.
//!
//! Steam gives every non-Steam shortcut its own prefix under
//! `steamapps/compatdata/<appid>`, where the appid is the shortcut's, recorded
//! in `userdata/<user>/config/shortcuts.vdf` (binary KeyValues) next to the
//! name the user sees. That name is the only thing on disk that says which game
//! the prefix belongs to: the catalog cannot resolve the appid, because Steam
//! derives it from the exe path and it differs on every machine.
//!
//! Unifideck (a Decky plugin that installs Epic, GOG, GameVault… games behind
//! shortcuts) keeps one prefix per game in `~/.local/share/unifideck/prefixes`
//! and links `compatdata/<appid>` to it, so the same rule covers it. Its
//! `games.map` also tells which shortcuts are games it installed, which matters
//! for games the catalog does not know: a shortcut named after an unknown title
//! could just as well be a launcher with a dozen games inside its prefix.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use hoard_manifest::ludusavi;

use crate::wine_prefixes::{PrefixKind, WinePrefix};

/// One non-Steam shortcut from `shortcuts.vdf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// Unsigned, the way Steam names the `compatdata` folder. The file stores
    /// it signed.
    pub app_id: u32,
    pub name: String,
}

/// The game a prefix belongs to, as detection should name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixOwner {
    pub slug: String,
    pub display_name: String,
}

/// Every shortcut of every Steam account on the machine.
pub fn list(libraries: &[PathBuf]) -> Vec<Shortcut> {
    let mut out: Vec<Shortcut> = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();
    for user in crate::steam::steam_user_dirs(libraries).unwrap_or_default() {
        let Ok(bytes) = std::fs::read(user.join("config/shortcuts.vdf")) else {
            continue;
        };
        for s in parse_vdf(&bytes) {
            if seen.insert(s.app_id) {
                out.push(s);
            }
        }
    }
    out
}

/// Appids of the shortcuts Unifideck put in front of a game it installed.
pub fn unifideck_games() -> HashSet<u32> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return HashSet::new();
    };
    std::fs::read_to_string(home.join(".local/share/unifideck/games.map"))
        .map(|text| parse_games_map(&text))
        .unwrap_or_default()
}

/// The prefixes that belong to exactly one game, keyed by `prefix_root` as
/// listed.
///
/// A shortcut's name has to resolve to the catalog, or the shortcut has to be
/// one Unifideck made for a game. And no other `compatdata` entry may lead to
/// the same real prefix: a launcher run from Steam (Battle.net, EA, Ubisoft
/// Connect) installs every game inside its own prefix, and tools that add one
/// shortcut per game point them all at that prefix through links. Such a
/// prefix belongs to no game in particular.
pub fn prefix_owners(
    prefixes: &[WinePrefix],
    shortcuts: &[Shortcut],
    unifideck: &HashSet<u32>,
) -> HashMap<PathBuf, PrefixOwner> {
    let by_id: HashMap<u32, &Shortcut> = shortcuts.iter().map(|s| (s.app_id, s)).collect();
    let proton: Vec<&WinePrefix> = prefixes
        .iter()
        .filter(|p| p.kind == PrefixKind::Proton)
        .collect();
    let mut sharing: HashMap<PathBuf, usize> = HashMap::new();
    for p in &proton {
        *sharing.entry(real(&p.prefix_root)).or_default() += 1;
    }

    let mut out = HashMap::new();
    for p in proton {
        let Some(shortcut) = p
            .identifier
            .parse::<u32>()
            .ok()
            .and_then(|id| by_id.get(&id))
        else {
            continue;
        };
        if sharing.get(&real(&p.prefix_root)).copied().unwrap_or(0) > 1 {
            tracing::info!(
                target: "hoard::detect",
                shortcut = %shortcut.name,
                prefix = %p.prefix_root.display(),
                "detect: shortcut prefix shared with another shortcut, no single owner"
            );
            continue;
        }
        let owner = match catalog_entry(&shortcut.name) {
            Some(entry) => PrefixOwner {
                slug: entry.slug.clone(),
                display_name: entry.display_name.clone(),
            },
            None if unifideck.contains(&shortcut.app_id) => PrefixOwner {
                slug: crate::library::catalog_slug(
                    &ludusavi::slugify(&shortcut.name),
                    Some(&shortcut.name),
                ),
                display_name: shortcut.name.clone(),
            },
            None => continue,
        };
        if owner.slug.is_empty() {
            continue;
        }
        out.insert(p.prefix_root.clone(), owner);
    }
    out
}

/// Store titles carry suffixes the catalog leaves out ("Deluxe Edition"), so a
/// name that misses exactly gets one more try on its leading words.
fn catalog_entry(name: &str) -> Option<&'static ludusavi::LudusaviEntry> {
    ludusavi::find_by_canon_name(name).or_else(|| ludusavi::find_by_name_prefix(name))
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Reads the shortcuts out of a binary KeyValues file. A file it cannot follow
/// to the end gives nothing rather than half: Steam rewrites it whole, so a
/// truncated one is being written right now and the next scan reads it again.
fn parse_vdf(bytes: &[u8]) -> Vec<Shortcut> {
    let mut reader = Reader { buf: bytes, pos: 0 };
    let Some(root) = read_map(&mut reader, 0) else {
        return Vec::new();
    };
    let Some(Value::Map(entries)) = field(&root, "shortcuts") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|(_, v)| match v {
            Value::Map(fields) => Some(fields),
            _ => None,
        })
        .filter_map(|fields| {
            let Some(Value::Int(id)) = field(fields, "appid") else {
                return None;
            };
            let Some(Value::Str(name)) = field(fields, "appname") else {
                return None;
            };
            let name = name.trim();
            (!name.is_empty()).then(|| Shortcut {
                app_id: *id as u32,
                name: name.to_string(),
            })
        })
        .collect()
}

/// Steam has written `AppName` and `appname` in different versions.
fn field<'a>(map: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    map.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

enum Value {
    Map(Vec<(String, Value)>),
    Str(String),
    Int(i32),
    Other,
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let out = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(out)
    }

    fn cstr(&mut self) -> Option<String> {
        let len = self.buf.get(self.pos..)?.iter().position(|b| *b == 0)?;
        let s = String::from_utf8_lossy(self.take(len)?).into_owned();
        self.pos += 1;
        Some(s)
    }
}

fn read_map(r: &mut Reader, depth: usize) -> Option<Vec<(String, Value)>> {
    if depth > 16 {
        return None;
    }
    let mut out = Vec::new();
    loop {
        let kind = *r.take(1)?.first()?;
        if kind == 0x08 || kind == 0x0B {
            return Some(out);
        }
        let key = r.cstr()?;
        let value = match kind {
            0x00 => Value::Map(read_map(r, depth + 1)?),
            0x01 => Value::Str(r.cstr()?),
            0x02 => Value::Int(i32::from_le_bytes(r.take(4)?.try_into().ok()?)),
            0x03 => {
                r.take(4)?;
                Value::Other
            }
            0x07 | 0x0A => {
                r.take(8)?;
                Value::Other
            }
            _ => return None,
        };
        out.push((key, value));
    }
}

/// `games.map` lines are `store:id=exe\twork_dir\tappid`, the appid signed and
/// `0` or `-1` while Unifideck has not learnt it yet. Older lines have no third
/// column.
fn parse_games_map(text: &str) -> HashSet<u32> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('=').map(|(_, v)| v))
        .filter_map(|v| v.split('\t').nth(2))
        .filter_map(|id| id.trim().parse::<i64>().ok())
        .filter(|id| *id != 0 && *id != -1)
        .map(|id| id as u32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vdf_shortcut(out: &mut Vec<u8>, index: &str, app_id: i32, name_key: &str, name: &str) {
        out.push(0x00);
        out.extend_from_slice(index.as_bytes());
        out.push(0);
        out.push(0x02);
        out.extend_from_slice(b"appid\0");
        out.extend_from_slice(&app_id.to_le_bytes());
        out.push(0x01);
        out.extend_from_slice(name_key.as_bytes());
        out.push(0);
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        out.push(0x01);
        out.extend_from_slice(b"Exe\0\"/games/x/game.exe\"\0");
        out.push(0x02);
        out.extend_from_slice(b"LastPlayTime\0");
        out.extend_from_slice(&0i32.to_le_bytes());
        out.push(0x00);
        out.extend_from_slice(b"tags\0");
        out.push(0x01);
        out.extend_from_slice(b"0\0Unifideck\0");
        out.push(0x08);
        out.push(0x08);
    }

    fn vdf(entries: &[(i32, &str, &str)]) -> Vec<u8> {
        let mut out = vec![0x00];
        out.extend_from_slice(b"shortcuts\0");
        for (i, (id, key, name)) in entries.iter().enumerate() {
            vdf_shortcut(&mut out, &i.to_string(), *id, key, name);
        }
        out.push(0x08);
        out.push(0x08);
        out
    }

    fn proton(id: u32, root: &Path) -> WinePrefix {
        WinePrefix {
            kind: PrefixKind::Proton,
            identifier: id.to_string(),
            prefix_root: root.to_path_buf(),
        }
    }

    #[test]
    fn reads_the_appid_unsigned_and_either_spelling_of_the_name() {
        // -1_294_967_296 is how the file stores 3_000_000_000.
        let parsed = parse_vdf(&vdf(&[
            (-1_294_967_296, "AppName", "Hollow Knight"),
            (-123, "appname", "Celeste"),
        ]));
        assert_eq!(
            parsed,
            vec![
                Shortcut {
                    app_id: 3_000_000_000,
                    name: "Hollow Knight".into()
                },
                Shortcut {
                    app_id: (-123i32) as u32,
                    name: "Celeste".into()
                },
            ]
        );
    }

    #[test]
    fn a_truncated_file_gives_nothing() {
        let bytes = vdf(&[(-5, "AppName", "Celeste")]);
        assert!(parse_vdf(&bytes[..bytes.len() - 3]).is_empty());
        assert!(parse_vdf(&[]).is_empty());
    }

    #[test]
    fn games_map_keeps_known_appids_only() {
        let ids = parse_games_map(
            "# comment\n\
             gamevault:12=/g/a.exe\t/g\t-1294967296\n\
             gog:70=/games/HL2/hl2.exe\t/games/HL2\t-1\n\
             epic:a=/x\t/y\t0\n\
             itch:9=/old/v1.exe\n",
        );
        assert_eq!(ids, HashSet::from([3_000_000_000u32]));
    }

    #[test]
    fn a_catalogued_name_owns_its_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("compatdata/3000000000/pfx");
        std::fs::create_dir_all(&root).unwrap();
        let owners = prefix_owners(
            &[proton(3_000_000_000, &root)],
            &[Shortcut {
                app_id: 3_000_000_000,
                name: "Hollow Knight".into(),
            }],
            &HashSet::new(),
        );
        assert_eq!(owners[&root].slug, "hollow-knight");
    }

    /// An unknown title is only a game when Unifideck says it installed one
    /// there: the same name on a hand-made shortcut could be a launcher.
    #[test]
    fn an_unknown_name_needs_unifideck() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("compatdata/3000000000/pfx");
        std::fs::create_dir_all(&root).unwrap();
        let prefixes = [proton(3_000_000_000, &root)];
        let shortcuts = [Shortcut {
            app_id: 3_000_000_000,
            name: "Zzqx Tiny Indie".into(),
        }];

        assert!(prefix_owners(&prefixes, &shortcuts, &HashSet::new()).is_empty());

        let owners = prefix_owners(&prefixes, &shortcuts, &HashSet::from([3_000_000_000]));
        assert_eq!(
            owners[&root],
            PrefixOwner {
                slug: "zzqx-tiny-indie".into(),
                display_name: "Zzqx Tiny Indie".into()
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_prefix_two_shortcuts_lead_to_has_no_owner() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("launcher-prefix/pfx");
        std::fs::create_dir_all(&shared).unwrap();
        let compat = tmp.path().join("compatdata");
        for id in ["3000000000", "3000000001"] {
            std::fs::create_dir_all(compat.join(id)).unwrap();
            std::os::unix::fs::symlink(&shared, compat.join(id).join("pfx")).unwrap();
        }
        let owners = prefix_owners(
            &[
                proton(3_000_000_000, &compat.join("3000000000/pfx")),
                proton(3_000_000_001, &compat.join("3000000001/pfx")),
            ],
            &[
                Shortcut {
                    app_id: 3_000_000_000,
                    name: "Hollow Knight".into(),
                },
                Shortcut {
                    app_id: 3_000_000_001,
                    name: "Celeste".into(),
                },
            ],
            &HashSet::new(),
        );
        assert!(owners.is_empty());
    }

    #[test]
    fn a_steam_game_prefix_is_not_a_shortcut() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("compatdata/367520/pfx");
        std::fs::create_dir_all(&root).unwrap();
        let owners = prefix_owners(
            &[proton(367_520, &root)],
            &[Shortcut {
                app_id: 3_000_000_000,
                name: "Hollow Knight".into(),
            }],
            &HashSet::new(),
        );
        assert!(owners.is_empty());
    }
}
