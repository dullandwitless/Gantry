//! Google Cloud Secret Manager as the secret backend.
//!
//! Clean-room implementation of `specs/gcp-secret-manager.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::{
    db::DB,
    error::{Error, Result},
    utils::HTTP_CLIENT,
    variables::{build_crypt, decrypt, encrypt},
};

use super::{SecretBackend, SecretMigrationFailure, SecretMigrationReport};

/// Marker a variable holds instead of ciphertext when its value lives in Secret Manager.
pub const GCP_SM_MARKER: &str = "$gcp_sm:";
pub const BACKEND_NAME: &str = "gcp_secret_manager";

const DEFAULT_ENDPOINT: &str = "https://secretmanager.googleapis.com";
const DEFAULT_METADATA_HOST: &str = "metadata.google.internal";
const MAX_SECRET_ID_LEN: usize = 255;
/// Tokens are renewed this long before they expire.
const TOKEN_MARGIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GcpSecretManagerSettings {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// API base URL, for tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

pub fn is_gcp_sm_stored_value(value: &str) -> bool {
    value.starts_with(GCP_SM_MARKER)
}

pub fn gcp_sm_marker(path: &str) -> String {
    format!("{GCP_SM_MARKER}{path}")
}

/// Keeps ASCII letters and digits; writes anything else as `_` and two hex digits per byte.
fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            out.push_str(&format!("_{b:02x}"));
        }
    }
    out
}

/// The secret id of a variable (spec §3). `--` never occurs in an encoded component, so the id
/// is unique per (workspace, path).
pub fn secret_id(prefix: Option<&str>, workspace_id: &str, path: &str) -> Result<String> {
    let id = format!(
        "{}{}--{}",
        prefix.unwrap_or_default(),
        encode_component(workspace_id),
        encode_component(path)
    );
    if id.len() > MAX_SECRET_ID_LEN {
        return Err(Error::BadRequest(format!(
            "{path} is too long to be stored in Secret Manager: its secret id would have {} \
             characters, the limit is {MAX_SECRET_ID_LEN}",
            id.len()
        )));
    }
    Ok(id)
}

pub struct GcpSecretManagerBackend {
    settings: GcpSecretManagerSettings,
}

impl GcpSecretManagerBackend {
    pub fn new(settings: GcpSecretManagerSettings) -> Self {
        Self { settings }
    }

    fn secret_id(&self, workspace_id: &str, path: &str) -> Result<String> {
        secret_id(self.settings.prefix.as_deref(), workspace_id, path)
    }

    fn url(&self, rest: &str) -> String {
        let endpoint = self
            .settings
            .endpoint
            .as_deref()
            .unwrap_or(DEFAULT_ENDPOINT)
            .trim_end_matches('/');
        format!("{endpoint}/v1/projects/{}/{rest}", self.settings.project_id)
    }

    /// Sends a request and answers the status with the JSON body, if any.
    async fn send(
        &self,
        method: Method,
        url: String,
        body: Option<Value>,
    ) -> Result<(StatusCode, Value)> {
        let mut req = HTTP_CLIENT
            .request(method, &url)
            .bearer_auth(access_token().await?);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let res = req
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Secret Manager request failed: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        Ok((
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        ))
    }

