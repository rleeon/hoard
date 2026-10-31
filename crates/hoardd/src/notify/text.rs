//! What the notification says, and in which language.
//!
//! The daemon cannot use the frontend's i18n (it is Svelte JSON loaded in the
//! webview), so the sentences it sends live here, in the same eight languages the
//! app has. There are few of them. If one day there are many, the answer is sharing
//! the `.json` at compile time, not two catalogues that drift.
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
    restored_title: &'static str,
    /// `{name}`, `{version}`, `{size}`
    restored_body: &'static str,
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
    quota_full_title: &'static str,
    /// `{name}`
    quota_full_body: &'static str,
    trimmed_title: &'static str,
    /// `{name}`, `{count}`
    trimmed_body: &'static str,
    unreadable_title: &'static str,
    /// `{name}`, `{count}`
    unreadable_body: &'static str,
    /// Not one file could be read, so there is no copy at all.
    unreadable_none_title: &'static str,
    /// `{name}`, `{count}`
    unreadable_none_body: &'static str,
    blocked_title: &'static str,
    /// `{name}`, `{count}`
    blocked_body: &'static str,
    wrong_folder_title: &'static str,
    /// `{name}`
    wrong_folder_body: &'static str,
    conflicts_title: &'static str,
    /// `{name}`, `{count}`
    conflicts_body: &'static str,
    game_started_title: &'static str,
    game_stopped_title: &'static str,
    /// What a failed backup says in place of the engine's own text.
    error_unreachable: &'static str,
    error_timeout: &'static str,
    error_signed_out: &'static str,
    error_disk_full: &'static str,
    error_permission: &'static str,
}

const EN: Strings = Strings {
    saved_title: "Backup saved",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Save downloaded from the cloud",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "Backup failed",
    failed_retrying_title: "Backup failed (retrying)",
    failed_body: "{name}: {error}",
    too_large_title: "That save is over your plan",
    too_large_body: "{name}: {size} is over your {limit} per-save limit.",
    too_large_body_generic: "{name} is over your per-save limit.",
    too_large_server_title: "That save is over your server's limit",
    too_large_server_body: "{name}: over the {limit} per-snapshot limit on your own server.",
    too_large_server_body_sized: "{name}: {size} is over the {limit} per-snapshot limit on your own server.",
    too_large_proxy_title: "Something refused that upload",
    too_large_proxy_body: "{name} was refused as too large — not by Hoard, but by something in front of your server.",
    stuck_title: "Cloud restore is failing",
    stuck_body: "{name} — failures in a row: {count}. Hoard keeps retrying, less and less often.",
    update_ready_title: "An update is waiting",
    update_ready_body: "Hoard {version} is downloaded. Open Hoard to install it — your system will ask for permission.",
    quota_full_title: "Your storage is full",
    quota_full_body: "{name} wasn't uploaded. Free some space to keep backing up.",
    trimmed_title: "Partial backup",
    trimmed_body: "{name}: {count} older files didn't fit in your plan and were left out.",
    unreadable_title: "Some files couldn't be read",
    unreadable_body: "{name}: {count} files were left out of this backup.",
    unreadable_none_title: "Nothing was backed up",
    unreadable_none_body: "{name}: Hoard couldn't read any of its {count} files.",
    blocked_title: "A backup needs your attention",
    blocked_body: "{name}: Hoard stopped retrying after {count} conflicts. Open Hoard to fix it.",
    wrong_folder_title: "Nothing to back up",
    wrong_folder_body: "{name}: the save folder is empty and may be the wrong one.",
    conflicts_title: "Conflicting files kept aside",
    conflicts_body: "{name}: {count} local files were moved aside before applying the cloud version.",
    game_started_title: "Game started",
    game_stopped_title: "Game closed",
    error_unreachable: "can't reach the server",
    error_timeout: "the server took too long to answer",
    error_signed_out: "signed out, open Hoard to sign in again",
    error_disk_full: "the disk is full",
    error_permission: "a file could not be read (permission denied)",
};

