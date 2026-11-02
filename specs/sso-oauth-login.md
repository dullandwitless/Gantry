# Gantry Clean-Room Specification: Sign-in with OAuth (SSO)

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/advanced/setup_oauth
  (providers, the `/user/login_callback/<provider>` callback, `allowed_domains`, custom
  providers through `login_config` with `auth_url`, `token_url`, `userinfo_url`)
- `backend/windmill-api/openapi.yaml`: `POST /oauth/login_callback/{client_name}`
  (`loginWithOauth`, body `code` and `state`, answers the session token as text and a
  `token` cookie), `GET /oauth/list_logins` (`listOAuthLogins`)
- AGPL backend in this repository: `backend/oauth_login.json` (the provider registry),
  `windmill-oauth/src/lib.rs` (`OAuthClient`, `OAuthConfig`, `ClientWithScopes`,
  `AllClients`, `build_basic_client`, `oauth_redirect`, `exchange_code`,
  `http_get_user_info`, `GHEmailInfo`), `windmill-api/src/oauth2_oss.rs`, `src/monitor.rs`
  (rebuilding `OAUTH_CLIENTS` from the `oauths` setting), `windmill-common/src/oauth2.rs`
  (`REQUIRE_PREEXISTING_USER_FOR_OAUTH`), `windmill-api-users/src/users.rs` (`login`,
  `create_session_token`), `windmill-api/src/users_oss.rs` (`create_user`, username
  assignment), `windmill-common/src/users.rs` (`PENDING_OAUTH_LOGIN_TYPE`)
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Configuration

Providers come from the instance setting `oauths`, a map from provider name to an
`OAuthClient` (`id`, `secret`, optional `display_name`, `allowed_domains`, `login_config`,
`tenant`). An entry is a **login provider** when it has a non-empty `id` and `secret` and
either its name is in the built-in registry (`oauth_login.json`: github, gitlab, google,
microsoft, jumpcloud) or it carries a `login_config`. Its provider configuration is the
`login_config` when present, else the registry's. For `microsoft`, a `tenant` replaces
`common` in the authorize and token URLs.

An entry whose URLs do not parse is skipped and logged; the others still work. Saving the
setting rebuilds the providers (already done by the monitor).

Connecting resources through OAuth (`connect_config`, `/oauth/connect_*`) is a separate
feature and not covered here.

## 2. Listing providers

`GET /api/oauth/list_logins` answers `{"oauth": [{"type": <name>, "display_name": <name or
null>}], "saml": null, "auto_login": null}`, providers sorted by name. No SAML in Gantry.

## 3. Signing in

1. The browser opens `GET /api/oauth/login/{provider}`. The server answers a redirect to
   the provider's authorize URL with the provider's scopes and extra parameters, and a
   random `state` that it also sets as an HTTP-only CSRF cookie. An unknown provider is a
   400.
2. The provider sends the browser back to `{BASE_URL}/user/login_callback/{provider}` with
   `code` and `state`; the UI posts them to `POST /api/oauth/login_callback/{provider}`.
3. The server checks `state` against the cookie (mismatch: 400), exchanges the code for an
   access token at the token URL, and reads the user from the userinfo URL with it:
   - the email is the `email` field, lowercased. For GitHub, whose profile email may be
     missing or private, it is the primary verified address from
     `https://api.github.com/user/emails`. A userinfo response with `email_verified:
     false`, or no usable email, is refused;
   - the name is `name`, else `login`.
4. If the provider has `allowed_domains`, the email's domain must be one of them (case
   insensitive), else 403.
5. The account is the `password` row with that email:
   - disabled: refused (403);
   - exists: signed in. A `pending_oauth` account (created for SSO by an admin) becomes an
     account of this provider; any other account keeps its login type;
   - missing, with `require_preexisting_user_for_oauth` on: refused (403, "ask an
     administrator to create your account");
   - missing otherwise: created with login type = the provider name, no password,
     verified, the name from the provider, not a superadmin, and a username assigned the
     same way as for an admin-created account. Audited as `users.create`.
6. A session token is created exactly as for a password sign-in (`create_session_token`):
   returned as text and set as the `token` cookie. Audited as `users.login` with the
   provider. Failures are audited as `users.login_failure`.

Gantry puts no cap on the number of SSO accounts.

## 4. UI

The sign-in page lists a "Sign in with <display name or provider>" button per provider,
which navigates to `/api/oauth/login/{provider}`. The page at
`/user/login_callback/{provider}` posts the `code` and `state` from its URL, stores the
returned token like a password sign-in, and opens the app; an error is shown with a link
back to the sign-in page. Password sign-in stays available.