    /// Calls the API; `Ok(None)` for a 404, an error for any other failure.
    async fn call(
        &self,
        method: Method,
        url: String,
        body: Option<Value>,
    ) -> Result<Option<Value>> {
        let (status, body) = self.send(method, url, body).await?;
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(api_error(status, &body));
        }
        Ok(Some(body))
    }

    /// Lists one secret of the project, proving the token and its access (spec §5).
    pub async fn test_connection(&self) -> Result<()> {
        self.call(Method::GET, self.url("secrets?pageSize=1"), None)
            .await?
            .ok_or_else(|| {
                Error::BadRequest(format!("project {} not found", self.settings.project_id))
            })?;
        Ok(())
    }

    async fn create_secret_if_missing(
        &self,
        id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<()> {
        let body = json!({
            "replication": { "automatic": {} },
            "annotations": { "gantry-workspace": workspace_id, "gantry-path": path },
        });
        let (status, answer) = self
            .send(
                Method::POST,
                self.url(&format!("secrets?secretId={id}")),
                Some(body),
            )
            .await?;
        if status.is_success() || status == StatusCode::CONFLICT {
            Ok(())
        } else {
            Err(api_error(status, &answer))
        }
    }

    /// Destroys every enabled or disabled version of `id` other than `keep`.
    async fn destroy_versions_except(&self, id: &str, keep: &str) -> Result<()> {
        let listed = self
            .call(
                Method::GET,
                self.url(&format!(
                    "secrets/{id}/versions?filter=state:ENABLED OR state:DISABLED"
                )),
                None,
            )
            .await?
            .unwrap_or(Value::Null);
        let names = listed["versions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v["name"].as_str())
            .filter(|name| *name != keep);
        for name in names {
            let version = name.rsplit('/').next().unwrap_or_default();
            self.call(
                Method::POST,
                self.url(&format!("secrets/{id}/versions/{version}:destroy")),
                Some(json!({})),
            )
            .await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl SecretBackend for GcpSecretManagerBackend {
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String> {
        let id = self.secret_id(workspace_id, path)?;
        let version = self
            .call(
                Method::GET,
                self.url(&format!("secrets/{id}/versions/latest:access")),
                None,
            )
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "secret {path} of workspace {workspace_id} is not in Secret Manager ({id})"
                ))
            })?;
        let data = version["payload"]["data"].as_str().unwrap_or_default();
        let bytes = BASE64
            .decode(data)
            .map_err(|e| Error::internal_err(format!("Secret Manager payload of {id}: {e}")))?;
        String::from_utf8(bytes)
            .map_err(|e| Error::internal_err(format!("Secret Manager payload of {id}: {e}")))
    }

    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()> {
        let id = self.secret_id(workspace_id, path)?;
        self.create_secret_if_missing(&id, workspace_id, path)
            .await?;
        let added = self
            .call(
                Method::POST,
                self.url(&format!("secrets/{id}:addVersion")),
                Some(json!({ "payload": { "data": BASE64.encode(value) } })),
            )
            .await?
            .ok_or_else(|| Error::internal_err(format!("secret {id} disappeared while saving")))?;
        let new_version = added["name"].as_str().unwrap_or_default().to_string();
        // A replaced value must not stay readable (spec §4).
        self.destroy_versions_except(&id, &new_version).await
    }

    async fn delete_secret(&self, workspace_id: &str, path: &str) -> Result<()> {
        let id = self.secret_id(workspace_id, path)?;
        self.call(Method::DELETE, self.url(&format!("secrets/{id}")), None)
            .await?;
        Ok(())
    }

    fn check_path(&self, path: &str) -> Result<()> {
        // The workspace id is not known here; check the longest a path can make the id.
        secret_id(self.settings.prefix.as_deref(), &"w".repeat(50), path).map(|_| ())
    }

    fn backend_name(&self) -> &'static str {
        BACKEND_NAME
    }
}

fn api_error(status: StatusCode, body: &Value) -> Error {
    let message = body["error"]["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string());
    Error::internal_err(format!("Secret Manager answered {status}: {message}"))
}

struct CachedToken {
    token: String,
    renew_at: Instant,
}

lazy_static::lazy_static! {
    static ref TOKEN: Mutex<Option<CachedToken>> = Mutex::new(None);
}

/// The workload's service-account token from the metadata server (spec §2).
async fn access_token() -> Result<String> {
    let mut cached = TOKEN.lock().await;
    if let Some(t) = cached.as_ref().filter(|t| Instant::now() < t.renew_at) {
        return Ok(t.token.clone());
    }
    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        expires_in: u64,
    }
    let host = std::env::var("GCE_METADATA_HOST")
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| DEFAULT_METADATA_HOST.to_string());
    let res: TokenResponse = HTTP_CLIENT
        .get(format!(
            "http://{host}/computeMetadata/v1/instance/service-accounts/default/token"
        ))
        .header("Metadata-Flavor", "Google")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| {
            Error::internal_err(format!(
                "could not get a Google Cloud access token from the metadata server ({host}): {e}"
            ))
        })?
        .json()
        .await
        .map_err(|e| Error::internal_err(format!("unreadable metadata server token: {e}")))?;
    let lifetime = Duration::from_secs(res.expires_in).saturating_sub(TOKEN_MARGIN);
    *cached =
        Some(CachedToken { token: res.access_token.clone(), renew_at: Instant::now() + lifetime });
    Ok(res.access_token)
}