const ES: Strings = Strings {
    saved_title: "Copia guardada",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Partida descargada de la nube",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "La copia falló",
    failed_retrying_title: "La copia falló (reintentando)",
    failed_body: "{name}: {error}",
    too_large_title: "La partida supera tu plan",
    too_large_body: "{name}: {size} supera tu límite de {limit} por partida.",
    too_large_body_generic: "{name} supera tu límite por partida.",
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
    quota_full_title: "Tu almacenamiento está lleno",
    quota_full_body: "{name} no se ha subido. Libera espacio para seguir haciendo copias.",
    trimmed_title: "Copia parcial",
    trimmed_body: "{name}: {count} archivos antiguos no caben en tu plan y se han quedado fuera.",
    unreadable_title: "Algunos archivos no se pudieron leer",
    unreadable_body: "{name}: {count} archivos se han quedado fuera de esta copia.",
    unreadable_none_title: "No se ha copiado nada",
    unreadable_none_body: "{name}: Hoard no ha podido leer ninguno de sus {count} archivos.",
    blocked_title: "Una copia necesita tu atención",
    blocked_body: "{name}: Hoard dejó de reintentar tras {count} conflictos. Abre Hoard para resolverlo.",
    wrong_folder_title: "Nada que copiar",
    wrong_folder_body: "{name}: la carpeta de partidas está vacía y puede que no sea la correcta.",
    conflicts_title: "Archivos en conflicto guardados aparte",
    conflicts_body: "{name}: {count} archivos locales se apartaron antes de aplicar la versión de la nube.",
    game_started_title: "Juego iniciado",
    game_stopped_title: "Juego cerrado",
    error_unreachable: "no se puede conectar con el servidor",
    error_timeout: "el servidor tardó demasiado en responder",
    error_signed_out: "sesión cerrada, abre Hoard para volver a entrar",
    error_disk_full: "el disco está lleno",
    error_permission: "no se pudo leer un archivo (permiso denegado)",
};

const DE: Strings = Strings {
    saved_title: "Sicherung gespeichert",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Spielstand aus der Cloud geladen",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "Sicherung fehlgeschlagen",
    failed_retrying_title: "Sicherung fehlgeschlagen (neuer Versuch)",
    failed_body: "{name}: {error}",
    too_large_title: "Der Spielstand sprengt deinen Tarif",
    too_large_body: "{name}: {size} überschreitet das Limit von {limit} pro Spielstand.",
    too_large_body_generic: "{name} überschreitet dein Limit pro Spielstand.",
    too_large_server_title: "Der Spielstand sprengt das Limit deines Servers",
    too_large_server_body: "{name}: über dem Limit von {limit} pro Sicherung auf deinem eigenen Server.",
    too_large_server_body_sized: "{name}: {size} über dem Limit von {limit} pro Sicherung auf deinem eigenen Server.",
    too_large_proxy_title: "Etwas hat den Upload abgelehnt",
    too_large_proxy_body: "{name} wurde als zu groß abgelehnt — nicht von Hoard, sondern von etwas vor deinem Server.",
    stuck_title: "Die Wiederherstellung aus der Cloud schlägt fehl",
    stuck_body: "{name} — Fehler in Folge: {count}. Hoard versucht es weiter, immer seltener.",
    update_ready_title: "Ein Update wartet",
    update_ready_body: "Hoard {version} ist heruntergeladen. Öffne Hoard, um es zu installieren — dein System fragt nach der Berechtigung.",
    quota_full_title: "Dein Speicher ist voll",
    quota_full_body: "{name} wurde nicht hochgeladen. Gib Speicher frei, um weiter zu sichern.",
    trimmed_title: "Teilweise Sicherung",
    trimmed_body: "{name}: {count} ältere Dateien passen nicht in deinen Tarif und wurden ausgelassen.",
    unreadable_title: "Einige Dateien konnten nicht gelesen werden",
    unreadable_body: "{name}: {count} Dateien fehlen in dieser Sicherung.",
    unreadable_none_title: "Nichts wurde gesichert",
    unreadable_none_body: "{name}: Hoard konnte keine der {count} Dateien lesen.",
    blocked_title: "Eine Sicherung braucht deine Aufmerksamkeit",
    blocked_body: "{name}: Hoard hat nach {count} Konflikten aufgehört, es weiter zu versuchen. Öffne Hoard, um das zu beheben.",
    wrong_folder_title: "Nichts zu sichern",
    wrong_folder_body: "{name}: Der Spielstandordner ist leer und vielleicht der falsche.",
    conflicts_title: "Konfliktdateien beiseitegelegt",
    conflicts_body: "{name}: {count} lokale Dateien wurden vor dem Anwenden der Cloud-Version beiseitegelegt.",
    game_started_title: "Spiel gestartet",
    game_stopped_title: "Spiel beendet",
    error_unreachable: "Server nicht erreichbar",
    error_timeout: "der Server hat zu lange gebraucht",
    error_signed_out: "abgemeldet, öffne Hoard, um dich wieder anzumelden",
    error_disk_full: "die Festplatte ist voll",
    error_permission: "eine Datei konnte nicht gelesen werden (Zugriff verweigert)",
};

