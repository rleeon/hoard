//! Header login for the panel behind an authenticating proxy, end to end
//! (`routes::session::proxy_login`).
//!
//! Same approach as `admin_users.rs`: the real handler against a real
//! database. What it pins down is who gets a session out of a header and,
//! mostly, who does not: a peer outside `server.trusted_proxies`, a header
//! that arrives twice, a name with no account, and a server where the feature
//! is off or configured in a way it refuses to run with.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use hoard_server::clientip::TrustedProxies;
use hoard_server::routes::admin;
use hoard_server::routes::health::ServerState;
use hoard_server::routes::session::{self, NO_PASSWORD, SESSION_DEVICE_NAME};

const ROOT_ID: &str = "11111111-2222-4333-8444-555555555555";

struct Harness {
    state: Arc<ServerState>,
    _dir: tempfile::TempDir,
}

/// `panel` is the `[panel]` table's body; `trusted` the parsed trust list.
async fn harness(panel: &str, trusted: &[&str]) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_path_buf();
    let cfg_path = data_dir.join("config.toml");
    // A `\` inside a TOML basic string is an escape sequence, so a Windows path
    // written verbatim fails to parse before the first test body runs.
    let toml_path = |p: &std::path::Path| p.display().to_string().replace('\\', "/");
    std::fs::write(
        &cfg_path,
        format!(
            r#"
[server]
host = "127.0.0.1"
port = 12421
public_url = "http://localhost:12421"

[storage]
data_dir = "{data}"
max_snapshot_size_mb = 64
upload_timeout_secs = 600

[database]
url = "sqlite://{db}"
max_connections = 1

[auth]
token_lifetime_days = 365
allow_registration = true

[panel]
{panel}

[retention]
trash_retention_days = 30
tmp_cleanup_hours = 24

[logging]
level = "warn"
format = "pretty"
"#,
            data = toml_path(&data_dir),
            db = toml_path(&data_dir.join("hoard.db")),
        ),
    )
    .unwrap();

    let config = hoard_server::config::Config::load(&cfg_path).unwrap();
    let pool = hoard_server::db::connect(&config.database.url, 1)
        .await
        .unwrap();
    hoard_server::db::run_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, is_admin) VALUES (?,'root','x',1)",
    )
    .bind(ROOT_ID)
    .execute(&pool)
    .await
    .unwrap();
    let store = hoard_server::store::build_store(&config).await.unwrap();
    let list: Vec<String> = trusted.iter().map(|s| s.to_string()).collect();

    Harness {
        state: Arc::new(ServerState {
            trusted_proxies: TrustedProxies::parse(&list).0,
            pool,
            config,
            start_time: Instant::now(),
            store,
            events: Default::default(),
        }),
        _dir: dir,
    }
}

