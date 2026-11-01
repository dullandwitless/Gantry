#!/usr/bin/env bash
# Regenerates backend/.sqlx (sqlx's offline query cache) from Gantry's own
# sources against a throwaway local Postgres, so the cache only ever contains
# queries that exist in this repository.
#
# Requires: Postgres server binaries (initdb, pg_ctl) and sqlx-cli
#   cargo install sqlx-cli --no-default-features --features postgres,rustls
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
backend="$repo_root/backend"
features=${SQLX_FEATURES:-all_sqlx_features}
port=${PGPORT:-55432}

pg_bin=${PG_BIN:-$(dirname "$(command -v pg_ctl 2>/dev/null || ls -d /usr/lib/postgresql/*/bin/pg_ctl | tail -1)")}
datadir=$(mktemp -d)

run_pg() {
  # Postgres refuses to run as root.
  if [[ $EUID -eq 0 ]]; then
    chown -R postgres "$datadir"
    runuser -u postgres -- "$@"
  else
    "$@"
  fi
}

cleanup() {
  run_pg "$pg_bin/pg_ctl" -D "$datadir" -m immediate stop >/dev/null 2>&1 || true
  rm -rf "$datadir"
}
trap cleanup EXIT

run_pg "$pg_bin/initdb" -D "$datadir" -U postgres --auth=trust >/dev/null
run_pg "$pg_bin/pg_ctl" -D "$datadir" -o "-p $port -k /tmp" -l "$datadir/log" -w start >/dev/null

export DATABASE_URL="postgres://postgres@localhost:$port/gantry"
export SQLX_OFFLINE=false
# A one-off full build; incremental artifacts would only fill the disk.
export CARGO_INCREMENTAL=0
cd "$backend"
sqlx database create
sqlx migrate run --source migrations
rm -rf .sqlx
cargo sqlx prepare --workspace -- --workspace --all-targets --features "$features"
echo "Regenerated $(ls .sqlx | wc -l) query files in backend/.sqlx"
