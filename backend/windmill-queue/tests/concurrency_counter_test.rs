//! Tests for `update_concurrency_counter` — the per-script concurrency limiter
//! at the heart of `apply_concurrency_limit` (windmill-queue/jobs_ee.rs).
//!
//! The structural property under test is the **never-over-admit invariant**:
//! across N concurrent admission attempts on a single concurrency_id with
//! limit L, the number of admits must never exceed L.
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_counter_test \
//!       --features private,enterprise -- --nocapture
