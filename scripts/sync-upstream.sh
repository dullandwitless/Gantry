#!/usr/bin/env bash
# Brings an upstream Windmill release into Gantry without losing Gantry's own
# changes.
#
# Usage: scripts/sync-upstream.sh <path-to-upstream-checkout>
#
# The `upstream` branch only ever holds the output of import-upstream.sh, one
# commit per imported upstream release. This imports the new release onto that
# branch (in a temporary worktree, so main's checkout is untouched) and merges
# it into the current branch: only what changed upstream between the two
# imports is applied, and Gantry's edits conflict instead of being overwritten.
set -euo pipefail

upstream=${1:?usage: $0 <path-to-upstream-checkout>}
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: commit or stash your changes first" >&2
  exit 1
fi
git rev-parse --verify --quiet upstream >/dev/null || {
  echo "error: no 'upstream' branch; see PROVENANCE.md" >&2
  exit 1
}

worktree=$(mktemp -d)
trap 'git worktree remove --force "$worktree" >/dev/null 2>&1 || true' EXIT
git worktree add -q "$worktree" upstream

if ! "$repo_root/scripts/import-upstream.sh" "$upstream" "$worktree"; then
  echo "error: the import needs review (above); nothing was committed" >&2
  exit 1
fi

commit=$(git -C "$upstream" rev-parse --short HEAD)
git -C "$worktree" add -A
if git -C "$worktree" diff --cached --quiet; then
  echo "upstream $commit is already imported"
  exit 0
fi
git -C "$worktree" commit -q -m "Import upstream $commit"

if git merge --no-ff -m "Merge upstream $commit" upstream; then
  echo "Merged upstream $commit."
else
  echo "Merge of upstream $commit has conflicts: resolve them and commit." >&2
fi
cat <<EOF
Then regenerate what is derived from the sources and check the result:
  scripts/prepare-sqlx.sh
  (cd backend/windmill-api && ./build_openapi.sh)
  scripts/check-no-ee.sh
EOF
