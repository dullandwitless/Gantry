//! Regression test for the server-mode low-disk alert dedup tag.
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//!
//! ## Running the tests
//!
//! ```bash
//! cargo test -p windmill-common --test low_disk_alerts -- --ignored --nocapture
//! ```
