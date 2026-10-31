//! Stubs for the fork deployment requests feature.
//!
//! Every call is a no-op, so anchored comments are never marked obsolete
//! after an item change. The `fork_deployment_requests` integration test sets
//! the flag via a raw SQL UPDATE to keep the lifecycle test meaningful.

use crate::{db::DB, error::Result};

pub async fn mark_anchor_obsolete(
    _db: &DB,
    _fork_workspace_id: &str,
    _anchor_kind: &str,
    _anchor_path: &str,
) -> Result<()> {
    Ok(())
}

pub async fn close_open_request_on_merge(
    _db: &DB,
    _source_workspace_id: &str,
    _fork_workspace_id: &str,
) -> Result<()> {
    Ok(())
}
