/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// Re-export everything from windmill-api-workspaces
pub use windmill_api_workspaces::workspaces::*;
use windmill_api_workspaces::workspaces::{build_copilot_settings_state, InstanceAISummary};

use crate::ai::{invalidate_ai_request_cache_for_workspace, AIConfig};
use crate::db::ApiAuthed;
#[cfg(feature = "oauth2")]
use crate::oauth2_oss::workspace_connect_slack;
use crate::teams_oss::{
    connect_teams, edit_teams_command, run_teams_message_test_job,
    workspaces_list_available_teams_channels, workspaces_list_available_teams_ids,
};
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    error::{Error, JsonResult},
    utils::require_admin,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

use serde::Serialize;

/// Wraps the subcrate's workspaced_service with routes that depend on windmill-api internals.
pub fn workspaced_service() -> Router {
    let router = windmill_api_workspaces::workspaces::workspaced_service()
        .route("/edit_teams_command", post(edit_teams_command))
        .route(
            "/available_teams_ids",
            get(workspaces_list_available_teams_ids),
        )
        .route(
            "/available_teams_channels",
            get(workspaces_list_available_teams_channels),
        )
        .route("/connect_teams", post(connect_teams))
        .route(
            "/run_teams_message_test_job",
            post(run_teams_message_test_job),
        )
        .route("/tarball", get(crate::workspaces_export::tarball_workspace))
        .route("/edit_copilot_config", post(edit_copilot_config))
        .route(
            "/settings_document",
            get(crate::workspace_settings_document::get_settings_document)
                .post(crate::workspace_settings_document::apply_settings_document),
        )
        .route("/get_copilot_info", get(get_copilot_info))
        .route("/critical_alerts", get(get_critical_alerts))
        .route(
            "/critical_alerts/{id}/acknowledge",
            post(acknowledge_critical_alert),
        )
        .route(
            "/critical_alerts/acknowledge_all",
            post(acknowledge_all_critical_alerts),
        )
        .route("/critical_alerts/mute", post(mute_critical_alerts));

    #[cfg(feature = "oauth2")]
    let router = router.route("/connect_slack", post(workspace_connect_slack));

    router
}

pub(crate) async fn edit_copilot_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(ai_config): Json<AIConfig>,
) -> JsonResult<EditCopilotConfigResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    if let Some(ref custom_prompts) = ai_config.custom_prompts {
        for (mode, prompt) in custom_prompts.iter() {
            if prompt.len() > MAX_CUSTOM_PROMPT_LENGTH {
                return Err(Error::BadRequest(format!(
                    "Custom prompt for mode '{}' exceeds maximum length of {} characters (current: {})",
                    mode,
                    MAX_CUSTOM_PROMPT_LENGTH,
                    prompt.len()
                )));
            }
        }
    }

    ai_config.validate_model_pricing()?;
    ai_config.validate_token_limits()?;
    ai_config.validate_sessions_retention()?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_settings SET ai_config = $1 WHERE workspace_id = $2",
        sqlx::types::Json(&ai_config) as sqlx::types::Json<&AIConfig>,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    invalidate_ai_request_cache_for_workspace(&w_id);

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_copilot_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("ai_config", &format!("{:?}", ai_config)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "ai_config".to_string() },
        Some("AI configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    let workspace_has_config = ai_config.has_providers();
    let copilot_disabled = ai_config.copilot_disabled;
    let sessions_storage_disabled = ai_config.sessions_storage_disabled;
    let sessions_retention_days = ai_config.sessions_retention_days;
    let instance_ai_config =
        sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = 'ai_config'")
            .fetch_optional(&db)
            .await?;
    let settings_state =
        build_copilot_settings_state(workspace_has_config, instance_ai_config.as_ref());
    // A provider-less instance config (e.g. `{}`) is unconfigured, same as build_copilot_settings_state
    // treats it — so it must not shadow the free-tier fallback here either.
    let instance_config_with_providers = instance_ai_config
        .as_ref()
        .and_then(|v| serde_json::from_value::<AIConfig>(v.clone()).ok())
        .filter(|c| c.has_providers());
    let mut effective_ai_config = if workspace_has_config {
        ai_config
    } else if let Some(instance_config) = instance_config_with_providers {
        instance_config
    } else if let Some(free_config) =
        crate::ai_free_tier_oss::free_tier_copilot_config(&db, &authed.email).await?
    {
        // Same fallback as get_copilot_info: with nothing configured, surface Windmill's free
        // tier (a stub that never opts in) so clearing a workspace provider activates it immediately, instead of
        // returning an empty config that disables AI until the next page reload re-fetches it.
        free_config
    } else {
        AIConfig::default()
    };
    effective_ai_config.copilot_disabled = copilot_disabled;
    effective_ai_config.sessions_storage_disabled = sessions_storage_disabled;
    effective_ai_config.sessions_retention_days = sessions_retention_days;

    Ok(Json(EditCopilotConfigResponse {
        effective_ai_config,
        has_instance_ai_config: settings_state.has_instance_ai_config,
        uses_instance_ai_config: settings_state.uses_instance_ai_config,
        instance_ai_summary: settings_state.instance_ai_summary,
    }))
}

#[derive(Serialize)]
pub(crate) struct EditCopilotConfigResponse {
    effective_ai_config: AIConfig,
    has_instance_ai_config: bool,
    uses_instance_ai_config: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance_ai_summary: Option<InstanceAISummary>,
}

async fn get_copilot_info(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<AIConfig> {
    let workspace_ai_config = sqlx::query_scalar!(
        "SELECT ai_config as \"ai_config: sqlx::types::Json<AIConfig>\" FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "getting ai config: {e:#}"
        ))
    })?;

    let copilot_disabled = workspace_ai_config
        .as_ref()
        .is_some_and(|c| c.0.copilot_disabled);
    let sessions_storage_disabled = workspace_ai_config
        .as_ref()
        .is_some_and(|c| c.0.sessions_storage_disabled);
    let sessions_retention_days = workspace_ai_config
        .as_ref()
        .and_then(|c| c.0.sessions_retention_days);
    let instance_config =
        sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = 'ai_config'")
            .fetch_optional(&db)
            .await?
            .and_then(|v| serde_json::from_value::<AIConfig>(v).ok())
            // A provider-less instance config (e.g. `{}`) is unconfigured; don't let it shadow the
            // free-tier fallback, matching the proxy and edit_copilot_config paths.
            .filter(|c| c.has_providers());
    let mut effective =
        if let Some(workspace_ai_config) = workspace_ai_config.filter(|c| c.0.has_providers()) {
            workspace_ai_config.0
        } else if let Some(instance_config) = instance_config {
            instance_config
        } else if let Some(free_config) =
            crate::ai_free_tier_oss::free_tier_copilot_config(&db, &authed.email).await?
        {
            // Nothing configured: fall back to Windmill's free tier (a stub that never opts in). The config
            // carries a `free_tier` marker even once the user's grant is spent — with no
            // providers, but telling the client *why* AI is off.
            free_config
        } else {
            AIConfig::default()
        };
    effective.copilot_disabled = copilot_disabled;
    effective.sessions_storage_disabled = sessions_storage_disabled;
    effective.sessions_retention_days = sessions_retention_days;
    Ok(Json(effective))
}

pub async fn get_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

pub async fn acknowledge_critical_alert() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

pub async fn acknowledge_all_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

pub async fn mute_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}
