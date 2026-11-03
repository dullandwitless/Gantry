//! Workspace service accounts: creation and impersonation (specs/service-accounts.md).

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";

fn post(base: &str, path: &str, token: &str, body: Value) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post(format!("{base}{path}"))
        .bearer_auth(token)
        .json(&body)
}

async fn create(base: &str, token: &str, body: Value) -> anyhow::Result<(u16, String)> {
    let res = post(
        base,
        &format!("/w/{WS}/workspaces/create_service_account"),
        token,
        body,
    )
    .send()
    .await?;
    Ok((res.status().as_u16(), res.text().await?))
}

async fn impersonate(base: &str, token: &str, username: &str) -> anyhow::Result<(u16, String)> {
    let res = post(
        base,
        &format!("/w/{WS}/users/impersonate_service_account"),
        token,
        json!({ "username": username }),
    )
    .send()
    .await?;
    Ok((res.status().as_u16(), res.text().await?))
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn admins_create_service_accounts(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    // Only workspace admins.
    let (status, body) = create(&base, "SECRET_TOKEN_2", json!({ "username": "ci-bot" })).await?;
    assert!(
        !(200..300).contains(&status),
        "non-admin create: {status} {body}"
    );

    let (status, body) = create(
        &base,
        "SECRET_TOKEN",
        json!({ "username": "ci-bot", "operator": false, "add_to_deployers": true }),
    )
    .await?;
    assert_eq!(status, 201, "{body}");

    let (email, operator, is_admin, service): (String, bool, bool, bool) = sqlx::query_as(
        "SELECT email, operator, is_admin, is_service_account FROM usr
         WHERE workspace_id = $1 AND username = 'ci-bot'",
    )
    .bind(WS)
    .fetch_one(&db)
    .await?;
    assert_eq!(
        (email.as_str(), operator, is_admin, service),
        ("ci-bot@test-workspace.sa.invalid", false, false, true)
    );
    let mut groups: Vec<String> = sqlx::query_scalar(
        "SELECT group_ FROM usr_to_group WHERE workspace_id = $1 AND usr = 'ci-bot'",
    )
    .bind(WS)
    .fetch_all(&db)
    .await?;
    groups.sort();
    assert_eq!(groups, vec!["all", "wm_deployers"]);

    // Operators by default, never in wm_deployers unless asked.
    let (status, body) = create(&base, "SECRET_TOKEN", json!({ "username": "runner" })).await?;
    assert_eq!(status, 201, "{body}");
    let operator: bool = sqlx::query_scalar(
        "SELECT operator FROM usr WHERE workspace_id = $1 AND username = 'runner'",
    )
    .bind(WS)
    .fetch_one(&db)
    .await?;
    assert!(operator);

    // Refusals: a taken username (here or anywhere on the instance), admin and operator at once.
    for body in [
        json!({ "username": "ci-bot" }),
        json!({ "username": "test-user-2" }),
        json!({ "username": "admin-bot", "is_admin": true, "operator": true }),
    ] {
        let (status, text) = create(&base, "SECRET_TOKEN", body.clone()).await?;
        assert_eq!(status, 400, "{body}: {text}");
    }
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn admins_impersonate_service_accounts(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let (status, body) = create(&base, "SECRET_TOKEN", json!({ "username": "ci-bot" })).await?;
    assert_eq!(status, 201, "{body}");

    let (status, body) = impersonate(&base, "SECRET_TOKEN_2", "ci-bot").await?;
    assert!(
        !(200..300).contains(&status),
        "non-admin impersonation: {status} {body}"
    );
    assert_eq!(impersonate(&base, "SECRET_TOKEN", "nobody").await?.0, 404);
    assert_eq!(
        impersonate(&base, "SECRET_TOKEN", "test-user-2").await?.0,
        404,
        "people are not service accounts"
    );

    let (status, token) = impersonate(&base, "SECRET_TOKEN", "ci-bot").await?;
    assert_eq!(status, 201, "{token}");
    let whoami: Value = reqwest::Client::new()
        .get(format!("{base}/w/{WS}/users/whoami"))
        .bearer_auth(&token)
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(whoami["username"], "ci-bot", "{whoami}");

    let (label, workspace, lifetime_h): (String, Option<String>, f64) = sqlx::query_as(
        "SELECT label, workspace_id, extract(epoch FROM expiration - now())::float8 / 3600
         FROM token WHERE token_prefix = $1",
    )
    .bind(&token[..10])
    .fetch_one(&db)
    .await?;
    assert_eq!(label, "impersonation:test@windmill.dev");
    assert_eq!(workspace.as_deref(), Some(WS));
    assert!((23.9..=24.0).contains(&lifetime_h), "{lifetime_h}");

    // A disabled service account cannot be impersonated.
    sqlx::query("UPDATE usr SET disabled = true WHERE workspace_id = $1 AND username = 'ci-bot'")
        .bind(WS)
        .execute(&db)
        .await?;
    assert_eq!(impersonate(&base, "SECRET_TOKEN", "ci-bot").await?.0, 404);
    Ok(())
}
