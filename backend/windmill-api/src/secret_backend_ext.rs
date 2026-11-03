/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the API layer
//!
//! Backend resolution and read helpers live in `windmill_common::secret_backend`, and moving
//! externally stored secrets in `windmill_store::secret_backend_ext`; this module adds the checks
//! bulk path moves (user renames, offboarding) need.

use windmill_common::error::Result;

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

pub use windmill_store::secret_backend_ext::{
    rename_vault_secrets_with_prefix, MOVABLE_EXTERNAL_VALUE_SQL,
};
