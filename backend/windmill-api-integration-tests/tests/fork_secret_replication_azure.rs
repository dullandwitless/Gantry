//! End-to-end regression test for WIN-2161.
//!
//! Reproduces, through real product code, the state after a database-to-external
//! migration: a secret that was created under the database backend and then
//! *migrated* to an external backend (Azure Key Vault). Migration writes the
//! plaintext to the store but
//! leaves the encrypted ciphertext in `variable.value` (it never rewrites it to
//! a `$azure_kv:` marker). The bug: `clone_variables` only replicated
//! marker-valued secrets, so forking left the migrated secret unreplicated and
//! reads in the fork failed with "not found in Azure Key Vault".
//!
//! This drives the real `/migrate_secrets_to_azure_kv`, `/create_fork` and
//! `variables/get_value` endpoints against a local Azure Key Vault emulator
//! (lowkey-vault), which the `AzureKeyVaultBackend` talks to via its
//! static-token / self-signed-cert emulator mode.
//!
//! The test is `#[ignore]`d, so a plain `cargo test` skips it without creating
//! a database. Run it with `--ignored` (`AZURE_KV_URL` overrides the default
//! https://localhost:8443):
//! ```bash
//! podman run -d --name lowkey -p 8443:8443 \
//!     -e LOWKEY_ARGS="--LOWKEY_VAULT_NAMES=default" \
//!     docker.io/nagyesta/lowkey-vault:7.3.0
//!
//! cargo test -p windmill-api-integration-tests \
//!     --test fork_secret_replication_azure -- --ignored --nocapture
//! ```
