#!/usr/bin/env bash
# Imports the AGPLv3 backend from an upstream Windmill checkout, removing every
# piece of code the upstream LICENSE marks as proprietary (anything gated on the
# "enterprise"/"private" Cargo features).
#
# Usage: scripts/import-upstream.sh <path-to-upstream-checkout> <target-root>
#
# <target-root> is a checkout of the `upstream` branch; its backend/ is
# replaced wholesale, so never point it at a checkout of main. To bring an
# upstream release into Gantry, use scripts/sync-upstream.sh, which runs this
# and merges the result.
#
# The frontend is deliberately NOT imported: Gantry's UI is a clean-room
# implementation. Never copy anything from upstream frontend/ into this repo.
set -euo pipefail

usage="usage: $0 <path-to-upstream-checkout> <target-root>"
upstream=${1:?$usage}
target=$(cd "${2:?$usage}" && pwd)
# Tools and patches come from this script's own checkout, not the target's.
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
backend="$target/backend"

# Features whose code is proprietary under upstream's LICENSE.
disabled_features=(enterprise private enterprise_saml)

# Paths (relative to backend/) not imported:
not_imported=(
  # Tooling that only exists to manage the proprietary code.
  substitute_ee_code.sh
  write_latest_ee_ref.sh
  ee-repo-ref.txt
  check_no_symlink.sh
  # sqlx's offline query cache holds the SQL text of proprietary queries.
  # It is regenerated from the stripped sources (scripts/prepare-sqlx.sh).
  .sqlx
  # Upstream contributor instructions built around the proprietary workflow.
  AGENTS.md
  CLAUDE.md
  run_until_fail.sh
  update_sqlx.sh
  # States that it reproduces queries from the proprietary repository.
  windmill-queue/tests/native_retry_test.rs
  # Carry "All Rights Reserved" proprietary headers despite their location;
  # replaced by Gantry's own windmill-common/src/secret_backend/http_store.rs
  # (patches/0003).
  windmill-common/src/secret_backend/aws_sm_oss.rs
  windmill-common/src/secret_backend/azure_kv_oss.rs
  windmill-common/src/secret_backend/http_store_oss.rs
  windmill-common/tests/azure_kv_integration.rs
)

if [[ ! -f "$upstream/LICENSE" || ! -d "$upstream/backend" ]]; then
  echo "error: $upstream does not look like an upstream checkout" >&2
  exit 1
fi
if find "$upstream/backend" -name '*_ee.rs' -not -path '*/target/*' | grep -q .; then
  echo "error: upstream contains *_ee.rs files (proprietary); use a clean checkout" >&2
  exit 1
fi

commit=$(git -C "$upstream" rev-parse HEAD)

rm -rf "$backend"
cp -r "$upstream/backend" "$backend"
rm -rf "$backend/target"
for path in "${not_imported[@]}"; do rm -rf "${backend:?}/$path"; done

cp "$upstream/openflow.openapi.yaml" "$target/openflow.openapi.yaml"

# Test data outside backend/ that backend tests include by relative path.
shared_fixtures=(cli/test/fixtures/item_digest_vectors.json)
for path in "${shared_fixtures[@]}"; do
  mkdir -p "$target/$(dirname "$path")"
  cp "$upstream/$path" "$target/$path"
done

cargo build --release --quiet --manifest-path "$repo_root/tools/cfg-strip/Cargo.toml"
strip_args=()
for f in "${disabled_features[@]}"; do strip_args+=(--disable "$f"); done

# Exits non-zero while anything still needs manual review.
status=0
"$repo_root/tools/cfg-strip/target/release/cfg-strip" "${strip_args[@]}" "$backend" || status=$?

# User-visible product name and feature-gate messages (see tools/rebrand.py).
python3 "$repo_root/tools/rebrand.py" "$backend"

# Fix-ups the stripper cannot make (e.g. tests that read upstream frontend
# files, which Gantry never imports). Each must apply cleanly.
for patch in "$repo_root"/patches/*.patch; do
  [[ -e "$patch" ]] || continue
  git -C "$target" apply --whitespace=nowarn "$patch"
done

echo "$commit" > "$target/UPSTREAM_COMMIT"
echo "Imported upstream $commit"
if (( status != 0 )); then
  echo "Review the locations above before merging" >&2
fi
exit "$status"
