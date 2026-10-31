//! Tests for debouncing logic: both normal (push-time) and post-preprocessing debouncing.
//!
//! Run with:
//!   cargo test -p windmill-queue --test debounce_test --features private,enterprise -- --nocapture
//!
//! Requires a live database (migrations are applied automatically by sqlx::test).
