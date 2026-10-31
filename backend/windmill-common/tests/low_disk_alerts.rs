//! Regression test for the server-mode low-disk alert dedup tag.
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//! - Enterprise features enabled
//!
//! ## Running the tests
//!
//! ```bash
//! cargo test -p windmill-common --test low_disk_alerts --features private,enterprise -- --ignored --nocapture
//! ```
