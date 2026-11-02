//! Sign-in with OAuth providers (SSO).
//!
//! Clean-room implementation of `specs/sso-oauth-login.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use std::collections::HashMap;

use axum::{
    extract::{Extension, Path},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    audit::AuditAuthor,
    auth::safe_token_prefix,
    error::{Error, JsonResult, Result},
    oauth2::{InstanceEvent, REQUIRE_PREEXISTING_USER_FOR_OAUTH},
    users::{truncate_token, PENDING_OAUTH_LOGIN_TYPE},
    utils::IS_SECURE,
};
use windmill_oauth::{
    build_basic_client, exchange_code, http_get_user_info, oauth_redirect, ClientWithScopes,
    GHEmailInfo, OAuthCallback, OAuthClient, OAuthConfig, TokenResponse, OAUTH_CLIENTS,
    OAUTH_HTTP_CLIENT,
};

use crate::{
    db::DB,
    users::create_session_token,
    users_oss::{insert_account, NewAccount, PROFILE_FIELD_MAX_LEN},
    webhook_util::WebhookShared,
};

const GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";

pub fn routes() -> Router {
    Router::new()
        .route("/list_logins", get(list_logins))
        .route("/login/{client}", get(login))
        .route("/login_callback/{client}", post(login_callback))
}

/// The login providers configured in the `oauths` setting (spec §1). An entry that cannot be
/// built is logged and skipped.
pub fn login_clients(
    oauths: &HashMap<String, OAuthClient>,
    base_url: &str,
) -> HashMap<String, ClientWithScopes> {
    let registry: HashMap<String, OAuthConfig> =
        serde_json::from_str(include_str!("../../oauth_login.json"))
            .expect("oauth_login.json is valid");
    oauths
        .iter()
        .filter_map(|(name, client)| {
            match login_client(name, client, registry.get(name), base_url) {
                Ok(c) => c.map(|c| (name.clone(), c)),
                Err(e) => {
                    tracing::error!("Sign-in provider {name} skipped: {e:#}");
                    None
                }
            }
        })
        .collect()
}

fn login_client(
    name: &str,
    client: &OAuthClient,
    registered: Option<&OAuthConfig>,
    base_url: &str,
) -> Result<Option<ClientWithScopes>> {
    if client.id.is_empty() || client.secret.is_empty() {
        return Ok(None);
    }
    let Some(mut config) = client.login_config.clone().or_else(|| registered.cloned()) else {
        return Ok(None);
    };
    if name == "microsoft" {
        if let Some(tenant) = client.tenant.as_deref().filter(|t| !t.is_empty()) {
            for url in [&mut config.auth_url, &mut config.token_url] {
                *url = url.replace("/common/", &format!("/{tenant}/"));
            }
        }
    }
    let (_, oclient) = build_basic_client(
        name.to_string(),
        config.clone(),
        client.clone(),
        true,
        base_url,
        None,
    )?;
    Ok(Some(ClientWithScopes {
        display_name: client.display_name.clone(),
        client: oclient,
        scopes: config.scopes.unwrap_or_default(),
        extra_params: config.extra_params,
        extra_params_callback: config.extra_params_callback,
        allowed_domains: client.allowed_domains.clone(),
        userinfo_url: config.userinfo_url,
        connect_fields: None,
        grant_types: client.grant_types.clone(),
        token_response_path: config.token_response_path,
        token_url: config.token_url,
        has_shared_credentials: true,
    }))
}

#[derive(Serialize)]
struct LoginProvider {
    #[serde(rename = "type")]
    name: String,
    display_name: Option<String>,
}

#[derive(Serialize)]
struct Logins {
    oauth: Vec<LoginProvider>,
    saml: Option<String>,
    auto_login: Option<String>,
}

async fn list_logins() -> JsonResult<Logins> {
    let clients = OAUTH_CLIENTS.load();
    let mut oauth: Vec<LoginProvider> = clients
        .logins
        .iter()
        .map(|(name, c)| LoginProvider { name: name.clone(), display_name: c.display_name.clone() })
        .collect();
    oauth.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(Logins { oauth, saml: None, auto_login: None }))
}

