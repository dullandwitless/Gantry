# Gantry

An open-source developer platform for scripts, workflows and internal tools,
built on the AGPLv3 parts of [Windmill](https://github.com/windmill-labs/windmill)
with a clean-room frontend and clean-room implementations of its
enterprise features.

Status: early. The backend is imported, stripped and passing its test suite.
A first clean-room frontend (React, under `frontend/`) signs in, creates
workspaces and lists their contents against a live backend; see
`docs/issues/` for what is still broken and `specs/not-implemented.md` for
what the backend lacks.

## Layout

| Path | What |
| --- | --- |
| `backend/` | Rust backend, imported from upstream with proprietary code removed (AGPLv3) |
| `frontend/` | Clean-room web UI (React, Vite, Tailwind); `frontend/build/` is what the backend serves |
| `specs/` | Specs the frontend is written from, and the backlog of unimplemented features |
| `openflow.openapi.yaml` | OpenFlow spec (Apache 2.0) |
| `tools/cfg-strip/` | Removes feature-gated code from Rust sources and manifests |
| `scripts/sync-upstream.sh` | Merges a new upstream release (via the `upstream` branch) |
| `scripts/import-upstream.sh` | The import pipeline the sync runs |
| `scripts/check-no-ee.sh` | Fails if any proprietary feature gate remains (CI) |
| `scripts/prepare-sqlx.sh` | Regenerates the sqlx offline query cache |

Read [PROVENANCE.md](PROVENANCE.md) before contributing: the frontend and the
enterprise features are clean-room, and the rules there are not optional.
Frontend work starts from [specs/README.md](specs/README.md).

## License

AGPLv3 (`LICENSE-AGPL`), with the OpenAPI/OpenFlow specs under Apache 2.0
(`LICENSE-APACHE`). See [NOTICE](NOTICE).
