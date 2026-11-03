# Gantry Clean-Room Specification: Users, Passwords, Email and Global Settings

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml`: `POST /users/create` (`createUserGlobally`),
  `POST /users/setpassword`, `POST /users/set_password_of/{user}`,
  `POST /auth/request_password_reset`, `POST /auth/reset_password`,
  `GET /settings/list_global` and schema `GlobalSetting`, `POST /settings/test_smtp`
- AGPL backend in this repository: the callers and request types of the stubs
  (`windmill-api/src/users.rs`, `windmill-api-users/src/users.rs` `NewUser`,
  `EditPassword`, `login`, `request_password_reset`), `windmill-common/src/users.rs`
  (`PENDING_OAUTH_LOGIN_TYPE`, `usr_accepts_email`), `windmill-common/src/usernames.rs`,
  `windmill-common/src/server.rs` (`Smtp`), `windmill-common/src/worker.rs`
  (`SMTP_CONFIG`), `windmill-common/src/instance_config.rs` (`HIDDEN_SETTINGS`,
  `SERVER_SECRET_SETTINGS`), `windmill-common/src/oauth2.rs` (`InstanceEvent`),
  the migrations defining `password` and `pending_user`, and
  `tests/wm_token_confinement.rs`
- Windmill public documentation: https://www.windmill.dev/docs/advanced/instance_settings
  (Users, SMTP, automatic username creation) and
  https://www.windmill.dev/changelog/password-reset
- Crate documentation of `argon2` 0.6 and `mail-send` 0.4 (both already dependencies)
Attestation: I have not read Windmill's frontend source or proprietary code.

> **Superseded in part** by [single-organisation.md](single-organisation.md) §1 (2026-11-03):
> Gantry has no passwords, password reset or invites. `POST /users/create` takes no password,
> login type or `skip_email`, creates a `pending_oauth` account and sends no email; section 2
> and the password parts of the UI section no longer apply. Email (section 4) and the global
> settings listing are unchanged.

---

## 1. Creating a user — `POST /users/create`

Superadmin only (`require_super_admin`; a superadmin's job token is already refused
by the route wrapper). Body is `NewUser`.

1. The email is trimmed and lowercased; accounts are stored lowercased. It must be
   accepted by `usr_accepts_email`, so the account can later join workspaces.
   Otherwise 400.
2. `login_type` defaults to `password`.
   - `password`: `password` is required and non-empty (400 otherwise). It is
     hashed (section 3).
   - anything else (`pending_oauth`, an OAuth client key): `password` is ignored
     and no hash is stored.
3. `name` and `company`, when given, are at most 255 characters (the column width).
4. An existing `password` row for the address is a conflict: 400
   "user … already exists". Workspace membership (`usr`) without an account is not
   a conflict: that is someone invited before they had an account.
5. The instance username (`password.username`) follows automatic username
   creation (`automate_username_creation`, default on):
   - on: the username reserved for the address in `pending_user` if any (that row
     is then deleted, as it only reserves names for addresses without accounts);
     else the address's single existing workspace username, if it has exactly one
     and no other account holds it; else a new one from
     `generate_instance_wide_unique_username`.
   - off: none (NULL).
6. The row is inserted with `super_admin` from the body, `verified = true` (a
   superadmin vouches for the address), and the defaults for every other column.
7. Audit `users.create` (Create, `global`, the email). Instance event `UserAdded`.
8. Unless `skip_email` is true, a welcome email is sent if SMTP is configured
   (section 4). It names the instance URL and the login type, never the password.
   A failed email is logged and does not fail the request.
9. Response: 201, text `user <email> created`.

## 2. Setting a password — `POST /users/setpassword`, `POST /users/set_password_of/{user}`

Authorisation, job-token refusals, session revocation and MCP grant revocation
stay in the AGPL wrappers. The shared implementation:

1. The password must be non-empty (400).
2. It is hashed (section 3) and stored for the account, and the account's
   `login_type` becomes `password`: setting a password is choosing password
   sign-in, and the sign-in query only accepts `login_type = 'password'`. This
   also converts a `pending_oauth` account, as `PENDING_OAUTH_LOGIN_TYPE`
   describes.
3. No account for the address: 404.
4. Audit `users.setpassword` (Update, `global`, the email whose password changed).
5. Response: `password set` (own) or `password of <email> set`.

## 3. Hashing

Argon2id with the server's shared `Argon2` instance (crate defaults) and a random
salt per hash, stored as the PHC string (`$argon2id$v=19$…`). Hashes minted with
other parameters, such as the seeded admin's, keep verifying because the PHC
string carries its own parameters.

## 4. Email

Built with the `smtp` cargo feature (already defined; it enables `mail-send`).
Without it `SMTP_ENABLED` is false, sending returns an error saying the feature is
not compiled in, and `send_email_if_possible` logs and does nothing.

- Configuration is the `Smtp` struct (instance setting `smtp_settings` or the
  deprecated `SMTP_*` variables). `send_email_if_possible` uses the cached
  `SMTP_CONFIG`; the explicit functions take the config they are given.
- Transport: `disable_tls` → plain SMTP; else `tls_implicit` (default: true only
  on port 465) → TLS from connect; else STARTTLS, which must be offered.
  Credentials are used when a username is set. Certificates are verified.
- `clicktracking_off` adds SendGrid's `X-SMTPAPI` header turning click tracking
  off (documented by SendGrid; harmless to other servers).
- The optional timeout bounds the whole connect-and-send.
- Plain, HTML and the legacy `send_email` (plain) variants differ only in body type.
- `send_email_if_possible` returns at once and sends in a background task; failures
  are logged.

Emails sent, all through the existing AGPL callers except the first:
welcome on user creation (section 1), password reset links, workspace
invitations, deployment request notices, critical alerts, the settings SMTP test.

## 5. Listing global settings — `GET /settings/list_global`

Superadmin only. Returns `GlobalSetting[]` (`{name, value}`), ordered by name
(byte order),
excluding `HIDDEN_SETTINGS`, and excluding `SERVER_SECRET_SETTINGS` when this
server does not export server secrets (`EXPORT_SERVER_SECRETS`), the same filter
the instance config API applies.

## 6. UI

- Instance Admin → Users: list (`GET /users/list_as_super_admin`), and an
  "Add user" form (email, password, name, superadmin, send welcome email) calling
  `POST /users/create`. A superadmin can set another user's password
  (`POST /users/set_password_of/{email}`).
- Account menu → Change password: new password entered twice, `POST
  /users/setpassword`. The server signs out the user's other sessions; the UI says so.
- Sign-in page → "Forgot password?" when `GET /auth/is_smtp_configured` is true:
  `POST /auth/request_password_reset`, showing the server's neutral message.
- `/user/reset-password?token=…` (the link the reset email contains): new password
  twice, `POST /auth/reset_password`, then back to sign-in.
- Workspace invites (`POST /w/{workspace}/workspaces/invite_user`) need no
  account; the welcome and invite emails go out when SMTP is configured.
