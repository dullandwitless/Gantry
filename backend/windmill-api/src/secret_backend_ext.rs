/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the API layer
//!
//! Backend resolution and read helpers live in
//! `windmill_common::secret_backend`; this module keeps the API-specific bulk
//! rename helper used when renaming users.
//!
//! Note: HashiCorp Vault integration requires Enterprise Edition.
//! The OSS version only supports the database backend.

use windmill_common::{db::DB, error::Result};

/// Refuses a bulk path move (user rename, offboarding) that would carry HTTP secret store
/// references along: their store key is derived from the path, so moved references would read
/// other keys.
pub async fn reject_http_store_refs_under_prefix(
    conn: &mut sqlx::PgConnection,
    workspace_id: &str,
    prefix: &str,
) -> Result<()> {
    let paths: Vec<String> = sqlx::query_scalar(
        "SELECT path FROM variable WHERE workspace_id = $1 AND is_secret \
         AND left(path, length($2)) = $2 AND starts_with(value, '$http_store:') \
         ORDER BY path LIMIT 5",
    )
    .bind(workspace_id)
    .bind(prefix)
    .fetch_all(conn)
    .await?;
    if paths.is_empty() {
        return Ok(());
    }
    Err(windmill_common::error::Error::BadRequest(format!(
        "Secret variables under {prefix} reference the external HTTP secret store by their path \
         and cannot be moved: {}. Delete them, then recreate the references at their new paths \
         once this is done.",
        paths.join(", ")
    )))
}

pub async fn rename_vault_secrets_with_prefix(
    _db: &DB,
    _workspace_id: &str,
    _old_prefix: &str,
    _new_prefix: &str,
    _variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    // OSS: No Vault support, return empty
    Ok(vec![])
}
