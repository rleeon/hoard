//! Loopback HTTP listener for the OAuth desktop handoff.
//!
//! Snap/Flatpak-confined browsers (Ubuntu ships Firefox as a snap by default)
//! cannot dispatch a custom `hoard://` URL scheme to the host, so the
//! browser → app handoff after a Supabase OAuth round-trip silently fails: the
//! web "success" page sets `window.location.href = "hoard://…"` and nothing
//! reaches the app. The standard desktop-OAuth workaround (RFC 8252) is a
//! loopback redirect — the app listens on `http://127.0.0.1:<ephemeral>` (a URL
//! confined browsers *can* open), the web callback redirects the freshly-minted
//! tokens there, and we feed them into the very same `deep-link://new-url` path
//! the rest of the app already consumes. The custom scheme stays as a fallback.

use std::time::Duration;

use anyhow::{Context, Result};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// How long the listener waits for the browser to come back before giving up
/// and freeing the port. Generous — the user may have to pick an account or
/// approve a provider consent screen first.
const LISTEN_TIMEOUT: Duration = Duration::from_secs(300);

const OK_PAGE: &str = "<!doctype html><html lang=\"es\"><head><meta charset=\"utf-8\">\
<title>Hoard</title><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<style>body{font-family:system-ui,-apple-system,sans-serif;background:#0a0a0a;color:#e5e5e5;\
display:flex;min-height:100vh;align-items:center;justify-content:center;margin:0}\
.b{text-align:center;padding:2rem}h1{color:#34d399;font-size:1.5rem;margin:0 0 .5rem}\
p{color:#a1a1aa;margin:0}</style></head><body><div class=\"b\">\
<h1>Sesion iniciada</h1><p>Ya puedes volver a la app de Hoard. Puedes cerrar esta pestana.</p>\
</div></body></html>";

const WAIT_PAGE: &str = "<!doctype html><html lang=\"es\"><head><meta charset=\"utf-8\">\
<title>Hoard</title></head><body style=\"font-family:system-ui,sans-serif;background:#0a0a0a;\
color:#a1a1aa\"><p style=\"padding:2rem\">Esperando el inicio de sesion de Hoard...</p>\
</body></html>";

/// Bind an ephemeral loopback port and spawn a one-shot HTTP server that
/// captures the OAuth callback. Takes the single-use `state` nonce minted by
/// `cloud_login_url` (the same nonce is also stored in `AppState` so the final
/// `cloud_complete_login` re-checks it). Returns the bound `port` the web side
/// must redirect to. The spawned task lives until it captures a matching
/// callback or the timeout elapses.
///
/// The `state` nonce is the CSRF guard (RFC 6749 §10.12, RFC 8252 §8.9):
/// without it, any local process — or a web page that guessed the ephemeral
/// port — could POST attacker-controlled tokens to `/callback` and silently
/// log the app into the attacker's account, sending the user's backups to it.
pub async fn start(app: AppHandle, state: String) -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("binding loopback listener")?;
    let port = listener.local_addr()?.port();
    tracing::info!(port, "loopback OAuth listener up");

    let expected_state = state;
    tauri::async_runtime::spawn(async move {
        let deadline = tokio::time::sleep(LISTEN_TIMEOUT);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = &mut deadline => {
                    tracing::info!("loopback OAuth listener timed out");
                    break;
                }
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { continue };
                    match serve(stream, &expected_state).await {
                        Ok(Some((access, refresh))) => {
                            // Reuse the existing deep-link path: synthesize the
                            // same `hoard://auth/callback?…` URL the frontend's
                            // listener already parses, then bring the window up.
                            // Carry the already-validated `state` so the final
                            // `cloud_complete_login` re-checks it against the
                            // pending nonce, exactly like the direct hoard://
                            // fallback path.
                            let url = format!(
                                "hoard://auth/callback?access_token={access}&refresh_token={refresh}&state={expected_state}"
                            );
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.unminimize();
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            crate::capture_deep_link(&app, url, true);
                            break;
                        }
                        // A stray request (favicon probe, etc.) — keep waiting.
                        Ok(None) => continue,
                        Err(e) => {
                            tracing::warn!(error = %e, "loopback OAuth request failed");
                            continue;
                        }
                    }
                }
            }
        }
    });

    Ok(port)
}