/// The usual shape: header login on, proxy on the same box.
async fn enabled() -> Harness {
    harness(r#"trusted_header = "Remote-User""#, &["loopback"]).await
}

const PROXY: [u8; 4] = [127, 0, 0, 1];
const STRANGER: [u8; 4] = [203, 0, 113, 7];

async fn login(
    h: &Harness,
    peer: [u8; 4],
    users: &[&str],
) -> (StatusCode, HeaderMap, serde_json::Value) {
    login_in_groups(h, peer, users, None).await
}

async fn login_in_groups(
    h: &Harness,
    peer: [u8; 4],
    users: &[&str],
    groups: Option<&str>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let mut headers = HeaderMap::new();
    for u in users {
        headers.append("remote-user", HeaderValue::from_str(u).unwrap());
    }
    if let Some(g) = groups {
        headers.insert("remote-groups", HeaderValue::from_str(g).unwrap());
    }
    let peer = SocketAddr::from((peer, 40_000));
    let res = match session::proxy_login(State(h.state.clone()), ConnectInfo(peer), headers).await {
        Ok(res) => res,
        Err((status, axum::Json(body))) => return (status, HeaderMap::new(), body),
    };
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}

async fn add_user(h: &Harness, name: &str, admin: bool, hash: &str) {
    sqlx::query("INSERT INTO users (id, username, password_hash, is_admin) VALUES (?,?,?,?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(name)
        .bind(hash)
        .bind(admin as i64)
        .execute(&h.state.pool)
        .await
        .unwrap();
}

async fn user_row(h: &Harness, name: &str) -> Option<(String, i64)> {
    sqlx::query_as("SELECT password_hash, is_admin FROM users WHERE username = ?")
        .bind(name)
        .fetch_optional(&h.state.pool)
        .await
        .unwrap()
}

async fn browser_sessions(h: &Harness) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM api_tokens WHERE device_name = ?")
        .bind(SESSION_DEVICE_NAME)
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
}

/// The cookie that comes back must be a working one: the token it carries is
/// an `api_tokens` row of the existing user, named like any browser session.
#[tokio::test]
async fn the_proxy_names_an_account_and_gets_a_session() {
    let h = enabled().await;
    let (status, headers, body) = login(&h, PROXY, &["root"]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "root");
    assert_eq!(body["is_admin"], true);

    let cookie = headers
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap();
    let token = cookie
        .strip_prefix("hoard_session=")
        .and_then(|rest| rest.split(';').next())
        .unwrap();
    let owner: String = sqlx::query_scalar(
        "SELECT user_id FROM api_tokens WHERE token_hash = ? AND device_name = ?",
    )
    .bind(hoard_core::hashing::hash_token(token))
    .bind(SESSION_DEVICE_NAME)
    .fetch_one(&h.state.pool)
    .await
    .unwrap();
    assert_eq!(owner, ROOT_ID);
}

/// An identity provider that tells `Root` from `root` and lets people sign up
/// would otherwise hand whoever registers `Root` this server's admin.
#[tokio::test]
async fn another_case_is_another_person_by_default() {
    let h = enabled().await;
    let (status, _, body) = login(&h, PROXY, &["ROOT"]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "unknown_user");
    assert_eq!(browser_sessions(&h).await, 0);
}

/// Opted in, the header matches the way the password login does: the case is
/// forgiven when only one account fits, and the stored spelling comes back.
#[tokio::test]
async fn ignore_case_matches_like_the_password_login() {
    let h = harness(
        "trusted_header = \"Remote-User\"\ntrusted_header_ignore_case = true",
        &["loopback"],
    )
    .await;
    let (status, _, body) = login(&h, PROXY, &["ROOT"]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "root");
}

/// The first visit of someone new creates a member, not an admin, with no
/// password to guess; the second visit finds the same account.
#[tokio::test]
async fn create_accounts_makes_a_passwordless_member_once() {
    let h = harness(
        "trusted_header = \"Remote-User\"\ntrusted_header_create_accounts = true",
        &["loopback"],
    )
    .await;
    for _ in 0..2 {
        let (status, _, body) = login(&h, PROXY, &["ana@example.com"]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["username"], "ana@example.com");
        assert_eq!(body["is_admin"], false);
    }
    assert_eq!(
        user_row(&h, "ana@example.com").await,
        Some((NO_PASSWORD.to_string(), 0))
    );
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
    assert_eq!(users, 2);
}

#[tokio::test]
async fn create_accounts_refuses_a_name_it_could_not_store() {
    let h = harness(
        "trusted_header = \"Remote-User\"\ntrusted_header_create_accounts = true",
        &["loopback"],
    )
    .await;
    let (status, _, body) = login(&h, PROXY, &["Ana Pérez"]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "bad_username");
}

const GROUPS: &str = "trusted_header = \"Remote-User\"\n\
                      trusted_groups_header = \"Remote-Groups\"\n\
                      trusted_admin_group = \"hoard-admins\"";

/// Being in the group makes an admin, leaving it takes the rights away. Both
/// separators the common proxies use are understood.
#[tokio::test]
async fn the_admin_group_promotes_and_demotes() {
    let h = harness(GROUPS, &["loopback"]).await;
    add_user(&h, "ana", false, NO_PASSWORD).await;

    let (_, _, body) = login_in_groups(&h, PROXY, &["ana"], Some("users|hoard-admins")).await;
    assert_eq!(body["is_admin"], true);
    assert_eq!(user_row(&h, "ana").await.unwrap().1, 1);

    let (_, _, body) = login_in_groups(&h, PROXY, &["ana"], Some("users, family")).await;
    assert_eq!(body["is_admin"], false);
    assert_eq!(user_row(&h, "ana").await.unwrap().1, 0);
}

/// The users tab refuses to demote the last admin, and so does this: a server
/// with no admins can only be repaired from a shell.
#[tokio::test]
async fn the_groups_never_demote_the_last_admin() {
    let h = harness(GROUPS, &["loopback"]).await;
    let (status, _, body) = login_in_groups(&h, PROXY, &["root"], Some("users")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_admin"], true);
    assert_eq!(user_row(&h, "root").await.unwrap().1, 1);
}

/// A proxy that forgot to send groups is not asking for anyone's rights to
/// change.
#[tokio::test]
async fn no_groups_header_leaves_the_rights_alone() {
    let h = harness(GROUPS, &["loopback"]).await;
    add_user(&h, "ana", true, NO_PASSWORD).await;
    let (_, _, body) = login(&h, PROXY, &["ana"]).await;
    assert_eq!(body["is_admin"], true);
    assert_eq!(user_row(&h, "ana").await.unwrap().1, 1);
}

/// A short header session is what lets the identity provider lock someone out
/// within minutes. Below five minutes it would spend its life renewing.
#[tokio::test]
async fn header_sessions_can_be_short() {
    for (minutes, secs) in [(15, 900), (1, 300)] {
        let h = harness(
            &format!(
                "trusted_header = \"Remote-User\"\ntrusted_header_session_minutes = {minutes}"
            ),
            &["loopback"],
        )
        .await;
        let (status, headers, body) = login(&h, PROXY, &["root"]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["expires_in_secs"], secs);
        let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains(&format!("Max-Age={secs}")), "{cookie}");
    }
}

async fn password_login(
    h: &Harness,
    user: &str,
    password: &str,
) -> (StatusCode, serde_json::Value) {
    let body = session::LoginBody {
        username: user.to_string(),
        password: password.to_string(),
    };
    let peer = SocketAddr::from((PROXY, 40_000));
    match session::login(
        State(h.state.clone()),
        ConnectInfo(peer),
        HeaderMap::new(),
        axum::Json(body),
    )
    .await
    {
        Ok(res) => (res.status(), serde_json::Value::Null),
        Err((status, axum::Json(body))) => (status, body),
    }
}

/// `password_login = false` leaves the identity provider and pasted tokens as
/// the only ways in, even for a request that finds the handler some other way.
#[tokio::test]
async fn password_login_off_refuses_the_form() {
    let h = harness(
        "trusted_header = \"Remote-User\"\npassword_login = false",
        &["loopback"],
    )
    .await;
    let hash = hoard_core::hashing::hash_password("correct horse").unwrap();
    add_user(&h, "ana", false, &hash).await;
    let (status, body) = password_login(&h, "ana", "correct horse").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_enabled");
}

/// An account with no password answers the form exactly like one that doesn't
/// exist: a wrong-credentials refusal, not an error about its hash.
#[tokio::test]
async fn a_passwordless_account_cannot_use_the_form() {
    let h = enabled().await;
    add_user(&h, "ana", false, NO_PASSWORD).await;
    let (status, body) = password_login(&h, "ana", "anything at all").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
}

/// What the gate reads before anyone signs in. Header login only counts as on
/// when it would actually run.
#[tokio::test]
async fn the_gate_learns_which_ways_in_are_open() {
    let h = harness(
        "trusted_header = \"Remote-User\"\npassword_login = false\nlogout_url = \"https://auth.example.com/logout\"",
        &["loopback"],
    )
    .await;
    let cfg = serde_json::to_value(session::auth_config(State(h.state.clone())).await.0).unwrap();
    assert_eq!(cfg["password_login"], false);
    assert_eq!(cfg["header_login"], true);
    assert_eq!(cfg["logout_url"], "https://auth.example.com/logout");

    let h = harness(r#"trusted_header = "Remote-User""#, &["any"]).await;
    let cfg = serde_json::to_value(session::auth_config(State(h.state.clone())).await.0).unwrap();
    assert_eq!(cfg["header_login"], false);
    assert_eq!(cfg["password_login"], true);
}

async fn admin_creates(h: &Harness, password: &str) -> Result<String, serde_json::Value> {
    let actor = hoard_server::auth::AuthUser {
        user_id: uuid::Uuid::parse_str(ROOT_ID).unwrap(),
        username: "root".into(),
        is_admin: true,
    };
    let body = admin::NewUser {
        username: "player-two".into(),
        password: password.into(),
        is_admin: false,
        storage_quota_bytes: None,
    };
    admin::create_user(
        axum::Extension(actor),
        State(h.state.clone()),
        axum::Json(body),
    )
    .await
    .map(|(_, created)| created.0.username.clone())
    .map_err(|(_, body)| body.0)
}

/// A blank password in the users tab is an account for the identity provider,
/// but only where there is one; anywhere else it is a forgotten field.
#[tokio::test]
async fn a_blank_password_is_only_taken_with_header_login() {
    let h = enabled().await;
    assert_eq!(admin_creates(&h, "").await.unwrap(), "player-two");
    assert_eq!(
        user_row(&h, "player-two").await,
        Some((NO_PASSWORD.to_string(), 0))
    );

    let h = harness("", &["loopback"]).await;
    let err = admin_creates(&h, "").await.unwrap_err();
    assert_eq!(err["error"], "password_too_short");
}

/// The whole feature rests on this one: anyone can write the header, so it
/// counts only from a peer the operator named.
#[tokio::test]
async fn a_direct_caller_cannot_name_itself() {
    let h = enabled().await;
    let (status, _, body) = login(&h, STRANGER, &["root"]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "untrusted_proxy");
    assert_eq!(browser_sessions(&h).await, 0);
}

/// A proxy that appends its value instead of replacing the client's forwards
/// both, the client's first. Picking either would be guessing.
#[tokio::test]
async fn a_header_sent_twice_is_refused() {
    let h = enabled().await;
    let (status, _, body) = login(&h, PROXY, &["root", "someone"]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "ambiguous_identity");
    assert_eq!(browser_sessions(&h).await, 0);
}

/// Being let in by the identity provider is not the same as having an account
/// here. Nothing is created; the name comes back so the gate can say whose
/// account is missing.
#[tokio::test]
async fn a_name_with_no_account_is_refused_and_not_created() {
    let h = enabled().await;
    let (status, _, body) = login(&h, PROXY, &["newcomer"]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "unknown_user");
    assert_eq!(body["username"], "newcomer");

    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
    assert_eq!(users, 1);
    assert_eq!(browser_sessions(&h).await, 0);
}

#[tokio::test]
async fn no_header_from_the_proxy_is_no_identity() {
    let h = enabled().await;
    let (status, _, body) = login(&h, PROXY, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "no_identity");

    let (status, _, _) = login(&h, PROXY, &["   "]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Off by default: a server that never configured it must not grow a way in
/// just because a proxy in front of it happens to send `Remote-User`.
#[tokio::test]
async fn unset_it_does_nothing() {
    for panel in ["", r#"trusted_header = """#] {
        let h = harness(panel, &["loopback"]).await;
        let (status, _, body) = login(&h, PROXY, &["root"]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "panel: {panel:?}");
        assert_eq!(body["error"], "not_enabled");
    }
}

/// `trusted_proxies = ["any"]` would make the header the caller's to write.
/// Boot refuses to mount the route on it; the handler refuses too, so the
/// guarantee does not depend on how the router was put together.
#[tokio::test]
async fn a_trust_list_of_everyone_turns_it_off() {
    let h = harness(r#"trusted_header = "Remote-User""#, &["any"]).await;
    let (status, _, _) = login(&h, STRANGER, &["root"]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(browser_sessions(&h).await, 0);
}
