# Gantry Clean-Room Specification: One Organisation, One Workspace

Author: clean-room session (Claude)
Date: 2026-11-03
Sources:
- The project owner's decisions (2026-11-03): Gantry serves one organisation on Google Cloud,
  behind Identity-Aware Proxy. Invites and passwords, several workspaces, and public sharing
  are cut. Custom roles, instance groups and service accounts stay.
- `specs/identity-aware-proxy.md` (sign-in), `specs/service-accounts.md`,
  `specs/custom-roles.md`
- AGPL backend in this repository: `windmill-api-users/src/users.rs`,
  `windmill-api/src/users.rs`, `windmill-api/src/users_oss.rs`,
  `windmill-api/src/external_auth.rs`, `windmill-api-workspaces/src/workspaces.rs`,
  `windmill-api/src/apps.rs`
- `backend/windmill-api/openapi.yaml` for the routes named below
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 0. Approach

Windmill is built for many organisations sharing an instance. Gantry is not, so the parts
that only exist for that are taken out rather than configured off:

- A feature whose code stands on its own (passwords, invites) is **deleted**: routes,
  handlers, tests and UI.
- A feature woven into shared code (forks, public and guest apps) is **cut**: its routes answer
  404 in front of the API and the UI has no way to it, while its code stays compiled.
- Shared code that merely knows about a removed feature (the role in-use check reading
  `workspace_invite`, exports listing invites, columns of `workspace_settings`) stays as it is
  and reads empty data. Tables and columns stay, so upstream migrations keep applying.

A later upstream change to a deleted handler conflicts on merge; the resolution is to keep the
deletion.

## 1. Accounts without invites or passwords

People sign in through IAP (deployed) or as a mock user (local development), and nothing
else. Who may reach Gantry at all is IAP's access policy.

**Removed routes**

| Route | Was |
|---|---|
| `POST /api/auth/login` | password sign-in |
| `POST /api/auth/request_password_reset`, `POST /api/auth/reset_password` | password reset |
| `GET /api/auth/is_password_login_disabled` | |
| `POST /api/users/login_links`, `GET /api/auth/login_link/{token}` | one-time sign-in links |
| `POST /api/users/setpassword`, `POST /api/users/set_password_of/{user}` | passwords |
| `POST /api/users/set_login_type/{user}` | login type is set by sign-in |
| `GET /api/auth/is_first_time_setup` | the seeded admin's password hint |
| `GET /api/users/list_invites`, `POST /api/users/accept_invite`, `POST /api/users/decline_invite` | invites |
| `POST /api/w/{w}/workspaces/invite_user`, `.../delete_invite`, `GET .../list_pending_invites` | invites |
| `POST /api/w/{w}/workspaces/edit_auto_invite`, `GET /api/workspaces/allowed_domain_auto_invite` | auto-invite |

The workspace settings document refuses an `auto_invite` key (400).

**Kept**: `POST /api/users/create` (a superadmin creates an account ahead of its first
sign-in, for `require_preexisting_user_for_oauth`). It takes no password: the account is
created with login type `pending_oauth`, which its first sign-in replaces. No welcome email is
sent. A request with any other field (`password`, `login_type`, `skip_email`) is refused
(422).

**Superadmins**: `GANTRY_SUPERADMINS`, a comma-separated list of emails (environment). An
account in the list becomes a superadmin when it signs in. Removing an address from the list
does not demote anyone; a superadmin does that in the UI. This replaces the seeded
`admin@windmill.dev` password as the way to bootstrap an instance.

**Joining the workspace**: an account's first sign-in (a new account, or a `pending_oauth`
one created by a superadmin) adds it to the workspace (§2) with the workspace's default role
for new members, and to its `all` group, under its instance username (an account without
one, as when automatic username creation is off, is left to an admin). Later sign-ins change
nothing, so a member an admin
removed stays removed. Before the workspace exists, nobody is added. Admins add instance users
with the existing `POST /api/w/{w}/workspaces/add_user`.

## 2. One workspace

The instance has exactly one workspace besides `admins` (which holds instance-level scripts
and settings and stays as it is).

- `POST /api/workspaces/create` is for superadmins only, whatever
  `CREATE_WORKSPACE_REQUIRE_SUPERADMIN` says, and only while there is no workspace other than
  `admins` (an archived one does not count). It is how a new instance is set up. Afterwards it
  is a 400 ("this instance already has its workspace, <id>"). Every enabled account that exists
  then joins the new workspace as in §1, so whoever signed in before setup is not left out.
- **Cut**: forks and dev workspaces (`create_fork`, `fork_creation_status`,
  `attach_dev_workspace`, `detach_dev_workspace`, `get_dev_workspace`,
  `create_workspace_fork_branch`, `reset_diff_tally`, `compare`, `seed_full_diff`,
  `drop_forked_datatable_databases`, `drop_forked_ducklake_namespaces`,
  `edit_add_admins_and_developers_to_forks`, and the global `POST /api/workspaces/create_fork`),
  deployment requests (`/api/w/{w}/deployment_request/*`), and archiving (`POST
  /api/w/{w}/workspaces/archive`, `POST /api/workspaces/unarchive/{w}`), since an archived
  workspace would let a second one be created.
- Leaving the workspace or the instance stays: it is a member's own choice, and an account that
  left signs in again without rejoining (§1).
- **UI**: no workspace switcher. The app works in the one workspace; before it exists a
  superadmin sees the setup form and anyone else a "not set up yet" message.

These handlers are woven into shared code, so they stay compiled: a layer in front of the API
answers 404 for their routes (`windmill-api/src/single_organisation.rs`).

## 3. No public sharing

- Apps are for signed-in members. Saving an app as `anonymous` (public) or `guest` is refused
  (400) unless it is already stored that way. An app stored as `anonymous` is held to members
  with read access, like a `publisher` app, and runs as its publisher.
- Guests do not exist: `instance_supports_guests` is false, which every guest gate already
  consults.
- **Cut** (404, same layer): `GET /api/w/{w}/apps_u/public_app/{secret}`,
  `.../guest_entry/{secret}`, `.../embed_token/{secret}`, `.../public_resource/{*path}`, guest
  settings (`edit_guest_jwt_key`, `guest_usage`, `GET /api/users/guests`) and the public-app
  rate limit (`public_app_rate_limit`).
- **Kept**: `GET /api/w/{w}/apps/embed_token/p/{*path}` (the scoped token a sandboxed app's
  frame uses for a signed-in viewer) and `apps/secret_of*` (version ids that raw app data is
  served by).
