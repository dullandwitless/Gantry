# Gantry Clean-Room Specification: Sign-in through Identity-Aware Proxy, and Mock Users

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Google Cloud documentation, "Securing your app with signed headers":
  https://docs.cloud.google.com/iap/docs/signed-headers-howto (the `x-goog-iap-jwt-assertion`
  header, ES256, the JWK key set at https://www.gstatic.com/iap/verify/public_key-jwk, the
  `exp`, `iat`, `aud` and `iss` checks, the `email` claim, the warning that unsigned
  `x-goog-authenticated-user-*` headers can be forged)
- AGPL backend in this repository: `windmill-api-users/src/users.rs` (`create_session_token`,
  the `/auth` routes), `windmill-api/src/users_oss.rs` (`insert_account`),
  `windmill-common/src/oauth2.rs` (`REQUIRE_PREEXISTING_USER_FOR_OAUTH`),
  `windmill-common/src/users.rs` (`PENDING_OAUTH_LOGIN_TYPE`), `windmill-common/src/lib.rs`
  (`BASE_URL`)
- The project owner's decision (2026-11-02): authenticate through Google Cloud IAP rather than
  OAuth providers, with mock users for local development
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Identity-Aware Proxy

Gantry deployed behind IAP trusts IAP's **signed** identity header and nothing else from the
proxy: the unsigned `x-goog-authenticated-user-*` headers are ignored.

**Configuration** (environment, read on each request):

- `IAP_AUDIENCE`: the expected `aud`, e.g.
  `/projects/123/global/backendServices/456` or
  `/projects/123/locations/us-central1/services/gantry`. IAP sign-in is on exactly when this
  is set.
- `IAP_JWKS_URL`: the key set, default `https://www.gstatic.com/iap/verify/public_key-jwk`.

**Verifying** `x-goog-iap-jwt-assertion`: an ES256 JWT whose `kid` names a key of the key
set, with `iss` = `https://cloud.google.com/iap`, `aud` = `IAP_AUDIENCE`, `exp` in the future
and `iat` in the past (30 seconds of leeway each), and an `email` claim. For Identity
Platform users the email carries an `issuer/tenant:` prefix; the address is the part after
the last `:`. Emails are lowercased. The key set is cached for an hour and fetched again
early when a token names an unknown key, at most every 30 seconds.

**Signing in**: `POST /api/auth/iap` verifies the header of that request and signs its user
in (§3). A missing or invalid assertion is a 401 that says why; IAP sign-in being off is a
404.

**UI**: when IAP is on, the app exchanges the assertion at every load, before anything else,
so the session always belongs to whoever IAP admitted, including after switching Google
accounts. There is no sign-in page. If the exchange fails, the app shows the error. Signing
out ends the Gantry session; the next load signs in again through IAP.

## 2. Mock users (local development)

**Configuration**: `GANTRY_MOCK_USERS`, a comma-separated list of email addresses.

Mock users are **available** only when that list is non-empty, IAP is off, and the instance's
base URL is on `localhost`, `127.0.0.1` or `[::1]`. Otherwise the list is ignored and a
request to use it is a 404, so a stray variable in a deployed environment opens nothing.

**Signing in**: `POST /api/auth/mock_login` with `{"email": ...}` signs in that user (§3) if
the address is in the list (400 otherwise), without a password.

**UI**: the sign-in page offers "Continue as <email>" for each mock user. There is no password
sign-in (specs/single-organisation.md §1).

## 3. Signing a user in

Shared by IAP (login type `iap`) and mock users (login type `mock`):

- an account with that email that is disabled: refused (403);
- an existing account: signed in, keeping its rights and login type, except that a
  `pending_oauth` account (created by an admin for external sign-in) takes the new login type;
- no account, with `require_preexisting_user_for_oauth` on: refused (403, "ask an
  administrator to create your account");
- no account otherwise: created, verified, without password, a superadmin only if
  `GANTRY_SUPERADMINS` lists it (specs/single-organisation.md §1), with an
  instance username assigned as for admin-created accounts. Audited as `users.create`.

The session is a normal session token (`create_session_token`), returned as text and set as
the `token` cookie. Sign-ins are audited as `users.login`, refusals as `users.login_failure`,
each with the method.

## 4. Discovery

`GET /api/auth/sign_in_options` (no authentication) answers
`{"iap": <bool>, "mock_users": [<email>...]}`, `mock_users` empty unless available.
