// Open-source build: Windmill's free AI tier does not exist. These stubs make the
// callers in `ai.rs` / `workspaces.rs` compile while disabling the feature entirely —
// `resolve_free_tier_credentials` never opts in, so the proxy falls through to its
// normal "AI resource not configured" path and the copilot stays hidden.
//
// Caller contract (enforced by the private impl, restated here for parity): the `email`
// passed to `resolve_free_tier_credentials` / `free_tier_copilot_config` MUST be the
// authenticated caller's own identity (an `ApiAuthed` email), never a client-supplied one —
// it selects whose lent-key grant is spent and whose usage is read.

use crate::ai::AIConfig;
use crate::db::DB;
use axum::body::Bytes;
use windmill_ai::ai_providers::AIProvider;
use windmill_ai::credentials::ProviderCredentials;
use windmill_common::error::Result;

pub struct FreeTierLease;

pub async fn resolve_free_tier_credentials(
    _provider: &AIProvider,
    _db: &DB,
    _ai_path: &str,
    _email: &str,
    _body: &Bytes,
) -> Result<Option<(ProviderCredentials, FreeTierLease)>> {
    Ok(None)
}

pub fn enforce_free_tier_body(body: &Bytes) -> Result<Bytes> {
    Ok(body.clone())
}

pub async fn free_tier_copilot_config(_db: &DB, _email: &str) -> Result<Option<AIConfig>> {
    Ok(None)
}

pub fn record_json_usage(_db: DB, _lease: FreeTierLease, _bytes: &[u8]) {}

pub fn meter_usage<S>(
    upstream: S,
    _db: DB,
    _lease: FreeTierLease,
) -> impl futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>>
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream
}
