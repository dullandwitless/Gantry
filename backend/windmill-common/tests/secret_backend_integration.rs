//! Integration tests for HashiCorp Vault secret backend.
//!
//! These tests require:
//! 1. A PostgreSQL database (handled by sqlx test framework)
//! 2. A running HashiCorp Vault instance
//!
//! They are `#[ignore]`d, so a plain `cargo test` skips them without creating a
//! database; pass `--ignored` to run them.
//!
//! Environment variables:
//! - VAULT_ADDR         - Vault server address (default: http://127.0.0.1:8200)
//! - VAULT_TOKEN        - Static token for static token tests (default: test-root-token)
//! - BASE_URL           - Windmill instance URL for JWT tests (default: http://localhost:8000)
//!
//! Run tests (static token mode):
//! ```bash
//! VAULT_TOKEN=your-token cargo test -p windmill-common \
//!     --test secret_backend_integration --features private,enterprise -- --ignored --nocapture
//! ```
//!
//! Run tests (JWT mode - requires Windmill instance running for JWKS endpoint):
//! ```bash
//! BASE_URL=http://localhost:8000 cargo test -p windmill-common \
//!     --test secret_backend_integration --features private,enterprise,openidconnect \
//!     -- --ignored --nocapture
//! ```

mod tests {
    #[test]
    fn test_vault_requires_enterprise() {
        println!("Vault integration tests require Enterprise Edition features");
        println!("Run with: cargo test --features private,enterprise,openidconnect");
    }
}
