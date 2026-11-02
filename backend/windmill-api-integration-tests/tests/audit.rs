use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_audit_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/audit");

    // GET /list returns 200 (empty array)
    let resp = authed(client().get(format!("{base}/list"))).send().await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /audit/list",
    );

    Ok(())
}

fn bearer(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

async fn list(base: &str, token: &str, query: &str) -> anyhow::Result<(u16, serde_json::Value)> {
    let resp = bearer(client().get(format!("{base}/list?{query}")), token)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    Ok((
        status,
        serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)),
    ))
}

/// specs/audit-logs.md: writes record the real identity, reads follow row-level security,
/// and the filters behave as documented.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_audit_rows_written_and_listed(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    ensure_today_audit_partition(&db).await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api");
    let base = format!("{api}/w/test-workspace/audit");

    // An action that audits under the workspace: creating a variable, as the admin and as
    // a non-admin member.
    for (token, path) in [
        ("SECRET_TOKEN", "u/test-user/by_admin"),
        ("SECRET_TOKEN_2", "u/test-user-2/by_member"),
    ] {
        let resp = bearer(
            client().post(format!("{api}/w/test-workspace/variables/create")),
            token,
        )
        .json(&serde_json::json!({
            "path": path, "value": "v", "is_secret": false, "description": ""
        }))
        .send()
        .await?;
        assert_2xx(
            resp.status().as_u16(),
            &resp.text().await?,
            "variables/create",
        );
    }

    // The admin sees both rows, newest first.
    let (status, rows) = list(&base, "SECRET_TOKEN", "operation=variables.create").await?;
    assert_eq!(status, 200, "{rows}");
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["resource"], "u/test-user-2/by_member");
    assert_eq!(rows[0]["username"], "test-user-2");
    assert_eq!(rows[0]["action_kind"], "Create");
    assert!(rows[0]["id"].as_i64() > rows[1]["id"].as_i64());

    // A member sees only their own.
    let (status, rows) = list(&base, "SECRET_TOKEN_2", "operation=variables.create").await?;
    assert_eq!(status, 200, "{rows}");
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["username"], "test-user-2");

    // Filters.
    let (_, rows) = list(
        &base,
        "SECRET_TOKEN",
        "operation=variables.&resource=u/test-user/",
    )
    .await?;
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    let (_, rows) = list(
        &base,
        "SECRET_TOKEN",
        "operation=variables.create&username=test-user",
    )
    .await?;
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    let (_, rows) = list(
        &base,
        "SECRET_TOKEN",
        "operations=variables.create&action_kind=delete",
    )
    .await?;
    assert_eq!(rows.as_array().unwrap().len(), 0, "{rows}");
    let (_, rows) = list(
        &base,
        "SECRET_TOKEN",
        "exclude_operations=variables.create&resource=u/test-user",
    )
    .await?;
    assert_eq!(rows.as_array().unwrap().len(), 0, "{rows}");
    let (status, _) = list(&base, "SECRET_TOKEN", "action_kind=drop").await?;
    assert_eq!(status, 400);
    // A LIKE wildcard in a prefix filter is literal.
    let (_, rows) = list(&base, "SECRET_TOKEN", "operation=%25").await?;
    assert_eq!(rows.as_array().unwrap().len(), 0, "{rows}");

    // Keyset cursor.
    let (_, all) = list(&base, "SECRET_TOKEN", "operation=variables.create").await?;
    let newest = all[0]["id"].as_i64().unwrap();
    let (_, rows) = list(
        &base,
        "SECRET_TOKEN",
        &format!("operation=variables.create&before_id={newest}"),
    )
    .await?;
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");

    // A single row, and another workspace's id is not found through this one.
    let resp = bearer(client().get(format!("{base}/get/{newest}")), "SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let resp = bearer(
        client().get(format!("{api}/w/no-such-workspace/audit/get/{newest}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_ne!(resp.status(), 200);

    // Every workspace at once is for superadmins only.
    let (status, _) = list(&base, "SECRET_TOKEN_2", "all_workspaces=true").await?;
    assert_eq!(status, 401);
    let (status, rows) = list(&base, "SECRET_TOKEN", "all_workspaces=true").await?;
    assert_eq!(status, 200, "{rows}");

    Ok(())
}
