//! OIDC sign-in for the panel against a provider that runs inside the test:
//! discovery, keys, token and userinfo endpoints on a real socket, ID tokens
//! signed with a real Ed25519 key. The browser's part (following redirects,
//! carrying the state cookie) is played by hand, so each check can break one
//! step on purpose.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use hoard_server::clientip::TrustedProxies;
use hoard_server::routes::health::ServerState;
use hoard_server::routes::oidc::{self, CallbackParams};
use hoard_server::routes::session::NO_PASSWORD;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ROOT_ID: &str = "11111111-2222-4333-8444-555555555555";
const CLIENT_ID: &str = "hoard";
const REDIRECT: &str = "http://localhost:12421/panel/auth/oidc/callback";

// Test-only key, generated for this file.
const KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIASQf9SPgzkBckXUW89+gpr7ZbspldnO4J+AkauZMlQ1
-----END PRIVATE KEY-----";
const KEY_X: &str = "XJXIQ7AsKQlhOBTznxZbuGFfx0ktZPsRLHPe2sOyGKg";

/// What the provider hands out for one code: the claims of the ID token, what
/// userinfo adds, and the PKCE challenge the code was issued against.
#[derive(Clone, Default)]
struct Grant {
    challenge: String,
    id_claims: Value,
    userinfo: Value,
}

#[derive(Default)]
struct Provider {
    base: String,
    grants: HashMap<String, Grant>,
}

type Shared = Arc<Mutex<Provider>>;

async fn provider() -> (String, Shared) {
    let shared: Shared = Arc::default();
    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(|State(p): State<Shared>| async move {
                let base = p.lock().unwrap().base.clone();
                Json(json!({
                    "issuer": base,
                    "authorization_endpoint": format!("{base}/authorize"),
                    "token_endpoint": format!("{base}/token"),
                    "jwks_uri": format!("{base}/jwks"),
                    "userinfo_endpoint": format!("{base}/userinfo"),
                }))
            }),
        )
        .route(
            "/jwks",
            get(|| async {
                Json(json!({ "keys": [
                    // A key the server can't use, to show it is skipped, not fatal.
                    { "kty": "unknown", "kid": "junk" },
                    { "kty": "OKP", "crv": "Ed25519", "x": KEY_X, "kid": "k1", "use": "sig", "alg": "EdDSA" },
                ]}))
            }),
        )
        .route("/token", post(token))
        .route(
            "/userinfo",
            get(|State(p): State<Shared>, headers: HeaderMap| async move {
                let code = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer access-"))
                    .unwrap_or_default()
                    .to_string();
                let grant = p.lock().unwrap().grants.get(&code).cloned().unwrap_or_default();
                Json(grant.userinfo)
            }),
        )
        .with_state(shared.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    shared.lock().unwrap().base = base.clone();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, shared)
}

async fn token(
    State(p): State<Shared>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let refuse = |why: &str| (StatusCode::BAD_REQUEST, Json(json!({ "error": why })));
    // The secret arrives the way the spec prefers: HTTP Basic.
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("{CLIENT_ID}:s3cret"));
    if headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        != Some(format!("Basic {basic}").as_str())
    {
        return Err(refuse("invalid_client"));
    }
    if form.get("redirect_uri").map(String::as_str) != Some(REDIRECT) {
        return Err(refuse("redirect_uri mismatch"));
    }
    let code = form.get("code").cloned().unwrap_or_default();
    let grant = p
        .lock()
        .unwrap()
        .grants
        .remove(&code)
        .ok_or_else(|| refuse("invalid_grant"))?;
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    if challenge != grant.challenge {
        return Err(refuse("PKCE verifier does not match"));
    }
    // Userinfo answers for this code afterwards.
    p.lock().unwrap().grants.insert(code.clone(), grant.clone());

    let mut head = Header::new(Algorithm::EdDSA);
    head.kid = Some("k1".into());
    let key = EncodingKey::from_ed_pem(KEY_PEM.as_bytes()).unwrap();
    let id_token = jsonwebtoken::encode(&head, &grant.id_claims, &key).unwrap();
    Ok(Json(json!({
        "id_token": id_token,
        "access_token": format!("access-{code}"),
        "token_type": "Bearer",
    })))
}

struct Harness {
    state: Arc<ServerState>,
    provider: Shared,
    issuer: String,
    _dir: tempfile::TempDir,
}

/// `extra` goes into `[panel.oidc]` after issuer, client id and secret.
async fn harness(extra: &str) -> Harness {
    harness_with(|issuer| {
        format!(
            "[panel.oidc]\nissuer = \"{issuer}\"\nclient_id = \"{CLIENT_ID}\"\nclient_secret = \"s3cret\"\n{extra}"
        )
    })
    .await
}

