#!/usr/bin/env bash
# Fails if any code gated on upstream's proprietary features remains.
# Run in CI on every change, and after every upstream import.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

pattern='feature *= *"(enterprise|private|enterprise_saml)"|"[A-Za-z0-9_-]+\??/(enterprise|private|enterprise_saml)"|^ *(enterprise|private|enterprise_saml) *='

failed=0
if hits=$(grep -rnE "$pattern" --include='*.rs' --include='Cargo.toml' backend); then
  echo "Proprietary feature gates remain:" >&2
  echo "$hits" >&2
  failed=1
fi
if hits=$(find backend -name '*_ee.rs' -not -path '*/target/*' | grep .); then
  echo "Proprietary *_ee.rs files present:" >&2
  echo "$hits" >&2
  failed=1
fi

if (( failed == 0 )); then
  echo "OK: no proprietary feature gates found"
fi
exit "$failed"
