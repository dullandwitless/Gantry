# Frontend against a live backend: findings, 2026-11-02

Observed by running `gantry` (features `quickjs,static_frontend`, serving
`frontend/build`) at http://localhost:8000 and loading it in a browser, signed
in with a superadmin token in `localStorage.gantry_token`. Every finding is
behaviour of the running instance or a comparison with
`backend/windmill-api/openapi.yaml`.

## 1. API calls are missing the `/api` prefix (blocking)

The backend serves its API under `/api/...` and the frontend for every other
path. The client calls `/workspaces/list`, `/w/{workspace}/...`, and so on with
no prefix, so each call gets `200 text/html` (the app's own `index.html`)
instead of JSON:

```
GET /workspaces/list      -> 200 text/html, "<!doctype html>..."
GET /api/workspaces/list  -> 200 application/json, "[]"
```

The OpenAPI paths are relative to the server URL `/api`.

## 2. A non-JSON response blanks the whole app (blocking)

The client returns the HTML text as the result, components call array methods
on it, and React unmounts the app: the page is solid black. Console:

```
TypeError: x.some is not a function
Uncaught TypeError: l.map is not a function
```

The app needs an error boundary, and the client should treat a non-JSON
response to a JSON endpoint as an error.

## 3. Failures fall back to demo data, hiding them

When a request fails, the client substitutes built-in sample data (for example
a "Demo Workspace" with id `demo`), so a backend that is down or refusing
looks like a working app with content. Errors should be shown, not replaced.

## 4. The selected workspace defaults to `demo`, which does not exist

Requests go to `/w/demo/...` on a fresh instance. A superadmin token that is
not a member of any workspace gets `[]` from `/api/workspaces/list`; the UI
should then offer to create a workspace (`POST /api/workspaces/create`) rather
than assume one.

## 5. Paths that do not exist in the API

| Client calls | OpenAPI has |
| --- | --- |
| `GET /w/{workspace}/flows/get/p/{path}` | `GET /w/{workspace}/flows/get/{path}` (no `p/`) |
| `POST /w/{workspace}/jobs/list_search` | `GET /w/{workspace}/jobs/list` |
| `POST /w/{workspace}/jobs/cancel/{id}` | `POST /w/{workspace}/jobs_u/queue/cancel/{id}` |
| `GET /w/{workspace}/concurrencyGroups/list` | `GET /concurrency_groups/list` |

Scripts use `/scripts/get/p/{path}`, which does exist; flows do not follow the
same pattern.

## 6. No sign-in

There is no login screen; a token has to be put into `localStorage` by hand.
`POST /api/auth/login` exists for email and password.
