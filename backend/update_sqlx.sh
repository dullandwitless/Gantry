#!/usr/bin/env bash

set -e

# Check if running on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    # Run cargo sqlx prepare with deno_core_mac
    echo "Running cargo sqlx prepare with deno_core_mac..."
    cargo sqlx prepare --workspace -- --workspace --all-targets --features all_sqlx_features,deno_core_mac,deno_core,mcp
else
    # Run cargo sqlx prepare
    echo "Running cargo sqlx prepare..."
    cargo sqlx prepare --workspace -- --workspace --all-targets --features all_sqlx_features,ee,deno_core,mcp
fi
