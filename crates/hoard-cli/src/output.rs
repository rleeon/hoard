//! Machine-readable output (`--json`): the contract agents parse.
//!
//! With `--json`, stdout carries exactly one JSON envelope and nothing else:
//! `{"ok":true,"data":...}` when the command succeeds, `{"ok":false,"error":...}`
//! when it fails, with a non-zero exit code. Human logs keep going to stderr, so
//! an agent that reads only stdout always gets valid JSON, failure included, which
//! is why the error envelope goes to stdout too.
//!
//! The shapes live here and are never borrowed from `hoard-agent`. Serializing an
//! engine struct straight to stdout would turn its fields into public API and make
//! every refactor a breaking change for the agents parsing us. Same append-only
//! discipline as the IPC wire: add fields, never repurpose or remove one, and bump
//! the agent contract when the surface really changes.

use anyhow::Result;
use hoard_agent::api::ApiError;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set once from `main` after parsing the global `--json` flag.
static JSON: AtomicBool = AtomicBool::new(false);

pub fn set_json(on: bool) {
    JSON.store(on, Ordering::Relaxed);
}

pub fn json() -> bool {
    JSON.load(Ordering::Relaxed)
}

#[derive(Serialize)]
struct Ok_<'a, T> {
    ok: bool,
    data: &'a T,
}

#[derive(Serialize)]
struct Err_<'a> {
    ok: bool,
    error: ErrBody<'a>,
}

#[derive(Serialize)]
struct ErrBody<'a> {
    code: &'a str,
    message: String,
    /// Only on `rate_limited`, so a caller waits the window the server named
    /// instead of guessing a backoff.
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u32>,
}

/// Render a command's result: the JSON envelope under `--json`, otherwise
/// whatever `human` prints. Every command that returns data goes through here,
/// so no command can forget to honour the flag.
pub fn emit<T: Serialize>(value: &T, human: impl FnOnce(&T)) -> Result<()> {
    if json() {
        let env = Ok_ {
            ok: true,
            data: value,
        };
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        human(value);
    }
    Ok(())
}

/// An error the CLI raises itself, carrying the stable `code` callers branch on.
/// Errors coming up from the agent are classified in [`classify`] instead; this is
/// for the cases only the CLI knows about.
#[derive(Debug)]
pub struct Coded {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for Coded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Coded {}

/// Build a coded error: `return Err(output::err("not_tracked", "…"))`.
pub fn err(code: &'static str, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Coded {
        code,
        message: message.into(),
    })
}

/// Whether a prompt would actually reach a person.
///
/// False under `--json`, in a pipe, or with stdin redirected, where a prompt is
/// not a question but a hang: the caller is a script or an assistant, and it will
/// sit there until something times out. Commands ask this before printing anything
/// that waits on stdin, and fail with a coded error instead.
///
/// This doubles as the guard on destructive commands: no terminal means nobody is
/// there to say yes, so `--yes` has to be explicit.
pub fn interactive() -> bool {
    use std::io::IsTerminal;
    !json() && std::io::stdin().is_terminal()
}

/// `CliConfig::require_token`, coded. "You are not signed in" is the most
/// common thing a caller has to be told, and as a generic error it is
/// guesswork: every command that needs a session goes through here so the
/// answer is always `no_session` with exit 2.
pub fn require_token(cfg: &hoard_agent::config::CliConfig) -> Result<String> {
    cfg.require_token()
        .map(|t| t.to_string())
        .map_err(|e| err("no_session", format!("{e:#}")))
}

/// A failure, sorted into something a caller can act on.
pub struct Classified {
    /// Stable vocabulary. New codes may appear; existing ones don't change
    /// meaning.
    pub code: &'static str,
    /// Grouped so a shell script can branch without parsing JSON. Codes within a
    /// group share a reaction, which is the whole point of the grouping:
    /// 2 sign in, 3 it isn't there, 4 wait, 5 free space or upgrade,
    /// 6 the network, 1 everything else.
    pub exit: i32,
    /// Present on 429: how long the server asked us to wait.
    pub retry_after_seconds: Option<u32>,
}

