# Gantry Clean-Room Specification: Google Cloud Secret Manager as the Secret Backend

Author: clean-room session (Claude)
Date: 2026-11-03
Sources:
- Google Cloud documentation: Secret Manager REST API
  (https://docs.cloud.google.com/secret-manager/docs/reference/rest/v1/projects.secrets.versions,
  https://docs.cloud.google.com/secret-manager/docs/reference/rest/v1/projects.secrets.versions/access,
  https://docs.cloud.google.com/secret-manager/docs/add-secret-version); service-account
  access tokens from the metadata server
  (https://docs.cloud.google.com/compute/docs/access/authenticate-workloads,
  https://docs.cloud.google.com/compute/docs/metadata/querying-metadata,
  https://docs.cloud.google.com/kubernetes-engine/docs/how-to/workload-identity,
  https://docs.cloud.google.com/run/docs/securing/service-identity)
- AGPL backend in this repository: `windmill-common/src/secret_backend/` (`SecretBackend`,
  `SecretBackendConfig`, `SecretMigrationReport`, the resolver, the database backend),
  `windmill-store/src/secret_backend_ext.rs` (writes, deletes, renames),
  `windmill-api-workspaces/src/workspaces.rs` (secrets copied into a fork),
  `windmill-api-settings/src/lib.rs`, `windmill-common/src/global_settings.rs`
  (`SECRET_BACKEND_SETTING`)
- The project owner's decision (2026-11-03): Gantry runs on Google Cloud only, and Secret
  Manager is the only external secret backend
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

Secret variables are encrypted in the database with a per-workspace key. The AGPL tree has the
seam for external backends: the `secret_backend` instance setting, a `SecretBackend` trait, and
a convention that a variable stored externally holds a marker `$<backend>:<path>` instead of
ciphertext. Every external backend (Vault, Azure Key Vault, AWS Secrets Manager, an HTTP store)
is a stub. Those stay stubs: Google Cloud Secret Manager is the one Gantry implements.

## 2. Configuration

The `secret_backend` instance setting (superadmin):

```json
{ "type": "GcpSecretManager", "project_id": "my-project", "prefix": "gantry-" }
```

- `project_id` (required): the project holding the secrets.
- `prefix` (optional): prepended to every secret id, so several instances can share a project
  and IAM conditions can match on the name.
- `endpoint` (optional): the API base URL, default `https://secretmanager.googleapis.com`;
  for tests.

**Authentication** is the service account of the workload, from the metadata server
(`/computeMetadata/v1/instance/service-accounts/default/token` with `Metadata-Flavor: Google`),
as on Compute Engine, GKE with Workload Identity, and Cloud Run. `GCE_METADATA_HOST` overrides
the metadata host, as Google's libraries do. Tokens are cached until a minute before they
expire. The account needs `roles/secretmanager.admin` on the project, or a role allowing
secret creation, version add/access/destroy, and secret deletion.

Local development keeps the database backend (no setting).

## 3. Where a secret lives

A variable `<path>` of workspace `<workspace>` is the secret
`<prefix><enc(workspace)>--<enc(path)>`, where `enc` keeps ASCII letters and digits and writes
every other character as `_` followed by its two hex digits (`/` is `_2f`, `_` is `_5f`, `-` is
`_2d`). The result only uses characters Secret Manager accepts, and since `enc` never produces
`--`, distinct variables never share a secret. A name longer than 255 characters is refused when
the variable is saved.

Each secret carries the annotations `gantry-workspace` and `gantry-path`, so it can be
recognised in the console. Replication is automatic.

## 4. Behaviour

- **Saving** a secret variable writes a new version of its secret (creating the secret if
  needed) and destroys the older versions, so a replaced value is gone; the variable stores the
  marker `$gcp_sm:<path>`.
- **Reading**: a `$gcp_sm:` value reads the latest version. Any other secret value is decrypted
  from the database as before, so secrets saved before the backend was configured keep working
  until migrated.
- **Deleting** a variable deletes its secret; a secret that is already gone is fine.
- **Renaming** a variable (or the folder holding it) copies the value to the new name's secret,
  deletes the old one, and stores the new marker.
- **Forking** a workspace copies each `$gcp_sm:` secret to the fork's names.
- If a `$gcp_sm:` value is read while the backend is not configured, the error says so.

## 5. Administration (superadmin)

- `POST /api/settings/test_gcp_secret_manager` with the settings: obtains a token and lists one
  secret of the project. Answers what failed.
- `POST /api/settings/migrate_secrets_to_gcp_secret_manager`: writes every database-encrypted
  secret to Secret Manager and replaces its value with the marker.
- `POST /api/settings/migrate_secrets_from_gcp_secret_manager`: the reverse for every `$gcp_sm:` secret,
  re-encrypting it in the database and deleting the Secret Manager secret.

Both migrations answer a `SecretMigrationReport` (totals and per-secret failures) and carry on
past a failed secret, which keeps its previous value.