async fn login(Path(client): Path<String>, cookies: Cookies) -> Result<Redirect> {
    let clients = OAUTH_CLIENTS.load();
    let extra_params = clients
        .logins
        .get(&client)
        .ok_or_else(|| Error::BadRequest(format!("unknown sign-in provider {client}")))?
        .extra_params
        .clone();
    oauth_redirect(
        &clients.logins,
        client,
        cookies,
        None,
        extra_params,
        IS_SECURE.load(std::sync::atomic::Ordering::Relaxed),
    )
}

#[derive(Deserialize)]
struct LoginCallback {
    code: String,
    state: String,
}

/// Who the provider says the user is.
#[derive(Debug, PartialEq, Eq)]
struct ProviderUser {
    email: String,
    name: Option<String>,
}

async fn login_callback(
    Path(provider): Path<String>,
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Json(callback): Json<LoginCallback>,
) -> Result<String> {
    let client = OAUTH_CLIENTS
        .load()
        .logins
        .get(&provider)
        .cloned()
        .ok_or_else(|| Error::BadRequest(format!("unknown sign-in provider {provider}")))?;

    let token: TokenResponse = exchange_code(
        OAuthCallback { code: callback.code, state: callback.state, params: HashMap::new() },
        &cookies,
        client.client.clone(),
        client.extra_params_callback.clone(),
        &OAUTH_HTTP_CLIENT,
    )
    .await?;
    let user = provider_user(&provider, &client, token.access_token.as_ref()).await?;
    check_allowed_domain(&user.email, client.allowed_domains.as_deref())?;

    sign_in(&db, &webhook, &provider, user, cookies).await
}

/// Reads the user's email and name from the provider (spec §3.3).
async fn provider_user(
    provider: &str,
    client: &ClientWithScopes,
    access_token: &str,
) -> Result<ProviderUser> {
    let userinfo_url = client.userinfo_url.as_deref().ok_or_else(|| {
        Error::BadConfig(format!("sign-in provider {provider} has no userinfo_url"))
    })?;
    let info: serde_json::Value =
        http_get_user_info(&OAUTH_HTTP_CLIENT, userinfo_url, access_token).await?;
    let mut user = user_from_info(&info)?;
    if provider == "github" && user.email.is_empty() {
        let emails: Vec<GHEmailInfo> =
            http_get_user_info(&OAUTH_HTTP_CLIENT, GITHUB_EMAILS_URL, access_token).await?;
        user.email = github_email(&emails)
            .ok_or_else(|| {
                Error::BadRequest("your GitHub account has no verified primary email".to_string())
            })?
            .to_lowercase();
    }
    if user.email.is_empty() {
        return Err(Error::BadRequest(format!(
            "{provider} did not share your email address"
        )));
    }
    Ok(user)
}

/// The user from a userinfo response; the email is empty when the response has none.
fn user_from_info(info: &serde_json::Value) -> Result<ProviderUser> {
    if info["email_verified"] == serde_json::Value::Bool(false) {
        return Err(Error::BadRequest(
            "the provider has not verified your email address".to_string(),
        ));
    }
    let text = |key: &str| info[key].as_str().map(str::trim).filter(|v| !v.is_empty());
    Ok(ProviderUser {
        email: text("email").unwrap_or_default().to_lowercase(),
        name: text("name")
            .or_else(|| text("login"))
            .map(|n| n.chars().take(PROFILE_FIELD_MAX_LEN).collect()),
    })
}

fn github_email(emails: &[GHEmailInfo]) -> Option<&str> {
    emails
        .iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email.as_str())
}

fn check_allowed_domain(email: &str, allowed: Option<&[String]>) -> Result<()> {
    let Some(allowed) = allowed.filter(|a| !a.is_empty()) else {
        return Ok(());
    };
    let domain = email.rsplit_once('@').map(|(_, d)| d).unwrap_or_default();
    if allowed
        .iter()
        .any(|a| a.trim().eq_ignore_ascii_case(domain))
    {
        Ok(())
    } else {
        Err(Error::PermissionDenied(format!(
            "{email} is not in a domain allowed to sign in"
        )))
    }
}

