//! Integration tests for AWS Secrets Manager secret backend.
//!
//! These tests require a running LocalStack instance with the `secretsmanager` service.
//!
//! ## Setup
//!
//! 1. Start LocalStack:
//!
//!    ```bash
//!    docker run -d --name localstack -p 4566:4566 \
//!      -e SERVICES=secretsmanager \
//!      localstack/localstack:3.8
//!    ```
//!
//! 2. Run the tests (they are `#[ignore]`d, so a plain `cargo test` skips them):
//!
//!    ```bash
//!    cargo test -p windmill-common \
//!      --test aws_sm_integration -- --ignored --nocapture
//!    ```
//!
//! ## Environment variables
//!
//! - `AWS_SM_ENDPOINT`        - LocalStack endpoint (default: http://localhost:4566)
//! - `AWS_SM_REGION`          - AWS region (default: us-east-1)

mod tests {
    #[test]
    fn test_aws_sm_requires_enterprise() {
        println!("AWS Secrets Manager integration tests require Enterprise Edition features");
        println!("Run with: cargo test -p windmill-common --features private,enterprise");
    }
}