const FR: Strings = Strings {
    saved_title: "Sauvegarde enregistrée",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Partie téléchargée depuis le cloud",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "Échec de la sauvegarde",
    failed_retrying_title: "Échec de la sauvegarde (nouvelle tentative)",
    failed_body: "{name} : {error}",
    too_large_title: "Cette partie dépasse votre offre",
    too_large_body: "{name} : {size} dépasse votre limite de {limit} par partie.",
    too_large_body_generic: "{name} dépasse votre limite par partie.",
    too_large_server_title: "Cette partie dépasse la limite de votre serveur",
    too_large_server_body: "{name} : au-delà de la limite de {limit} par sauvegarde de votre propre serveur.",
    too_large_server_body_sized: "{name} : {size} au-delà de la limite de {limit} par sauvegarde de votre propre serveur.",
    too_large_proxy_title: "Quelque chose a refusé cet envoi",
    too_large_proxy_body: "{name} a été refusé comme trop volumineux — pas par Hoard, mais par quelque chose devant votre serveur.",
    stuck_title: "La restauration depuis le cloud échoue",
    stuck_body: "{name} — échecs consécutifs : {count}. Hoard réessaie, de moins en moins souvent.",
    update_ready_title: "Une mise à jour attend",
    update_ready_body: "Hoard {version} est téléchargée. Ouvre Hoard pour l'installer — ton système demandera l'autorisation.",
    quota_full_title: "Votre stockage est plein",
    quota_full_body: "{name} n'a pas été envoyé. Libérez de l'espace pour continuer les sauvegardes.",
    trimmed_title: "Sauvegarde partielle",
    trimmed_body: "{name} : {count} fichiers anciens ne tiennent pas dans votre offre et ont été laissés de côté.",
    unreadable_title: "Certains fichiers n'ont pas pu être lus",
    unreadable_body: "{name} : {count} fichiers manquent dans cette sauvegarde.",
    unreadable_none_title: "Rien n'a été sauvegardé",
    unreadable_none_body: "{name} : Hoard n'a pu lire aucun de ses {count} fichiers.",
    blocked_title: "Une sauvegarde demande votre attention",
    blocked_body: "{name} : Hoard a cessé de réessayer après {count} conflits. Ouvrez Hoard pour le résoudre.",
    wrong_folder_title: "Rien à sauvegarder",
    wrong_folder_body: "{name} : le dossier de sauvegarde est vide et n'est peut-être pas le bon.",
    conflicts_title: "Fichiers en conflit mis de côté",
    conflicts_body: "{name} : {count} fichiers locaux ont été mis de côté avant d'appliquer la version du cloud.",
    game_started_title: "Jeu lancé",
    game_stopped_title: "Jeu fermé",
    error_unreachable: "impossible de joindre le serveur",
    error_timeout: "le serveur a mis trop de temps à répondre",
    error_signed_out: "session fermée, ouvrez Hoard pour vous reconnecter",
    error_disk_full: "le disque est plein",
    error_permission: "un fichier n'a pas pu être lu (permission refusée)",
};

