//! Sign-in through Google Cloud Identity-Aware Proxy, and mock users for local development.
//!
//! Clean-room implementation of `specs/identity-aware-proxy.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use std::{
    collections::HashMap,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use axum::{
    extract::Extension,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{Jwk, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_cookies::Cookies;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    audit::AuditAuthor,
    auth::safe_token_prefix,
    error::{Error, JsonResult, Result},
    oauth2::{InstanceEvent, REQUIRE_PREEXISTING_USER_FOR_OAUTH},
    users::{truncate_token, PENDING_OAUTH_LOGIN_TYPE},
    utils::HTTP_CLIENT,
    BASE_URL,
};

use crate::{
    db::DB,
    users::create_session_token,
    users_oss::{insert_account, NewAccount},
    webhook_util::WebhookShared,
};

pub const IAP_ASSERTION_HEADER: &str = "x-goog-iap-jwt-assertion";
pub const IAP_ISSUER: &str = "https://cloud.google.com/iap";
const DEFAULT_IAP_JWKS_URL: &str = "https://www.gstatic.com/iap/verify/public_key-jwk";
const KEYS_MAX_AGE: Duration = Duration::from_secs(3600);
const KEYS_MIN_REFETCH: Duration = Duration::from_secs(30);
const CLOCK_LEEWAY_S: u64 = 30;

const IAP_LOGIN_TYPE: &str = "iap";
const MOCK_LOGIN_TYPE: &str = "mock";

pub fn routes() -> Router {
    Router::new()
        .route("/sign_in_options", get(sign_in_options))
        .route("/iap", post(iap_login))
        .route("/mock_login", post(mock_login))
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn iap_audience() -> Option<String> {
    env("IAP_AUDIENCE")
}

/// The mock users when they are available (spec §2): configured, IAP off, and a local base URL.
fn available_mock_users() -> Vec<String> {
    let configured: Vec<String> = env("GANTRY_MOCK_USERS")
        .map(|list| {
            list.split(',')
                .map(|e| e.trim().to_lowercase())
                .filter(|e| !e.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if configured.is_empty() || iap_audience().is_some() || !is_local_url(&BASE_URL.load()) {
        return vec![];
    }
    configured
}

fn is_local_url(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| {
        matches!(
            u.host(),
            Some(url::Host::Domain("localhost"))
                | Some(url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST))
                | Some(url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST))
        )
    })
}

#[derive(Serialize)]
struct SignInOptions {
    iap: bool,
    mock_users: Vec<String>,
}

async fn sign_in_options() -> JsonResult<SignInOptions> {
    Ok(Json(SignInOptions {
        iap: iap_audience().is_some(),
        mock_users: available_mock_users(),
    }))
}

async fn iap_login(
    headers: HeaderMap,
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
) -> Result<String> {
    let audience = iap_audience().ok_or_else(|| {
        Error::NotFound("Identity-Aware Proxy sign-in is not enabled".to_string())
    })?;
    let assertion = headers
        .get(IAP_ASSERTION_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            Error::NotAuthorized(format!(
                "no {IAP_ASSERTION_HEADER} header: the request did not come through Identity-Aware Proxy"
            ))
        })?;
    let email = IAP_KEYS.verify(assertion, &audience).await?;
    sign_in(&db, &webhook, IAP_LOGIN_TYPE, &email, cookies).await
}

#[derive(Deserialize)]
struct MockLogin {
    email: String,
}

async fn mock_login(
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Json(MockLogin { email }): Json<MockLogin>,
) -> Result<String> {
    let users = available_mock_users();
    if users.is_empty() {
        return Err(Error::NotFound("mock users are not enabled".to_string()));
    }
    let email = email.trim().to_lowercase();
    if !users.contains(&email) {
        return Err(Error::BadRequest(format!("{email} is not a mock user")));
    }
    sign_in(&db, &webhook, MOCK_LOGIN_TYPE, &email, cookies).await
}

lazy_static::lazy_static! {
    static ref IAP_KEYS: KeySet = KeySet::new(
        env("IAP_JWKS_URL").unwrap_or_else(|| DEFAULT_IAP_JWKS_URL.to_string()),
    );
}

/// IAP's signing keys, fetched from `url` and cached (spec §1).
pub struct KeySet {
    url: String,
    cache: RwLock<Option<(JwkSet, Instant)>>,
}

#[derive(Deserialize)]
struct IapClaims {
    email: String,
    iat: u64,
}

impl KeySet {
    pub fn new(url: String) -> Self {
        Self { url, cache: RwLock::new(None) }
    }

    /// Verifies an IAP assertion for `audience` and returns the user's email.
    pub async fn verify(&self, assertion: &str, audience: &str) -> Result<String> {
        let invalid = |why: String| Error::NotAuthorized(format!("invalid IAP assertion: {why}"));
        let header = decode_header(assertion).map_err(|e| invalid(e.to_string()))?;
        if header.alg != Algorithm::ES256 {
            return Err(invalid(format!("unexpected algorithm {:?}", header.alg)));
        }
        let kid = header.kid.ok_or_else(|| invalid("no key id".to_string()))?;
        let jwk = self
            .key(&kid)
            .await?
            .ok_or_else(|| invalid(format!("unknown key {kid}")))?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|e| invalid(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&[audience]);
        validation.set_issuer(&[IAP_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iat", "aud", "iss"]);
        validation.leeway = CLOCK_LEEWAY_S;
        let claims = decode::<IapClaims>(assertion, &key, &validation)
            .map_err(|e| invalid(e.to_string()))?
            .claims;
        // `jsonwebtoken` checks `exp` but not that `iat` is in the past.
        if claims.iat > jsonwebtoken::get_current_timestamp() + CLOCK_LEEWAY_S {
            return Err(invalid("issued in the future".to_string()));
        }
        email_from_claim(&claims.email).ok_or_else(|| invalid("no email".to_string()))
    }

    async fn key(&self, kid: &str) -> Result<Option<Jwk>> {
        if let Some((keys, fetched)) = &*self.cache.read().await {
            if let Some(jwk) = keys.find(kid) {
                if fetched.elapsed() < KEYS_MAX_AGE {
                    return Ok(Some(jwk.clone()));
                }
            } else if fetched.elapsed() < KEYS_MIN_REFETCH {
                return Ok(None);
            }
        }
        let mut cache = self.cache.write().await;
        // Another request may have refreshed the keys while this one waited.
        if let Some((keys, fetched)) = &*cache {
            if fetched.elapsed() < KEYS_MIN_REFETCH {
                return Ok(keys.find(kid).cloned());
            }
        }
        let keys: JwkSet = HTTP_CLIENT
            .get(&self.url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| Error::InternalErr(format!("could not fetch IAP keys: {e}")))?
            .json()
            .await
            .map_err(|e| Error::InternalErr(format!("could not read IAP keys: {e}")))?;
        let found = keys.find(kid).cloned();
        *cache = Some((keys, Instant::now()));
        Ok(found)
    }
}

/// The address in an IAP `email` claim, without the `issuer/tenant:` prefix Identity Platform
/// users carry.
fn email_from_claim(claim: &str) -> Option<String> {
    let email = claim
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    email.contains('@').then_some(email)
}

/// Signs a user in by an identity established outside Gantry (spec §3).
async fn sign_in(
    db: &DB,
    webhook: &WebhookShared,
    method: &str,
    email: &str,
    cookies: Cookies,
) -> Result<String> {
    let author = |token: Option<&str>| AuditAuthor {
        email: email.to_string(),
        username: email.to_string(),
        username_override: None,
        token_prefix: token.map(safe_token_prefix),
    };
    let details = || -> Option<HashMap<&str, &str>> { Some(HashMap::from([("method", method)])) };
    let mut tx = db.begin().await?;

    let account: Option<(bool, bool, String)> = sqlx::query_as(
        "SELECT super_admin, disabled, login_type FROM password WHERE email = $1 FOR UPDATE",
    )
    .bind(email)
    .fetch_optional(&mut *tx)
    .await?;

    let refusal = match &account {
        Some((_, true, _)) => Some(Error::PermissionDenied(format!("{email} is disabled"))),
        None if REQUIRE_PREEXISTING_USER_FOR_OAUTH.load(Ordering::Relaxed) => {
            Some(Error::PermissionDenied(format!(
                "there is no account for {email}: ask an administrator to create your account"
            )))
        }
        _ => None,
    };
    if let Some(refusal) = refusal {
        audit_log(
            &mut *tx,
            &author(None),
            "users.login_failure",
            ActionKind::Create,
            "global",
            None,
            details(),
        )
        .await?;
        tx.commit().await?;
        return Err(refusal);
    }

    let created = account.is_none();
    let super_admin = match account {
        Some((super_admin, _, login_type)) => {
            if login_type == PENDING_OAUTH_LOGIN_TYPE {
                sqlx::query("UPDATE password SET login_type = $1 WHERE email = $2")
                    .bind(method)
                    .bind(email)
                    .execute(&mut *tx)
                    .await?;
            }
            super_admin
        }
        None => {
            insert_account(
                &mut tx,
                &NewAccount {
                    email,
                    password_hash: None,
                    login_type: method,
                    super_admin: false,
                    name: None,
                    company: None,
                },
            )
            .await?;
            audit_log(
                &mut *tx,
                &author(None),
                "users.create",
                ActionKind::Create,
                "global",
                Some(email),
                details(),
            )
            .await?;
            false
        }
    };

    let token = create_session_token(email, super_admin, None, false, &mut tx, cookies).await?;
    audit_log(
        &mut *tx,
        &author(Some(&token)),
        "users.login",
        ActionKind::Create,
        "global",
        Some(&truncate_token(&token)),
        details(),
    )
    .await?;
    tx.commit().await?;

    if created {
        webhook.send_instance_event(InstanceEvent::UserAdded { email: email.to_string() });
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_platform_prefixes_are_dropped() {
        assert_eq!(
            email_from_claim("Ada@Example.com").as_deref(),
            Some("ada@example.com")
        );
        assert_eq!(
            email_from_claim("securetoken.google.com/p/t:ada@example.com").as_deref(),
            Some("ada@example.com")
        );
        assert_eq!(email_from_claim("accounts.google.com:12345"), None);
    }

    #[test]
    fn only_loopback_base_urls_are_local() {
        assert!(is_local_url("http://localhost:8000"));
        assert!(is_local_url("http://127.0.0.1:8000"));
        assert!(is_local_url("http://[::1]:8000"));
        assert!(!is_local_url("https://gantry.example.com"));
        assert!(!is_local_url("http://localhost.example.com"));
        assert!(!is_local_url(""));
    }
}
