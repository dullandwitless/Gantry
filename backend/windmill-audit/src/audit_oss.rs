use windmill_common::{
    audit::AuditAuthor,
    db::{Authable, DbWithOptAuthed},
};

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use {
    crate::{ActionKind, AuditLog, ListAuditLogQuery},
    sqlx::{Postgres, Transaction},
    std::collections::HashMap,
    windmill_common::{
        error::{Error, Result},
        utils::Pagination,
    },
};

impl<'a, T: Authable + AuditAuthorable + Sync> AuditAuthorable for DbWithOptAuthed<'a, T> {
    fn email(&self) -> &str {
        match self {
            DbWithOptAuthed::UserDB { authed, .. } => AuditAuthorable::email(*authed),
            DbWithOptAuthed::DB { audit_author, .. } => audit_author.email(),
        }
    }
    fn username(&self) -> &str {
        match self {
            DbWithOptAuthed::UserDB { authed, .. } => AuditAuthorable::username(*authed),
            DbWithOptAuthed::DB { audit_author, .. } => audit_author.username(),
        }
    }
    fn username_override(&self) -> Option<&str> {
        match self {
            DbWithOptAuthed::UserDB { authed, .. } => AuditAuthorable::username_override(*authed),
            DbWithOptAuthed::DB { .. } => None,
        }
    }
}

impl AuditAuthorable for AuditAuthor {
    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }

    fn token_prefix(&self) -> Option<&str> {
        self.token_prefix.as_deref()
    }
}

/// Where an audit row is written decides what a failed insert breaks. On a pool connection it
/// breaks nothing else, so it is logged and the action goes on. On a connection, which may hold
/// the action's transaction, the failure has aborted that transaction and its COMMIT would roll
/// back silently, so it is returned.
pub trait AuditExecutor<'c>: sqlx::Executor<'c, Database = sqlx::Postgres> {
    const ISOLATED: bool;
}

impl<'c> AuditExecutor<'c> for &'c sqlx::Pool<sqlx::Postgres> {
    const ISOLATED: bool = true;
}

impl<'c> AuditExecutor<'c> for &'c mut sqlx::PgConnection {
    const ISOLATED: bool = false;
}

pub trait AuditAuthorable {
    fn username(&self) -> &str;
    fn email(&self) -> &str;
    fn username_override(&self) -> Option<&str>;
    fn token_prefix(&self) -> Option<&str> {
        None
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c, E: AuditExecutor<'c>>(
    _db: E,
    _author: &impl AuditAuthorable,
    mut _operation: &str,
    _action_kind: ActionKind,
    _w_id: &str,
    mut _resource: Option<&str>,
    _parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    // Stub: not implemented.
    Ok(())
}

pub async fn audit_log_many<'c, E: AuditExecutor<'c>>(
    _db: E,
    _author: &impl AuditAuthorable,
    _operation: &str,
    _action_kind: ActionKind,
    _w_id: &str,
    _resources: &[String],
) -> Result<()> {
    // Stub: not implemented.
    Ok(())
}

pub async fn list_audit(
    _tx: Transaction<'_, Postgres>,
    _w_id: String,
    _pagination: Pagination,
    _lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    // Stub: not implemented.
    return Ok(vec![]);
}

pub async fn get_audit(tx: Transaction<'_, Postgres>, _id: i32, _w_id: &str) -> Result<AuditLog> {
    // Stub: not implemented.
    tx.commit().await?;
    Err(Error::NotFound(
        "Audit log not not available in Windmill Community edition".to_string(),
    ))
}