const IT: Strings = Strings {
    saved_title: "Backup salvato",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Salvataggio scaricato dal cloud",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "Backup non riuscito",
    failed_retrying_title: "Backup non riuscito (nuovo tentativo)",
    failed_body: "{name}: {error}",
    too_large_title: "Questo salvataggio supera il tuo piano",
    too_large_body: "{name}: {size} supera il tuo limite di {limit} per salvataggio.",
    too_large_body_generic: "{name} supera il tuo limite per salvataggio.",
    too_large_server_title: "Questo salvataggio supera il limite del tuo server",
    too_large_server_body: "{name}: oltre il limite di {limit} per copia del tuo server.",
    too_large_server_body_sized: "{name}: {size} oltre il limite di {limit} per copia del tuo server.",
    too_large_proxy_title: "Qualcosa ha rifiutato il caricamento",
    too_large_proxy_body: "{name} è stato rifiutato perché troppo grande — non da Hoard, ma da qualcosa davanti al tuo server.",
    stuck_title: "Il ripristino dal cloud sta fallendo",
    stuck_body: "{name} — errori di fila: {count}. Hoard continua a riprovare, sempre più di rado.",
    update_ready_title: "C'è un aggiornamento in attesa",
    update_ready_body: "Hoard {version} è scaricato. Apri Hoard per installarlo: il sistema ti chiederà il permesso.",
    quota_full_title: "Il tuo spazio è pieno",
    quota_full_body: "{name} non è stato caricato. Libera spazio per continuare con i backup.",
    trimmed_title: "Backup parziale",
    trimmed_body: "{name}: {count} file più vecchi non rientrano nel tuo piano e sono stati esclusi.",
    unreadable_title: "Alcuni file non si possono leggere",
    unreadable_body: "{name}: {count} file sono rimasti fuori da questo backup.",
    unreadable_none_title: "Non è stato salvato nulla",
    unreadable_none_body: "{name}: Hoard non è riuscito a leggere nessuno dei suoi {count} file.",
    blocked_title: "Un backup richiede la tua attenzione",
    blocked_body: "{name}: Hoard ha smesso di riprovare dopo {count} conflitti. Apri Hoard per risolverlo.",
    wrong_folder_title: "Niente da salvare",
    wrong_folder_body: "{name}: la cartella dei salvataggi è vuota e forse non è quella giusta.",
    conflicts_title: "File in conflitto messi da parte",
    conflicts_body: "{name}: {count} file locali sono stati messi da parte prima di applicare la versione del cloud.",
    game_started_title: "Gioco avviato",
    game_stopped_title: "Gioco chiuso",
    error_unreachable: "impossibile raggiungere il server",
    error_timeout: "il server ha impiegato troppo a rispondere",
    error_signed_out: "sessione chiusa, apri Hoard per accedere di nuovo",
    error_disk_full: "il disco è pieno",
    error_permission: "impossibile leggere un file (permesso negato)",
};

const JA: Strings = Strings {
    saved_title: "バックアップを保存しました",
    saved_body: "{name} · v{version}（{size}）",
    restored_title: "クラウドからセーブをダウンロードしました",
    restored_body: "{name} · v{version}（{size}）",
    failed_title: "バックアップに失敗しました",
    failed_retrying_title: "バックアップに失敗しました（再試行中）",
    failed_body: "{name}: {error}",
    too_large_title: "このセーブはプランの上限を超えています",
    too_large_body: "{name}: {size} はセーブごとの上限 {limit} を超えています。",
    too_large_body_generic: "{name} はセーブごとの上限を超えています。",
    too_large_server_title: "このセーブはサーバーの上限を超えています",
    too_large_server_body: "{name}: 自分のサーバーのバックアップごとの上限 {limit} を超えています。",
    too_large_server_body_sized: "{name}: {size} は自分のサーバーのバックアップごとの上限 {limit} を超えています。",
    too_large_proxy_title: "アップロードが拒否されました",
    too_large_proxy_body: "{name} はサイズ超過で拒否されました。Hoard ではなく、サーバーの手前にある何かが拒否しています。",
    stuck_title: "クラウドからの復元に失敗しています",
    stuck_body: "{name} — 連続失敗: {count} 回。Hoard は間隔を空けながら再試行を続けます。",
    update_ready_title: "アップデートが待機中です",
    update_ready_body: "Hoard {version} をダウンロード済みです。Hoard を開いてインストールしてください。システムが許可を求めます。",
    quota_full_title: "ストレージがいっぱいです",
    quota_full_body: "{name} はアップロードされませんでした。バックアップを続けるには空き容量を確保してください。",
    trimmed_title: "一部のみバックアップしました",
    trimmed_body: "{name}: 古いファイル {count} 個はプランの上限に収まらず、除外されました。",
    unreadable_title: "読み取れないファイルがあります",
    unreadable_body: "{name}: {count} 個のファイルはこのバックアップに含まれていません。",
    unreadable_none_title: "バックアップされませんでした",
    unreadable_none_body: "{name}: {count} 個のファイルをどれも読み取れませんでした。",
    blocked_title: "バックアップの確認が必要です",
    blocked_body: "{name}: 競合が {count} 回続いたため再試行を停止しました。Hoard を開いて解決してください。",
    wrong_folder_title: "バックアップするものがありません",
    wrong_folder_body: "{name}: セーブフォルダーが空です。フォルダーが間違っている可能性があります。",
    conflicts_title: "競合したファイルを退避しました",
    conflicts_body: "{name}: クラウドのバージョンを適用する前に、ローカルのファイル {count} 個を退避しました。",
    game_started_title: "ゲームを開始しました",
    game_stopped_title: "ゲームを終了しました",
    error_unreachable: "サーバーに接続できません",
    error_timeout: "サーバーの応答に時間がかかりすぎました",
    error_signed_out: "サインアウトしました。Hoard を開いて再度サインインしてください",
    error_disk_full: "ディスクがいっぱいです",
    error_permission: "ファイルを読み取れませんでした（アクセス拒否）",
};

