/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Resolution of the configured secret backend.
//!
//! Lives in `windmill-common` (rather than the API/store crates) so that
//! lower-level helpers such as [`crate::variables::get_variable_or_self`] can
//! route secret reads through the configured backend. With an external backend
//! (Vault / Azure Key Vault / AWS Secrets Manager / HTTP store), the `variable.value` column
//! holds a `$vault:`/`$azure_kv:`/`$aws_sm:`/`$http_store:` marker rather than base64
//! ciphertext, so decrypting it directly fails — reads must go through the
//! backend instead.
//!
//! Note: external backends require Enterprise Edition. The OSS version only
//! supports the database backend.

use std::sync::Arc;

use crate::{
    db::DB,
    error::{Error, Result},
    secret_backend::{database::DatabaseBackend, SecretBackend},
    variables::{build_crypt, decrypt},
};

pub async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    Ok(Arc::new(DatabaseBackend::new(db.clone())))
}

pub async fn is_vault_backend_configured(_db: &DB) -> Result<bool> {
    Ok(false)
}

/// Get a secret value using the configured backend
///
/// For database backend: decrypts `encrypted_value` using the workspace key
/// For external backends (EE only): fetches from the backend at `path`,
/// ignoring `encrypted_value` (which holds only a `$...:` marker)
pub async fn get_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    encrypted_value: &str,
) -> Result<String> {
    if super::is_external_ref_value(encrypted_value) {
        return super::resolve_external_ref(db, workspace_id, path, encrypted_value).await;
    }
    let backend = get_secret_backend(db).await?;

    match backend.backend_name() {
        "database" => {
            // Use existing database decryption
            let mc = build_crypt(db, workspace_id).await?;
            decrypt(&mc, encrypted_value.to_string()).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
            })
        }
        "hashicorp_vault" => {
            // Fetch from Vault directly
            backend.get_secret(workspace_id, path).await
        }
        "azure_key_vault" => backend.get_secret(workspace_id, path).await,
        "aws_secrets_manager" => backend.get_secret(workspace_id, path).await,
        // The store is read-only and has no migrate-to, so secrets that were encrypted in the
        // database before it was configured stay there and keep decrypting.
        "http_store" if is_http_store_stored_value(encrypted_value) => {
            backend.get_secret(workspace_id, path).await
        }
        "http_store" => {
            let mc = build_crypt(db, workspace_id).await?;
            decrypt(&mc, encrypted_value.to_string()).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
            })
        }
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Check if a value is stored in Vault (indicated by the $vault: prefix)
pub fn is_vault_stored_value(value: &str) -> bool {
    value.starts_with("$vault:")
}

/// Check if a value is stored in Azure Key Vault (indicated by the $azure_kv: prefix)
pub fn is_azure_kv_stored_value(value: &str) -> bool {
    value.starts_with("$azure_kv:")
}

/// Check if a value is stored in AWS Secrets Manager (indicated by the $aws_sm: prefix)
pub fn is_aws_sm_stored_value(value: &str) -> bool {
    value.starts_with("$aws_sm:")
}

/// Check if a value is a reference to a key of the HTTP secret store (the $http_store: prefix)
pub fn is_http_store_stored_value(value: &str) -> bool {
    value.starts_with("$http_store:")
}

/// Check if a value is stored in any external secret backend
pub fn is_external_stored_value(value: &str) -> bool {
    is_vault_stored_value(value)
        || is_azure_kv_stored_value(value)
        || is_aws_sm_stored_value(value)
        || is_http_store_stored_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_markers_are_detected() {
        assert!(is_external_stored_value("$vault:u/admin/secret"));
        assert!(is_external_stored_value("$azure_kv:u/admin/secret"));
        assert!(is_external_stored_value("$aws_sm:u/admin/secret"));
        assert!(is_external_stored_value("$http_store:u/admin/secret"));
    }

    #[test]
    fn base64_ciphertext_is_not_treated_as_external() {
        // A base64 magic_crypt blob must route through `decrypt`, never the
        // external backend. The leading `$` is what distinguishes a marker from
        // ciphertext; decrypting a marker fails with "Invalid byte 36" (`$`),
        // which is the bug this gate prevents.
        for v in [
            "bm90LWEtbWFya2Vy",
            "AAAA1234+/abcd==",
            "",
            "$something_else",
        ] {
            assert!(!is_external_stored_value(v), "unexpected external: {v:?}");
        }
    }
}
