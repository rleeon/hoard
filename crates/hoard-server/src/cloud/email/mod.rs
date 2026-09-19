//! Transactional email via Resend's HTTP API.
//!
//! Delivery is best-effort and *optional*: with no `cloud.email.api_key`
//! configured every caller still does its real work and the message is simply
//! skipped, so a fresh deploy works before an email provider is provisioned.
//! Nothing here is ever the only way a user learns something: each notice has
//! an in-app counterpart.
//!
//! # Where the words live
//!
//! Not in this file. Each message is a pair of files in `templates/`:
//!
//! * `<name>.txt`, the plain-text part. **The first line is the subject**,
//!   then a blank line, then the body. Editing copy means editing this file
//!   and nothing else.
//! * `<name>.html`, the HTML part: the `<tr>` rows that drop into
//!   `layout.html`, which carries the header, the footer and the chrome.
//!
//! Both are wrapped by `layout.txt` / `layout.html`. Values are interpolated
//! as `{{name}}` and are HTML-escaped on the way into the HTML part, so a
//! template can hold a user-supplied game name without further thought.
//!
//! Tables and inline styles are not nostalgia: Gmail drops `<style>` blocks
//! and most clients ignore anything cleverer.
//!
//! # Language
//!
//! Messages go out in English because `profiles` has no language column yet;
//! filling one needs a client release. The footer links every message to its
//! own page on the site, which *is* translated, so a reader who does not read
//! English has somewhere to go. See `notice_url`.

use super::incidents::{self, Kind};
use crate::config::EmailConfig;
use anyhow::Result;
use time::OffsetDateTime;

const RESEND_ENDPOINT: &str = "https://api.resend.com/emails";

/// Where the translated explanation of each notice lives. Cloud-only, so the
/// public site is a fair constant; self-hosters who turn email on get links to
/// the same public explanation, which is the useful thing to read anyway.
const SITE: &str = "https://hoard.services";

/// Attempts per message. Resend is not in anyone's critical path, but a single
/// 502 losing a "your account is full" notice is worth one cheap retry.
const ATTEMPTS: u32 = 2;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// One message's two parts plus the slug of its page on the site.
#[derive(Clone, Copy)]
pub struct Template {
    text: &'static str,
    html: &'static str,
    /// Path under `SITE` for the translated version, or `""` for none.
    notice: &'static str,
}

/// Bind a template name to its files at compile time, so a typo or a deleted
/// file is a build error rather than an email that goes out half-rendered.
macro_rules! template {
    ($name:literal, $notice:literal) => {
        Template {
            text: include_str!(concat!("templates/", $name, ".txt")),
            html: include_str!(concat!("templates/", $name, ".html")),
            notice: $notice,
        }
    };
}

const LAYOUT_TEXT: &str = include_str!("templates/layout.txt");
const LAYOUT_HTML: &str = include_str!("templates/layout.html");

const EXPORT_READY: Template = template!("export_ready", "/notices/export-ready");
const STORAGE_PURGE_STARTED: Template =
    template!("storage_purge_started", "/notices/storage-purge-started");
const STORAGE_FULL: Template = template!("storage_full", "/notices/storage-full");
const SAVE_TOO_LARGE: Template = template!("save_too_large", "/notices/save-too-large");
const ARCHIVE_EXPIRING: Template = template!("archive_expiring", "/notices/archive-expiring");
const DEVICES_FULL: Template = template!("devices_full", "/notices/devices-full");

/// True when the config has enough to actually send (key + from address).
pub fn is_configured(cfg: &EmailConfig) -> bool {
    !cfg.api_key.is_empty() && !cfg.from.is_empty()
}

/// Send the "export ready" email. Returns `Ok(false)` (a no-op, not an error)
/// when email isn't configured, so callers can fire-and-log without special
/// casing the disabled path.
pub async fn send_export_ready(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    download_url: &str,
    expires: OffsetDateTime,
) -> Result<bool> {
    let expires = expires
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    send(
        cfg,
        to,
        offers,
        EXPORT_READY,
        &[("download_url", download_url), ("expires", &expires)],
    )
    .await
}

