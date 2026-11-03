use crate::workspaces::{
    reject_admin_and_operator, resolve_new_member_role, EditAutoInvite, NewServiceAccount,
};
use http::StatusCode;
use windmill_api_auth::ApiAuthed;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{Error, Result},
    usernames::validate_username,
    utils::require_admin,
    DB,
};

/// Gantry has no invites: members join on their first sign-in (specs/single-organisation.md
/// §1). Reached only from the workspace settings document.
pub async fn edit_auto_invite(
    _authed: ApiAuthed,
    _db: DB,
    _w_id: String,
    _ea: EditAutoInvite,
) -> Result<String> {
    Err(Error::BadRequest(
        "Gantry has no auto-invite: members join the workspace when they first sign in".to_string(),
    ))
}

const DEPLOYERS_GROUP_SUMMARY: &str =
    "Members can preserve the original author when deploying to this workspace";

/// The derived email of a service account: unique per workspace and username, and never
/// deliverable (`.invalid` is a reserved top-level domain).
pub fn service_account_email(w_id: &str, username: &str) -> String {
    format!("{username}@{w_id}.sa.invalid")
}

/// Creates a workspace service account (specs/service-accounts.md, section 2).
/// Clean-room implementation; attestation: written without reading Windmill's frontend source
/// or proprietary code.
pub async fn create_service_account(
    authed: ApiAuthed,
    db: DB,
    w_id: String,
    nu: NewServiceAccount,
) -> Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;
    if w_id == "admins" {
        return Err(Error::BadRequest(
            "The admins workspace is reserved for superadmins".to_string(),
        ));
    }
    reject_admin_and_operator(nu.is_admin, nu.operator)?;
    let username = nu.username.trim().to_string();
    validate_username(&username)?;
    let (role, operator) = resolve_new_member_role(&db, &w_id, nu.operator, None).await?;
    let email = service_account_email(&w_id, &username);

    let mut tx = db.begin().await?;

    // Taken anywhere on the instance, the name could later collide with that person joining
    // this workspace.
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1
                UNION SELECT 1 FROM password WHERE username = $1
                UNION SELECT 1 FROM pending_user WHERE username = $1)",
    )
    .bind(&username)
    .fetch_one(&mut *tx)
    .await?;
    if taken {
        return Err(Error::BadRequest(format!(
            "the username {username} is already taken"
        )));
    }

    sqlx::query(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator, workspace_role, is_service_account)
         VALUES ($1, $2, $3, $4, $5, $6, true)",
    )
    .bind(&w_id)
    .bind(&email)
    .bind(&username)
    .bind(nu.is_admin)
    .bind(operator)
    .bind(&role)
    .execute(&mut *tx)
    .await?;

    if nu.add_to_deployers {
        // Workspaces created before the group existed get it from a migration; newer ones may not.
        sqlx::query(
            "INSERT INTO group_ (workspace_id, name, summary, extra_perms)
             VALUES ($1, 'wm_deployers', $2, '{}'::jsonb) ON CONFLICT DO NOTHING",
        )
        .bind(&w_id)
        .bind(DEPLOYERS_GROUP_SUMMARY)
        .execute(&mut *tx)
        .await?;
    }
    let groups: &[&str] = if nu.add_to_deployers {
        &["all", "wm_deployers"]
    } else {
        &["all"]
    };
    for group in groups {
        sqlx::query(
            "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(&w_id)
        .bind(&username)
        .bind(group)
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.create_service_account",
        ActionKind::Create,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("service account {username} created"),
    ))
}
