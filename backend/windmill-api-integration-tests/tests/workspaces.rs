use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_workspace_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");
    let global_base = format!("http://localhost:{port}/api/workspaces");

    // ===== Global endpoints =====

    // --- list ---
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|w| w["id"] == "test-workspace"));

    // --- list_as_superadmin ---
    let resp = authed(client().get(format!("{global_base}/list_as_superadmin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|w| w["id"] == "test-workspace"));

    // --- users (user's workspaces) ---
    let resp = authed(client().get(format!("{global_base}/users")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(workspaces.iter().any(|w| w["id"] == "test-workspace"));

    // --- exists ---
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "test-workspace"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "nonexistent-workspace"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- exists_username (validates username is available) ---
    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "available-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // ===== Workspace-scoped endpoints (read) =====

    // --- get_settings ---
    let resp = authed(client().get(format!("{base}/get_settings")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert!(settings.is_object());

    // --- get_deploy_to ---
    let resp = authed(client().get(format!("{base}/get_deploy_to")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_premium ---
    let resp = authed(client().get(format!("{base}/is_premium")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- default_app ---
    let resp = authed(client().get(format!("{base}/default_app")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- default_scripts ---
    let resp = authed(client().get(format!("{base}/default_scripts")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- encryption_key ---
    let resp = authed(client().get(format!("{base}/encryption_key")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_dependency_map ---
    let resp = authed(client().get(format!("{base}/get_dependency_map")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_as_superadmin ---
    let resp = authed(client().get(format!("{base}/get_as_superadmin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["id"], "test-workspace");

    // --- get_workspace_name ---
    let resp = authed(client().get(format!("{base}/get_workspace_name")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let name = resp.text().await?;
    assert_eq!(name, "test-workspace");

    // --- get_usage ---
    let resp = authed(client().get(format!("{base}/usage")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_used_triggers ---
    let resp = authed(client().get(format!("{base}/used_triggers")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<serde_json::Value>().await?;

    // --- get_secondary_storage_names ---
    let resp = authed(client().get(format!("{base}/get_secondary_storage_names")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- get_dependents (empty, no dependencies exist) ---
    let resp = authed(client().get(format!("{base}/get_dependents/u/test-user/nonexistent")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let dependents = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(dependents.is_empty());

    // --- get_dependents_amounts ---
    let resp = authed(client().post(format!("{base}/get_dependents_amounts")))
        .json(&json!(["u/test-user/some_script"]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_ducklakes ---
    let resp = authed(client().get(format!("{base}/list_ducklakes")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- list_datatables ---
    let resp = authed(client().get(format!("{base}/list_datatables")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- list_datatable_schemas ---
    let resp = authed(client().get(format!("{base}/list_datatable_schemas")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // ===== Workspace-scoped endpoints (mutations) =====

    // --- update (edit_workspace) ---
    let resp = authed(client().post(format!("{base}/update")))
        .json(&json!({"name": "renamed-workspace", "owner": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // --- change_workspace_name ---
    let resp = authed(client().post(format!("{base}/change_workspace_name")))
        .json(&json!({"new_name": "Test Workspace Renamed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "change_workspace_name: {}",
        resp.text().await?
    );

    // verify name changed
    let resp = authed(client().get(format!("{base}/get_workspace_name")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.text().await?, "Test Workspace Renamed");

    // --- change_workspace_color ---
    let resp = authed(client().post(format!("{base}/change_workspace_color")))
        .json(&json!({"color": "#FF5733"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "change_workspace_color: {}",
        resp.text().await?
    );

    // --- edit_webhook ---
    let resp = authed(client().post(format!("{base}/edit_webhook")))
        .json(&json!({"webhook": "https://example.com/hook"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "edit_webhook: {}", resp.text().await?);

    // verify in settings
    let resp = authed(client().get(format!("{base}/get_settings")))
        .send()
        .await
        .unwrap();
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(settings["webhook"], "https://example.com/hook");

    // clear webhook
    let resp = authed(client().post(format!("{base}/edit_webhook")))
        .json(&json!({"webhook": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().post(format!("{base}/edit_webhook")))
        .json(&json!({"webhook": "http://127.0.0.1/hook"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("ALLOW_PRIVATE_WEBHOOK_URLS"));

    // --- edit_slack_command ---
    let resp = authed(client().post(format!("{base}/edit_slack_command")))
        .json(&json!({"slack_command_script": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_slack_command: {}",
        resp.text().await?
    );

    // --- edit_error_handler (new format) ---
    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "extra_args": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_error_handler: {}",
        resp.text().await?
    );

    // --- edit_success_handler (new format) ---
    let resp = authed(client().post(format!("{base}/edit_success_handler")))
        .json(&json!({"path": null, "extra_args": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_success_handler: {}",
        resp.text().await?
    );

    // --- edit_default_scripts ---
    let resp = authed(client().post(format!("{base}/default_scripts")))
        .json(&json!(null))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_default_scripts: {}",
        resp.text().await?
    );

    // --- edit_default_app (may return 200 or error) ---
    let resp = authed(client().post(format!("{base}/edit_default_app")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_default_app: unexpected status {}",
        resp.status()
    );

    // --- set_environment_variable ---
    let resp = authed(client().post(format!("{base}/set_environment_variable")))
        .json(&json!({"name": "TEST_ENV_VAR", "value": "test_value"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "set_environment_variable: {}",
        resp.text().await?
    );

    // --- edit_large_file_storage_config ---
    let resp = authed(client().post(format!("{base}/edit_large_file_storage_config")))
        .json(&json!({"large_file_storage": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_large_file_storage_config: {}",
        resp.text().await?
    );

    // --- edit_deploy_ui_config ---
    let resp = authed(client().post(format!("{base}/edit_deploy_ui_config")))
        .json(&json!({"deploy_ui": null}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_deploy_ui_config: unexpected status {}",
        resp.status()
    );

    // --- edit_git_sync_config ---
    let resp = authed(client().post(format!("{base}/edit_git_sync_config")))
        .json(&json!({"git_sync_settings": null}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_git_sync_config: unexpected status {}",
        resp.status()
    );

    // --- git_sync_deploy_mode (response shape + default when no git-sync configured) ---
    let resp = authed(client().get(format!("{base}/git_sync_deploy_mode")))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "git_sync_deploy_mode: unexpected status"
    );
    let mode = resp.json::<serde_json::Value>().await?;
    assert_eq!(mode["configured"], json!(false));
    assert_eq!(mode["deploy_on_push"], json!(false));

    // --- update_operator_settings ---
    let resp = authed(client().post(format!("{base}/operator_settings")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "update_operator_settings: {}",
        resp.text().await?
    );

    // --- rebuild_dependency_map ---
    let resp = authed(client().post(format!("{base}/rebuild_dependency_map")))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "rebuild_dependency_map: {}",
        resp.text().await?
    );

    // --- add_user ---
    let resp = authed(client().post(format!("{base}/add_user")))
        .json(&json!({
            "email": "newuser@windmill.dev",
            "is_admin": false,
            "operator": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "add_user: {}", resp.text().await?);

    // ===== Critical alerts (may return 404) =====

    // --- get critical_alerts ---
    let resp = authed(client().get(format!("{base}/critical_alerts")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "critical_alerts: unexpected status {}",
        resp.status()
    );

    // --- acknowledge critical alert (nonexistent id) ---
    let resp = authed(client().post(format!("{base}/critical_alerts/1/acknowledge")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "acknowledge_critical_alert: unexpected status {}",
        resp.status()
    );

    // --- acknowledge_all critical alerts ---
    let resp = authed(client().post(format!("{base}/critical_alerts/acknowledge_all")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "acknowledge_all_critical_alerts: unexpected status {}",
        resp.status()
    );

    // --- mute critical alerts ---
    let resp = authed(client().post(format!("{base}/critical_alerts/mute")))
        .json(&json!({"mute_critical_alerts": false}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "mute_critical_alerts: unexpected status {}",
        resp.status()
    );

    // ===== Tarball export =====

    // --- tarball (download workspace as tar archive) ---
    let resp = authed(client().get(format!("{base}/tarball")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "tarball: {}", resp.status());

    // --- create_workspace_require_superadmin ---
    let resp = authed(client().get(format!("{global_base}/create_workspace_require_superadmin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_copilot_settings_state_reports_instance_ai_fallback_flags(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let instance_ai_config = json!({
        "providers": {
            "openai": {
                "resource_path": "u/test-user/openai_instance",
                "models": ["gpt-4o-mini"]
            }
        },
        "default_model": { "provider": "openai", "model": "gpt-4o-mini" },
        "metadata_model": { "provider": "openai", "model": "gpt-4o-mini" }
    });
    let workspace_ai_config = json!({
        "providers": {
            "anthropic": {
                "resource_path": "u/test-user/anthropic_workspace",
                "models": ["claude-3-5-haiku-latest"]
            }
        }
    });

    sqlx::query("UPDATE workspace_settings SET ai_config = NULL WHERE workspace_id = $1")
        .bind("test-workspace")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind("ai_config")
    .bind(instance_ai_config)
    .execute(&db)
    .await?;

    let resp = authed(client().get(format!("{base}/get_copilot_settings_state")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(settings["has_instance_ai_config"], true);
    assert_eq!(settings["uses_instance_ai_config"], true);
    assert_eq!(
        settings["instance_ai_summary"]["providers"][0]["provider"],
        "openai"
    );
    assert_eq!(
        settings["instance_ai_summary"]["providers"][0]["models"][0],
        "gpt-4o-mini"
    );
    assert_eq!(
        settings["instance_ai_summary"]["metadata_model"]["model"],
        "gpt-4o-mini"
    );

    sqlx::query("UPDATE workspace_settings SET ai_config = $1 WHERE workspace_id = $2")
        .bind(workspace_ai_config)
        .bind("test-workspace")
        .execute(&db)
        .await?;

    let resp = authed(client().get(format!("{base}/get_copilot_settings_state")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(settings["has_instance_ai_config"], true);
    assert_eq!(settings["uses_instance_ai_config"], false);
    assert_eq!(
        settings["instance_ai_summary"]["providers"][0]["provider"],
        "openai"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_copilot_info_ignores_empty_instance_ai_row(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    sqlx::query("UPDATE workspace_settings SET ai_config = NULL WHERE workspace_id = $1")
        .bind("test-workspace")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind("ai_config")
    .bind(json!({}))
    .execute(&db)
    .await?;

    let resp = authed(client().get(format!("{base}/get_copilot_info")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert!(settings["providers"].is_null());

    Ok(())
}

/// A workspace with no provider of its own is served the instance config, but the
/// `copilot_disabled` flag must still come from the workspace's own row.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_copilot_info_keeps_workspace_copilot_disabled_over_instance_fallback(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    sqlx::query("UPDATE workspace_settings SET ai_config = $1 WHERE workspace_id = $2")
        .bind(json!({ "copilot_disabled": true }))
        .bind("test-workspace")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind("ai_config")
    .bind(json!({
        "providers": {
            "openai": {
                "resource_path": "u/test-user/openai_instance",
                "models": ["gpt-4o-mini"]
            }
        }
    }))
    .execute(&db)
    .await?;

    let resp = authed(client().get(format!("{base}/get_copilot_info")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(
        settings["providers"]["openai"]["models"][0], "gpt-4o-mini",
        "instance providers are still served"
    );
    assert_eq!(settings["copilot_disabled"], true);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_error_handler_instance_alerts_fallback(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let stored = || async {
        sqlx::query_scalar!(
            "SELECT error_handler_fallback_to_instance_alerts FROM workspace_settings WHERE workspace_id = 'test-workspace'"
        )
        .fetch_one(&db)
        .await
    };

    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "fallback_to_instance_alerts": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "enable: {}", resp.text().await?);
    assert!(stored().await?);

    // A client that predates the setting (the CLI pushing settings.yaml) omits the field and
    // must not silently turn it back off.
    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "extra_args": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "omitted: {}", resp.text().await?);
    assert!(stored().await?);

    sqlx::query!(
        "UPDATE workspace SET parent_workspace_id = 'test-workspace' WHERE id = 'test-workspace'"
    )
    .execute(&db)
    .await?;
    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "fallback_to_instance_alerts": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "fork must be rejected");

    // The settings page stops offering the option once the workspace is a fork, so its next save
    // sends `false`: that must go through rather than lock the whole error handler behind a 400.
    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "fallback_to_instance_alerts": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "disable on fork: {}",
        resp.text().await?
    );
    assert!(!stored().await?);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let resp = authed(client().get(format!("{base}/get_imports/u/test-user/nonexistent_script")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let imports = resp.json::<Vec<String>>().await?;
    assert!(imports.is_empty());

    Ok(())
}

/// A dbt project names a warehouse and nothing else, so the workspace setting is
/// the only place the connection is decided. Two things have to hold for that to
/// work at all: the setting round-trips as the MAP the resolver reads (an
/// envelope stored verbatim makes every warehouse name unresolvable), and the
/// route that serves the name to a worker with no database stays job-scoped.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_dbt_warehouses(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{base}/workspaces/edit_dbt_warehouses")))
        .json(&json!({
            "dbt_warehouses": { "main": { "resource_path": "u/admin/wh", "target": "prod" } }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(format!("{base}/workspaces/get_settings")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(
        settings["dbt_warehouses"],
        json!({ "main": { "resource_path": "u/admin/wh", "target": "prod" } })
    );

    // A user token is not a job token: the warehouses a workspace configures are
    // a running job's business, not a browsable list.
    let resp = authed(client().get(format!("{base}/dbt/warehouse/main")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    Ok(())
}

/// Admin and operator are one role stored as two flags: granting one must clear the other, and a
/// request asking for both is refused, on update and on add.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_admin_and_operator_are_exclusive(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api/w/test-workspace");
    let flags = async || -> anyhow::Result<(bool, bool)> {
        Ok(sqlx::query_as(
            "SELECT is_admin, operator FROM usr WHERE workspace_id = 'test-workspace' AND username = 'operator'",
        )
        .fetch_one(&db)
        .await?)
    };

    let resp = authed(client().post(format!("{api}/users/update/operator")))
        .json(&json!({"is_admin": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert_eq!(flags().await?, (true, false));

    let resp = authed(client().post(format!("{api}/users/update/operator")))
        .json(&json!({"operator": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert_eq!(flags().await?, (false, true));

    let resp = authed(client().post(format!("{api}/users/update/operator")))
        .json(&json!({"is_admin": true, "operator": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    assert_eq!(flags().await?, (false, true));

    let resp = authed(client().post(format!("{api}/workspaces/add_user")))
        .json(&json!({"email": "both@windmill.dev", "is_admin": true, "operator": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    // Rows written before the rule can still carry both flags: the admin wins, and `whoami`
    // says so, since the frontend hides every gated page from an operator.
    sqlx::query(
        "UPDATE usr SET is_admin = true, operator = true
         WHERE workspace_id = 'test-workspace' AND username = 'operator'",
    )
    .execute(&db)
    .await?;
    let resp = client()
        .get(format!("{api}/users/whoami"))
        .header("Authorization", "Bearer OPERATOR_TOKEN_1")
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(
        (me["is_admin"].as_bool(), me["operator"].as_bool()),
        (Some(true), Some(false))
    );
    let resp = client()
        .post(format!("{api}/scripts/create"))
        .header("Authorization", "Bearer OPERATOR_TOKEN_1")
        .json(&json!({
            "path": "u/operator/legacy_admin", "summary": "", "description": "",
            "content": "export function main() {}", "language": "deno", "schema": {}
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    Ok(())
}

/// specs/single-organisation.md §2: a superadmin creates the instance's one workspace, which
/// every account that exists by then joins.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_the_one_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let create = format!(
        "http://localhost:{}/api/workspaces/create",
        server.addr.port()
    );

    let resp = authed(client().post(&create))
        .json(&json!({"id": "second", "name": "second"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    // With the fixture's workspace archived, the instance has none.
    sqlx::query("UPDATE workspace SET deleted = true WHERE id = 'test-workspace'")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('automate_username_creation', 'false') \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .execute(&db)
    .await?;

    let resp = client()
        .post(&create)
        .bearer_auth("SECRET_TOKEN_2")
        .json(&json!({"id": "main", "name": "main", "username": "jean_luc"}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "only a superadmin: {}",
        resp.text().await?
    );

    let resp = authed(client().post(&create))
        .json(&json!({"id": "main", "name": "main", "username": "jean-luc"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    let resp = authed(client().post(&create))
        .json(&json!({"id": "main", "name": "main", "username": "jean_luc"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let members: Vec<String> =
        sqlx::query_scalar("SELECT email FROM usr WHERE workspace_id = 'main' ORDER BY email")
            .fetch_all(&db)
            .await?;
    assert_eq!(
        members,
        [
            "test2@windmill.dev",
            "test3@windmill.dev",
            "test@windmill.dev"
        ]
    );
    Ok(())
}
