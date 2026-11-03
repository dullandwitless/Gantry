# Gantry Clean-Room Specification: Workspace Service Accounts

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/core_concepts/roles_and_permissions
  (service accounts: workspace-scoped automation identities that cannot log in, act through
  tokens; created from user management with only a username, the email being derived; operator
  by default, or developer, or admin; `wm_deployers` for CI deploy identities; admins
  impersonate them to check their permissions) and
  https://www.windmill.dev/changelog/workspace-service-accounts
- `backend/windmill-api/openapi.yaml`: `POST /w/{workspace}/workspaces/create_service_account`
  (`username`, `is_admin`, `operator` defaulting to true, `add_to_deployers`; 201 text),
  `POST /w/{workspace}/users/impersonate_service_account` (`username`; 201 with the
  impersonation token), `POST /w/{workspace}/users/exit_impersonation`
- AGPL backend in this repository: `windmill-api-workspaces/src/workspaces.rs` (`add_user`,
  `NewServiceAccount`, `reject_admin_and_operator`, `resolve_new_member_role`),
  `windmill-api-users/src/users.rs` (`impersonate`, `exit_impersonation`, the refusal to
  refresh `impersonation:` tokens and its "24h expiry", `cap_token_expiration` exempting service
  accounts, the global user listing of `is_service_account` rows),
  `windmill-common/src/auth.rs` (`impersonation:` labels), `windmill-common/src/usernames.rs`
  (`validate_username`), migration `20260326200000_service_accounts`
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

A service account is a `usr` row with `is_service_account = true` and no `password` row. The
AGPL code already lists them among users, counts them for seats, lets their tokens outlive the
maximum token lifetime, refuses to turn an `impersonation:` token into a session, and restores
the admin's own session cookie on `exit_impersonation`. Creating one and impersonating one are
missing.

## 2. Creating a service account

`POST /w/{workspace}/workspaces/create_service_account`, by a workspace admin only:

- `username` must pass `validate_username` and must not be anyone's username anywhere on the
  instance (another workspace member, an account, or a pending reservation), so the name can
  never collide with a person later added to the workspace. 400 otherwise.
- `is_admin` and `operator` cannot both be true (400). `operator` defaults to true. The
  workspace role is resolved as for any new member.
- Not in the `admins` workspace (400), like other members.
- Its email is derived: `<username>@<workspace>.sa.invalid`. The `.invalid` top-level domain
  is reserved and never delivers, so nothing is ever mailed to it, and the address is unique
  per workspace and username.
- It joins the `all` group, and `wm_deployers` when `add_to_deployers` is true.
- Audited as `users.create_service_account` with the email. Answers 201
  `service account <username> created`.

It cannot sign in: it has no account to sign in with. Deleting it is deleting the workspace
member, as for anyone.

## 3. Impersonating a service account

`POST /w/{workspace}/users/impersonate_service_account` with `{"username"}`, by a workspace
admin only, and not with a job token:

- the target must be a service account of this workspace that is not disabled (404 otherwise);
- the server mints a token for the service account's email, scoped to the workspace, labelled
  `impersonation:<admin email>` (so actions are credited to the admin acting through it), not
  superadmin, expiring after 24 hours, and never refreshable into a session;
- the token replaces the `token` cookie and is returned (201). The caller keeps its own token
  to give back to `exit_impersonation`, which restores the cookie;
- audited as `users.impersonate_service_account` with the service account and token prefix.

While impersonating, the admin can create tokens for the service account in the usual way;
those are its long-lived automation tokens.
