/*
 * Gantry clean-room implementation of global user creation.
 * Specification: specs/users-and-email.md (sections 1–2), specs/single-organisation.md (§1).
 * This file and its contents are licensed under the AGPLv3 License.
 */

use crate::db::ApiAuthed;

use crate::users::NewUser;

use crate::{db::DB, webhook_util::WebhookShared};

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

/// Width of `password.name` and `password.company`.
pub(crate) const PROFILE_FIELD_MAX_LEN: usize = 255;

/// Creates an account ahead of its first sign-in, which gives it its login type.
pub async fn create_user(
    authed: ApiAuthed,
    db: DB,
    webhook: WebhookShared,
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

    let mut tx = db.begin().await?;

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password WHERE email = $1)")
        .bind(&email)
        .fetch_one(&mut *tx)
        .await?;
    if exists {
        return Err(Error::BadRequest(format!("user {email} already exists")));
    }

    insert_account(
        &mut tx,
        &NewAccount {
            email: &email,
            login_type: PENDING_OAUTH_LOGIN_TYPE,
            super_admin: nu.super_admin,
            name: nu.name.as_deref(),
            company: nu.company.as_deref(),
        },
    )
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

    Ok((StatusCode::CREATED, format!("user {email} created")))
}

pub(crate) struct NewAccount<'a> {
    pub email: &'a str,
    pub login_type: &'a str,
    pub super_admin: bool,
    pub name: Option<&'a str>,
    pub company: Option<&'a str>,
}

/// Inserts the `password` row of a new, verified account without a password, with its instance
/// username. The caller has checked that no account has this email.
pub(crate) async fn insert_account(
    tx: &mut Transaction<'_, Postgres>,
    account: &NewAccount<'_>,
) -> Result<()> {
    let username = instance_username_for_new_account(tx, account.email).await?;
    sqlx::query(
        "INSERT INTO password
            (email, login_type, super_admin, verified, name, company, username)
         VALUES ($1, $2, $3, true, $4, $5, $6)",
    )
    .bind(account.email)
    .bind(account.login_type)
    .bind(account.super_admin)
    .bind(account.name)
    .bind(account.company)
    .bind(&username)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
