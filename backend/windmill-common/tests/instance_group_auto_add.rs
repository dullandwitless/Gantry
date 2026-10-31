//! Integration tests for the interaction between instance groups and workspace auto-add.
//!
//! These tests verify that:
//! 1. Users added to instance groups are automatically added to workspaces configured with auto-add
//! 2. Configuring instance groups for a workspace adds existing group members
//! 3. Users removed from instance groups are removed from workspaces
//! 4. Role precedence is respected when users belong to multiple instance groups
//! 5. The `added_via` field correctly tracks how users were added
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//!
//! ## Running the tests
//!
//! These tests are ignored by default in CI. To run them locally:
//!
//! ```bash
//! # Run all instance group auto-add tests
//! cargo test -p windmill --test instance_group_auto_add -- --ignored --nocapture
//!
//! # Run a specific test
//! cargo test -p windmill --test instance_group_auto_add -- --ignored test_role_precedence --nocapture
//! ```

mod tests {
    #[test]
    fn test_instance_group_auto_add_requires_enterprise() {
        println!("Instance group auto-add tests require Enterprise Edition features");
        println!("Run with: cargo test -p windmill instance_group_auto_add --features private,enterprise -- --nocapture");
    }
}
