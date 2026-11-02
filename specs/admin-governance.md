# Gantry Clean-Room Specification: Administration, Governance & Security

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `user`, `group`, `granular_acl`, `token`, `oauth`, `admin`, `setting`, `settings`, `config`, `worker`, `concurrencyGroups`, `volume`, `health`)
- Schemas: `User`, `EditWorkspaceUser`, `GlobalUserInfo`, `WorkerPing`, `ConcurrencyGroup`
- `GET /api/workers/list` and `GET /api/workers/custom_tags` on a running Gantry instance (`docs/issues/2026-11-02-frontend-live-run.md`, finding 7)
- `specs/not-implemented.md` (Notes on user creation stubs, email stubs, and SSO/SAML/SCIM enterprise stand-ins)
- Windmill Public Documentation (`https://windmill.dev/docs/workspace_management`): Workspaces, members, groups, granular ACLs, API tokens, workers, concurrency limits, settings
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Administration and Governance encompasses workspace-level membership and access control, security policies, API token delegation, and instance-wide infrastructure oversight (worker pools, concurrency groups, server health).

---

## 2. Core User Capabilities

### 2.1 Workspace Access & Membership
- **Member Directory**:
  - List workspace members, their assigned roles (`Admin`, `Member`, `Viewer`, `Operator`), and join date (`/w/{workspace}/users/list`).
  - Edit member role permissions or revoke membership.
- **Workspace Groups**:
  - Create and manage functional groups (e.g., `data-team`, `sre-oncall`) and assign group memberships (`/w/{workspace}/groups/*`).
- **Granular Access Control Lists (ACLs)**:
  - Configure resource-level permissions (Read, Write, Execute, Admin) on specific folders, scripts, flows, resources, and variables (`/w/{workspace}/granular_acl/*`).
  - Hierarchical inheritance: subfolders and nested assets inherit permissions from parent folders unless explicitly overridden.

### 2.2 API Tokens & Scoped Credentials
- **Token Generation**:
  - Generate workspace API tokens for automated CI/CD pipelines, CLI usage, and webhook clients.
  - Define granular permission scopes (`/tokens/list/scopes`) and optional expiration dates.
  - Revoke compromised or expired tokens instantly.

### 2.3 Worker Pool & Infrastructure Oversight
- **Worker Node Monitoring**:
  - Workers come from `GET /workers/list`, an array of `WorkerPing`. The default
    returns workers that pinged in the last 5 minutes; `ping_since` (seconds)
    widens or narrows that window. One card per entry, keyed by `worker`:
    - `worker` (name) and `worker_group`; `ip`; `wm_version`.
    - Tags from `custom_tags`, which may be absent or null (show "none").
    - `jobs_executed`; `last_job_id` when the caller may see it.
    - Last ping: `last_ping`, seconds since the worker's last ping (may be
      null). A worker is shown as stale once that exceeds 60 seconds, and as
      draining when `draining` is true.
    - Memory: `memory_usage` of `memory`, both in bytes, when present.
    - Occupancy: `occupancy_rate_15s`/`_5m`/`_30m`, fractions 0–1, when present.
  - `GET /workers/custom_tags` returns tag names only (`string[]`), not workers;
    it is for tag pickers, never for the worker list.
  - An empty list is shown as "no worker has pinged in the last 5 minutes", and
    a failed request as an error, never as sample workers.
- **Concurrency Groups**:
  - Configure named concurrency queues with maximum parallel job limits (e.g. limiting third-party API calls or batch jobs to 5 concurrent executions) (`/w/{workspace}/concurrencyGroups/*`).
- **Worker Volumes**:
  - Inspect persistent storage volumes mounted across worker nodes (`/w/{workspace}/volumes/*`).

### 2.4 Instance Settings & System Health
- **Global Settings & Flags**:
  - Configure instance-level settings, default job timeouts, retention policies, and base URLs (`/settings/*`, `/configs/*`).
- **System Health Diagnostics**:
  - Inspect backend health endpoints (`/health/status`, `/health/detailed`) displaying database latency, queue backlog, and memory consumption.

### 2.5 Clean-Room Awareness & Authentication Stubs
- Per `specs/not-implemented.md`:
  - User self-registration, password hashing, and invite emails are currently stubs (`users_oss.rs`, `email_oss.rs`).
  - Enterprise SSO (SAML, SCIM) and external instance DBs are proprietary upstream features.
  - The UI provides clear indicators on user invite forms and SSO configuration screens explaining that these capabilities are slated for clean-room implementation, while primary authentication uses seeded admin credentials and API tokens.

---

## 3. UI Views & Component Architecture

1. **Workspace Settings (`/w/:workspace/settings`)**:
   - Tab 1: **Members**: Member list, role dropdown, invite button.
   - Tab 2: **Groups**: Group list, member assignment, role bindings.
   - Tab 3: **Granular ACLs**: Path-based permission matrix.
   - Tab 4: **API Tokens**: Token generator with scope checkboxes and active token table.

2. **Instance Admin Panel (`/admin`)**:
   - **Workers**: Real-time worker cluster dashboard.
   - **Concurrency Groups**: Limit editor and queue depth monitor.
   - **Volumes**: Storage mount table.
   - **Settings & Config**: Key-value system configuration editor.
   - **Health**: System status check and subsystem diagnostics.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| List members | `GET` | `/w/{workspace}/users/list` | Returns workspace users and roles |
| Edit member role | `POST` | `/w/{workspace}/users/edit_role` | Updates user permission level |
| List groups | `GET` | `/w/{workspace}/groups/list` | Returns workspace groups |
| Add granular ACL | `POST` | `/w/{workspace}/granular_acl/add` | Grants path-specific permission |
| Token scopes | `GET` | `/tokens/list/scopes` | Returns available permission scopes |
| List workers | `GET` | `/workers/list` | `WorkerPing[]`; tags in `custom_tags` |
| List worker tags | `GET` | `/workers/custom_tags` | `string[]` of tag names |
| List concurrency groups | `GET` | `/concurrency_groups/list` | Returns concurrency throttle limits |
| Health check | `GET` | `/health/status` | Returns backend health status |
