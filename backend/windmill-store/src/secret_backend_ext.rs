/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the store layer
//!
//! Write-side helpers for integrating the SecretBackend trait with variable
//! operations. Backend resolution and read helpers live in
//! `windmill_common::secret_backend` (so lower-level crates can resolve secrets
//! too) and are re-exported here for existing callers.
//!
//! Note: only the database backend is implemented.

use windmill_common::{
    db::DB,
    error::{Error, Result},
    variables::{build_crypt, encrypt},
};

pub use windmill_common::secret_backend::{
    check_external_ref, gcp_sm_marker, get_secret_backend, get_secret_value,
    is_aws_sm_stored_value, is_azure_kv_stored_value, is_external_ref_value,
    is_external_stored_value, is_gcp_sm_stored_value, is_http_store_stored_value,
    is_vault_backend_configured, is_vault_stored_value, parse_external_ref, resolves_externally,
};

/// The HTTP store's key is derived from the variable path, so moving a reference would point it
/// at a different key of a store Windmill cannot write to.
pub fn http_store_rename_error(path: &str) -> Error {
    Error::BadRequest(format!(
        "{path} references a secret in the external HTTP secret store by its path and cannot be \
         renamed. Delete it and create a new reference at the new path instead."
    ))
}

/// Store a secret value using the configured backend
///
/// For database backend: encrypts using workspace key and returns encrypted value
/// For vault backend (stub): stores in Vault and returns a placeholder for DB storage
pub async fn store_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    plain_value: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;

    match backend.backend_name() {
        "database" => {
            // Use existing database encryption
            let mc = build_crypt(db, workspace_id).await?;
            Ok(encrypt(&mc, plain_value))
        }
        "hashicorp_vault" => {
            // Store in Vault and return a marker for DB
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$vault:{}", path))
        }
        "azure_key_vault" => {
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$azure_kv:{}", path))
        }
        "aws_secrets_manager" => {
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$aws_sm:{}", path))
        }
        windmill_common::secret_backend::gcp_secret_manager::BACKEND_NAME => {
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(gcp_sm_marker(path))
        }
        // Read-only store: an empty value creates a reference to the key the path maps to, and
        // the value itself is managed in the store.
        "http_store" if plain_value.is_empty() => {
            backend.check_path(path)?;
            Ok(format!("$http_store:{}", path))
        }
        "http_store" => Err(Error::BadRequest(format!(
            "Secrets are read from an external HTTP secret store, which Gantry cannot write to. \
             Set the value of {path} in the store and save this secret with an empty value to \
             reference it."
        ))),
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Store a secret Windmill itself writes, such as the OAuth access token it refreshes. The
/// read-only HTTP store cannot hold it, so it is encrypted in the database instead, where reads
/// find it: they only go to that store for `$http_store:` references.
pub async fn store_managed_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    plain_value: &str,
) -> Result<String> {
    if get_secret_backend(db).await?.backend_name() == "http_store" {
        let mc = build_crypt(db, workspace_id).await?;
        return Ok(encrypt(&mc, plain_value));
    }
    store_secret_value(db, workspace_id, path, plain_value).await
}

/// Persist a freshly minted OAuth access token to the secret variable backing
/// a resource, routing through the configured secret backend.
///
/// This is the write counterpart of the lazy on-fetch OAuth refresh: it stores
/// the token via [`store_managed_secret_value`] (which writes to the external backend —
/// AWS Secrets Manager / Azure Key Vault / Vault — when one is configured, or
/// encrypts for the database backend and the read-only HTTP store) and updates `variable.value` with the
/// returned value (the encrypted blob for the DB backend, or a `$...:` marker
/// for an external backend). Using a raw `UPDATE variable SET value = <encrypted>`
/// here instead would leave the external store frozen at its connect-time token
/// while reads (which resolve through the backend) keep serving the stale value.
///
/// If persisting the token fails — most likely a transient error talking to an
/// external backend — the account must not end up marked fresh while the served
/// secret is stale, or the on-fetch refresh gate (`now() > expires_at`) would keep
/// serving the stale token for the whole token lifetime. So on failure we reset
/// `expires_at` to the past and record `refresh_error` — looking the account up via
/// `variable.account` — and the very next fetch retries the refresh. This also
/// covers another refresher of the account having marked it fresh meanwhile.
///
/// Authorization contract: this performs NO access control. It writes the
/// caller-supplied token into the secret variable at `path` and may mutate the
/// linked `account` row, so callers MUST have already authorized the operation
/// against `workspace_id`/`path` (the OAuth refresh adapters only run after the
/// read path has resolved and gated the variable). It is therefore kept
/// `pub(crate)` and intended solely for the in-crate refresh adapters.
#[cfg(feature = "oauth2")]
pub(crate) async fn store_oauth_token_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    token: &str,
) -> Result<()> {
    let persist = async {
        let value = store_managed_secret_value(db, workspace_id, path, token).await?;
        sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
            .bind(value)
            .bind(workspace_id)
            .bind(path)
            .execute(db)
            .await?;
        Ok::<(), Error>(())
    }
    .await;

    if let Err(e) = persist {
        // Mark the account expired again so the next fetch re-runs the refresh
        // instead of serving the now-stale token until it naturally expires. The
        // account id is the one linked from the variable being refreshed.
        let account_id: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT account FROM variable WHERE workspace_id = $1 AND path = $2",
        )
        .bind(workspace_id)
        .bind(path)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .flatten();

        if let Some(account_id) = account_id {
            if let Err(reset_err) = sqlx::query(
                "UPDATE account SET expires_at = now() - interval '1 minute', refresh_error = $1 \
                 WHERE workspace_id = $2 AND id = $3",
            )
            .bind(format!(
                "OAuth token was refreshed but persisting it to the secret backend failed: {e}"
            ))
            .bind(workspace_id)
            .bind(account_id)
            .execute(db)
            .await
            {
                tracing::error!(
                    workspace_id = %workspace_id,
                    account_id = %account_id,
                    "failed to reset account expiry after token persistence error: {reset_err}"
                );
            }
        }
        return Err(e);
    }

    Ok(())
}

