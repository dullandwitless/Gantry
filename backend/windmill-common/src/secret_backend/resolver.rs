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
//! Note: the database and Google Cloud Secret Manager backends are implemented; the other
//! external backends are stubs.

use std::sync::Arc;

use crate::{
    db::DB,
    error::{Error, Result},
    global_settings::{load_value_from_global_settings, SECRET_BACKEND_SETTING},
    secret_backend::{
        create_secret_backend, gcp_secret_manager, GcpSecretManagerBackend, SecretBackend,
        SecretBackendConfig,
    },
    variables::{build_crypt, decrypt},
};

/// The `secret_backend` instance setting; the database when unset.
pub async fn secret_backend_config(db: &DB) -> Result<SecretBackendConfig> {
    match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        None | Some(serde_json::Value::Null) => Ok(SecretBackendConfig::Database),
        Some(v) => serde_json::from_value(v).map_err(|e| {
            Error::internal_err(format!("invalid {SECRET_BACKEND_SETTING} setting: {e}"))
        }),
    }
}

pub async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    create_secret_backend(db.clone(), &secret_backend_config(db).await?).await
}

/// Whether secrets are written somewhere other than the database.
pub async fn is_vault_backend_configured(db: &DB) -> Result<bool> {
    Ok(get_secret_backend(db).await?.backend_name() != "database")
}

/// The Secret Manager backend a `$gcp_sm:` value needs, which must be the configured one.
async fn gcp_secret_manager_backend(db: &DB, path: &str) -> Result<GcpSecretManagerBackend> {
    match secret_backend_config(db).await? {
        SecretBackendConfig::GcpSecretManager(settings) => {
            Ok(GcpSecretManagerBackend::new(settings))
        }
        _ => Err(Error::internal_err(format!(
            "{path} is stored in Google Cloud Secret Manager, which is not the configured secret \
             backend: configure it again, or restore the secret"
        ))),
    }
}

/// Get a secret value using the configured backend
///
/// For database backend: decrypts `encrypted_value` using the workspace key
/// For external backends (stubs here): fetches from the backend at `path`,
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
    // Wherever secrets are written now, a marked one lives in Secret Manager and any other is
    // database ciphertext, so switching backends never strands a secret.
    if gcp_secret_manager::is_gcp_sm_stored_value(encrypted_value) {
        return gcp_secret_manager_backend(db, path)
            .await?
            .get_secret(workspace_id, path)
            .await;
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
        // Not marked, so saved before Secret Manager was configured and still in the database.
        gcp_secret_manager::BACKEND_NAME => {
            let mc = build_crypt(db, workspace_id).await?;
            decrypt(&mc, encrypted_value.to_string()).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
            })
        }
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
    gcp_secret_manager::is_gcp_sm_stored_value(value)
        || is_vault_stored_value(value)
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
        assert!(is_external_stored_value("$gcp_sm:u/admin/secret"));
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
