//! What the notification says, and in which language.
//!
//! The daemon cannot use the frontend's i18n (it is Svelte JSON loaded in the
//! webview), so the four sentences it sends live here, in the same eight languages
//! the app has. There are few of them and they do not grow: what moved is **who**
//! notifies, not how many things are notified about. If one day there are many, the
//! answer is sharing the `.json` at compile time, not two catalogues that drift.
//!
//! The language comes from the preference the user picked in Settings
//! (`prefs.language`, which until now only the frontend read) and, when they have
//! not touched it, from the environment (`LC_ALL`/`LC_MESSAGES`/`LANG`). A
//! background service that notifies in a different language from the window reads
//! like a different program.

use hoard_core::ipc::events::TooLargeKind;

use super::Kind;

/// Un aviso ya escrito, listo para el transporte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

/// Los idiomas de la app (`ui/src/lib/i18n/locales`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Pt,
    Zh,
}

impl Lang {
    /// The user's language: what they picked in the app first, and failing that,
    /// what the environment says. Anything we do not recognise falls back to
    /// English, which is the source language.
    pub fn for_user(pref: Option<&str>) -> Self {
        pref.and_then(Self::parse)
            .or_else(Self::from_env)
            .unwrap_or(Lang::En)
    }

    /// `"es"`, `"es-ES"`, `"es_ES.UTF-8"` → [`Lang::Es`].
    fn parse(tag: &str) -> Option<Self> {
        let head = tag
            .split(['-', '_', '.'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match head.as_str() {
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            "fr" => Some(Lang::Fr),
            "it" => Some(Lang::It),
            "ja" => Some(Lang::Ja),
            "pt" => Some(Lang::Pt),
            "zh" => Some(Lang::Zh),
            _ => None,
        }
    }

    fn from_env() -> Option<Self> {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|key| std::env::var(key).ok())
            .filter(|value| !value.is_empty())
            .find_map(|value| Self::parse(&value))
    }

    fn strings(self) -> Strings {
        match self {
            Lang::En => EN,
            Lang::Es => ES,
            Lang::De => DE,
            Lang::Fr => FR,
            Lang::It => IT,
            Lang::Ja => JA,
            Lang::Pt => PT,
            Lang::Zh => ZH,
        }
    }
}

/// One language's sentences. The slots (`{name}`, `{version}`...) are filled in by
/// [`fill`]; a test checks that no language drops one along the way.
#[derive(Debug, Clone, Copy)]
struct Strings {
    saved_title: &'static str,
    /// `{name}`, `{version}`, `{size}`
    saved_body: &'static str,
    failed_title: &'static str,
    failed_retrying_title: &'static str,
    /// `{name}`, `{error}`
    failed_body: &'static str,
    too_large_title: &'static str,
    /// `{name}`, `{size}`, `{limit}`
    too_large_body: &'static str,
    /// No numbers: a self-hosted 413 carries neither limit nor size. `{name}`
    too_large_body_generic: &'static str,
    /// A 413 from the user's own server: the cap is the operator's
    /// (`storage.max_snapshot_size_mb`), not a plan's, so the sentence must not
    /// mention plans, and must not print a size. Self-hosted aborts mid-stream
    /// and reports how far it got, never the save's size. `{name}`, `{limit}`
    too_large_server_title: &'static str,
    too_large_server_body: &'static str,
    /// The same, for when the server **did** say the size: the content-addressed
    /// path declares the whole version before moving a byte, so there the number is
    /// exact and not a floor. `{name}`, `{size}`, `{limit}`
    too_large_server_body_sized: &'static str,
    /// A 413 nobody at Hoard wrote: a reverse proxy or tunnel in front of the
    /// server refused the body. Nothing in Hoard's settings changes it. `{name}`
    too_large_proxy_title: &'static str,
    too_large_proxy_body: &'static str,
    stuck_title: &'static str,
    /// `{name}`, `{count}`
    stuck_body: &'static str,
    /// There is a new version and this machine **cannot** install it on its own:
    /// somebody has to approve the privilege dialog. It is the only update
    /// notification that gets sent, and that is why: on the routes that apply
    /// themselves there is nothing to ask anybody, so notifying would be noise.
    update_ready_title: &'static str,
    /// `{version}`
    update_ready_body: &'static str,
}

const EN: Strings = Strings {
    saved_title: "Backup saved",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "Backup failed",
    failed_retrying_title: "Backup failed (retrying)",
    failed_body: "{name}: {error}",
    too_large_title: "That save is over your plan",
    too_large_body: "{name}: {size} is over your plan's {limit} per-save limit.",
    too_large_body_generic: "{name} is over your plan's per-save limit.",
    too_large_server_title: "That save is over your server's limit",
    too_large_server_body: "{name}: over the {limit} per-snapshot limit on your own server.",
    too_large_server_body_sized: "{name}: {size} is over the {limit} per-snapshot limit on your own server.",
    too_large_proxy_title: "Something refused that upload",
    too_large_proxy_body: "{name} was refused as too large — not by Hoard, but by something in front of your server.",
    stuck_title: "Cloud restore is failing",
    stuck_body: "{name} — failures in a row: {count}. Hoard keeps retrying, less and less often.",
    update_ready_title: "An update is waiting",
    update_ready_body: "Hoard {version} is downloaded. Open Hoard to install it — your system will ask for permission.",
};

const ES: Strings = Strings {
    saved_title: "Copia guardada",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "La copia falló",
    failed_retrying_title: "La copia falló (reintentando)",
    failed_body: "{name}: {error}",
    too_large_title: "La partida supera tu plan",
    too_large_body: "{name}: {size} supera el límite de {limit} por partida de tu plan.",
    too_large_body_generic: "{name} supera el límite por partida de tu plan.",
    too_large_server_title: "La partida supera el límite de tu servidor",
    too_large_server_body: "{name}: supera el límite de {limit} por copia de tu propio servidor.",
    too_large_server_body_sized: "{name}: {size} supera el límite de {limit} por copia de tu propio servidor.",
    too_large_proxy_title: "Algo rechazó esa subida",
    too_large_proxy_body: "{name} se rechazó por grande, y no fue Hoard: fue algo que hay delante de tu servidor.",
    stuck_title: "La restauración desde la nube está fallando",
    stuck_body:
        "{name} — fallos seguidos: {count}. Hoard sigue reintentando, cada vez con menos frecuencia.",
    update_ready_title: "Hay una actualización esperando",
    update_ready_body: "Hoard {version} está descargada. Abre Hoard para instalarla; tu sistema te pedirá permiso.",
};

const DE: Strings = Strings {
    saved_title: "Sicherung gespeichert",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "Sicherung fehlgeschlagen",
    failed_retrying_title: "Sicherung fehlgeschlagen (neuer Versuch)",
    failed_body: "{name}: {error}",
    too_large_title: "Der Spielstand sprengt deinen Tarif",
    too_large_body: "{name}: {size} überschreitet das Limit von {limit} pro Spielstand.",
    too_large_body_generic: "{name} überschreitet das Limit deines Tarifs pro Spielstand.",
    too_large_server_title: "Der Spielstand sprengt das Limit deines Servers",
    too_large_server_body: "{name}: über dem Limit von {limit} pro Sicherung auf deinem eigenen Server.",
    too_large_server_body_sized: "{name}: {size} über dem Limit von {limit} pro Sicherung auf deinem eigenen Server.",
    too_large_proxy_title: "Etwas hat den Upload abgelehnt",
    too_large_proxy_body: "{name} wurde als zu groß abgelehnt — nicht von Hoard, sondern von etwas vor deinem Server.",
    stuck_title: "Die Wiederherstellung aus der Cloud schlägt fehl",
    stuck_body: "{name} — Fehler in Folge: {count}. Hoard versucht es weiter, immer seltener.",
    update_ready_title: "Ein Update wartet",
    update_ready_body: "Hoard {version} ist heruntergeladen. Öffne Hoard, um es zu installieren — dein System fragt nach der Berechtigung.",
};

const FR: Strings = Strings {
    saved_title: "Sauvegarde enregistrée",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "Échec de la sauvegarde",
    failed_retrying_title: "Échec de la sauvegarde (nouvelle tentative)",
    failed_body: "{name} : {error}",
    too_large_title: "Cette partie dépasse votre offre",
    too_large_body: "{name} : {size} dépasse la limite de {limit} par partie de votre offre.",
    too_large_body_generic: "{name} dépasse la limite par partie de votre offre.",
    too_large_server_title: "Cette partie dépasse la limite de votre serveur",
    too_large_server_body: "{name} : au-delà de la limite de {limit} par sauvegarde de votre propre serveur.",
    too_large_server_body_sized: "{name} : {size} au-delà de la limite de {limit} par sauvegarde de votre propre serveur.",
    too_large_proxy_title: "Quelque chose a refusé cet envoi",
    too_large_proxy_body: "{name} a été refusé comme trop volumineux — pas par Hoard, mais par quelque chose devant votre serveur.",
    stuck_title: "La restauration depuis le cloud échoue",
    stuck_body: "{name} — échecs consécutifs : {count}. Hoard réessaie, de moins en moins souvent.",
    update_ready_title: "Une mise à jour attend",
    update_ready_body: "Hoard {version} est téléchargée. Ouvre Hoard pour l'installer — ton système demandera l'autorisation.",
};

const IT: Strings = Strings {
    saved_title: "Backup salvato",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "Backup non riuscito",
    failed_retrying_title: "Backup non riuscito (nuovo tentativo)",
    failed_body: "{name}: {error}",
    too_large_title: "Questo salvataggio supera il tuo piano",
    too_large_body: "{name}: {size} supera il limite di {limit} per salvataggio del tuo piano.",
    too_large_body_generic: "{name} supera il limite per salvataggio del tuo piano.",
    too_large_server_title: "Questo salvataggio supera il limite del tuo server",
    too_large_server_body: "{name}: oltre il limite di {limit} per copia del tuo server.",
    too_large_server_body_sized: "{name}: {size} oltre il limite di {limit} per copia del tuo server.",
    too_large_proxy_title: "Qualcosa ha rifiutato il caricamento",
    too_large_proxy_body: "{name} è stato rifiutato perché troppo grande — non da Hoard, ma da qualcosa davanti al tuo server.",
    stuck_title: "Il ripristino dal cloud sta fallendo",
    stuck_body: "{name} — errori di fila: {count}. Hoard continua a riprovare, sempre più di rado.",
    update_ready_title: "C'è un aggiornamento in attesa",
    update_ready_body: "Hoard {version} è scaricato. Apri Hoard per installarlo: il sistema ti chiederà il permesso.",
};

const JA: Strings = Strings {
    saved_title: "バックアップを保存しました",
    saved_body: "{name} · v{version}（{size}）",
    failed_title: "バックアップに失敗しました",
    failed_retrying_title: "バックアップに失敗しました（再試行中）",
    failed_body: "{name}: {error}",
    too_large_title: "このセーブはプランの上限を超えています",
    too_large_body: "{name}: {size} はプランのセーブごとの上限 {limit} を超えています。",
    too_large_body_generic: "{name} はプランのセーブごとの上限を超えています。",
    too_large_server_title: "このセーブはサーバーの上限を超えています",
    too_large_server_body: "{name}: 自分のサーバーのバックアップごとの上限 {limit} を超えています。",
    too_large_server_body_sized: "{name}: {size} は自分のサーバーのバックアップごとの上限 {limit} を超えています。",
    too_large_proxy_title: "アップロードが拒否されました",
    too_large_proxy_body: "{name} はサイズ超過で拒否されました。Hoard ではなく、サーバーの手前にある何かが拒否しています。",
    stuck_title: "クラウドからの復元に失敗しています",
    stuck_body: "{name} — 連続失敗: {count} 回。Hoard は間隔を空けながら再試行を続けます。",
    update_ready_title: "アップデートが待機中です",
    update_ready_body: "Hoard {version} をダウンロード済みです。Hoard を開いてインストールしてください。システムが許可を求めます。",
};

const PT: Strings = Strings {
    saved_title: "Cópia guardada",
    saved_body: "{name} · v{version} ({size})",
    failed_title: "A cópia falhou",
    failed_retrying_title: "A cópia falhou (a tentar de novo)",
    failed_body: "{name}: {error}",
    too_large_title: "Este save excede o teu plano",
    too_large_body: "{name}: {size} excede o limite de {limit} por save do teu plano.",
    too_large_body_generic: "{name} excede o limite por save do teu plano.",
    too_large_server_title: "Este save excede o limite do teu servidor",
    too_large_server_body: "{name}: acima do limite de {limit} por cópia do teu próprio servidor.",
    too_large_server_body_sized: "{name}: {size} acima do limite de {limit} por cópia do teu próprio servidor.",
    too_large_proxy_title: "Algo recusou este envio",
    too_large_proxy_body: "{name} foi recusado por ser grande demais — não pelo Hoard, mas por algo à frente do teu servidor.",
    stuck_title: "O restauro a partir da nuvem está a falhar",
    stuck_body:
        "{name} — falhas seguidas: {count}. O Hoard continua a tentar, cada vez menos vezes.",
    update_ready_title: "Há uma atualização à espera",
    update_ready_body: "O Hoard {version} está descarregado. Abre o Hoard para o instalar — o teu sistema vai pedir permissão.",
};

const ZH: Strings = Strings {
    saved_title: "备份已保存",
    saved_body: "{name} · v{version}（{size}）",
    failed_title: "备份失败",
    failed_retrying_title: "备份失败（正在重试）",
    failed_body: "{name}：{error}",
    too_large_title: "该存档超出你的套餐",
    too_large_body: "{name}：{size} 超过套餐中每个存档 {limit} 的上限。",
    too_large_body_generic: "{name} 超过套餐中每个存档的上限。",
    too_large_server_title: "该存档超出你的服务器上限",
    too_large_server_body: "{name}：超过你自己服务器每次备份 {limit} 的上限。",
    too_large_server_body_sized: "{name}：{size} 超过你自己服务器每次备份 {limit} 的上限。",
    too_large_proxy_title: "有东西拒绝了这次上传",
    too_large_proxy_body: "{name} 因过大被拒绝——不是 Hoard，而是你服务器前面的某个环节。",
    stuck_title: "云端恢复持续失败",
    stuck_body: "{name} — 连续失败：{count} 次。Hoard 会继续重试，频率逐渐降低。",
    update_ready_title: "有一个更新在等待",
    update_ready_body: "Hoard {version} 已下载。打开 Hoard 安装它——系统会请求权限。",
};

/// Escribe el aviso.
pub fn render(kind: &Kind, name: &str, lang: Lang) -> Note {
    let s = lang.strings();
    match kind {
        Kind::BackupSaved { version, bytes } => Note {
            title: s.saved_title.to_string(),
            body: fill(
                s.saved_body,
                &[
                    ("name", name),
                    ("version", &version.to_string()),
                    ("size", &bytes_human(*bytes)),
                ],
            ),
        },
        Kind::BackupFailed { error, retrying } => Note {
            title: if *retrying {
                s.failed_retrying_title.to_string()
            } else {
                s.failed_title.to_string()
            },
            body: fill(s.failed_body, &[("name", name), ("error", error)]),
        },
        // Three different things answer 413 and only one of them is a plan, so
        // the sentence follows `kind`, the same split the window makes. It used
        // to key off `limit_bytes == 0`, back when a self-hosted 413 carried no
        // numbers at all; 1.1.3 gave it `limit_bytes`, which silently turned
        // every self-hosted rejection into the Cloud sentence with a `0 B` size
        // (`actual_bytes` is Cloud-only). A self-hoster on their own server was
        // told their save was "over your plan's 1.0 GB per-save limit" when the
        // real answer was `storage.max_snapshot_size_mb` in their config.toml.
        Kind::BackupTooLarge {
            kind,
            limit_bytes,
            actual_bytes,
        } => match kind {
            TooLargeKind::ServerLimit if *limit_bytes > 0 => Note {
                title: s.too_large_server_title.to_string(),
                // The size only goes in when the server actually knew it: the
                // content-addressed path declares the whole version up front. A
                // mid-stream abort only knows how far it got, and printing that
                // as the save's size is a number that looks precise and lies.
                body: if *actual_bytes > 0 {
                    fill(
                        s.too_large_server_body_sized,
                        &[
                            ("name", name),
                            ("size", &bytes_human(*actual_bytes)),
                            ("limit", &bytes_human(*limit_bytes)),
                        ],
                    )
                } else {
                    fill(
                        s.too_large_server_body,
                        &[("name", name), ("limit", &bytes_human(*limit_bytes))],
                    )
                },
            },
            TooLargeKind::Proxy => Note {
                title: s.too_large_proxy_title.to_string(),
                body: fill(s.too_large_proxy_body, &[("name", name)]),
            },
            _ => Note {
                title: s.too_large_title.to_string(),
                body: if *limit_bytes == 0 || *actual_bytes == 0 {
                    fill(s.too_large_body_generic, &[("name", name)])
                } else {
                    fill(
                        s.too_large_body,
                        &[
                            ("name", name),
                            ("size", &bytes_human(*actual_bytes)),
                            ("limit", &bytes_human(*limit_bytes)),
                        ],
                    )
                },
            },
        },
        Kind::RestoreStuck { failures } => Note {
            title: s.stuck_title.to_string(),
            body: fill(
                s.stuck_body,
                &[("name", name), ("count", &failures.to_string())],
            ),
        },
        // No lleva `name`: no habla de una partida, habla de la app.
        Kind::UpdateReady { version } => Note {
            title: s.update_ready_title.to_string(),
            body: fill(s.update_ready_body, &[("version", version)]),
        },
    }
}

/// Sustituye `{clave}` por su valor. Deliberadamente tonto: son plantillas
/// nuestras, no entrada del usuario.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in values {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

/// A readable size, with the same cut-off points as the UI (`formatBytes` in
/// `stores/agent.ts`) so the notification and the window do not quote different
/// numbers for the same file.
fn bytes_human(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let n_f = n as f64;
    if n_f < KB {
        format!("{n} B")
    } else if n_f < MB {
        format!("{:.1} KB", n_f / KB)
    } else if n_f < GB {
        format!("{:.0} MB", n_f / MB)
    } else {
        format!("{:.1} GB", n_f / GB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Pt,
        Lang::Zh,
    ];

    /// A misspelt slot is not a compile error: it comes out verbatim in the user's
    /// notification ("{nombre}: failed"). This test is the net.
    #[test]
    fn every_language_keeps_its_placeholders() {
        for lang in ALL {
            let s = lang.strings();
            for hole in ["{name}", "{version}", "{size}"] {
                assert!(s.saved_body.contains(hole), "{lang:?} saved_body: {hole}");
            }
            for hole in ["{name}", "{error}"] {
                assert!(s.failed_body.contains(hole), "{lang:?} failed_body: {hole}");
            }
            for hole in ["{name}", "{size}", "{limit}"] {
                assert!(
                    s.too_large_body.contains(hole),
                    "{lang:?} too_large_body: {hole}"
                );
            }
            assert!(s.too_large_body_generic.contains("{name}"));
            for hole in ["{name}", "{limit}"] {
                assert!(
                    s.too_large_server_body.contains(hole),
                    "{lang:?} too_large_server_body: {hole}"
                );
            }
            assert!(s.too_large_proxy_body.contains("{name}"));
            for hole in ["{name}", "{size}", "{limit}"] {
                assert!(
                    s.too_large_server_body_sized.contains(hole),
                    "{lang:?} too_large_server_body_sized: {hole}"
                );
            }
            for hole in ["{name}", "{count}"] {
                assert!(s.stuck_body.contains(hole), "{lang:?} stuck_body: {hole}");
            }
        }
    }

    /// No empty sentences: a notification with no title is invisible in GNOME.
    #[test]
    fn nothing_renders_empty_and_nothing_leaks_a_hole() {
        let kinds = [
            Kind::BackupSaved {
                version: 3,
                bytes: 5 * 1024 * 1024,
            },
            Kind::BackupFailed {
                error: "boom".into(),
                retrying: true,
            },
            Kind::BackupFailed {
                error: "boom".into(),
                retrying: false,
            },
            Kind::BackupTooLarge {
                kind: TooLargeKind::PlanCap,
                limit_bytes: 1024,
                actual_bytes: 4096,
            },
            Kind::BackupTooLarge {
                kind: TooLargeKind::PlanCap,
                limit_bytes: 0,
                actual_bytes: 0,
            },
            Kind::BackupTooLarge {
                kind: TooLargeKind::ServerLimit,
                limit_bytes: 1024 * 1024 * 1024,
                actual_bytes: 0,
            },
            Kind::BackupTooLarge {
                kind: TooLargeKind::ServerLimit,
                limit_bytes: 1024 * 1024 * 1024,
                actual_bytes: 3_827_416_709,
            },
            Kind::BackupTooLarge {
                kind: TooLargeKind::Proxy,
                limit_bytes: 0,
                actual_bytes: 0,
            },
            Kind::RestoreStuck { failures: 3 },
        ];
        for lang in ALL {
            for kind in &kinds {
                let note = render(kind, "Factorio", lang);
                assert!(!note.title.trim().is_empty(), "{lang:?} {kind:?}");
                assert!(!note.body.trim().is_empty(), "{lang:?} {kind:?}");
                assert!(!note.body.contains('{'), "unfilled hole: {}", note.body);
                assert!(note.body.contains("Factorio"), "{lang:?} {kind:?}");
            }
        }
    }

    /// A 413 from the user's own server must not mention a plan and must not
    /// print a size. Both happened in 1.1.3: the arm keyed off `limit_bytes`,
    /// which self-hosted started sending, so the notification read
    /// "0 B is over your plan's 1.0 GB per-save limit" to someone who had never
    /// signed in to Cloud.
    #[test]
    fn a_self_hosted_rejection_names_the_server_not_a_plan() {
        let note = render(
            &Kind::BackupTooLarge {
                kind: TooLargeKind::ServerLimit,
                limit_bytes: 1024 * 1024 * 1024,
                actual_bytes: 0,
            },
            "Factorio",
            Lang::En,
        );
        assert!(note.body.contains("1.0 GB"), "{}", note.body);
        assert!(!note.body.contains("0 B"), "{}", note.body);
        for lang in ALL {
            let note = render(
                &Kind::BackupTooLarge {
                    kind: TooLargeKind::ServerLimit,
                    limit_bytes: 1024 * 1024 * 1024,
                    actual_bytes: 0,
                },
                "Factorio",
                lang,
            );
            assert!(!note.body.contains("0 B"), "{lang:?}: {}", note.body);
        }
    }

    #[test]
    fn the_users_choice_beats_the_environment() {
        assert_eq!(Lang::for_user(Some("es-ES")), Lang::Es);
        assert_eq!(Lang::for_user(Some("ja")), Lang::Ja);
        // Un idioma que la app no tiene no puede dejar el aviso en blanco.
        assert_eq!(Lang::for_user(Some("eu")), Lang::for_user(None));
    }

    #[test]
    fn locale_tags_are_parsed_the_way_the_environment_writes_them() {
        assert_eq!(Lang::parse("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::parse("pt-BR"), Some(Lang::Pt));
        assert_eq!(Lang::parse("C"), None);
        assert_eq!(Lang::parse(""), None);
    }

    #[test]
    fn sizes_read_like_the_ui() {
        assert_eq!(bytes_human(512), "512 B");
        assert_eq!(bytes_human(2048), "2.0 KB");
        assert_eq!(bytes_human(5 * 1024 * 1024), "5 MB");
        assert_eq!(bytes_human(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn a_retrying_failure_says_so() {
        let retry = render(
            &Kind::BackupFailed {
                error: "no".into(),
                retrying: true,
            },
            "Factorio",
            Lang::Es,
        );
        let final_ = render(
            &Kind::BackupFailed {
                error: "no".into(),
                retrying: false,
            },
            "Factorio",
            Lang::Es,
        );
        assert_ne!(retry.title, final_.title);
    }
}
