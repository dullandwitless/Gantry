//! Stubs for advanced pipeline features: partition backfill is refused, and
//! materialization-plan assembly runs the plan verbatim — dbt-like
//! commit-then-test, with no write-audit-publish. (Freshness lives elsewhere: the fresh/stale
//! badge is CE in the assets API, the active watchdog is windmill-queue's
//! `freshness_watchdog`.)

use crate::error::Error;

pub fn backfill_todo() -> Error {
    Error::internal_err("Pipeline partition backfill requires the enterprise edition".to_string())
}

/// Assemble a materialization plan into the statement list the DuckDB executor
/// runs. The public build executes the plan verbatim — dbt-like
/// commit-then-test: a failing `// data_test` still fails the run and stops
/// the cascade, but the written slice stays live.
pub fn finalize_materialize_query(
    plan: windmill_parser::sql_materialize::MaterializePlan,
    _asset_path: &str,
) -> Vec<String> {
    plan.stmts.into_iter().map(|s| s.sql).collect()
}