async fn harness_with(oidc_block: impl Fn(&str) -> String) -> Harness {
    let (issuer, provider) = provider().await;
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_path_buf();
    let cfg_path = data_dir.join("config.toml");
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

{oidc}

[retention]
trash_retention_days = 30
tmp_cleanup_hours = 24

[logging]
level = "warn"
format = "pretty"
"#,
            data = toml_path(&data_dir),
            db = toml_path(&data_dir.join("hoard.db")),
            oidc = oidc_block(&issuer),
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

    Harness {
        state: Arc::new(ServerState {
            trusted_proxies: TrustedProxies::parse(&["loopback".to_string()]).0,
            pool,
            config,
            start_time: Instant::now(),
            store,
            events: Default::default(),
        }),
        provider,
        issuer,
        _dir: dir,
    }
}

fn location(res: &Response) -> String {
    res.headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn cookie(res: &Response, name: &str) -> Option<String> {
    res.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| c.strip_prefix(&format!("{name}=")))
        .map(|c| c.split(';').next().unwrap_or_default().to_string())
        .filter(|v| !v.is_empty())
}

/// A sign-in up to the provider's consent screen: what the browser carries
/// back, and what the server asked the provider for.
struct Started {
    state: String,
    nonce: String,
    challenge: String,
    cookie: String,
    query: HashMap<String, String>,
}

async fn start(h: &Harness) -> Started {
    let res = oidc::start(State(h.state.clone()), HeaderMap::new()).await;
    assert_eq!(res.status(), StatusCode::FOUND);
    let to = reqwest::Url::parse(&location(&res)).unwrap();
    assert_eq!(to.path(), "/authorize");
    let query: HashMap<String, String> = to.query_pairs().into_owned().collect();
    Started {
        state: query["state"].clone(),
        nonce: query["nonce"].clone(),
        challenge: query["code_challenge"].clone(),
        cookie: cookie(&res, "hoard_oidc").expect("state cookie"),
        query,
    }
}

fn id_claims(h: &Harness, nonce: &str, extra: Value) -> Value {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut claims = json!({
        "iss": h.issuer,
        "aud": CLIENT_ID,
        "sub": "subject-1",
        "iat": now,
        "exp": now + 300,
        "nonce": nonce,
    });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        claims[k] = v;
    }
    claims
}

/// The provider approves: it issues a code for these claims, and the browser
/// comes back to the callback with it.
async fn come_back(h: &Harness, s: &Started, id_claims: Value, userinfo: Value) -> Response {
    let code = format!("code-{}", s.state);
    h.provider.lock().unwrap().grants.insert(
        code.clone(),
        Grant {
            challenge: s.challenge.clone(),
            id_claims,
            userinfo,
        },
    );
    callback(h, &s.cookie, &code, &s.state).await
}

async fn callback(h: &Harness, cookie: &str, code: &str, state: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("hoard_oidc={cookie}")).unwrap(),
    );
    oidc::callback(
        State(h.state.clone()),
        headers,
        Query(CallbackParams {
            code: Some(code.into()),
            state: Some(state.into()),
            error: None,
            error_description: None,
        }),
    )
    .await
}

async fn session_owner(h: &Harness, res: &Response) -> String {
    let token = cookie(res, "hoard_session").expect("session cookie");
    sqlx::query_scalar("SELECT u.username FROM api_tokens t JOIN users u ON u.id = t.user_id WHERE t.token_hash = ?")
        .bind(hoard_core::hashing::hash_token(&token))
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
}

