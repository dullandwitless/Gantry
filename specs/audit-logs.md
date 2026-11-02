# Gantry Clean-Room Specification: Audit Logs

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml`: `GET /w/{workspace}/audit/list` (`listAuditLogs`),
  `GET /w/{workspace}/audit/get/{id}`, schema `AuditLog`, parameters `Before`,
  `After`, `Username`, `Operation`, `ResourceName`, `ActionKind`
- AGPL backend in this repository: `windmill-audit/src/lib.rs` (`ActionKind`,
  `AuditLog`, `ListAuditLogQuery`), the `AuditExecutor` and `AuditAuthorable`
  traits in `windmill-audit/src/audit_oss.rs`, `windmill-api/src/audit.rs`,
  `windmill-api-auth` (`username_override` is derived from a token label),
  `windmill-common/src/db.rs` (`UserDB::begin`), migrations
  `20260311100000_audit_partitioning` and `20261022000335_audit_insert_policy_windmill_user`,
  `src/monitor.rs` (`manage_audit_partitions`, retention)
- Windmill public documentation: https://www.windmill.dev/docs/core_concepts/audit_logs
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

The AGPL tree creates `audit_partitioned` (daily range partitions on `timestamp`),
keeps partitions ahead of time and drops old ones (retention, default 14 days on
Community), and restricts reads with row-level security: `windmill_admin` sees every
row, `windmill_user` sees rows whose `username` is its session user. Only writing
and reading rows is missing.

## 2. Writing — `audit_log`, `audit_log_many`

One row per action (one per resource for `audit_log_many`), into `audit_partitioned`:

| column | value |
| --- | --- |
| `workspace_id` | the `w_id` argument (`global` for instance-level actions) |
| `username` | the author's `username()` |
| `email` | the author's `email()` |
| `operation` | the operation name, e.g. `users.create` |
| `action_kind` | `ActionKind` |
| `resource` | the resource, if any |
| `parameters` | the parameters as a JSON object of strings, if any (see below) |
| `span` | not recorded |
| `id`, `timestamp` | database defaults |

- **Identity is never taken from a token label.** `username_override` is derived
  from a token's label, which its creator chooses freely, so it is not the row's
  `username`. When present and different from the username it is recorded as the
  parameter `username_override`. A token prefix, when present, is recorded as
  `token_prefix` so an action can be traced to the token that made it.
- Values longer than their column (`operation` 50, `resource`/`username`/`email`
  255 characters) are truncated on a character boundary. An audit row must never
  fail its caller's action for being long.
- Failure handling follows `AuditExecutor::ISOLATED`: on a pool connection a
  failed insert is logged and the action goes on; on a connection that may hold the
  caller's transaction the error is returned, because that transaction is aborted.

## 3. Reading — `GET /w/{workspace}/audit/list`

Runs in the caller's `UserDB` transaction, so RLS applies: workspace admins (and
superadmins) see all rows, other users only their own. That matches the public
docs ("regular users can only see their own audit logs").

Filters, all optional and combined with AND:

| parameter | meaning |
| --- | --- |
| `username` | exact username |
| `operation` | operation equal to, or starting with, the value |
| `operations` | comma-separated exact operations to include |
| `exclude_operations` | comma-separated exact operations to exclude |
| `resource` | resource equal to, or starting with, the value |
| `action_kind` | `Create`, `Update`, `Delete` or `Execute`, case-insensitive; anything else is 400 |
| `before` | `timestamp <= before` |
| `after` | `timestamp > after` |
| `before_id` | `id < before_id` (keyset cursor) |
| `all_workspaces` | every workspace instead of `{workspace}`; superadmins only (401 otherwise), since RLS alone would let any workspace admin read every workspace |

Prefix filters treat `%`, `_` and `\` in the value literally. Rows are ordered by
`id` descending and paginated with `page`/`per_page`.

`GET /w/{workspace}/audit/get/{id}` returns that row of the workspace, under the
same RLS, or 404.

## 4. UI

Workspace Settings → Audit log: a table of time, user, operation, action kind and
resource, filters for user, operation, action kind and a date range, "load more"
using `before_id`, and the parameters of a row on demand. Visible to everyone; a
non-admin sees only their own actions, and the page says so.
