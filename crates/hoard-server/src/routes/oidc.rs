//! OpenID Connect sign-in for the panel: the authorization code flow with PKCE,
//! for an identity provider the browser talks to directly, no proxy in front.
//!
//! `/panel/auth/oidc/start` sends the browser to the provider with a fresh
//! state, nonce and PKCE challenge. `/panel/auth/oidc/callback` takes the code
//! back, trades it for an ID token, checks the token against the provider's
//! published keys and turns its claims into an account the way the header
//! login does ([`crate::routes::session`]). What comes out is the ordinary
//! browser session.
//!
//! What the flow remembers between the two routes lives in memory for ten
//! minutes. A restart in the middle costs one more click and nothing else.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
};
use base64::Engine;
use jsonwebtoken::{jwk::Jwk, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::config::OidcConfig;
use crate::routes::health::ServerState;
use crate::routes::session;

pub const START_PATH: &str = "/panel/auth/oidc/start";
pub const CALLBACK_PATH: &str = "/panel/auth/oidc/callback";

const STATE_COOKIE: &str = "hoard_oidc";
const FLOW_TTL: Duration = Duration::from_secs(600);
/// Sign-ins in flight at once. Each is a few hundred bytes; the cap only stops
/// someone hammering `/start` from growing the map without end.
const MAX_PENDING: usize = 1024;
/// How long the discovery document and the key set are reused. A key the cache
/// doesn't know triggers a refetch on the spot, so a provider that rotates its
/// keys is never locked out for this long.
const CACHE_TTL: Duration = Duration::from_secs(3600);

struct Pending {
    nonce: String,
    verifier: String,
    started: Instant,
}

fn pending() -> &'static Mutex<HashMap<String, Pending>> {
    static P: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();
    P.get_or_init(Default::default)
}

fn remember(state: String, flow: Pending) {
    let mut map = pending().lock().unwrap_or_else(|e| e.into_inner());
    map.retain(|_, p| p.started.elapsed() < FLOW_TTL);
    if map.len() >= MAX_PENDING {
        if let Some(oldest) = map
            .iter()
            .min_by_key(|(_, p)| p.started)
            .map(|(k, _)| k.clone())
        {
            map.remove(&oldest);
        }
    }
    map.insert(state, flow);
}

/// One use only: a callback replayed with the same state finds nothing.
fn take(state: &str) -> Option<Pending> {
    let mut map = pending().lock().unwrap_or_else(|e| e.into_inner());
    map.remove(state).filter(|p| p.started.elapsed() < FLOW_TTL)
}

#[derive(Clone, Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
    #[serde(default)]
    userinfo_endpoint: Option<String>,
    #[serde(default)]
    token_endpoint_auth_methods_supported: Vec<String>,
}

struct Cached<T> {
    value: T,
    fetched: Instant,
}

fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    })
}

async fn get_json<T: serde::de::DeserializeOwned>(
    req: reqwest::RequestBuilder,
) -> anyhow::Result<T> {
    let res = req.send().await?;
    let status = res.status();
    if !status.is_success() {
        // The provider's own words are what tells an operator which setting is
        // wrong ("invalid_client", "redirect_uri mismatch"), so they go to the log.
        let body: String = res
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect();
        bail!("{status}: {body}");
    }
    Ok(res.json().await?)
}

async fn discovery(issuer: &str) -> anyhow::Result<Discovery> {
    static CACHE: OnceLock<Mutex<HashMap<String, Cached<Discovery>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    {
        let map = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = map.get(issuer) {
            if c.fetched.elapsed() < CACHE_TTL {
                return Ok(c.value.clone());
            }
        }
    }
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let doc: Discovery = get_json(http().get(&url))
        .await
        .with_context(|| format!("fetching {url}"))?;
    // A document that names another issuer is a misconfiguration or an
    // impostor; either way its keys vouch for nobody here.
    if doc.issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
        bail!("discovery names issuer {} instead of {issuer}", doc.issuer);
    }
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(
        issuer.to_string(),
        Cached {
            value: doc.clone(),
            fetched: Instant::now(),
        },
    );
    Ok(doc)
}

