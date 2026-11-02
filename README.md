# Gantry

An open-source developer platform for scripts, workflows and internal tools,
built on the AGPLv3 parts of [Windmill](https://github.com/windmill-labs/windmill)
with a clean-room frontend and clean-room implementations of its
enterprise features.

Status: early. The backend is imported, stripped and passing its test suite;
the frontend has not been started.

## Layout

| Path | What |
| --- | --- |
| `backend/` | Rust backend, imported from upstream with proprietary code removed (AGPLv3) |
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
