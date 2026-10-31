//! Stub for the pipeline freshness watchdog. The active backstop —
//! re-running a `// freshness`-annotated producer whose output aged past its
//! window — is not implemented: the tick is a no-op. The passive fresh/stale
//! badge on the asset graph still works.

use windmill_common::DB;

pub async fn tick(_db: &DB) {}