const PT: Strings = Strings {
    saved_title: "Cópia guardada",
    saved_body: "{name} · v{version} ({size})",
    restored_title: "Save descarregado da nuvem",
    restored_body: "{name} · v{version} ({size})",
    failed_title: "A cópia falhou",
    failed_retrying_title: "A cópia falhou (a tentar de novo)",
    failed_body: "{name}: {error}",
    too_large_title: "Este save excede o teu plano",
    too_large_body: "{name}: {size} excede o teu limite de {limit} por save.",
    too_large_body_generic: "{name} excede o teu limite por save.",
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
    quota_full_title: "O teu armazenamento está cheio",
    quota_full_body: "{name} não foi enviado. Liberta espaço para continuares a fazer cópias.",
    trimmed_title: "Cópia parcial",
    trimmed_body: "{name}: {count} ficheiros antigos não cabem no teu plano e ficaram de fora.",
    unreadable_title: "Alguns ficheiros não puderam ser lidos",
    unreadable_body: "{name}: {count} ficheiros ficaram de fora desta cópia.",
    unreadable_none_title: "Não foi copiado nada",
    unreadable_none_body: "{name}: o Hoard não conseguiu ler nenhum dos seus {count} ficheiros.",
    blocked_title: "Uma cópia precisa da tua atenção",
    blocked_body: "{name}: o Hoard deixou de tentar após {count} conflitos. Abre o Hoard para resolver.",
    wrong_folder_title: "Nada para copiar",
    wrong_folder_body: "{name}: a pasta de saves está vazia e pode não ser a correta.",
    conflicts_title: "Ficheiros em conflito postos de lado",
    conflicts_body: "{name}: {count} ficheiros locais foram postos de lado antes de aplicar a versão da nuvem.",
    game_started_title: "Jogo iniciado",
    game_stopped_title: "Jogo fechado",
    error_unreachable: "não é possível contactar o servidor",
    error_timeout: "o servidor demorou demasiado a responder",
    error_signed_out: "sessão terminada, abre o Hoard para voltares a entrar",
    error_disk_full: "o disco está cheio",
    error_permission: "não foi possível ler um ficheiro (permissão recusada)",
};