/// "Old versions are going." Repeats daily while the purge keeps running, and
/// carries the link that ends it.
#[allow(clippy::too_many_arguments)]
pub async fn send_storage_purge_started(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    used: i64,
    limit: i64,
    deleted_versions: usize,
    deleted_games: usize,
    mute_token: uuid::Uuid,
) -> Result<bool> {
    let percent = if limit > 0 { used * 100 / limit } else { 100 };
    send(
        cfg,
        to,
        offers,
        STORAGE_PURGE_STARTED,
        &[
            ("used", &fmt_bytes(used)),
            ("limit", &fmt_bytes(limit)),
            ("percent", &percent.to_string()),
            ("deleted_versions", &deleted_versions.to_string()),
            ("deleted_games", &deleted_games.to_string()),
            ("mute_url", &format!("{SITE}/notices/mute?t={mute_token}")),
        ],
    )
    .await
}

/// "This backup did not fit." The 402, once, and it names the game rather than
/// claiming the account is full: the quota check rejects an upload that does
/// not fit, which can happen with most of the plan still free.
pub async fn send_storage_full(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    game_slug: &str,
    requested: i64,
    used: i64,
    limit: i64,
) -> Result<bool> {
    send(
        cfg,
        to,
        offers,
        STORAGE_FULL,
        &[
            ("game", &prettify_slug(game_slug)),
            ("requested", &fmt_bytes(requested)),
            ("used", &fmt_bytes(used)),
            ("limit", &fmt_bytes(limit)),
            ("free", &fmt_bytes((limit - used).max(0))),
        ],
    )
    .await
}

/// "This one game is over the per-game cap." Once per save.
#[allow(clippy::too_many_arguments)]
pub async fn send_save_too_large(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    game_slug: &str,
    size: i64,
    limit: i64,
    plan: &str,
    pro_limit: i64,
) -> Result<bool> {
    send(
        cfg,
        to,
        offers,
        SAVE_TOO_LARGE,
        &[
            ("game", &prettify_slug(game_slug)),
            ("size", &fmt_bytes(size)),
            ("limit", &fmt_bytes(limit)),
            ("plan", &prettify_slug(plan)),
            ("pro_limit", &fmt_bytes(pro_limit)),
        ],
    )
    .await
}

/// "Your archived game is about to go." Once per save, days before the cron.
#[allow(clippy::too_many_arguments)]
pub async fn send_archive_expiring(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    game_slug: &str,
    days: i64,
    archived_on: &str,
    delete_on: &str,
    versions: i64,
    size: i64,
) -> Result<bool> {
    send(
        cfg,
        to,
        offers,
        ARCHIVE_EXPIRING,
        &[
            ("game", &prettify_slug(game_slug)),
            ("days", &days.to_string()),
            ("archived_on", archived_on),
            ("delete_on", delete_on),
            ("versions", &versions.to_string()),
            ("size", &fmt_bytes(size)),
        ],
    )
    .await
}

/// "Every device slot is taken." Once, until one frees up.
#[allow(clippy::too_many_arguments)]
pub async fn send_devices_full(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    device_name: &str,
    device_os: &str,
    used: i64,
    limit: i64,
    plan: &str,
) -> Result<bool> {
    send(
        cfg,
        to,
        offers,
        DEVICES_FULL,
        &[
            ("device_name", device_name),
            ("device_os", device_os),
            ("used", &used.to_string()),
            ("limit", &limit.to_string()),
            ("plan", &prettify_slug(plan)),
        ],
    )
    .await
}

/// Bytes the way the app writes them, so an email and the storage screen agree.
fn fmt_bytes(n: i64) -> String {
    const KB: f64 = 1024.0;
    let n = n.max(0) as f64;
    let (val, unit) = if n >= KB * KB * KB {
        (n / (KB * KB * KB), "GB")
    } else if n >= KB * KB {
        (n / (KB * KB), "MB")
    } else if n >= KB {
        (n / KB, "KB")
    } else {
        return format!("{n:.0} B");
    };
    // One decimal below 10 ("1.7 GB"), none above ("512 MB"): the extra digit
    // stops mattering exactly when the number gets big.
    if val < 10.0 {
        format!("{val:.1} {unit}")
    } else {
        format!("{val:.0} {unit}")
    }
}

