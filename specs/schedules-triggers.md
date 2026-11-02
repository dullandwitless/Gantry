# Gantry Clean-Room Specification: Schedules & Triggers

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `schedule`, `trigger`, `http_trigger`, `postgres_trigger`, `websocket_trigger`, `amqp_trigger`, `mqtt_trigger`, `email_trigger`)
- Schemas: `Schedule`, `ScheduleWJobs`, `NewSchedule`, `EditSchedule`, `HttpTrigger`, `NewHttpTrigger`, `EditHttpTrigger`
- `specs/not-implemented.md` (Notes on open-source vs proprietary trigger types)
- Windmill Public Documentation (`https://windmill.dev/docs/triggers`): Cron schedules, webhooks, database triggers, message queues
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Schedules and Triggers drive automated execution of Gantry scripts and flows, transforming static functions into unattended cron jobs, reactive webhooks, and event-driven consumers.

---

## 2. Core User Capabilities

### 2.1 Cron Schedules
- **Cron Expression Builder**:
  - Intuitive visual builder for recurring schedules (Hourly, Daily, Weekly, Monthly) plus standard 5/6-field cron expressions.
  - Interactive schedule preview (`POST /schedules/preview`) showing human-readable translation and the next 5 upcoming execution timestamps.
- **Runnable Association**:
  - Bind schedules to any script or flow in the workspace.
  - Specify static execution arguments or dynamic context presets.
  - Timezone configuration supporting IANA timezone identifiers (e.g., `UTC`, `America/New_York`, `Europe/London`).
- **Management & Toggles**:
  - Enable/disable toggle for pausing scheduled jobs without deleting configuration (`POST /w/{workspace}/schedules/setenabled/{path}`).
  - Execution history view displaying past runs spawned by this specific schedule.

### 2.2 HTTP & Webhook Triggers
- **Webhook Endpoint Management**:
  - Generate unique public or authenticated webhook URLs for scripts and flows (`/w/{workspace}/http_triggers/*`).
  - Configure HTTP methods (GET, POST, PUT, DELETE).
- **Authentication & Security**:
  - Bearer token authorization, header verification, or anonymous public webhooks.
- **Payload & Header Mapping**:
  - Automatically map incoming HTTP request body, query parameters, and headers into target script argument fields.

### 2.3 Event-Driven Triggers
- **PostgreSQL Listener Triggers (`postgres_trigger`)**:
  - React to database state changes via PostgreSQL `LISTEN` / `NOTIFY` events.
  - Define channel names and payload parsing scripts.
- **WebSocket, AMQP & MQTT Triggers**:
  - Listen to streaming WebSocket channels, RabbitMQ/AMQP exchanges, or MQTT message brokers.

### 2.4 Proprietary Trigger Clean-Room Awareness
- Per `specs/not-implemented.md`, inbound email triggers and cloud message queues (Kafka, SQS, GCP Pub/Sub, Azure Service Bus) are proprietary upstream features.
- The UI displays these options with a `Clean-room backlog` badge and links to Gantry roadmap items rather than raising uncaught backend exceptions.

---

## 3. UI Views & Component Architecture

1. **Schedules Dashboard**:
   - Table of all active/paused schedules with path, cron pattern, target runnable, next run countdown, and enabled toggle switch.
   - Schedule Drawer:
     - Target script/flow picker.
     - Cron expression input with live human-readable preview.
     - Timezone selector.
     - Default arguments editor.

2. **Webhook Triggers View**:
   - List of webhook endpoints with copyable webhook URLs and test trigger simulation.
   - Webhook configuration drawer with routing, auth settings, and parameter mappings.

3. **Event Triggers View**:
   - PostgreSQL listener hooks and streaming message consumers.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| List schedules | `GET` | `/w/{workspace}/schedules/list` | Returns workspace schedules |
| Preview schedule | `POST` | `/schedules/preview` | Generates next execution timestamps |
| Create schedule | `POST` | `/w/{workspace}/schedules/create` | `{ path, schedule, script_path, args, timezone }` |
| Toggle schedule | `POST` | `/w/{workspace}/schedules/setenabled/{path}` | `{ enabled: boolean }` |
| Delete schedule | `DELETE` | `/w/{workspace}/schedules/delete/{path}` | Deletes schedule |
| List HTTP triggers | `GET` | `/w/{workspace}/http_triggers/list` | Returns webhook triggers |
| Create HTTP trigger | `POST` | `/w/{workspace}/http_triggers/create` | Configures webhook endpoint |
| List PG triggers | `GET` | `/w/{workspace}/postgres_triggers/list` | Returns PostgreSQL event listeners |
