/*
 * Gantry clean-room implementation of global user creation and password setting.
 * Specification: specs/users-and-email.md (sections 1–3).
 * This file and its contents are licensed under the AGPLv3 License.
 */

use std::sync::Arc;

use crate::db::ApiAuthed;

use crate::users::{EditPassword, NewUser};

use crate::{db::DB, webhook_util::WebhookShared};

use argon2::{Argon2, PasswordHasher};

use axum::{extract::Extension, Json};

use http::StatusCode;

use serde::Deserialize;

use sqlx::{Postgres, Transaction};
use windmill_api_auth::require_super_admin;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::error::{Error, Result};
use windmill_common::global_settings::AUTOMATE_USERNAME_CREATION_SETTING;
use windmill_common::oauth2::InstanceEvent;
use windmill_common::usernames::generate_instance_wide_unique_username;
use windmill_common::users::{usr_accepts_email, PENDING_OAUTH_LOGIN_TYPE};

const PASSWORD_LOGIN_TYPE: &str = "password";

/// Width of `password.name` and `password.company`.
const PROFILE_FIELD_MAX_LEN: usize = 255;

/// Width of `password.login_type`.
const LOGIN_TYPE_MAX_LEN: usize = 50;

pub async fn create_user(
    authed: ApiAuthed,
    db: DB,
    webhook: WebhookShared,
    argon2: Arc<Argon2<'_>>,
    nu: NewUser,
) -> Result<(StatusCode, String)> {
    require_super_admin(&db, &authed).await?;

    let email = nu.email.trim().to_lowercase();
    if !usr_accepts_email(&db, &email).await? {
        return Err(Error::BadRequest(format!(
            "{email:?} is not a valid email address"
        )));
    }
    for (field, value) in [("name", &nu.name), ("company", &nu.company)] {
        if value
            .as_ref()
            .is_some_and(|v| v.chars().count() > PROFILE_FIELD_MAX_LEN)
        {
            return Err(Error::BadRequest(format!(
                "{field} is longer than {PROFILE_FIELD_MAX_LEN} characters"
            )));
        }
    }

    let login_type = nu
        .login_type
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(PASSWORD_LOGIN_TYPE)
        .to_string();
    if login_type.chars().count() > LOGIN_TYPE_MAX_LEN {
        return Err(Error::BadRequest(format!(
            "login_type is longer than {LOGIN_TYPE_MAX_LEN} characters"
        )));
    }
    let password_hash = if login_type == PASSWORD_LOGIN_TYPE {
        let password = nu
            .password
            .filter(|p| !p.is_empty())
            .ok_or_else(|| Error::BadRequest("a password is required".to_string()))?;
        Some(hash_password(argon2, password)?)
    } else {
        None
    };

    let mut tx = db.begin().await?;

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password WHERE email = $1)")
        .bind(&email)
        .fetch_one(&mut *tx)
        .await?;
    if exists {
        return Err(Error::BadRequest(format!("user {email} already exists")));
    }

    let username = instance_username_for_new_account(&mut tx, &email).await?;

    sqlx::query(
        "INSERT INTO password
            (email, password_hash, login_type, super_admin, verified, name, company, username)
         VALUES ($1, $2, $3, $4, true, $5, $6, $7)",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&login_type)
    .bind(nu.super_admin)
    .bind(&nu.name)
    .bind(&nu.company)
    .bind(&username)
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.create",
        ActionKind::Create,
        "global",
        Some(&email),
        None,
    )
    .await?;

    tx.commit().await?;

    webhook.send_instance_event(InstanceEvent::UserAdded { email: email.clone() });

    if !nu.skip_email.unwrap_or(false) {
        send_welcome_email(&email, &login_type);
    }

    Ok((StatusCode::CREATED, format!("user {email} created")))
}