/// Read one HTTP request; if it's the OAuth callback *with a matching `state`*,
/// reply with a small "you can close this" page and return the decoded
/// `(access, refresh)` tokens. A request whose `state` doesn't match (a stray
/// probe, or a forged callback racing the real one) gets the wait page and
/// `None`, so the listener keeps waiting for the genuine redirect.
async fn serve(mut stream: TcpStream, expected_state: &str) -> Result<Option<(String, String)>> {
    // Request line + headers are tiny; one read of the first chunk is enough to
    // see `GET /callback?…` — we never need the (absent) body.
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).await.context("reading request")?;
    let head = String::from_utf8_lossy(&buf[..n]);
    let Some(line) = head.lines().next() else {
        return Ok(None);
    };
    // "GET /callback?access_token=…&refresh_token=…&state=… HTTP/1.1"
    let target = line.split_whitespace().nth(1).unwrap_or("");
    let Some((_, query)) = target.split_once('?') else {
        respond(&mut stream, WAIT_PAGE).await?;
        return Ok(None);
    };

    match parse_callback(query, expected_state) {
        Some((access, refresh)) => {
            respond(&mut stream, OK_PAGE).await?;
            Ok(Some((access, refresh)))
        }
        None => {
            respond(&mut stream, WAIT_PAGE).await?;
            Ok(None)
        }
    }
}

/// Parse the callback query string and enforce the CSRF `state` nonce. Returns
/// the tokens only when the echoed `state` matches the one this login attempt
/// generated; any mismatch, missing state, or empty access token yields `None`.
/// Split out from [`serve`] so the security-critical check is unit-testable
/// without a live socket.
fn parse_callback(query: &str, expected_state: &str) -> Option<(String, String)> {
    let mut access = None;
    let mut refresh = String::new();
    let mut state: Option<String> = None;
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        match k {
            "access_token" => access = Some(percent_decode(v)),
            "refresh_token" => refresh = percent_decode(v),
            "state" => state = Some(percent_decode(v)),
            _ => {}
        }
    }

    // Fail closed: an empty expected nonce (shouldn't happen) or a non-matching
    // echoed state means we never hand back tokens.
    if expected_state.is_empty() || state.as_deref() != Some(expected_state) {
        if state.is_some() {
            tracing::warn!("loopback OAuth: rejected callback with mismatched state nonce");
        }
        return None;
    }

    match access {
        Some(a) if !a.is_empty() => Some((a, refresh)),
        _ => None,
    }
}

async fn respond(stream: &mut TcpStream, body: &str) -> Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(resp.as_bytes())
        .await
        .context("writing response")?;
    let _ = stream.flush().await;
    Ok(())
}

/// Minimal percent-decoder for query values. The web side `encodeURIComponent`s
/// each token; JWT access tokens only contain URL-safe chars, but Supabase
/// refresh tokens can carry base64 `+`/`/` which arrive as `%2B`/`%2F`, so we
/// decode defensively. Note: `encodeURIComponent` never emits a bare `+`, so we
/// deliberately do NOT treat `+` as a space (that's form-encoding, not this).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plain_jwt_unchanged() {
        let jwt = "eyJhbGc.iOiJI-zI1_Ni..sig";
        assert_eq!(percent_decode(jwt), jwt);
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("a%2Bb%2Fc%3D"), "a+b/c=");
    }

    #[test]
    fn leaves_literal_plus_alone() {
        // encodeURIComponent never emits a bare '+', but if one slips through it
        // must stay a '+', not become a space.
        assert_eq!(percent_decode("a+b"), "a+b");
    }

    #[test]
    fn callback_accepts_matching_state() {
        let q = "access_token=tok&refresh_token=ref&state=abc123";
        assert_eq!(
            parse_callback(q, "abc123"),
            Some(("tok".to_string(), "ref".to_string()))
        );
    }

    #[test]
    fn callback_rejects_wrong_state() {
        let q = "access_token=tok&refresh_token=ref&state=evil";
        assert_eq!(parse_callback(q, "abc123"), None);
    }

    #[test]
    fn callback_rejects_missing_state() {
        // The pre-fix forged-callback shape: tokens with no state must not pass.
        let q = "access_token=tok&refresh_token=ref";
        assert_eq!(parse_callback(q, "abc123"), None);
    }

    #[test]
    fn callback_rejects_empty_expected_state() {
        let q = "access_token=tok&state=";
        assert_eq!(parse_callback(q, ""), None);
    }
}