/// The signing key for `kid`. Keys the library can't read (an encryption key,
/// a curve it doesn't know) are skipped one by one instead of failing the set.
async fn signing_key(jwks_uri: &str, kid: Option<&str>) -> anyhow::Result<Jwk> {
    static CACHE: OnceLock<Mutex<HashMap<String, Cached<Vec<Jwk>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let pick = |keys: &[Jwk]| -> Option<Jwk> {
        let usable: Vec<&Jwk> = keys
            .iter()
            .filter(|k| {
                !matches!(
                    k.common.public_key_use,
                    Some(jsonwebtoken::jwk::PublicKeyUse::Encryption)
                )
            })
            .collect();
        match kid {
            Some(kid) => usable
                .into_iter()
                .find(|k| k.common.key_id.as_deref() == Some(kid))
                .cloned(),
            None if usable.len() == 1 => Some(usable[0].clone()),
            None => None,
        }
    };

    {
        let map = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = map.get(jwks_uri) {
            if c.fetched.elapsed() < CACHE_TTL {
                if let Some(k) = pick(&c.value) {
                    return Ok(k);
                }
            }
        }
    }

    #[derive(Deserialize)]
    struct RawSet {
        keys: Vec<Value>,
    }
    let raw: RawSet = get_json(http().get(jwks_uri))
        .await
        .with_context(|| format!("fetching {jwks_uri}"))?;
    let keys: Vec<Jwk> = raw
        .keys
        .into_iter()
        .filter_map(|k| serde_json::from_value(k).ok())
        .collect();
    let found = pick(&keys);
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(
        jwks_uri.to_string(),
        Cached {
            value: keys,
            fetched: Instant::now(),
        },
    );
    found.ok_or_else(|| anyhow!("no signing key {kid:?} in {jwks_uri}"))
}

fn random_token() -> String {
    // Two v4 UUIDs: 244 random bits from the OS generator, in characters that
    // PKCE and URLs both accept.
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn pkce_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn redirect_uri(state: &ServerState) -> String {
    format!(
        "{}{CALLBACK_PATH}",
        state.config.server.public_url.trim_end_matches('/')
    )
}

fn clear_state_cookie() -> String {
    format!("{STATE_COOKIE}=; Path=/panel/auth/oidc; HttpOnly; SameSite=Lax; Max-Age=0")
}

/// Back to the gate with a reason it can translate. The panel reads the query
/// once and removes it from the address bar.
fn back_to_gate(code: &str, user: Option<&str>) -> Response {
    let mut target = reqwest::Url::parse("http://panel.invalid/panel").expect("static URL");
    target.query_pairs_mut().append_pair("login_error", code);
    if let Some(user) = user {
        target.query_pairs_mut().append_pair("user", user);
    }
    let location = format!("{}?{}", target.path(), target.query().unwrap_or_default());
    (
        StatusCode::FOUND,
        AppendHeaders([
            (header::LOCATION, location),
            (header::SET_COOKIE, clear_state_cookie()),
        ]),
    )
        .into_response()
}

/// `GET /panel/auth/oidc/start`: off to the provider.
pub async fn start(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    let oidc = &state.config.panel.oidc;
    let Some((issuer, client_id)) = oidc.enabled() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let doc = match discovery(issuer).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, issuer, "panel: OIDC discovery failed");
            return back_to_gate("oidc_failed", None);
        }
    };
    let Ok(mut url) = reqwest::Url::parse(&doc.authorization_endpoint) else {
        tracing::warn!(endpoint = %doc.authorization_endpoint, "panel: OIDC authorization endpoint is not a URL");
        return back_to_gate("oidc_failed", None);
    };

    let flow_state = random_token();
    let nonce = random_token();
    let verifier = random_token();
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri(&state))
        .append_pair("scope", &oidc.scopes())
        .append_pair("state", &flow_state)
        .append_pair("nonce", &nonce)
        .append_pair("code_challenge", &pkce_challenge(&verifier))
        .append_pair("code_challenge_method", "S256");
    remember(
        flow_state.clone(),
        Pending {
            nonce,
            verifier,
            started: Instant::now(),
        },
    );

    // Lax, not Strict: the callback is a top-level navigation coming back from
    // the provider's site, and a Strict cookie would stay behind.
    let mut cookie = format!(
        "{STATE_COOKIE}={flow_state}; Path=/panel/auth/oidc; HttpOnly; SameSite=Lax; Max-Age={}",
        FLOW_TTL.as_secs()
    );
    if session::is_secure(&headers) {
        cookie.push_str("; Secure");
    }
    (
        StatusCode::FOUND,
        AppendHeaders([
            (header::LOCATION, url.to_string()),
            (header::SET_COOKIE, cookie),
        ]),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// `GET /panel/auth/oidc/callback`: the provider sends the browser back here.
pub async fn callback(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Query(q): Query<CallbackParams>,
) -> Response {
    let oidc = &state.config.panel.oidc;
    let Some((issuer, client_id)) = oidc.enabled() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(error) = q.error {
        tracing::warn!(%error, description = ?q.error_description, "panel: the identity provider refused the sign-in");
        return back_to_gate("oidc_failed", None);
    }
    let (Some(code), Some(flow_state)) = (q.code, q.state) else {
        return back_to_gate("oidc_failed", None);
    };
    // The state has to come back to the browser that started the flow.
    // Otherwise someone could start a sign-in of their own, stop at the
    // callback URL, and get a victim's browser to finish it: the victim would
    // then be working in the attacker's account without knowing.
    if session::cookie_value(&headers, STATE_COOKIE).as_deref() != Some(flow_state.as_str()) {
        return back_to_gate("oidc_expired", None);
    }
    let Some(flow) = take(&flow_state) else {
        return back_to_gate("oidc_expired", None);
    };

    let claims = match finish(&state, oidc, issuer, client_id, &code, &flow).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), issuer, "panel: OIDC sign-in failed");
            return back_to_gate("oidc_failed", None);
        }
    };
    sign_in(&state, &headers, oidc, &claims).await
}