/// Sort an error into a code, an exit status and (for 429) a wait.
pub fn classify(e: &anyhow::Error) -> Classified {
    let plain = |code: &'static str, exit: i32| Classified {
        code,
        exit,
        retry_after_seconds: None,
    };

    if let Some(c) = e.downcast_ref::<Coded>() {
        let exit = match c.code {
            "no_session" => 2,
            "not_tracked" => 3,
            _ => 1,
        };
        return plain(c.code, exit);
    }

    match e.downcast_ref::<ApiError>() {
        Some(ApiError::Unauthorized) => plain("unauthorized", 2),
        Some(ApiError::Forbidden) => plain("forbidden", 2),
        Some(ApiError::NotFound) => plain("not_found", 3),
        // Nothing to trim and nothing to wait for: the account is at its limit
        // until the user frees space or upgrades. Same group as an oversized
        // save, which is the other "this will fail identically next time".
        Some(ApiError::QuotaExceeded(_)) => plain("quota_exceeded", 5),
        Some(ApiError::TooLarge(_)) => plain("too_large", 5),
        Some(ApiError::Archived) => plain("archived", 5),
        Some(ApiError::RateLimited {
            retry_after_seconds,
            ..
        }) => Classified {
            code: "rate_limited",
            exit: 4,
            retry_after_seconds: Some(*retry_after_seconds),
        },
        Some(ApiError::Network(_)) => plain("network", 6),
        Some(ApiError::StorageUnreachable { .. }) => plain("storage_unreachable", 6),
        // Same class and exit code as any other 409: `--json` is a contract,
        // and a non-fast-forward is still "conflict" to whoever is scripting us.
        Some(ApiError::NonFastForward(_)) => plain("conflict", 1),
        Some(ApiError::Conflict(_)) => plain("conflict", 1),
        Some(ApiError::BadRequest(_)) => plain("bad_request", 1),
        Some(ApiError::Server { .. }) => plain("server", 1),
        None => plain("error", 1),
    }
}

/// Print the failure envelope (`--json`) or the plain `error: …` line.
/// Returns the exit code `main` should use.
pub fn emit_error(e: &anyhow::Error) -> i32 {
    let c = classify(e);
    if json() {
        let env = Err_ {
            ok: false,
            error: ErrBody {
                code: c.code,
                message: format!("{e:#}"),
                retry_after_seconds: c.retry_after_seconds,
            },
        };
        // Never let a serializer bug swallow the error itself.
        match serde_json::to_string_pretty(&env) {
            Ok(s) => println!("{s}"),
            Err(_) => eprintln!("error: {e:#}"),
        }
    } else {
        eprintln!("error: {e:#}");
    }
    c.exit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coded_errors_keep_their_code() {
        let e = err("not_tracked", "nope");
        let c = classify(&e);
        assert_eq!(c.code, "not_tracked");
        assert_eq!(c.exit, 3);
    }

    #[test]
    fn a_429_carries_the_wait() {
        let e = anyhow::Error::new(ApiError::RateLimited {
            kind: hoard_agent::api::RateLimitKind::Budget,
            retry_after_seconds: 3600,
            body: String::new(),
        });
        let c = classify(&e);
        assert_eq!(c.code, "rate_limited");
        assert_eq!(c.exit, 4);
        // Without this an agent retries a wait it can't see, which is the loop
        // the server's brake exists to stop.
        assert_eq!(c.retry_after_seconds, Some(3600));
    }

    #[test]
    fn an_unclassified_error_is_generic_not_a_panic() {
        let c = classify(&anyhow::anyhow!("something odd"));
        assert_eq!(c.code, "error");
        assert_eq!(c.exit, 1);
    }

    /// Context added with `.context(…)` must not hide the typed cause.
    #[test]
    fn context_does_not_lose_the_code() {
        use anyhow::Context;
        let e = Err::<(), _>(ApiError::NotFound)
            .context("while fetching the save")
            .unwrap_err();
        assert_eq!(classify(&e).code, "not_found");
    }
}