/// Delete a secret from the configured backend (if using Vault)
///
/// For database backend: no-op (DB delete is handled separately)
/// For vault backend (stub): deletes from Vault
pub async fn delete_secret_from_backend(db: &DB, workspace_id: &str, path: &str) -> Result<()> {
    if is_vault_backend_configured(db).await? {
        let backend = get_secret_backend(db).await?;
        // Ignore NotFound errors during deletion (secret might not exist in Vault)
        match backend.delete_secret(workspace_id, path).await {
            Ok(()) => Ok(()),
            Err(Error::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        Ok(())
    }
}

/// Whether a secret's value is a marker naming where an external backend keeps it, so that
/// renaming the variable must move the stored secret too.
pub fn is_movable_external_value(value: &str) -> bool {
    is_gcp_sm_stored_value(value)
        || is_vault_stored_value(value)
        || is_azure_kv_stored_value(value)
        || is_aws_sm_stored_value(value)
}

/// SQL condition, on a `variable` row, matching [`is_movable_external_value`].
pub const MOVABLE_EXTERNAL_VALUE_SQL: &str = "(starts_with(value, '$gcp_sm:') \
     OR starts_with(value, '$vault:') OR starts_with(value, '$azure_kv:') \
     OR starts_with(value, '$aws_sm:'))";

/// Moves a Secret Manager secret to the name of `new_path` (specs/gcp-secret-manager.md §4).
async fn move_gcp_secret(
    db: &DB,
    workspace_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;
    if backend.backend_name() != windmill_common::secret_backend::gcp_secret_manager::BACKEND_NAME {
        return Err(Error::BadRequest(format!(
            "{old_path} is stored in Google Cloud Secret Manager, which is not the configured \
             secret backend, so it cannot be renamed"
        )));
    }
    let value = backend.get_secret(workspace_id, old_path).await?;
    backend.set_secret(workspace_id, new_path, &value).await?;
    backend.delete_secret(workspace_id, old_path).await?;
    Ok(gcp_sm_marker(new_path))
}

pub async fn rename_vault_secret(
    db: &DB,
    workspace_id: &str,
    old_path: &str,
    new_path: &str,
    current_value: &str,
) -> Result<Option<String>> {
    if is_http_store_stored_value(current_value) {
        return Err(http_store_rename_error(old_path));
    }
    if is_gcp_sm_stored_value(current_value) {
        return move_gcp_secret(db, workspace_id, old_path, new_path)
            .await
            .map(Some);
    }
    if is_vault_stored_value(current_value) {
        tracing::warn!(
            "Variable has $vault: prefix but Vault is not supported in Gantry. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$vault:{}", new_path)));
    }
    if is_azure_kv_stored_value(current_value) {
        tracing::warn!(
            "Variable has $azure_kv: prefix but Azure Key Vault is not supported in Gantry. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$azure_kv:{}", new_path)));
    }
    if is_aws_sm_stored_value(current_value) {
        tracing::warn!(
            "Variable has $aws_sm: prefix but AWS Secrets Manager is not supported in Gantry. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$aws_sm:{}", new_path)));
    }
    Ok(None)
}

/// Moves the externally stored secrets among `variables` (path, value) from under `old_prefix`
/// to under `new_prefix`, and answers the new value of each one moved, by old path. Only Secret
/// Manager secrets can move; the other backends are not supported.
pub async fn rename_vault_secrets_with_prefix(
    db: &DB,
    workspace_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    let mut moved = vec![];
    for (path, value) in variables {
        let Some(rest) = path.strip_prefix(old_prefix) else {
            continue;
        };
        if is_gcp_sm_stored_value(&value) {
            let new_path = format!("{new_prefix}{rest}");
            let new_value = move_gcp_secret(db, workspace_id, &path, &new_path).await?;
            moved.push((path, new_value));
        }
    }
    Ok(moved)
}
