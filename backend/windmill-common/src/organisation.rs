//! One organisation, one workspace: the instance's workspace, its superadmins, and members
//! joining on their first sign-in.
//!
//! Clean-room implementation of `specs/single-organisation.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use sqlx::{PgConnection, Postgres, Transaction};

use crate::{
    error::Result,
    roles::{default_role_name, workspace_roles_on},
};

/// The workspace holding instance-level scripts and settings, beside the instance's one
/// workspace.
pub const ADMINS_WORKSPACE: &str = "admins";

/// Emails made superadmins when they sign in (spec §1).
const SUPERADMINS_ENV: &str = "GANTRY_SUPERADMINS";

/// The instance's workspace, once a superadmin has created it (spec §2).
pub async fn the_workspace(conn: &mut PgConnection) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM workspace
         WHERE id <> $1 AND NOT deleted AND parent_workspace_id IS NULL
         ORDER BY id LIMIT 1",
    )
    .bind(ADMINS_WORKSPACE)
    .fetch_optional(conn)
    .await?)
}

/// Whether `GANTRY_SUPERADMINS` lists `email` (lowercase).
pub fn is_listed_superadmin(email: &str) -> bool {
    std::env::var(SUPERADMINS_ENV).is_ok_and(|list| listed(&list, email))
}

fn listed(list: &str, email: &str) -> bool {
    list.split(',')
        .any(|entry| entry.trim().eq_ignore_ascii_case(email))
}

/// A member added to the workspace: where, and under which username.
pub struct Joined {
    pub workspace_id: String,
    pub username: String,
}

/// Adds the account `email` to the instance's workspace with the default role for new members
/// and to its `all` group, under its instance username. Does nothing when the workspace does
/// not exist yet, the account is already a member, or it has no instance username.
pub async fn join_the_workspace(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<Option<Joined>> {
    let Some(workspace_id) = the_workspace(&mut **tx).await? else {
        return Ok(None);
    };
    let member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
    )
    .bind(&workspace_id)
    .bind(email)
    .fetch_one(&mut **tx)
    .await?;
    if member {
        return Ok(None);
    }

    let roles = workspace_roles_on(&mut **tx, &workspace_id).await?;
    let role = default_role_name(roles.iter().map(|r| r.name.as_str()));
    let operator = roles
        .iter()
        .find(|r| r.name == role)
        .is_some_and(|r| !r.capabilities.code);
    // Joining takes the account's instance username: without one (username creation off, or
    // an account with different usernames per workspace) an admin adds it with a chosen one.
    let username: Option<String> =
        sqlx::query_scalar("SELECT username FROM password WHERE email = $1")
            .bind(email)
            .fetch_optional(&mut **tx)
            .await?
            .flatten();
    let Some(username) = username else {
        tracing::info!("{email} has no instance username, so it does not join {workspace_id}");
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, operator, workspace_role)
         VALUES ($1, $2, $3, false, $4, $5)",
    )
    .bind(&workspace_id)
    .bind(email)
    .bind(&username)
    .bind(operator)
    .bind(&role)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, 'all')
         ON CONFLICT DO NOTHING",
    )
    .bind(&workspace_id)
    .bind(&username)
    .execute(&mut **tx)
    .await?;
    Ok(Some(Joined { workspace_id, username }))
}

#[cfg(test)]
mod tests {
    use super::listed;

    #[test]
    fn the_superadmin_list_ignores_case_and_spaces() {
        assert!(listed(
            "ada@example.com, Grace@Example.com",
            "grace@example.com"
        ));
        assert!(!listed("ada@example.com", "grace@example.com"));
        assert!(!listed("", "ada@example.com"));
    }
}