const ZH: Strings = Strings {
    saved_title: "备份已保存",
    saved_body: "{name} · v{version}（{size}）",
    restored_title: "已从云端下载存档",
    restored_body: "{name} · v{version}（{size}）",
    failed_title: "备份失败",
    failed_retrying_title: "备份失败（正在重试）",
    failed_body: "{name}：{error}",
    too_large_title: "该存档超出你的套餐",
    too_large_body: "{name}：{size} 超过每个存档 {limit} 的上限。",
    too_large_body_generic: "{name} 超过每个存档的上限。",
    too_large_server_title: "该存档超出你的服务器上限",
    too_large_server_body: "{name}：超过你自己服务器每次备份 {limit} 的上限。",
    too_large_server_body_sized: "{name}：{size} 超过你自己服务器每次备份 {limit} 的上限。",
    too_large_proxy_title: "有东西拒绝了这次上传",
    too_large_proxy_body: "{name} 因过大被拒绝——不是 Hoard，而是你服务器前面的某个环节。",
    stuck_title: "云端恢复持续失败",
    stuck_body: "{name} — 连续失败：{count} 次。Hoard 会继续重试，频率逐渐降低。",
    update_ready_title: "有一个更新在等待",
    update_ready_body: "Hoard {version} 已下载。打开 Hoard 安装它——系统会请求权限。",
    quota_full_title: "你的存储空间已满",
    quota_full_body: "{name} 未上传。请释放空间以继续备份。",
    trimmed_title: "部分备份",
    trimmed_body: "{name}：{count} 个较旧的文件超出套餐上限，未包含在内。",
    unreadable_title: "部分文件无法读取",
    unreadable_body: "{name}：{count} 个文件未包含在此次备份中。",
    unreadable_none_title: "未备份任何内容",
    unreadable_none_body: "{name}：{count} 个文件都无法读取。",
    blocked_title: "有备份需要你处理",
    blocked_body: "{name}：连续 {count} 次冲突后，Hoard 已停止重试。请打开 Hoard 解决。",
    wrong_folder_title: "没有可备份的内容",
    wrong_folder_body: "{name}：存档文件夹为空，可能选错了文件夹。",
    conflicts_title: "冲突文件已另存",
    conflicts_body: "{name}：应用云端版本前，已将 {count} 个本地文件移到一旁。",
    game_started_title: "游戏已启动",
    game_stopped_title: "游戏已关闭",
    error_unreachable: "无法连接服务器",
    error_timeout: "服务器响应时间过长",
    error_signed_out: "已退出登录，请打开 Hoard 重新登录",
    error_disk_full: "磁盘已满",
    error_permission: "无法读取文件（权限被拒绝）",
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
        Kind::RestoreDone { version, bytes } => Note {
            title: s.restored_title.to_string(),
            body: fill(
                s.restored_body,
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
            body: fill(
                s.failed_body,
                &[("name", name), ("error", &plain_error(error, &s))],
            ),
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
        Kind::QuotaFull => Note {
            title: s.quota_full_title.to_string(),
            body: fill(s.quota_full_body, &[("name", name)]),
        },
        Kind::BackupTrimmed { omitted } => Note {
            title: s.trimmed_title.to_string(),
            body: fill(
                s.trimmed_body,
                &[("name", name), ("count", &omitted.to_string())],
            ),
        },
        Kind::FilesUnreadable { count, uploaded } => {
            let (title, body) = if *uploaded {
                (s.unreadable_title, s.unreadable_body)
            } else {
                (s.unreadable_none_title, s.unreadable_none_body)
            };
            Note {
                title: title.to_string(),
                body: fill(body, &[("name", name), ("count", &count.to_string())]),
            }
        }
        Kind::NeedsAttention { conflicts } => Note {
            title: s.blocked_title.to_string(),
            body: fill(
                s.blocked_body,
                &[("name", name), ("count", &conflicts.to_string())],
            ),
        },
        Kind::WrongFolder => Note {
            title: s.wrong_folder_title.to_string(),
            body: fill(s.wrong_folder_body, &[("name", name)]),
        },
        Kind::ConflictsKept { count } => Note {
            title: s.conflicts_title.to_string(),
            body: fill(
                s.conflicts_body,
                &[("name", name), ("count", &count.to_string())],
            ),
        },
        Kind::GameStarted => Note {
            title: s.game_started_title.to_string(),
            body: name.to_string(),
        },
        Kind::GameStopped => Note {
            title: s.game_stopped_title.to_string(),
            body: name.to_string(),
        },
        // No lleva `name`: no habla de una partida, habla de la app.
        Kind::UpdateReady { version } => Note {
            title: s.update_ready_title.to_string(),
            body: fill(s.update_ready_body, &[("version", version)]),
        },
    }
}

/// The longest engine error a notification repeats as it is.
const ERROR_CHARS: usize = 90;

/// The engine's errors are written for logs: a URL, a whole context chain. A
/// notification gets the gist, and the full text stays in the service's log.
///
/// The `os error` codes are there because Windows words the same failure in the
/// language of the system, and its code for a full disk is not Linux's.
fn plain_error(error: &str, s: &Strings) -> String {
    let e = error.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| e.contains(needle));
    let gist = if has(&[
        "error sending request",
        "connection refused",
        "dns error",
        // A service that has not reached the server once since it started.
        "/v1/health probe",
    ]) {
        s.error_unreachable
    } else if has(&["timed out", "timeout"]) {
        s.error_timeout
    } else if has(&[" 401", "unauthorized"]) {
        s.error_signed_out
    } else if has(&["no space left", "(os error 28)", "(os error 112)"]) {
        s.error_disk_full
    } else if has(&["permission denied", "(os error 13)", "(os error 5)"]) {
        s.error_permission
    } else if error.chars().count() <= ERROR_CHARS {
        return error.to_string();
    } else {
        let kept: String = error.chars().take(ERROR_CHARS - 3).collect();
        return format!("{kept}...");
    };
    gist.to_string()
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
                assert!(
                    s.restored_body.contains(hole),
                    "{lang:?} restored_body: {hole}"
                );
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
                for body in [
                    s.trimmed_body,
                    s.unreadable_body,
                    s.unreadable_none_body,
                    s.blocked_body,
                    s.conflicts_body,
                ] {
                    assert!(body.contains(hole), "{lang:?} {body}: {hole}");
                }
            }
            assert!(s.quota_full_body.contains("{name}"));
            assert!(s.wrong_folder_body.contains("{name}"));
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
            Kind::RestoreDone {
                version: 4,
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
            Kind::QuotaFull,
            Kind::BackupTrimmed { omitted: 2 },
            Kind::FilesUnreadable {
                count: 2,
                uploaded: true,
            },
            Kind::FilesUnreadable {
                count: 2,
                uploaded: false,
            },
            Kind::NeedsAttention { conflicts: 5 },
            Kind::WrongFolder,
            Kind::ConflictsKept { count: 2 },
            Kind::GameStarted,
            Kind::GameStopped,
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

    /// Nobody can act on a URL and a context chain in a toast on a handheld.
    #[test]
    fn a_failure_says_the_gist_not_the_log_line() {
        let refused = "cas init: error sending request for url \
                       (http://10.0.2.2:12431/v1/saves/7a4a6dbb/cas/init): client error (Connect)";
        for lang in ALL {
            let s = lang.strings();
            let note = render(
                &Kind::BackupFailed {
                    error: refused.into(),
                    retrying: false,
                },
                "Factorio",
                lang,
            );
            assert!(!note.body.contains("http"), "{lang:?}: {}", note.body);
            assert!(note.body.contains(s.error_unreachable), "{}", note.body);
        }
        let s = Lang::En.strings();
        for (error, gist) in [
            ("the /v1/health probe hasn't succeeded", s.error_unreachable),
            ("operation timed out", s.error_timeout),
            ("server said 401", s.error_signed_out),
            (
                "write: No space left on device (os error 28)",
                s.error_disk_full,
            ),
            ("write failed (os error 112)", s.error_disk_full),
            ("open: Permission denied (os error 13)", s.error_permission),
            ("open failed (os error 5)", s.error_permission),
        ] {
            assert_eq!(plain_error(error, &s), gist, "{error}");
        }
        // Another code that only starts the same is not the same failure.
        assert_eq!(plain_error("(os error 53)", &s), "(os error 53)");
    }

    /// Cut by characters: a byte cut through a multi-byte letter would panic.
    #[test]
    fn a_long_error_is_cut_and_a_short_one_kept() {
        let s = Lang::En.strings();
        assert_eq!(plain_error("boom", &s), "boom");
        let cut = plain_error(&"é".repeat(200), &s);
        assert_eq!(cut.chars().count(), ERROR_CHARS);
        assert!(cut.ends_with("..."));
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