async fn user_row(h: &Harness, name: &str) -> Option<(String, i64)> {
    sqlx::query_as("SELECT password_hash, is_admin FROM users WHERE username = ?")
        .bind(name)
        .fetch_optional(&h.state.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_request_to_the_provider_carries_pkce_and_the_scopes() {
    let h = harness(r#"scopes = ["profile", "groups"]"#).await;
    let s = start(&h).await;
    assert_eq!(s.query["client_id"], CLIENT_ID);
    assert_eq!(s.query["redirect_uri"], REDIRECT);
    assert_eq!(s.query["response_type"], "code");
    assert_eq!(s.query["code_challenge_method"], "S256");
    assert_eq!(s.query["scope"], "openid profile groups");
    assert_eq!(s.cookie, s.state);
}

#[tokio::test]
async fn a_signed_token_for_an_existing_account_opens_a_session() {
    let h = harness("").await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({ "preferred_username": "root" }));
    let res = come_back(&h, &s, claims, json!({})).await;
    assert_eq!(location(&res), "/panel");
    assert_eq!(session_owner(&h, &res).await, "root");
    // The state cookie is spent.
    assert_eq!(cookie(&res, "hoard_oidc"), None);
}

/// The sign-in has to finish in the browser that started it, or someone could
/// make a victim finish theirs.
#[tokio::test]
async fn a_callback_without_the_browsers_cookie_is_refused() {
    let h = harness("").await;
    let s = start(&h).await;
    let res = callback(&h, "someone-elses", "code", &s.state).await;
    assert_eq!(location(&res), "/panel?login_error=oidc_expired");
}

#[tokio::test]
async fn a_callback_works_once() {
    let h = harness("").await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({ "preferred_username": "root" }));
    let first = come_back(&h, &s, claims.clone(), json!({})).await;
    assert_eq!(location(&first), "/panel");
    let again = come_back(&h, &s, claims, json!({})).await;
    assert_eq!(location(&again), "/panel?login_error=oidc_expired");
}

/// Each of these is a token the server must not believe, for a different reason.
#[tokio::test]
async fn tokens_that_dont_hold_up_are_refused() {
    let h = harness("").await;
    let bad = [
        json!({ "preferred_username": "root", "nonce": "another sign-in" }),
        json!({ "preferred_username": "root", "aud": "another-app" }),
        json!({ "preferred_username": "root", "iss": "https://impostor.example" }),
        json!({ "preferred_username": "root", "exp": 1_000 }),
    ];
    for extra in bad {
        let s = start(&h).await;
        let claims = id_claims(&h, &s.nonce, extra.clone());
        let res = come_back(&h, &s, claims, json!({})).await;
        assert_eq!(location(&res), "/panel?login_error=oidc_failed", "{extra}");
        assert_eq!(cookie(&res, "hoard_session"), None);
    }
}

#[tokio::test]
async fn an_unknown_name_says_whose_account_is_missing() {
    let h = harness("").await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({ "preferred_username": "ana" }));
    let res = come_back(&h, &s, claims, json!({})).await;
    assert_eq!(
        location(&res),
        "/panel?login_error=oidc_unknown_user&user=ana"
    );
    assert_eq!(user_row(&h, "ana").await, None);
}

/// Matched exactly unless told otherwise, like the header login.
#[tokio::test]
async fn case_is_forgiven_only_when_asked() {
    for (extra, expected) in [
        ("", "/panel?login_error=oidc_unknown_user&user=ROOT"),
        ("ignore_case = true", "/panel"),
    ] {
        let h = harness(extra).await;
        let s = start(&h).await;
        let claims = id_claims(&h, &s.nonce, json!({ "preferred_username": "ROOT" }));
        let res = come_back(&h, &s, claims, json!({})).await;
        assert_eq!(location(&res), expected, "{extra:?}");
    }
}

/// Email as the name, an account made on the spot, and admin rights from a
/// groups claim that only userinfo carries (Authelia's default).
#[tokio::test]
async fn new_people_get_an_account_and_groups_decide_admin() {
    let h = harness(
        "username_claim = \"email\"\ncreate_accounts = true\ngroups_claim = \"groups\"\nadmin_group = \"hoard-admins\"",
    )
    .await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({ "email": "ana@example.com" }));
    let userinfo = json!({ "sub": "subject-1", "groups": ["family", "hoard-admins"] });
    let res = come_back(&h, &s, claims, userinfo).await;
    assert_eq!(location(&res), "/panel");
    assert_eq!(session_owner(&h, &res).await, "ana@example.com");
    assert_eq!(
        user_row(&h, "ana@example.com").await,
        Some((NO_PASSWORD.to_string(), 1))
    );
}

/// Userinfo has to be about the person the signed token is about.
#[tokio::test]
async fn userinfo_for_someone_else_is_refused() {
    let h = harness("").await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({}));
    let userinfo = json!({ "sub": "subject-2", "preferred_username": "root" });
    let res = come_back(&h, &s, claims, userinfo).await;
    assert_eq!(location(&res), "/panel?login_error=oidc_failed");
}

#[tokio::test]
async fn no_name_claim_is_said_as_such() {
    let h = harness("").await;
    let s = start(&h).await;
    let claims = id_claims(&h, &s.nonce, json!({}));
    let res = come_back(&h, &s, claims, json!({ "sub": "subject-1" })).await;
    assert_eq!(location(&res), "/panel?login_error=oidc_no_name");
}

#[tokio::test]
async fn off_until_configured() {
    let h = harness_with(|_| String::new()).await;
    let res = oidc::start(State(h.state.clone()), HeaderMap::new()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
