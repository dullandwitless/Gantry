# Specs for the clean-room frontend

What is still missing on the backend side is listed in
[not-implemented.md](not-implemented.md).

Gantry's frontend is written from scratch, from specs in this directory. The
rules in [PROVENANCE.md](../PROVENANCE.md) apply; this file is how to work
within them.

## Who may write here

Specs and frontend code are written only by people and sessions that have
never read Windmill's frontend source (any of `frontend/` in the upstream
repository, or the JavaScript the hosted product serves) or any of its
proprietary code. The person or session that runs `scripts/sync-upstream.sh`
works with upstream sources and does not write specs or frontend code.

## What a spec may be based on

- Gantry's own API: `backend/windmill-api/openapi.yaml` (Apache 2.0), and the
  responses of a running Gantry instance.
- `openflow.openapi.yaml` (Apache 2.0) for the flow format.
- Windmill's public documentation, screenshots and videos, used for what the
  product does, never as a template to trace.
- Behaviour observed by using a running instance.

Not allowed: upstream frontend source, minified or not; upstream browser
bundles opened in devtools; code snippets from issues, PRs or forums that come
from the upstream frontend.

## How a spec is written

One file per area, e.g. `specs/flow-editor.md`. Each starts with:

```
Author: <name or session>
Date: <yyyy-mm-dd>
Sources: <every document, page or API path consulted>
Attestation: I have not read Windmill's frontend source or proprietary code.
```

A spec describes behaviour (what the user can do, what the UI shows, which API
calls back it) rather than layout copied from Windmill. Data formats that only
upstream's frontend defines (app layouts, flow canvas positions) are specified
from API payloads observed on a running instance.

Frontend pull requests name the specs they implement and carry the same
attestation.

## Starting a clean session

Open a new session that has never had the upstream repository attached, attach
only `dullandwitless/Gantry`, and start it with:

> You are working on Gantry's frontend, a clean-room implementation. Read
> PROVENANCE.md and specs/README.md first and follow them strictly: never
> clone, open or search windmill-labs/windmill or its frontend code. Start by
> writing specs/overview.md: the product areas the UI needs, derived from
> backend/windmill-api/openapi.yaml and Windmill's public documentation, with
> the sources section filled in.