/// `cyberpunk-2077` in the database, "Cyberpunk 2077" in a sentence. Slugs are
/// all we store, and a raw one in a subject line reads like a bug report.
fn prettify_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render `tpl` with `vars` and hand it to Resend.
///
/// `Ok(false)` means "email is switched off", which is a normal state and not
/// a failure. An error means the message was meant to go out and did not.
/// `offers` is the reader's opt-out token when the message may carry an offer
/// for Pro, and `None` when it must not: they said no, or they already pay.
async fn send(
    cfg: &EmailConfig,
    to: &str,
    offers: Option<uuid::Uuid>,
    tpl: Template,
    vars: &[(&str, &str)],
) -> Result<bool> {
    if !is_configured(cfg) {
        return Ok(false);
    }
    let msg = render(tpl, vars, offers);
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "from": cfg.from,
        "to": [to],
        "subject": msg.subject,
        "html": msg.html,
        "text": msg.text,
    });

    let mut last: Option<anyhow::Error> = None;
    for attempt in 1..=ATTEMPTS {
        match post(&client, cfg, &payload).await {
            Ok(()) => return Ok(true),
            Err(e) => {
                // Only a transport hiccup or Resend having a bad minute is
                // worth repeating. A 4xx is our own payload and will be just
                // as wrong the second time.
                let retryable = e.retryable && attempt < ATTEMPTS;
                tracing::warn!(attempt, retryable, error = %e.inner, "email send failed");
                last = Some(e.inner);
                if !retryable {
                    break;
                }
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
    incidents::record(Kind::Other, "email send failed");
    Err(last.unwrap_or_else(|| anyhow::anyhow!("email send failed")))
}

/// A send failure plus whether trying again could plausibly help.
struct SendError {
    inner: anyhow::Error,
    retryable: bool,
}

async fn post(
    client: &reqwest::Client,
    cfg: &EmailConfig,
    payload: &serde_json::Value,
) -> std::result::Result<(), SendError> {
    let resp = client
        .post(RESEND_ENDPOINT)
        .bearer_auth(&cfg.api_key)
        .json(payload)
        .send()
        .await
        .map_err(|e| SendError {
            inner: anyhow::Error::new(e).context("resend send"),
            retryable: true,
        })?;

    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    Err(SendError {
        inner: anyhow::anyhow!("resend returned {status}: {body}"),
        retryable: status.as_u16() == 429 || status.is_server_error(),
    })
}

/// A rendered message, ready to post.
struct Rendered {
    subject: String,
    text: String,
    html: String,
}

/// Fill `tpl` in, wrap it in the layout, and split the subject off the text
/// part's first line.
fn render(tpl: Template, vars: &[(&str, &str)], offers: Option<uuid::Uuid>) -> Rendered {
    let (subject, body_text) = split_subject(tpl.text);
    let notice_url = if tpl.notice.is_empty() {
        format!("{SITE}/")
    } else {
        format!("{SITE}{}", tpl.notice)
    };

    let subject = fill(subject, vars, false);
    let body_text = fill(body_text, vars, false);
    let body_html = fill(tpl.html, vars, true);
    // The line most clients show next to the subject. Taking it from the text
    // part keeps it honest: it is the message's own first sentence, never a
    // separate string somebody forgets to update.
    let preheader = body_text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");

    let text = LAYOUT_TEXT
        .replace("{{body}}", body_text.trim_end())
        .replace("{{notice_url}}", &notice_url);
    let html = LAYOUT_HTML
        .replace("{{body}}", &body_html)
        .replace("{{subject}}", &html_escape(&subject))
        .replace("{{preheader}}", &html_escape(preheader))
        .replace("{{notice_url}}", &html_escape(&notice_url));

    // Offers last, over the assembled message, because the footer's opt-out
    // line lives in the layout and has to come and go with the pitch it
    // refers to.
    let offers_url = offers
        .map(|t| format!("{SITE}/notices/no-offers?t={t}"))
        .unwrap_or_default();
    let text = collapse_blank_lines(&offer_sections(&text, offers.is_some()))
        .replace("{{offers_url}}", &offers_url);
    let html = offer_sections(&html, offers.is_some())
        .replace("{{offers_url}}", &html_escape(&offers_url));

    Rendered {
        subject,
        text,
        html,
    }
}

/// Keep or drop every `{{#offer}}...{{/offer}}` section.
///
/// Everything that sells Pro sits inside one: the green block, the one-line
/// pitch in the export mail, and the footer line that offers the opt-out.
/// Dropping them is what "no offers" means, and the notice around them is left
/// whole. An unclosed section runs to the end of the message, so a template
/// mistake can only ever hide too much from someone who opted out, never show
/// them an offer.
fn offer_sections(s: &str, keep: bool) -> String {
    const OPEN: &str = "{{#offer}}";
    const CLOSE: &str = "{{/offer}}";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find(OPEN) {
        out.push_str(&rest[..a]);
        let inner = &rest[a + OPEN.len()..];
        let (section, after) = match inner.find(CLOSE) {
            Some(b) => (&inner[..b], &inner[b + CLOSE.len()..]),
            None => (inner, ""),
        };
        if keep {
            out.push_str(section);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A dropped section leaves the blank lines that framed it behind. In HTML
/// nobody sees them; in the text part they read as a hole.
fn collapse_blank_lines(s: &str) -> String {
    let mut out = s.to_string();
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out
}

/// First line is the subject; the rest, past the blank line, is the body.
fn split_subject(text: &str) -> (&str, &str) {
    match text.split_once('\n') {
        Some((first, rest)) => (first.trim(), rest.trim_start_matches('\n')),
        None => (text.trim(), ""),
    }
}

/// Replace every `{{key}}`. `escape` is on for the HTML part, so a value can
/// carry a game name with an ampersand in it.
fn fill(s: &str, vars: &[(&str, &str)], escape: bool) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        let v = if escape {
            html_escape(v)
        } else {
            (*v).to_string()
        };
        out = out.replace(&format!("{{{{{k}}}}}"), &v);
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every template we ship must render with no `{{...}}` left over: a
    /// leftover placeholder is a value the caller forgot to pass, and it would
    /// go out to a user verbatim.
    #[test]
    fn export_ready_renders_completely() {
        let r = render(
            EXPORT_READY,
            &[
                ("download_url", "https://r2.example/x.zip?sig=a&b=c"),
                ("expires", "2026-09-25T10:40:57Z"),
            ],
            Some(TOKEN),
        );
        assert_eq!(r.subject, "Your Hoard data export is ready");
        for part in [&r.text, &r.html, &r.subject] {
            assert!(!part.contains("{{"), "unfilled placeholder in {part}");
        }
        // The URL is escaped in HTML and raw in text.
        assert!(r.html.contains("sig=a&amp;b=c"));
        assert!(r.text.contains("sig=a&b=c"));
        // Both parts carry the footer, including the way out for a reader who
        // does not read English.
        assert!(r.text.contains("/notices/export-ready"));
        assert!(r.html.contains("/notices/export-ready"));
        assert!(r.text.contains("discord.gg"));
    }

    #[test]
    fn subject_is_the_first_line() {
        let (s, body) = split_subject("Hello\n\nA body line.\n");
        assert_eq!(s, "Hello");
        assert_eq!(body, "A body line.\n");
    }

    /// The numbers a user compares against what the app shows them. A "2 GB"
    /// plan has to read as 2 GB, not 1.9.
    #[test]
    fn bytes_read_like_the_app_writes_them() {
        assert_eq!(fmt_bytes(2 * 1024 * 1024 * 1024), "2.0 GB");
        assert_eq!(fmt_bytes(1_846_835_937), "1.7 GB");
        assert_eq!(fmt_bytes(100 * 1024 * 1024 * 1024), "100 GB");
        assert_eq!(fmt_bytes(512 * 1024 * 1024), "512 MB");
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(-1), "0 B");
    }

    /// Slugs are all the database holds, and a raw one in a subject line reads
    /// like a bug report.
    #[test]
    fn slugs_become_titles() {
        assert_eq!(prettify_slug("cyberpunk-2077"), "Cyberpunk 2077");
        assert_eq!(prettify_slug("factorio"), "Factorio");
        assert_eq!(prettify_slug("surviving_mars"), "Surviving Mars");
        assert_eq!(prettify_slug(""), "");
    }

    /// A game name with an ampersand must not break the markup around it.
    #[test]
    fn values_are_escaped_in_html_and_raw_in_text() {
        let r = render(
            SAVE_TOO_LARGE,
            &[
                ("game", "Command & Conquer"),
                ("size", "1.4 GB"),
                ("limit", "1 GB"),
                ("plan", "Free"),
                ("pro_limit", "10 GB"),
            ],
            Some(TOKEN),
        );
        assert_eq!(r.subject, "Command & Conquer is too big to back up");
        assert!(r.html.contains("Command &amp; Conquer"));
        assert!(!r.html.contains("<strong>Command & Conquer"));
        assert!(r.text.contains("Command & Conquer"));
        assert!(!r.html.contains("{{"), "unfilled placeholder in html");
        assert!(!r.text.contains("{{"), "unfilled placeholder in text");
    }

    /// Every template ships complete: a placeholder nobody fills goes to a user
    /// verbatim, and the only way to catch that is to render them all.
    const TOKEN: uuid::Uuid = uuid::Uuid::from_u128(0x6f1c2a9e_0000_4000_8000_000000000042);

    /// Every notice, with the values its caller passes. Export is in the list
    /// because it carries an offer too.
    fn cases() -> Vec<(Template, Vec<(&'static str, &'static str)>)> {
        vec![
            (
                EXPORT_READY,
                vec![
                    ("download_url", "https://r2.example/x.zip"),
                    ("expires", "2026-09-25T10:40:57Z"),
                ],
            ),
            (
                SAVE_TOO_LARGE,
                vec![
                    ("game", "Factorio"),
                    ("size", "1.4 GB"),
                    ("limit", "1 GB"),
                    ("plan", "Free"),
                    ("pro_limit", "10 GB"),
                ],
            ),
            (
                STORAGE_PURGE_STARTED,
                vec![
                    ("used", "1.7 GB"),
                    ("limit", "2 GB"),
                    ("percent", "86"),
                    ("deleted_versions", "6"),
                    ("deleted_games", "2"),
                    ("mute_url", "https://hoard.services/notices/mute?t=abc"),
                ],
            ),
            (
                STORAGE_FULL,
                vec![
                    ("game", "Factorio"),
                    ("requested", "1.4 GB"),
                    ("used", "1.8 GB"),
                    ("limit", "2 GB"),
                    ("free", "200 MB"),
                ],
            ),
            (
                ARCHIVE_EXPIRING,
                vec![
                    ("game", "Cyberpunk 2077"),
                    ("days", "3"),
                    ("archived_on", "11 September"),
                    ("delete_on", "18 September"),
                    ("versions", "12"),
                    ("size", "640 MB"),
                ],
            ),
            (
                DEVICES_FULL,
                vec![
                    ("device_name", "ubserver"),
                    ("device_os", "Linux"),
                    ("used", "3"),
                    ("limit", "3"),
                    ("plan", "Free"),
                ],
            ),
        ]
    }

    #[test]
    fn every_template_renders_with_its_callers_values() {
        for (tpl, vars) in cases() {
            let r = render(tpl, &vars, Some(TOKEN));
            assert!(!r.subject.is_empty());
            for part in [&r.text, &r.html] {
                assert!(!part.contains("{{"), "unfilled placeholder: {part}");
                assert!(part.contains("hoard.services/pricing"), "no Pro pitch");
                assert!(
                    part.contains(&format!("/notices/no-offers?t={TOKEN}")),
                    "an offer without the way to refuse it"
                );
            }
        }
    }

    /// The refusal the law requires has to actually hold: someone who turned
    /// offers off gets every notice whole and not one word about Pro, in either
    /// part, including the footer line that would offer the opt-out again.
    #[test]
    fn opted_out_readers_get_the_notice_and_no_offer() {
        for (tpl, vars) in cases() {
            let r = render(tpl, &vars, None);
            for part in [&r.text, &r.html] {
                assert!(!part.contains("{{"), "leftover marker: {part}");
                assert!(!part.contains("pricing"), "offer left in: {part}");
                assert!(!part.contains("HOARD PRO"), "offer left in: {part}");
                assert!(!part.contains("no-offers"), "opt-out link left in");
                // The service half is intact.
                assert!(part.contains("/notices/"), "lost the notice link");
            }
            assert!(!r.text.contains("\n\n\n"), "a hole where the offer was");
            // What remains is still well-formed rows.
            assert_eq!(
                r.html.matches("<tr>").count(),
                r.html.matches("</tr>").count()
            );
        }
    }

    /// An unclosed section is a template mistake, and the safe failure is
    /// hiding too much from someone who opted out, never showing them an offer.
    #[test]
    fn an_unclosed_offer_section_hides_to_the_end() {
        assert_eq!(offer_sections("a {{#offer}}b{{/offer}} c", false), "a  c");
        assert_eq!(offer_sections("a {{#offer}}b{{/offer}} c", true), "a b c");
        assert_eq!(offer_sections("a {{#offer}}b c", false), "a ");
    }

    #[test]
    fn disabled_config_is_a_no_op_not_an_error() {
        let cfg = EmailConfig::default();
        assert!(!is_configured(&cfg));
    }
}
