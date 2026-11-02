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

/// Widths of the `audit_partitioned` columns written from caller-supplied text.
const OPERATION_MAX_LEN: usize = 50;
const TEXT_COLUMN_MAX_LEN: usize = 255;

fn fit(value: &str, max_chars: usize) -> &str {
    match value.char_indices().nth(max_chars) {
        Some((end, _)) => &value[..end],
        None => value,
    }
}

/// The `parameters` column: the caller's parameters plus how the author authenticated
/// (specs/audit-logs.md, section 2). `None` when there is nothing to record.
fn parameters_json(
    author: &impl AuditAuthorable,
    parameters: Option<HashMap<&str, &str>>,
) -> Option<serde_json::Value> {
    let mut map: serde_json::Map<String, serde_json::Value> = parameters
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.to_string(), v.into()))
        .collect();
    // A token label chooses `username_override`, so it is context, never the identity.
    if let Some(over) = author
        .username_override()
        .filter(|o| *o != author.username())
    {
        map.insert("username_override".to_string(), over.into());
    }
    if let Some(prefix) = author.token_prefix() {
        map.insert("token_prefix".to_string(), prefix.into());
    }
    (!map.is_empty()).then_some(serde_json::Value::Object(map))
}

/// A failed audit insert on an isolated (pool) connection is logged and the action goes on;
/// on a connection that may hold the caller's transaction it is returned.
fn handle_insert_result<'c, E: AuditExecutor<'c>>(
    result: std::result::Result<sqlx::postgres::PgQueryResult, sqlx::Error>,
    operation: &str,
) -> Result<()> {
    match result {
        Ok(_) => Ok(()),
        Err(e) if E::ISOLATED => {
            tracing::error!("writing the audit log entry for {operation} failed: {e:#}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c, E: AuditExecutor<'c>>(
    db: E,
    author: &impl AuditAuthorable,
    operation: &str,
    action_kind: ActionKind,
    w_id: &str,
    resource: Option<&str>,
    parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    let result = sqlx::query(
        "INSERT INTO audit_partitioned
            (workspace_id, username, email, operation, action_kind, resource, parameters)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(w_id)
    .bind(fit(author.username(), TEXT_COLUMN_MAX_LEN))
    .bind(fit(author.email(), TEXT_COLUMN_MAX_LEN))
    .bind(fit(operation, OPERATION_MAX_LEN))
    .bind(action_kind)
    .bind(resource.map(|r| fit(r, TEXT_COLUMN_MAX_LEN)))
    .bind(parameters_json(author, parameters))
    .execute(db)
    .await;
    handle_insert_result::<E>(result, operation)
}

pub async fn audit_log_many<'c, E: AuditExecutor<'c>>(
    db: E,
    author: &impl AuditAuthorable,
    operation: &str,
    action_kind: ActionKind,
    w_id: &str,
    resources: &[String],
) -> Result<()> {
    if resources.is_empty() {
        return Ok(());
    }
    let resources: Vec<&str> = resources
        .iter()
        .map(|r| fit(r, TEXT_COLUMN_MAX_LEN))
        .collect();
    let result = sqlx::query(
        "INSERT INTO audit_partitioned
            (workspace_id, username, email, operation, action_kind, resource, parameters)
         SELECT $1, $2, $3, $4, $5, resource, $7
         FROM UNNEST($6::text[]) AS resource",
    )
    .bind(w_id)
    .bind(fit(author.username(), TEXT_COLUMN_MAX_LEN))
    .bind(fit(author.email(), TEXT_COLUMN_MAX_LEN))
    .bind(fit(operation, OPERATION_MAX_LEN))
    .bind(action_kind)
    .bind(&resources)
    .bind(parameters_json(author, None))
    .execute(db)
    .await;
    handle_insert_result::<E>(result, operation)
}

/// `value` as a LIKE pattern matching itself and anything it prefixes.
fn prefix_pattern(value: &str) -> String {
    let mut pattern = String::with_capacity(value.len() + 1);
    for c in value.chars() {
        if matches!(c, '\\' | '%' | '_') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

fn comma_list(value: &Option<String>) -> Option<Vec<String>> {
    value.as_ref().map(|v| {
        v.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    })
}

fn parse_action_kind(value: &str) -> Result<ActionKind> {
    match value.to_ascii_lowercase().as_str() {
        "create" => Ok(ActionKind::Create),
        "update" => Ok(ActionKind::Update),
        "delete" => Ok(ActionKind::Delete),
        "execute" => Ok(ActionKind::Execute),
        _ => Err(Error::BadRequest(format!(
            "unknown action_kind {value:?}: expected Create, Update, Delete or Execute"
        ))),
    }
}

const AUDIT_LOG_COLUMNS: &str =
    "workspace_id, id, timestamp, username, operation, action_kind, resource, parameters, span";

/// Lists audit rows in the caller's transaction, so row-level security decides whose rows
/// are visible (specs/audit-logs.md, section 3). The caller authorises `all_workspaces`.
pub async fn list_audit(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    pagination: Pagination,
    lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    let action_kind = lq
        .action_kind
        .as_deref()
        .map(parse_action_kind)
        .transpose()?;
    let (per_page, offset) = windmill_common::utils::paginate(pagination);

    let rows = sqlx::query_as::<_, AuditLog>(&format!(
        "SELECT {AUDIT_LOG_COLUMNS} FROM audit_partitioned
         WHERE ($1 OR workspace_id = $2)
           AND ($3::text IS NULL OR username = $3)
           AND ($4::text IS NULL OR operation LIKE $4)
           AND ($5::text[] IS NULL OR operation = ANY($5))
           AND ($6::text[] IS NULL OR operation <> ALL($6))
           AND ($7::text IS NULL OR resource LIKE $7)
           AND ($8::action_kind IS NULL OR action_kind = $8)
           AND ($9::timestamptz IS NULL OR timestamp <= $9)
           AND ($10::timestamptz IS NULL OR timestamp > $10)
           AND ($11::bigint IS NULL OR id < $11)
         ORDER BY id DESC
         LIMIT $12 OFFSET $13"
    ))
    .bind(lq.all_workspaces.unwrap_or(false))
    .bind(&w_id)
    .bind(&lq.username)
    .bind(lq.operation.as_deref().map(prefix_pattern))
    .bind(comma_list(&lq.operations))
    .bind(comma_list(&lq.exclude_operations))
    .bind(lq.resource.as_deref().map(prefix_pattern))
    .bind(action_kind)
    .bind(lq.before)
    .bind(lq.after)
    .bind(lq.before_id)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows)
}

pub async fn get_audit(mut tx: Transaction<'_, Postgres>, id: i32, w_id: &str) -> Result<AuditLog> {
    let row = sqlx::query_as::<_, AuditLog>(&format!(
        "SELECT {AUDIT_LOG_COLUMNS} FROM audit_partitioned WHERE id = $1 AND workspace_id = $2"
    ))
    .bind(i64::from(id))
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    row.ok_or_else(|| Error::NotFound(format!("audit log entry {id} not found")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_truncates_on_a_character_boundary() {
        assert_eq!(fit("abc", 5), "abc");
        assert_eq!(fit("abcdef", 3), "abc");
        assert_eq!(fit("ééé", 2), "éé");
    }

    #[test]
    fn prefix_pattern_matches_wildcards_literally() {
        assert_eq!(prefix_pattern("jobs.run"), "jobs.run%");
        assert_eq!(prefix_pattern("50%_\\x"), "50\\%\\_\\\\x%");
    }

    #[test]
    fn action_kind_is_case_insensitive_and_closed() {
        assert!(matches!(
            parse_action_kind("Create"),
            Ok(ActionKind::Create)
        ));
        assert!(matches!(
            parse_action_kind("execute"),
            Ok(ActionKind::Execute)
        ));
        assert!(parse_action_kind("drop").is_err());
    }

    #[test]
    fn a_token_label_is_context_not_identity() {
        let author = AuditAuthor {
            email: "a@example.com".to_string(),
            username: "alice".to_string(),
            username_override: Some("webhook-x".to_string()),
            token_prefix: Some("abcdef1234".to_string()),
        };
        let params = parameters_json(&author, Some(HashMap::from([("k", "v")]))).unwrap();
        assert_eq!(params["k"], "v");
        assert_eq!(params["username_override"], "webhook-x");
        assert_eq!(params["token_prefix"], "abcdef1234");

        let plain = AuditAuthor { username_override: None, token_prefix: None, ..author };
        assert!(parameters_json(&plain, None).is_none());
    }
}
