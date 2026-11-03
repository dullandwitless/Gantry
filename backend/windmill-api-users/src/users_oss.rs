use crate::users::{set_session_cookie, ImpersonateServiceAccountRequest};
use http::StatusCode;
use tower_cookies::Cookies;
use windmill_api_auth::ApiAuthed;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    auth::{hash_token, TOKEN_PREFIX_LEN},
    error::{Error, Result},
    min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH,
    utils::{rd_string, require_admin},
    DB,
};

/// How long an impersonation lasts. The token is never refreshed into a session.
const IMPERSONATION_VALIDITY_SECONDS: i64 = 24 * 60 * 60;

/// Mints a token acting as one of the workspace's service accounts and makes it the session
/// (specs/service-accounts.md, section 3). The caller keeps `_current_token` to restore with
/// `exit_impersonation`.
/// Clean-room implementation; attestation: written without reading Windmill's frontend source
/// or proprietary code.
pub async fn impersonate_service_account(
    db: DB,
    authed: ApiAuthed,
    cookies: Cookies,
    _current_token: String,
    w_id: String,
    req: ImpersonateServiceAccountRequest,
) -> Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;
    if authed.job_id.is_some() {
        return Err(Error::NotAuthorized(
            "a job token ($WM_TOKEN) cannot impersonate a service account".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    let email: String = sqlx::query_scalar(
        "SELECT email FROM usr
         WHERE workspace_id = $1 AND username = $2 AND is_service_account AND NOT disabled",
    )
    .bind(&w_id)
    .bind(&req.username)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "no active service account {} in workspace {w_id}",
            req.username
        ))
    })?;

    let token = rd_string(32);
    let prefix = &token[..TOKEN_PREFIX_LEN];
    // Servers too old to look tokens up by hash still need the plaintext.
    let plaintext = (!MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await).then_some(token.as_str());
    sqlx::query(
        "INSERT INTO token
            (token_hash, token_prefix, token, email, label, expiration, super_admin, workspace_id)
         VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6), false, $7)",
    )
    .bind(hash_token(&token))
    .bind(prefix)
    .bind(plaintext)
    .bind(&email)
    .bind(format!("impersonation:{}", authed.email))
    .bind(IMPERSONATION_VALIDITY_SECONDS as f64)
    .bind(&w_id)
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.impersonate_service_account",
        ActionKind::Create,
        &w_id,
        Some(&email),
        Some([("token_prefix", prefix)].into()),
    )
    .await?;
    tx.commit().await?;

    set_session_cookie(&cookies, &token, IMPERSONATION_VALIDITY_SECONDS);
    Ok((StatusCode::CREATED, token))
}

pub(crate) async fn portal_cloud_trial_login(
    _email: &str,
) -> windmill_common::error::Result<crate::users::PortalTrialLogin> {
    Err(windmill_common::error::Error::FeatureUnavailable(
        "Starting a pre-approved trial from Windmill Cloud is not supported in Gantry".to_string(),
    ))
}