/// Signs the user in, creating their account if allowed (spec §3.5–6).
async fn sign_in(
    db: &DB,
    webhook: &WebhookShared,
    provider: &str,
    user: ProviderUser,
    cookies: Cookies,
) -> Result<String> {
    let email = user.email;
    let author = |token: Option<&str>| AuditAuthor {
        email: email.clone(),
        username: email.clone(),
        username_override: None,
        token_prefix: token.map(safe_token_prefix),
    };
    let mut tx = db.begin().await?;

    let account: Option<(bool, bool, String)> = sqlx::query_as(
        "SELECT super_admin, disabled, login_type FROM password WHERE email = $1 FOR UPDATE",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    let refusal = match &account {
        Some((_, true, _)) => Some(Error::PermissionDenied(format!("{email} is disabled"))),
        None if REQUIRE_PREEXISTING_USER_FOR_OAUTH.load(std::sync::atomic::Ordering::Relaxed) => {
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
            Some([("provider", provider)].into()),
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
                    .bind(provider)
                    .bind(&email)
                    .execute(&mut *tx)
                    .await?;
            }
            super_admin
        }
        None => {
            insert_account(
                &mut tx,
                &NewAccount {
                    email: &email,
                    password_hash: None,
                    login_type: provider,
                    super_admin: false,
                    name: user.name.as_deref(),
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
                Some(&email),
                Some([("provider", provider)].into()),
            )
            .await?;
            false
        }
    };

    let token = create_session_token(&email, super_admin, None, false, &mut tx, cookies).await?;
    audit_log(
        &mut *tx,
        &author(Some(&token)),
        "users.login",
        ActionKind::Create,
        "global",
        Some(&truncate_token(&token)),
        Some([("provider", provider)].into()),
    )
    .await?;
    tx.commit().await?;

    if created {
        webhook.send_instance_event(InstanceEvent::UserAdded { email });
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn userinfo_email_is_lowercased_and_name_falls_back_to_login() {
        let user = user_from_info(&json!({ "email": "Ada@Example.com", "login": "ada" })).unwrap();
        assert_eq!(
            user,
            ProviderUser { email: "ada@example.com".into(), name: Some("ada".into()) }
        );
    }

    #[test]
    fn an_unverified_email_is_refused() {
        assert!(user_from_info(&json!({ "email": "a@b.c", "email_verified": false })).is_err());
        assert!(user_from_info(&json!({ "email": "a@b.c", "email_verified": true })).is_ok());
    }

    #[test]
    fn github_uses_the_primary_verified_address() {
        let emails = vec![
            GHEmailInfo { email: "old@x.io".into(), verified: true, primary: false },
            GHEmailInfo { email: "main@x.io".into(), verified: true, primary: true },
        ];
        assert_eq!(github_email(&emails), Some("main@x.io"));
        let unverified =
            vec![GHEmailInfo { email: "main@x.io".into(), verified: false, primary: true }];
        assert_eq!(github_email(&unverified), None);
    }

    #[test]
    fn allowed_domains_match_case_insensitively() {
        let allowed = vec!["Example.com".to_string()];
        assert!(check_allowed_domain("ada@example.com", Some(&allowed)).is_ok());
        assert!(check_allowed_domain("ada@evil.com", Some(&allowed)).is_err());
        assert!(check_allowed_domain("ada@evil.com", None).is_ok());
        assert!(check_allowed_domain("ada@evil.com", Some(&[])).is_ok());
    }

    #[test]
    fn registry_providers_need_credentials_and_microsoft_takes_a_tenant() {
        let client = |id: &str, tenant: Option<&str>| OAuthClient {
            id: id.into(),
            secret: "s".into(),
            display_name: None,
            allowed_domains: None,
            connect_config: None,
            login_config: None,
            tenant: tenant.map(Into::into),
            grant_types: vec!["authorization_code".into()],
            signing_secret: None,
        };
        let oauths = HashMap::from([
            ("microsoft".to_string(), client("id", Some("contoso"))),
            ("google".to_string(), client("", None)),
            ("unknown".to_string(), client("id", None)),
        ]);
        let logins = login_clients(&oauths, "https://gantry.example.com");
        assert_eq!(logins.keys().collect::<Vec<_>>(), vec!["microsoft"]);
        assert!(logins["microsoft"].token_url.contains("/contoso/"));
    }
}