/// Moves every database-encrypted secret into Secret Manager (spec §5).
pub async fn migrate_secrets_to_gcp_secret_manager(
    db: &DB,
    settings: &GcpSecretManagerSettings,
) -> Result<SecretMigrationReport> {
    let backend = GcpSecretManagerBackend::new(settings.clone());
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT workspace_id, path, value FROM variable
         WHERE is_secret AND value != '' AND value NOT LIKE '$%'
         ORDER BY workspace_id, path",
    )
    .fetch_all(db)
    .await?;
    let mut report = report_for(rows.len());
    for (workspace_id, path, ciphertext) in rows {
        let moved = async {
            let mc = build_crypt(db, &workspace_id).await?;
            let plain = decrypt(&mc, ciphertext).map_err(|e| Error::internal_err(e.to_string()))?;
            backend.set_secret(&workspace_id, &path, &plain).await?;
            sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
                .bind(gcp_sm_marker(&path))
                .bind(&workspace_id)
                .bind(&path)
                .execute(db)
                .await?;
            Ok::<(), Error>(())
        }
        .await;
        record(&mut report, workspace_id, path, moved);
    }
    Ok(report)
}

/// Moves every Secret Manager secret back into the database, encrypted (spec §5).
pub async fn migrate_secrets_from_gcp_secret_manager(
    db: &DB,
    settings: &GcpSecretManagerSettings,
) -> Result<SecretMigrationReport> {
    let backend = GcpSecretManagerBackend::new(settings.clone());
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT workspace_id, path FROM variable
         WHERE is_secret AND starts_with(value, $1) ORDER BY workspace_id, path",
    )
    .bind(GCP_SM_MARKER)
    .fetch_all(db)
    .await?;
    let mut report = report_for(rows.len());
    for (workspace_id, path) in rows {
        let moved = async {
            let plain = backend.get_secret(&workspace_id, &path).await?;
            let mc = build_crypt(db, &workspace_id).await?;
            sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
                .bind(encrypt(&mc, &plain))
                .bind(&workspace_id)
                .bind(&path)
                .execute(db)
                .await?;
            if let Err(e) = backend.delete_secret(&workspace_id, &path).await {
                tracing::warn!(
                    "{workspace_id}/{path} is back in the database, but its Secret Manager secret \
                     could not be deleted: {e:#}"
                );
            }
            Ok::<(), Error>(())
        }
        .await;
        record(&mut report, workspace_id, path, moved);
    }
    Ok(report)
}

fn report_for(total: usize) -> SecretMigrationReport {
    SecretMigrationReport {
        total_secrets: total,
        migrated_count: 0,
        failed_count: 0,
        failures: vec![],
    }
}

fn record(report: &mut SecretMigrationReport, workspace_id: String, path: String, r: Result<()>) {
    match r {
        Ok(()) => report.migrated_count += 1,
        Err(e) => {
            report.failed_count += 1;
            report.failures.push(SecretMigrationFailure {
                workspace_id,
                path,
                error: e.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_ids_are_valid_and_distinct() {
        let id = secret_id(Some("gantry-"), "main", "u/alice/db_password").unwrap();
        assert_eq!(id, "gantry-main--u_2falice_2fdb_5fpassword");
        assert!(id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));

        // Paths that a naive `/` -> `_` mapping would merge stay apart.
        let a = secret_id(None, "w", "f/a_b").unwrap();
        let b = secret_id(None, "w", "f/a/b").unwrap();
        assert_ne!(a, b);
        // So do workspace/path splits that a plain separator would merge.
        assert_ne!(
            secret_id(None, "a-b", "c").unwrap(),
            secret_id(None, "a", "b-c").unwrap()
        );
    }

    #[test]
    fn overlong_names_are_refused() {
        assert!(secret_id(None, "main", &"x".repeat(300)).is_err());
    }

    #[test]
    fn markers_round_trip() {
        assert!(is_gcp_sm_stored_value(&gcp_sm_marker("u/alice/db")));
        assert!(!is_gcp_sm_stored_value("bm90LWEtbWFya2Vy"));
    }
}