/// The `password.username` a new account gets (spec section 1, step 5), or `None` when
/// automatic username creation is off.
async fn instance_username_for_new_account(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<Option<String>> {
    let automate: bool = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM global_settings WHERE name = $1",
    )
    .bind(AUTOMATE_USERNAME_CREATION_SETTING)
    .fetch_optional(&mut **tx)
    .await?
    .and_then(|v| v.as_bool())
    .unwrap_or(true);
    if !automate {
        return Ok(None);
    }

    // A username reserved while the address had no account: workspace memberships
    // created in the meantime already carry it.
    let pending: Option<String> =
        sqlx::query_scalar("DELETE FROM pending_user WHERE email = $1 RETURNING username")
            .bind(email)
            .fetch_optional(&mut **tx)
            .await?;
    if pending.is_some() {
        return Ok(pending);
    }

    // The one username the address already has across its workspaces, unless another
    // account holds it.
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT username FROM usr WHERE email = $1")
            .bind(email)
            .fetch_all(&mut **tx)
            .await?;
    if let [username] = existing.as_slice() {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND email != $2
                    UNION SELECT 1 FROM password WHERE username = $1
                    UNION SELECT 1 FROM pending_user WHERE username = $1)",
        )
        .bind(username)
        .bind(email)
        .fetch_one(&mut **tx)
        .await?;
        if !taken {
            return Ok(Some(username.clone()));
        }
    }

    Ok(Some(
        generate_instance_wide_unique_username(tx, email).await?,
    ))
}

fn send_welcome_email(email: &str, login_type: &str) {
    let base_url = (**windmill_common::BASE_URL.load()).clone();
    let sign_in = if login_type == PASSWORD_LOGIN_TYPE {
        "Sign in with this email address and the password your administrator gave you."
    } else if login_type == PENDING_OAUTH_LOGIN_TYPE {
        "Sign in with your organization's single sign-on using this email address."
    } else {
        "Sign in with your organization's single sign-on provider."
    };
    let content = format!(
        "An account has been created for you on Gantry{at}.\n\n{sign_in}\n",
        at = if base_url.is_empty() {
            String::new()
        } else {
            format!(" at {base_url}")
        },
    );
    windmill_common::email_oss::send_email_if_possible("Your Gantry account", &content, email);
}

pub async fn set_password(
    db: DB,
    argon2: Arc<Argon2<'_>>,
    authed: ApiAuthed,
    user_email: &str,
    ep: EditPassword,
) -> Result<String> {
    if ep.password.is_empty() {
        return Err(Error::BadRequest(
            "the password must not be empty".to_string(),
        ));
    }
    let password_hash = hash_password(argon2, ep.password)?;

    let mut tx = db.begin().await?;
    let updated =
        sqlx::query("UPDATE password SET password_hash = $1, login_type = $2 WHERE email = $3")
            .bind(&password_hash)
            .bind(PASSWORD_LOGIN_TYPE)
            .bind(user_email)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    if updated == 0 {
        return Err(Error::NotFound(format!("no user with email {user_email}")));
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.setpassword",
        ActionKind::Update,
        "global",
        Some(user_email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(if user_email == authed.email {
        "password set".to_string()
    } else {
        format!("password of {user_email} set")
    })
}

/// Argon2id with a fresh random salt, as a PHC string (spec section 3).
pub fn hash_password(argon2: Arc<Argon2<'_>>, password: String) -> Result<String> {
    argon2
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| Error::internal_err(format!("hashing password: {e}")))
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct OnboardingData {
    pub touch_point: String,
    pub use_case: String,
}

pub async fn submit_onboarding_data(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Json(_data): Json<OnboardingData>,
) -> Result<String> {
    Err(Error::internal_err("Not implemented in Gantry".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::PasswordVerifier;

    /// Width of `password.password_hash`.
    const PASSWORD_HASH_MAX_LEN: usize = 100;

    #[test]
    fn hash_password_round_trips_with_a_fresh_salt() {
        let argon2 = Arc::new(Argon2::default());
        let a = hash_password(argon2.clone(), "correct horse".to_string()).unwrap();
        let b = hash_password(argon2.clone(), "correct horse".to_string()).unwrap();

        assert!(a.starts_with("$argon2id$"), "{a}");
        assert!(
            a.len() <= PASSWORD_HASH_MAX_LEN,
            "{a} does not fit password.password_hash"
        );
        assert_ne!(a, b, "each hash must use its own salt");
        assert!(argon2.verify_password(b"correct horse", a.as_str()).is_ok());
        assert!(argon2.verify_password(b"wrong", a.as_str()).is_err());
    }
}
