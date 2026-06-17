//! PLAYTIME CATALOG — juegos online (o sin partida local que merezca copia)
//! cuyo tiempo de juego cuenta para el recap (hoard-wrapple) pero de los que
//! NO guardamos nada. El poll de procesos del agente los empareja por nombre de
//! ejecutable (case-insensitive, exacto), así que cuentan en cuanto su binario
//! corre, venga del launcher que venga (Steam, Epic, Battle.net, standalone).
//!
//! Esto es la contrapartida "solo-tiempo" de la detección de saves: estos
//! juegos se enrolan como [`crate::agent::WatchedSave`] con `track_only = true`.
//! La lista es curada a propósito (juegos reconocibles que tienen sentido en un
//! Wrapped); ampliarla es añadir una fila aquí.

/// Un juego que rastreamos solo por tiempo de juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaytimeGame {
    /// Slug estable (clave de atribución de horas y de la lista de exclusión).
    pub slug: &'static str,
    /// Nombre legible para la UI y el recap.
    pub display_name: &'static str,
    /// Nombres de ejecutable (en minúsculas) que el poll de procesos empareja.
    /// Se cubren variantes Windows y nativas porque el match es exacto.
    pub processes: &'static [&'static str],
}

/// Catálogo curado. Nombres de proceso en minúsculas (el match los baja a
/// minúsculas en ambos lados, pero los guardamos así para el lookup directo).
pub const PLAYTIME_CATALOG: &[PlaytimeGame] = &[
    PlaytimeGame {
        slug: "fortnite",
        display_name: "Fortnite",
        processes: &["fortniteclient-win64-shipping.exe"],
    },
    PlaytimeGame {
        slug: "rust",
        display_name: "Rust",
        processes: &["rustclient.exe", "rust.exe", "rustclient"],
    },
    PlaytimeGame {
        slug: "valorant",
        display_name: "VALORANT",
        processes: &["valorant-win64-shipping.exe"],
    },
    PlaytimeGame {
        slug: "league-of-legends",
        display_name: "League of Legends",
        processes: &["league of legends.exe"],
    },
    PlaytimeGame {
        slug: "counter-strike-2",
        display_name: "Counter-Strike 2",
        processes: &["cs2.exe", "cs2"],
    },
    PlaytimeGame {
        slug: "dota-2",
        display_name: "Dota 2",
        processes: &["dota2.exe", "dota2"],
    },
    PlaytimeGame {
        slug: "apex-legends",
        display_name: "Apex Legends",
        processes: &["r5apex.exe", "r5apex_dx12.exe"],
    },
    PlaytimeGame {
        slug: "overwatch-2",
        display_name: "Overwatch 2",
        processes: &["overwatch.exe"],
    },
    PlaytimeGame {
        slug: "grand-theft-auto-v",
        display_name: "Grand Theft Auto V",
        processes: &["gta5.exe", "gta5_enhanced.exe"],
    },
    PlaytimeGame {
        slug: "destiny-2",
        display_name: "Destiny 2",
        processes: &["destiny2.exe"],
    },
    PlaytimeGame {
        slug: "warframe",
        display_name: "Warframe",
        processes: &["warframe.x64.exe", "warframe.exe"],
    },
    PlaytimeGame {
        slug: "rocket-league",
        display_name: "Rocket League",
        processes: &["rocketleague.exe"],
    },
    PlaytimeGame {
        slug: "world-of-warcraft",
        display_name: "World of Warcraft",
        processes: &["wow.exe", "wowclassic.exe"],
    },
    PlaytimeGame {
        slug: "roblox",
        display_name: "Roblox",
        processes: &["robloxplayerbeta.exe"],
    },
    PlaytimeGame {
        slug: "pubg-battlegrounds",
        display_name: "PUBG: BATTLEGROUNDS",
        processes: &["tslgame.exe"],
    },
    PlaytimeGame {
        slug: "genshin-impact",
        display_name: "Genshin Impact",
        processes: &["genshinimpact.exe", "yuanshen.exe"],
    },
];

/// Entrada del catálogo cuyo `slug` coincide.
pub fn by_slug(slug: &str) -> Option<&'static PlaytimeGame> {
    PLAYTIME_CATALOG.iter().find(|g| g.slug == slug)
}

/// Entrada del catálogo que declara `proc_name` (comparación en minúsculas)
/// como uno de sus ejecutables. Permite identificar un juego online por su
/// proceso vivo aunque el escaneo de instalados no lo haya visto.
pub fn game_for_process(proc_name: &str) -> Option<&'static PlaytimeGame> {
    let lower = proc_name.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    PLAYTIME_CATALOG
        .iter()
        .find(|g| g.processes.iter().any(|p| *p == lower))
}

/// Entrada del catálogo cuyo nombre legible casa con `name` tras normalizar
/// (minúsculas, solo alfanuméricos). Empareja el nombre que da el storefront
/// (Steam "Rust", Epic "Fortnite") con nuestra fila. Devuelve la primera
/// coincidencia.
pub fn game_for_store_name(name: &str) -> Option<&'static PlaytimeGame> {
    let norm = normalize(name);
    if norm.is_empty() {
        return None;
    }
    PLAYTIME_CATALOG
        .iter()
        .find(|g| normalize(g.display_name) == norm || g.slug.replace('-', "") == norm)
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_lookup_is_case_insensitive() {
        let g = game_for_process("FortniteClient-Win64-Shipping.exe").expect("fortnite");
        assert_eq!(g.slug, "fortnite");
        assert!(game_for_process("notagame.exe").is_none());
        assert!(game_for_process("").is_none());
    }

    #[test]
    fn store_name_matches_steam_and_epic_names() {
        assert_eq!(game_for_store_name("Rust").map(|g| g.slug), Some("rust"));
        assert_eq!(
            game_for_store_name("Fortnite").map(|g| g.slug),
            Some("fortnite")
        );
        assert_eq!(
            game_for_store_name("Counter-Strike 2").map(|g| g.slug),
            Some("counter-strike-2")
        );
        assert!(game_for_store_name("Some Unlisted Indie").is_none());
    }

    #[test]
    fn slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for g in PLAYTIME_CATALOG {
            assert!(seen.insert(g.slug), "duplicate slug {}", g.slug);
            assert!(!g.processes.is_empty(), "{} has no processes", g.slug);
        }
    }
}
