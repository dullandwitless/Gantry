# Gantry Clean-Room Specification: Data Tables & Workspace Storage

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml` (tags: `workspace` (datatables, ducklake), `helpers` (s3, duckdb), `setting` (storage), `volume`)
- Schemas: `StorageFolder`, `AzureArmResource`
- `specs/not-implemented.md` (Notes on workspace S3 proxy, DuckLake maintenance, and Azure proxy)
- Windmill Public Documentation (`https://windmill.dev/docs/data_tables`): Relational SQL data tables, DuckLake namespaces, schema migrations, S3 large file storage
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Data Tables and Storage provide structured relational and analytical data management within Gantry workspaces, uniting PostgreSQL application tables, DuckDB / DuckLake analytical stores, and S3-compatible large file storage.

---

## 2. Core User Capabilities

### 2.1 Relational Data Tables (PostgreSQL)
- **Table Explorer & Schema Browser**:
  - Inspect tables, column types, primary keys, foreign key constraints, and indices within workspace databases (`/w/{workspace}/workspaces/list_datatables`, `list_datatable_tables`).
  - Interactive table schema viewer showing data dictionary details (`get_datatable_table_schema`).
- **Data Browser & Query Tool**:
  - Browse table rows with paging, sorting, and column filtering.
  - Execute parameterized SQL queries against workspace databases.
- **Database Migrations Manager**:
  - Track migration history, pending migrations, and status (`list_datatable_migrations`, `datatable_migrations_status`).
  - Create new migration scripts, run migrations, and rollback failed migrations (`create_datatable_migration`, `run_datatable_migrations`, `rollback_datatable_migrations`).

### 2.2 DuckLake & Analytical Storage
- **DuckLake Namespaces**:
  - Explore DuckLake analytical tables and Parquet namespaces (`list_ducklakes`).
  - Retrieve DuckDB connection configurations for zero-copy analytical queries in scripts (`POST /w/{workspace}/job_helpers/duckdb_connection_settings`).

### 2.3 S3-Compatible Large File Storage Browser
- **File Explorer**:
  - Hierarchical browser for workspace bucket files and directories (`/w/{workspace}/job_helpers/check_s3_folder_exists`).
  - Upload, download, move, and delete large assets (`upload_s3_file`, `download_s3_file`, `delete_s3_file`).
- **Tabular Data Previews**:
  - Render instant tabular previews of Parquet files directly in the browser (`apps_u/load_parquet_preview`).
  - Convert and download Parquet files as CSV (`download_s3_parquet_file_as_csv`).
  - Display storage usage metrics and bucket capacity indicators (`storage_usage`).

---

## 3. UI Views & Component Architecture

1. **Data Tables View**:
   - Left Sidebar: Database schemas and tables tree.
   - Right Pane:
     - Tab 1: **Data Viewer**: Interactive data grid with row pagination and column search.
     - Tab 2: **Schema Details**: Columns, types, nullability, indices.
     - Tab 3: **Migrations**: Chronological migration log, apply/rollback controls, and "New Migration" builder.

2. **Storage & S3 Explorer View**:
   - File system tree with breadcrumb navigation.
   - File details panel showing size, MIME type, last modified date, and presigned download URL.
   - Built-in preview modal for Parquet, CSV, JSON, images, and text files.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| List datatables | `GET` | `/w/{workspace}/workspaces/list_datatables` | Enumerates databases in workspace |
| List tables | `GET` | `/w/{workspace}/workspaces/list_datatable_tables` | Lists tables within selected database |
| Get table schema | `GET` | `/w/{workspace}/workspaces/get_datatable_table_schema` | Inspects columns and types |
| Run migrations | `POST` | `/w/{workspace}/workspaces/run_datatable_migrations/{name}` | Applies pending database migrations |
| List DuckLakes | `GET` | `/w/{workspace}/workspaces/list_ducklakes` | Enumerates DuckLake namespaces |
| Upload S3 file | `POST` | `/w/{workspace}/job_helpers/upload_s3_file` | Uploads file to workspace storage |
| Download S3 file | `GET` | `/w/{workspace}/job_helpers/download_s3_file` | Retrieves file bytes or presigned URL |
| Preview Parquet | `GET` | `/w/{workspace}/apps_u/load_parquet_preview/{path}` | Generates tabular JSON preview |
| Storage usage | `GET` | `/w/{workspace}/job_helpers/storage_usage` | Returns storage consumption statistics |
