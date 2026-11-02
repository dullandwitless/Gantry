# Gantry Clean-Room Specification: Scripts Studio

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `script`, `draft`, `helpers`, `npm_proxy`, `workspace_dependencies`, `dbt`, `job`)
- Schemas: `Script`, `NewScript`, `ScriptHistory`, `ScriptArgs`, `ScriptLang`, `Draft`
- Windmill Public Documentation (`https://windmill.dev/docs/scripts`): Languages (Python, TypeScript, Go, Bash, SQL, Rust, PHP), Parameter schema generation, Drafts, Script Locks, Test Runner, Hub
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Scripts Studio is the core development environment within Gantry for creating, testing, versioning, and deploying standalone runnable functions. Scripts serve as foundational building blocks for flows, apps, schedules, and triggers.

---

## 2. Core User Capabilities

### 2.1 Multi-Language Code Editing
- Users write code in any supported runtime:
  - `python3` (Python 3 with inline pip requirements or workspace dependencies)
  - `deno` / `bun` / `typescript` (TypeScript with npm/deno imports)
  - `go` (Go functions)
  - `bash` (Shell scripts)
  - `sql` (PostgreSQL / MySQL / Snowflake / BigQuery queries)
  - `rust` (Rust scripts)
  - `php` (PHP scripts)
- Code editor provides syntax highlighting, line numbers, code folding, autocompletion, and format-on-save.
- Script metadata configuration: path (e.g. `f/finance/generate_report`), summary description, runtime tags, execution timeout, and concurrency limits.

### 2.2 Dynamic Parameter Form Generation
- Scripts declare input parameters in function type signatures or header docstrings. The backend parses these into JSON schema arguments (`ScriptArgs`).
- The UI dynamically generates typed input controls:
  - Primitive types: String, Number, Integer, Boolean
  - Enums / Selects: String lists rendered as dropdowns
  - Complex structures: Object / JSON editor, Array builders
  - Resource references: Pickers filtered by resource type (e.g. `postgresql`, `s3`, `slack`)
  - Secret & Variable references: Quick-insert token picker for `$res:...` and `$var:...`

### 2.3 Drafts & Autosave
- Unsaved changes are automatically debounced and saved to `/w/{workspace}/drafts/save` under the user's identity.
- When opening an existing script with a pending draft, the UI warns the user and provides:
  - "Restore draft"
  - "Discard draft"
  - "View visual diff between saved script and draft"

### 2.4 Live Test Runner
- Developers can execute the current script directly from the editor without deploying a new immutable version.
- Test runs invoke `POST /w/{workspace}/jobs/run_wait_result/p/{path}` with test arguments.
- Test runner panel displays:
  - Execution status (Running, Succeeded, Failed)
  - Execution duration and memory usage
  - Live streamed stdout/stderr terminal output
  - Formatted JSON/text return payload with copy and download controls

### 2.5 Revision History, Diffs & Script Locks
- Every published change increments an immutable version hash.
- History view lists previous versions with author, timestamp, commit summary, and git sync ref (`GET /w/{workspace}/scripts/history/p/{path}`).
- Side-by-side or inline diff viewer allows comparing any historical version against the current code.
- Script locking (`is_locked`): Allows administrators to lock scripts to prevent concurrent overwrites or unauthorized edits.

---

## 3. UI Views and Component Layout

1. **Header Toolbar**:
   - Path breadcrumbs: `Workspace > Folder > script_name`
   - Language selector badge & runtime settings
   - Save / Deploy button (with draft indicator)
   - "Test Run" button (keyboard shortcut `Ctrl+Enter` / `Cmd+Enter`)
   - Actions menu: Version history, Clone, Move to folder, Delete, Export to Hub

2. **Main Workspace (Split View)**:
   - **Left Pane (Code Editor)**: Main Monaco / CodeMirror editor with language mode set.
   - **Right Pane (Collapsible Drawer)**:
     - **Tab 1: Test Arguments**: Dynamically rendered form inputs from `ScriptArgs`.
     - **Tab 2: Test Output**: Execution console, logs, status, return value, error trace.
     - **Tab 3: Settings & Metadata**: Timeout, cache TTL, worker group tags, dedicated worker requirement.
     - **Tab 4: History**: Timeline of versions with diff viewer.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Payload / Notes |
| :--- | :--- | :--- | :--- |
| Get script | `GET` | `/w/{workspace}/scripts/get/p/{path}` | Returns script source, `args`, `language`, `summary` |
| Create script | `POST` | `/w/{workspace}/scripts/create` | `{ path, content, language, summary, is_template }` |
| Update/Deploy | `POST` | `/w/{workspace}/scripts/update/{path}` | `{ content, summary, lock }` |
| Delete script | `DELETE` | `/w/{workspace}/scripts/delete/{path}` | Soft-deletes script to trash |
| Test execution | `POST` | `/w/{workspace}/jobs/run_wait_result/p/{path}` | `{ args: { ... } }` |
| Save draft | `POST` | `/w/{workspace}/drafts/save` | `{ kind: "script", path, value }` |
| Get draft | `GET` | `/w/{workspace}/drafts/get/script/{path}` | Retrieves user's pending draft |
| Script history | `GET` | `/w/{workspace}/scripts/history/p/{path}` | Returns list of `ScriptHistory` items |
| Hub script get | `GET` | `/scripts/hub/get/{path}` | Fetches community script definition |
