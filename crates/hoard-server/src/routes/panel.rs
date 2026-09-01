//! The panel's static files, compiled into the binary.
//!
//! Baked in with `include_str!` rather than served from a directory: a
//! self-hosted install is one binary plus a config file, and "copy the assets
//! next to it" is a step that will be forgotten on exactly the upgrade that
//! changes them. It also means the panel can never be out of step with the API
//! it talks to; they ship as the same artifact.
//!
//! Everything is same-origin and dependency-free, so the pages go out under a
//! CSP with no escape hatches: no inline script, no inline style, no remote
//! anything. Keep it that way: the moment a `style="…"` attribute or a CDN
//! font shows up, the header below has to be loosened for the whole panel.

use axum::{
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};

const INDEX_HTML: &str = include_str!("../../panel/index.html");
const PANEL_CSS: &str = include_str!("../../panel/panel.css");
const PANEL_JS: &str = include_str!("../../panel/panel.js");

/// The same eight locales the desktop app ships, by the same rule: a string the
/// user reads is translated, and a locale that exists is complete. The keys are
/// flat and dotted like `crates/hoard-desktop/ui/src/lib/i18n/locales`, so the
/// two sets read the same way even though they cannot share a file.
const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../../panel/locales/en.json")),
    ("es", include_str!("../../panel/locales/es.json")),
    ("fr", include_str!("../../panel/locales/fr.json")),
    ("de", include_str!("../../panel/locales/de.json")),
    ("pt", include_str!("../../panel/locales/pt.json")),
    ("it", include_str!("../../panel/locales/it.json")),
    ("ja", include_str!("../../panel/locales/ja.json")),
    ("zh", include_str!("../../panel/locales/zh.json")),
];

/// Deliberately strict, and reachable only because the panel has no third-party
/// anything. `frame-ancestors 'none'` matters more than it looks: the session
/// cookie is `SameSite=Strict`, but clickjacking a logged-in operator's own tab
/// needs no cookie of its own.
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
                   img-src 'self' data:; connect-src 'self'; font-src 'self'; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

fn asset(body: &'static str, content_type: &'static str, with_csp: bool) -> Response {
    let mut res = (StatusCode::OK, body).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    // No revalidation window: these change with the binary, and a stale panel
    // talking to a newer API is a bug report nobody can reproduce.
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, must-revalidate"),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if with_csp {
        h.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        );
        h.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        );
    }
    res
}

/// `GET /panel`
pub async fn index() -> Response {
    asset(INDEX_HTML, "text/html; charset=utf-8", true)
}

/// `GET /panel/panel.css`
pub async fn css() -> Response {
    asset(PANEL_CSS, "text/css; charset=utf-8", false)
}

/// `GET /panel/panel.js`
pub async fn js() -> Response {
    asset(PANEL_JS, "text/javascript; charset=utf-8", false)
}

/// `GET /panel/i18n/:lang`
///
/// Unauthenticated on purpose: the login screen is translated too, and it is
/// the one screen that by definition has no session yet.
pub async fn i18n(Path(lang): Path<String>) -> Response {
    let code = lang.trim_end_matches(".json");
    match LOCALES.iter().find(|(c, _)| *c == code) {
        Some((_, body)) => asset(body, "application/json; charset=utf-8", false),
        None => (StatusCode::NOT_FOUND, "unknown locale\n").into_response(),
    }
}

/// `GET /`: the bare URL is what a self-hoster types into a browser, and until
/// now it answered 404.
pub async fn root() -> Redirect {
    Redirect::temporary("/panel")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A locale that parses but is missing keys renders as a page full of raw
    /// dotted identifiers, which is worse than English. The desktop guards this
    /// with `check-i18n-keys.mjs`; these bundles are Rust-side, so the guard
    /// has to be too.
    #[test]
    fn every_locale_is_valid_json_and_complete() {
        let en: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(LOCALES[0].1).expect("en.json parses");
        assert_eq!(LOCALES[0].0, "en");
        assert!(en.len() > 50, "en.json looks truncated: {} keys", en.len());

        for (code, body) in LOCALES.iter().skip(1) {
            let map: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(body).unwrap_or_else(|e| panic!("{code}.json: {e}"));
            let missing: Vec<_> = en.keys().filter(|k| !map.contains_key(*k)).collect();
            let orphan: Vec<_> = map.keys().filter(|k| !en.contains_key(*k)).collect();
            assert!(missing.is_empty(), "{code}.json missing: {missing:?}");
            assert!(orphan.is_empty(), "{code}.json orphan: {orphan:?}");
            for (key, value) in &map {
                assert!(
                    value.as_str().is_some_and(|s| !s.trim().is_empty()),
                    "{code}.json: {key} is empty"
                );
            }
        }
    }

    /// A missing paren in `panel.js` is a blank page, and nothing else in the
    /// build would notice: the file is `include_str!`-ed, so it ships as bytes
    /// and only the browser ever parses it. This is not a JS parser; it is the
    /// cheapest check that catches the mistake that actually happens when you
    /// close nine nested `h(...)` calls on one line.
    #[test]
    fn the_script_is_balanced() {
        for (name, src) in [("panel.js", PANEL_JS), ("panel.css", PANEL_CSS)] {
            let mut depth = [0i32; 3]; // (), [], {}
            let mut chars = src.chars().peekable();
            let mut string: Option<char> = None;
            let mut escaped = false;
            while let Some(c) = chars.next() {
                if let Some(quote) = string {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == quote {
                        string = None;
                    }
                    continue;
                }
                match c {
                    '"' | '\'' | '`' => string = Some(c),
                    '/' if chars.peek() == Some(&'/') => {
                        for c in chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    }
                    '/' if chars.peek() == Some(&'*') => {
                        let mut prev = ' ';
                        for c in chars.by_ref() {
                            if prev == '*' && c == '/' {
                                break;
                            }
                            prev = c;
                        }
                    }
                    '(' => depth[0] += 1,
                    ')' => depth[0] -= 1,
                    '[' => depth[1] += 1,
                    ']' => depth[1] -= 1,
                    '{' => depth[2] += 1,
                    '}' => depth[2] -= 1,
                    _ => {}
                }
                assert!(
                    depth.iter().all(|d| *d >= 0),
                    "{name}: a bracket closes before it opens"
                );
            }
            assert_eq!(depth, [0, 0, 0], "{name}: unbalanced (), [], {{}}");
            assert!(string.is_none(), "{name}: unterminated string literal");
        }
    }

    /// The CSP is only honest if the HTML really has nothing inline. This is
    /// the check that fails the day someone adds a quick `<style>` block.
    #[test]
    fn the_page_has_nothing_inline_for_the_csp_to_block() {
        assert!(!INDEX_HTML.contains("<script>"), "inline <script> block");
        assert!(!INDEX_HTML.contains("<style"), "inline <style> block");
        assert!(!INDEX_HTML.contains(" style=\""), "inline style attribute");
        assert!(!INDEX_HTML.contains(" onclick="), "inline event handler");
    }
}
