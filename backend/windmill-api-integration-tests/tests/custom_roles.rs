//! Custom instance roles and workspace-only roles (specs/custom-roles.md).

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";

async fn set_instance_roles(base: &str, roles: Value) -> anyhow::Result<(u16, String)> {
    let res = reqwest::Client::new()
        .post(format!("{base}/settings/global/workspace_roles"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({ "value": roles }))
        .send()
        .await?;
    Ok((res.status().as_u16(), res.text().await?))
}

async fn set_overrides(base: &str, overrides: Value) -> anyhow::Result<(u16, String)> {
    let res = reqwest::Client::new()
        .post(format!("{base}/w/{WS}/workspaces/role_overrides"))
        .bearer_auth("SECRET_TOKEN")
        .json(&overrides)
        .send()
        .await?;
    Ok((res.status().as_u16(), res.text().await?))
}

/// The workspace's roles as (name, source), and whether it may add roles.
async fn workspace_roles(base: &str) -> anyhow::Result<(Vec<(String, String)>, bool)> {
    let body: Value = reqwest::Client::new()
        .get(format!("{base}/w/{WS}/workspaces/roles"))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    let roles = body["roles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["name"].as_str().unwrap().to_string(),
                r["source"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    Ok((roles, body["custom_roles_enabled"] == json!(true)))
}

fn roles(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(n, s)| (n.to_string(), s.to_string()))
        .collect()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn instance_and_workspace_roles_can_be_customised(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    let (listed, enabled) = workspace_roles(&base).await?;
    assert!(enabled);
    assert_eq!(
        listed,
        roles(&[("developer", "instance"), ("operator", "instance")])
    );

    // A superadmin adds an instance role.
    let (status, body) = set_instance_roles(
        &base,
        json!([
            { "name": "developer", "label": "Developer" },
            { "name": "operator", "label": "Operator" },
            { "name": "auditor", "label": "Auditor", "capabilities": { "manage_schedules": true } }
        ]),
    )
    .await?;
    assert_eq!(status, 200, "{body}");

    // A workspace admin adds a role of the workspace's own.
    let (status, body) = set_overrides(
        &base,
        json!({ "reviewer": { "label": "Reviewer", "capabilities": { "manage_triggers": true } } }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");

    let (listed, _) = workspace_roles(&base).await?;
    assert_eq!(
        listed,
        roles(&[
            ("developer", "instance"),
            ("operator", "instance"),
            ("auditor", "instance"),
            ("reviewer", "workspace"),
        ])
    );

    // A role members still hold cannot be removed: the fixture's members are developers.
    let (status, body) = set_instance_roles(&base, json!([{ "name": "auditor" }])).await?;
    assert_eq!(status, 400, "{body}");
    Ok(())
}
