# Gantry Clean-Room Specification: Apps Studio & Custom UI Builder

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `app`, `raw_app`, `helpers`)
- Schemas: `ListableApp`, `ListableRawApp`, `AppWithLastVersion`, `AppDeployed`, `AppHistory`
- Windmill Public Documentation (`https://windmill.dev/docs/apps`): Full-code React/Svelte raw apps, SDK preview token, component execution, S3 storage integration, guest and embed access
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Apps Studio enables developers and internal tooling teams to build custom web applications, operator dashboards, and interactive portals powered directly by Gantry scripts, flows, and data queries.

---

## 2. Core User Capabilities

### 2.1 Full-Code (Raw) App Development
- **Modern Component Architecture**: Build production-grade frontends using modern TypeScript, React, or Svelte.
- **Worker-Side Source Compilation**:
  - Developers upload application sources (package manifests, components, assets) to Gantry (`POST /w/{workspace}/apps/create_raw_source`, `update_raw_source`).
  - Gantry workers bundle and compile the application inside isolated worker sandboxes.
- **In-Browser Live SDK Preview**:
  - The editor mints short-lived SDK preview tokens (`POST /w/{workspace}/apps/preview_sdk_token`) allowing interactive live reloading and component testing directly inside an embedded development iframe.

### 2.2 Backend Runnable & Component Execution
- Applications execute Gantry backend runnables securely without exposing API keys:
  - Invokes `POST /w/{workspace}/apps_u/execute_component/{path}` on behalf of the application author's permission boundary.
  - Returns structured results directly to client-side frontend state handlers.

### 2.3 S3 Storage & Tabular Data Integration
- Built-in data helpers for rich data interactions:
  - Presigned S3 file uploads and downloads directly from the UI (`/w/{workspace}/apps_u/upload_s3_file/*`, `delete_s3_file`).
  - High-performance server-side data file preview generation:
    - Parquet file table preview (`/w/{workspace}/apps_u/load_parquet_preview/{path}`)
    - CSV file preview (`/w/{workspace}/apps_u/load_csv_preview/{path}`)
    - Table row count calculations (`/w/{workspace}/apps_u/load_table_count/{path}`)
  - Secure signed object URLs for client-side media and asset rendering (`POST /w/{workspace}/apps/sign_s3_objects`).

### 2.4 Deployment & Access Sharing
- **Custom Vanity Paths**: Expose deployed applications under clean paths (e.g., `/a/customer-portal` or `/public/inventory`).
- **Guest Entry**: Toggle whether unauthenticated visitors can access the application without a Gantry user account (`/apps_u/guest_entry_by_custom_path/*`).
- **Embed Tokens**: Generate time-bounded or domain-scoped iframe embed tokens for seamless embedding into third-party portals (`/apps_u/embed_token_by_custom_path/*`).

---

## 3. UI Views & Component Architecture

1. **Apps List & Gallery**:
   - Grid and table views displaying deployed applications, custom URLs, last edited timestamp, and guest status.
   - Quick launch button opening the live deployed application.

2. **Raw App Studio**:
   - Multi-file code editor tree (`src/App.tsx`, `package.json`, components).
   - Side-by-side split screen with live interactive preview iframe.
   - Build log console displaying worker-side compilation output and bundle statistics.

3. **App Settings & Permissions Modal**:
   - Vanity URL slug configuration.
   - Guest entry toggle and anonymous execution role assignment.
   - Embedded iframe domain whitelist and token generator.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| List apps | `GET` | `/w/{workspace}/apps/list` | Returns list of apps in workspace |
| Get app | `GET` | `/w/{workspace}/apps/get/p/{path}` | Fetches app asset definitions |
| Create raw app | `POST` | `/w/{workspace}/apps/create_raw_source` | Bundles and compiles source on worker |
| Update raw app | `POST` | `/w/{workspace}/apps/update_raw_source/{path}` | Updates app source and re-compiles |
| Mint SDK preview token | `POST` | `/w/{workspace}/apps/preview_sdk_token` | Temporary token for development preview |
| Execute component | `POST` | `/w/{workspace}/apps_u/execute_component/{path}` | Runs backend script on behalf of app |
| Parquet preview | `GET` | `/w/{workspace}/apps_u/load_parquet_preview/{path}` | Returns tabular sample of Parquet file |
| CSV preview | `GET` | `/w/{workspace}/apps_u/load_csv_preview/{path}` | Returns tabular sample of CSV file |
| Check custom path | `GET` | `/w/{workspace}/apps/custom_path_exists/{custom_path}` | Validates vanity URL availability |
