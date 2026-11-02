# Gantry Clean-Room Specification: Runs Engine & Observability

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `job`, `service_logs`, `metrics`, `audit`)
- Schemas: `QueuedJob`, `CompletedJob`, `Job`, `FlowStatus`, `FlowStatusModule`, `Audit`
- `specs/not-implemented.md` (Notes on excluded proprietary indexer and direct log file access)
- Windmill Public Documentation (`https://windmill.dev/docs/core_concepts/jobs`): Job states, execution lifecycle, real-time log streaming, cancel/retry, service logs
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

The Runs Engine and Observability suite provides comprehensive operational visibility into all background and synchronous executions within Gantry. It includes real-time job monitoring, live terminal log streaming, flow step execution trees, job lifecycle controls, and system service diagnostics.

---

## 2. Core User Capabilities

### 2.1 Job Queue & Execution List
- **Status Filtering**: Instant tabs and filters for:
  - `Queued`: Jobs waiting for available worker threads in target worker groups.
  - `Running`: Actively executing jobs with live execution duration counters.
  - `Completed`: Succeeded jobs with return values.
  - `Failed`: Errored jobs with exit codes and error summaries.
  - `Canceled`: Manually terminated jobs.
  - `Suspended`: Flows paused at human approval checkpoints.
- **Search & Query Filter**: Filter executions by:
  - Runnable path (e.g. `u/alice/etl_pipeline`)
  - Runnable kind (Script, Flow, Raw App)
  - Trigger kind (Manual, Webhook, Schedule, Flow Parent, MCP, Native Trigger)
  - Initiator identity (User email, API token, Service account)
  - Time range and minimum/maximum runtime duration

### 2.2 Execution Detail Viewer
- **Header & Summary Card**:
  - Job ID, status badge, execution time, memory usage, worker node name.
  - Trigger metadata and parent job link (for child steps).
- **Control Bar**:
  - `Cancel`: Terminate running job (`POST /w/{workspace}/jobs_u/queue/cancel/{id}`).
  - `Rerun`: Rerun job with identical arguments.
  - `Edit & Rerun`: Open script or flow runner pre-populated with this run's arguments.
- **Arguments & Return Value Inspector**:
  - Formatted JSON tree viewer with search and copy-to-clipboard.
  - Download result payload as JSON or CSV.
- **Terminal Log Streamer**:
  - High-performance virtualized log console for streaming stdout and stderr.
  - Line numbers, timestamps, log level highlighting (INFO, WARN, ERROR).
  - Search within logs, autoscroll-to-bottom toggle, and full log download.

### 2.3 Flow Execution Tree & Step Inspector
- When viewing a Flow execution, the UI renders the step graph alongside the timeline:
  - Each step displays its execution status, duration, retry attempts, and worker node.
  - Clicking any step node loads that specific step's arguments, logs, and outputs in the detail pane without losing parent context.

### 2.4 Approvals Inbox (Human-in-the-Loop)
- Dedicated view for operators to review pending flow approval checkpoints:
  - Form view displaying requested inputs, review context, and initiator comments.
  - One-click "Approve" (with optional payload) or "Reject".

### 2.5 System Service Logs & Audit Logs
- **Service Logs**: Inspect host and worker daemon log files (`/service_logs/list_files`, `/service_logs/get_log_file/{path}`).
- **Audit Logs**: View chronological workspace audit records of asset mutations, permissions changes, and job triggers (`/w/{workspace}/audit/list`).

---

## 3. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| Search jobs | `GET` | `/w/{workspace}/jobs/list` | Filters by status, path, tags, date range |
| Get job details | `GET` | `/w/{workspace}/jobs/get/{id}` | Retrieves full `Job` object and execution state |
| Get job logs | `GET` | `/w/{workspace}/jobs/get_logs/{id}` | Fetches raw execution logs |
| Cancel job | `POST` | `/w/{workspace}/jobs_u/queue/cancel/{id}` | Signals worker to abort job execution |
| List service logs | `GET` | `/service_logs/list_files` | Enumerates available host service log files |
| Read service log | `GET` | `/service_logs/get_log_file/{path}` | Reads raw host service log |
| List audit logs | `GET` | `/w/{workspace}/audit/list` | Returns chronological workspace audit records |