#[derive(Deserialize)]
struct Tokens {
    id_token: String,
    #[serde(default)]
    access_token: Option<String>,
}

async fn finish(
    state: &ServerState,
    oidc: &OidcConfig,
    issuer: &str,
    client_id: &str,
    code: &str,
    flow: &Pending,
) -> anyhow::Result<Map<String, Value>> {
    let doc = discovery(issuer).await?;
    let redirect = redirect_uri(state);
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &redirect),
        ("code_verifier", &flow.verifier),
    ];
    let mut req = http().post(&doc.token_endpoint);
    match oidc.client_secret() {
        // Basic is the spec's default; the form is for providers that say they
        // take nothing else.
        Some(secret) if posts_secret(&doc) => {
            form.push(("client_id", client_id));
            form.push(("client_secret", secret));
        }
        Some(secret) => req = req.basic_auth(client_id, Some(secret)),
        None => form.push(("client_id", client_id)),
    }
    let tokens: Tokens = get_json(req.form(&form))
        .await
        .context("exchanging the code at the token endpoint")?;

    let mut claims = verify_id_token(&doc, client_id, &tokens.id_token, &flow.nonce).await?;

    // Some providers keep the ID token lean and hand the rest out at userinfo
    // (Authelia does by default). Asked only when something is missing.
    let wants_more = !claims.contains_key(oidc.username_claim())
        || oidc
            .admin_group()
            .is_some_and(|(claim, _)| !claims.contains_key(claim));
    if let (true, Some(endpoint), Some(access)) =
        (wants_more, &doc.userinfo_endpoint, &tokens.access_token)
    {
        let extra: Map<String, Value> = get_json(http().get(endpoint).bearer_auth(access))
            .await
            .context("reading userinfo")?;
        // Userinfo has to describe the same person the signed token does.
        if extra.get("sub") != claims.get("sub") {
            bail!("userinfo describes another subject than the ID token");
        }
        for (k, v) in extra {
            claims.entry(k).or_insert(v);
        }
    }
    Ok(claims)
}

fn posts_secret(doc: &Discovery) -> bool {
    let methods = &doc.token_endpoint_auth_methods_supported;
    !methods.is_empty()
        && !methods.iter().any(|m| m == "client_secret_basic")
        && methods.iter().any(|m| m == "client_secret_post")
}

