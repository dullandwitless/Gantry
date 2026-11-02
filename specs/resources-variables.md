# Gantry Clean-Room Specification: Resources & Variables

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `resource`, `variable`)
- Schemas: `Resource`, `ResourceType`, `CreateResource`, `EditResource`, `Variable`, `CreateVariable`, `EditVariable`, `VariableExpirationHandlerConfig`
- `specs/not-implemented.md` (Notes on database secret storage vs excluded external secret stores)
- Windmill Public Documentation (`https://windmill.dev/docs/resources`): Resource types JSON schemas, credential instances, variables, encryption, expiration handlers
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Resources and Variables provide centralized configuration and secret management across Gantry. They allow decoupling credentials, connection strings, API tokens, and environment parameters from script and flow logic.

---

## 2. Core User Capabilities

### 2.1 Resource Types (JSON Schema Registry)
- **Schema Definitions**: Each `ResourceType` represents a distinct service or protocol (e.g., `postgresql`, `mysql`, `s3`, `slack`, `github`, `smtp`, `openai`).
- **Schema Editor & Inspector**:
  - View required fields, data types, and default property structures.
  - Create and register custom resource types with JSON schemas defining input validation rules and UI field hints (e.g. password masks, file pickers, multiline text).

### 2.2 Resource Instances
- **Credential & Connection Management**:
  - Instantiate and store connection profiles conforming to a selected `ResourceType`.
  - Sensitive fields within resources are automatically encrypted at rest.
- **Permission Boundaries**:
  - Assign resource ownership and read/write/execute permissions to specific users or workspace groups.
- **Script Autowiring**:
  - Scripts referencing a resource type parameter (e.g. `pg: postgresql`) can easily select from available resource instances in the test runner and flow canvas.

### 2.3 Variables Management
- **Key-Value Store**:
  - Store workspace configuration values, constants, and secret strings accessible in scripts via `$var:path` or helper APIs.
- **Plaintext vs. Encrypted Values**:
  - Plaintext variables for non-sensitive settings (e.g., environment names, feature flags, base URLs).
  - Encrypted variables (`POST /w/{workspace}/variables/encrypt`) for secrets (e.g. API tokens, signing keys) where values are masked in the UI and decrypted only in worker runtime sandboxes.
- **Expiration Policies**:
  - Configure automatic variable expiration dates and cleanup notification hooks (`VariableExpirationHandlerConfig`).

---

## 3. UI Views & Component Architecture

1. **Resources View**:
   - Tab 1: **Resource Instances**: Searchable table showing resource path, type badge, description, and last updated date.
   - Tab 2: **Resource Types**: Library of installed schemas with "New Resource Type" JSON schema builder.
   - Resource Creation/Edit Drawer: Dynamically generates form inputs based on the selected `ResourceType` schema.

2. **Variables View**:
   - Searchable table displaying variable path, type (Plaintext or Encrypted Secret), description, expiration status, and actions.
   - Variable Creation/Edit Modal: Path input, Secret toggle, Value editor, Expiration date picker.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| List resource types | `GET` | `/w/{workspace}/resources/type/list` | Returns all available resource schemas |
| List resources | `GET` | `/w/{workspace}/resources/list` | Returns instantiated resources |
| Create resource | `POST` | `/w/{workspace}/resources/create` | `{ path, resource_type, value, description }` |
| Update resource value | `POST` | `/w/{workspace}/resources/update_value/{path}` | Updates credential payload |
| Delete resource | `DELETE` | `/w/{workspace}/resources/delete/{path}` | Removes resource instance |
| List variables | `GET` | `/w/{workspace}/variables/list` | Returns workspace variables |
| Create variable | `POST` | `/w/{workspace}/variables/create` | `{ path, value, is_secret, description }` |
| Encrypt variable | `POST` | `/w/{workspace}/variables/encrypt` | Encrypts payload before persisting |
| Delete variable | `DELETE` | `/w/{workspace}/variables/delete/{path}` | Deletes variable |
