//! Administration of the Google Cloud Secret Manager backend (specs/gcp-secret-manager.md §5).
//! Clean-room implementation; attestation: written without reading Windmill's frontend source or
//! proprietary code.

use axum::{extract::Extension, routing::post, Json, Router};
use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{JsonResult, Result},
    secret_backend::{
        gcp_secret_manager::{
            migrate_secrets_from_gcp_secret_manager, migrate_secrets_to_gcp_secret_manager,
        },
        GcpSecretManagerBackend, GcpSecretManagerSettings, SecretMigrationReport,
    },
    DB,
};

pub fn routes() -> Router {
    Router::new()
        .route("/test_gcp_secret_manager", post(test_connection))
        .route(
            "/migrate_secrets_to_gcp_secret_manager",
            post(migrate_to_gcp_secret_manager),
        )
        .route(
            "/migrate_secrets_from_gcp_secret_manager",
            post(migrate_from_gcp_secret_manager),
        )
}

async fn test_connection(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<GcpSecretManagerSettings>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    GcpSecretManagerBackend::new(settings.clone())
        .test_connection()
        .await?;
    Ok(format!(
        "Connected to Secret Manager in project {}",
        settings.project_id
    ))
}

async fn migrate_to_gcp_secret_manager(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<GcpSecretManagerSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;
    let report = migrate_secrets_to_gcp_secret_manager(&db, &settings).await?;
    audit_migration(
        &db,
        &authed,
        "secrets.migrate_to_gcp_secret_manager",
        &report,
    )
    .await?;
    Ok(Json(report))
}

async fn migrate_from_gcp_secret_manager(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<GcpSecretManagerSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;
    let report = migrate_secrets_from_gcp_secret_manager(&db, &settings).await?;
    audit_migration(
        &db,
        &authed,
        "secrets.migrate_from_gcp_secret_manager",
        &report,
    )
    .await?;
    Ok(Json(report))
}

async fn audit_migration(
    db: &DB,
    authed: &ApiAuthed,
    operation: &str,
    report: &SecretMigrationReport,
) -> Result<()> {
    let migrated = report.migrated_count.to_string();
    let failed = report.failed_count.to_string();
    audit_log(
        db,
        authed,
        operation,
        ActionKind::Update,
        "global",
        None,
        Some([("migrated", migrated.as_str()), ("failed", failed.as_str())].into()),
    )
    .await
}
