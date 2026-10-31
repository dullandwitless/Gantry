# Provenance and clean-room rules

Gantry must never contain code that Windmill Labs licenses under proprietary
terms. This file records how that is ensured. Keep it current: it is the
project's evidence if provenance is ever questioned.

## What upstream's license says

From upstream `LICENSE` (commit in `UPSTREAM_COMMIT`):

- `backend/` is AGPLv3, **except** code under the compile flag `enterprise`,
  which is proprietary.
- `frontend/` is AGPLv3, **except** code that requires a positive license check
  to activate, which is proprietary.
- Forks must not include any of the proprietary code.
- The clients (`python-client/`, `deno-client/`, `go-client/`,
  `powershell-client/`) and the OpenAPI/OpenFlow specs are Apache 2.0.
- Upstream's published binaries and Docker images contain proprietary code and
  are never used as a basis for Gantry.

Upstream keeps the bulk of its proprietary code in a private repository
(`windmill-ee-private`), swapped into the public tree as `*_ee.rs` files under
the `private` feature. The public tree contains AGPL stand-ins (`*_oss.rs`) and
the inline `enterprise`-gated snippets.

## How the backend is imported

`scripts/import-upstream.sh <upstream-checkout>`:

1. Refuses a checkout containing any `*_ee.rs` file.
2. Copies `backend/`, minus the paths listed in the script (EE tooling, the
   sqlx query cache, upstream contributor docs, one test that states it
   reproduces proprietary queries).
3. Runs `tools/cfg-strip` with `enterprise`, `private` and `enterprise_saml`
   disabled. It deletes, from source, every item, statement, field, variant,
   match arm, argument and attribute that those features gate, folds `cfg!`
   uses to constants, removes the features from every `Cargo.toml`, and drops
   optional dependencies only they enabled. It exits non-zero while anything
   needs manual review.
4. Records the upstream commit in `UPSTREAM_COMMIT`.

Files that carry a proprietary notice ("All Rights Reserved", "Unauthorized
copying ... prohibited") are treated as proprietary regardless of where they sit
in upstream's tree, and are never imported. Where other code depends on them,
Gantry provides its own implementation, written only from the compile errors
their absence causes (never from their contents). The four such files found so
far are listed in `scripts/import-upstream.sh`; the git history was rewritten
on 2026-10-31 so that it never contains them.

`scripts/check-no-ee.sh` verifies no gated code remains and runs in CI.
`scripts/prepare-sqlx.sh` regenerates `backend/.sqlx` from Gantry's own sources.

The repository history starts from the stripped tree; upstream git history,
which contains the proprietary snippets, is never merged.

## The frontend is clean-room

Gantry's UI is written from scratch. Nothing from upstream `frontend/` is
copied, ported or consulted.

- **Implementers** of the UI and of Gantry's enterprise-equivalent features
  must not have read upstream frontend code or any proprietary code (including
  the inline snippets `cfg-strip` removes). The person or session that runs an
  upstream import is "dirty" and does not implement those features.
- **Specs** are written from public material only: upstream's public docs,
  Gantry's own API (`backend/windmill-api/openapi.yaml`, Apache 2.0), and
  observed behaviour of a running Gantry instance. Specs live in `specs/`
  with author and date.
- **Data formats** that only upstream frontend code defines (e.g. app layouts)
  are specified from observed API payloads, never from source.
- Each clean-room contribution records who wrote it, from which specs, and
  attests to the above.

## Outstanding

- Comments in `backend/` still name a few proprietary modules (e.g.
  `oidc_ee::jwks`); they carry no code and will be cleaned up.
- Crates whose functionality was almost entirely proprietary (notably
  `windmill-indexer`) remain as near-empty shells until replaced.
- Names, binaries and branding still say "windmill" and must be renamed
  before any release (trademark).