async fn verify_id_token(
    doc: &Discovery,
    client_id: &str,
    token: &str,
    nonce: &str,
) -> anyhow::Result<Map<String, Value>> {
    let head = jsonwebtoken::decode_header(token).context("reading the ID token header")?;
    // Only signatures checked against the provider's published keys. An HMAC
    // token would be checked against the client secret, which this server
    // keeps in a config file: that is not the provider vouching for anyone.
    if matches!(
        head.alg,
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
    ) {
        bail!(
            "ID token signed with {:?}; only public-key signatures are accepted",
            head.alg
        );
    }
    let jwk = signing_key(&doc.jwks_uri, head.kid.as_deref()).await?;
    let key = DecodingKey::from_jwk(&jwk).context("reading the provider's key")?;

    let mut v = Validation::new(head.alg);
    v.set_issuer(&[&doc.issuer]);
    v.set_audience(&[client_id]);
    v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    v.leeway = 60;
    let data = jsonwebtoken::decode::<Map<String, Value>>(token, &key, &v)
        .context("checking the ID token")?;

    if data.claims.get("nonce").and_then(Value::as_str) != Some(nonce) {
        bail!("ID token nonce does not match this sign-in");
    }
    Ok(data.claims)
}

fn in_group(value: &Value, group: &str) -> bool {
    match value {
        Value::Array(items) => items.iter().filter_map(Value::as_str).any(|g| g == group),
        Value::String(s) => session::split_groups(s).any(|g| g == group),
        _ => false,
    }
}

async fn sign_in(
    state: &ServerState,
    headers: &HeaderMap,
    oidc: &OidcConfig,
    claims: &Map<String, Value>,
) -> Response {
    let claim = oidc.username_claim();
    let Some(username) = claims
        .get(claim)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        tracing::warn!(
            claim,
            "panel: the identity provider sent no usable name claim"
        );
        return back_to_gate("oidc_no_name", None);
    };

    let found = match session::lookup_user(&state.pool, username, oidc.ignore_case).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(error = %e, "panel: OIDC account lookup failed");
            return back_to_gate("oidc_failed", None);
        }
    };
    let (user_id, real_username, mut is_admin) = match found {
        Some((id, name, admin, _)) => (id, name, admin),
        None if oidc.create_accounts => {
            match session::create_passwordless_account(&state.pool, username).await {
                Ok(account) => account,
                Err((_, body)) => {
                    let code = body.0["error"]
                        .as_str()
                        .unwrap_or("oidc_failed")
                        .to_string();
                    return back_to_gate(&code, None);
                }
            }
        }
        None => {
            tracing::warn!(%username, "panel: OIDC sign-in names no account here, refused");
            return back_to_gate("oidc_unknown_user", Some(username));
        }
    };

    if let Some((groups_claim, admin_group)) = oidc.admin_group() {
        match claims.get(groups_claim) {
            None => tracing::warn!(
                username = %real_username,
                claim = groups_claim,
                "panel: OIDC sign-in without the groups claim; admin rights left as they were"
            ),
            Some(groups) => {
                let wanted = in_group(groups, admin_group);
                if wanted != is_admin {
                    match session::sync_admin(&state.pool, &user_id, &real_username, wanted).await {
                        Ok(now) => is_admin = now,
                        Err(_) => return back_to_gate("oidc_failed", None),
                    }
                }
            }
        }
    }

    let ttl_secs = state.config.panel.session_secs();
    let token = match session::mint_session(&state.pool, &user_id, ttl_secs).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "panel: OIDC session mint failed");
            return back_to_gate("oidc_failed", None);
        }
    };
    tracing::info!(username = %real_username, is_admin, "panel: login via OIDC");

    (
        StatusCode::FOUND,
        AppendHeaders([
            (header::LOCATION, "/panel".to_string()),
            (
                header::SET_COOKIE,
                session::session_cookie(&token, ttl_secs, session::is_secure(headers)),
            ),
            (header::SET_COOKIE, clear_state_cookie()),
        ]),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7636, appendix B.
    #[test]
    fn the_challenge_is_the_spec_example() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn groups_come_as_a_list_or_a_string() {
        assert!(in_group(
            &serde_json::json!(["users", "hoard-admins"]),
            "hoard-admins"
        ));
        assert!(in_group(
            &serde_json::json!("users|hoard-admins"),
            "hoard-admins"
        ));
        assert!(!in_group(
            &serde_json::json!(["hoard-admins-old"]),
            "hoard-admins"
        ));
        assert!(!in_group(&serde_json::json!(7), "hoard-admins"));
    }

    #[test]
    fn a_replayed_state_finds_nothing() {
        remember(
            "replayed".into(),
            Pending {
                nonce: "n".into(),
                verifier: "v".into(),
                started: Instant::now(),
            },
        );
        assert!(take("replayed").is_some());
        assert!(take("replayed").is_none());
    }
}
