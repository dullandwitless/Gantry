//! OSS stubs for the fork deployment requests feature. The EE build
//! overrides these via `deployment_requests_ee.rs` when the `private`
//! feature is active.
//!
//! These helpers are called from fork item-change hooks in EE code (see
//! `windmill-git-sync/src/git_sync_ee.rs::tally_deployed_object_changes`).
//! The stub exists so code paths that wire the hook compile on OSS even
//! though the obsolescence behavior itself is EE-only.
//!
//! On OSS the call is a no-op, so anchored-comment obsolescence after an
//! item change only actually fires when compiled with `--features private`.
//! The `fork_deployment_requests` integration test simulates the EE effect
//! via a raw SQL UPDATE to keep the lifecycle test meaningful on CE builds.

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
