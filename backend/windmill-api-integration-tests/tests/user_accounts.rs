//! Global user creation and the global settings listing (specs/users-and-email.md, sections 1
//! and 5; specs/single-organisation.md §1: accounts have no password).

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn bearer(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_user_ahead_of_sign_in(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let create = |token: &'static str, body: serde_json::Value| {
        bearer(client().post(format!("{base}/users/create")), token)
            .json(&body)
            .send()
    };

    // Only a superadmin creates users.
    let resp = create(
        "SECRET_TOKEN_2",
        json!({ "email": "new@example.com", "super_admin": false }),
    )
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "non-superadmin create: {}",
        resp.text().await?
    );

    // Passwords and login types are refused, not ignored.
    for extra in [
        json!({ "password": "pw-1" }),
        json!({ "login_type": "password" }),
    ] {
        let mut body = json!({ "email": "new@example.com", "super_admin": false });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let resp = create("SECRET_TOKEN", body).await?;
        assert!(
            resp.status().is_client_error(),
            "{}: {}",
            resp.status(),
            resp.text().await?
        );
    }

    let resp = create(
        "SECRET_TOKEN",
        json!({ "email": "  New@Example.com ", "super_admin": false, "name": "New User" }),
    )
    .await?;
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let (login_type, hash, verified, username): (String, Option<String>, bool, Option<String>) =
        sqlx::query_as(
            "SELECT login_type, password_hash, verified, username FROM password WHERE email = $1",
        )
        .bind("new@example.com")
        .fetch_one(&db)
        .await?;
    assert_eq!(login_type, "pending_oauth", "the first sign-in sets it");
    assert_eq!(hash, None);
    assert!(verified);
    assert_eq!(username.as_deref(), Some("new"));

    // The address is stored lowercased, so a second create is a conflict.
    let resp = create(
        "SECRET_TOKEN",
        json!({ "email": "new@example.com", "super_admin": false }),
    )
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "duplicate create: {}",
        resp.text().await?
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_user_adopts_reserved_username(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    // Added to a workspace before having an account: the username is reserved.
    sqlx::query("INSERT INTO pending_user (email, username) VALUES ($1, $2)")
        .bind("added@example.com")
        .bind("added_reserved")
        .execute(&db)
        .await?;

    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "email": "added@example.com", "super_admin": false }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let username: Option<String> =
        sqlx::query_scalar("SELECT username FROM password WHERE email = $1")
            .bind("added@example.com")
            .fetch_one(&db)
            .await?;
    assert_eq!(username.as_deref(), Some("added_reserved"));

    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pending_user WHERE email = $1")
        .bind("added@example.com")
        .fetch_one(&db)
        .await?;
    assert_eq!(pending, 0, "the reservation is consumed by the account");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_global_settings(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES
            ('base_url', '\"https://gantry.example.com\"'),
            ('automate_username_creation', 'true')
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .execute(&db)
    .await?;

    let resp = bearer(
        client().get(format!("{base}/settings/list_global")),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "non-superadmin list: {}",
        resp.text().await?
    );

    let resp = bearer(
        client().get(format!("{base}/settings/list_global")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let settings: Vec<serde_json::Value> = resp.json().await?;
    let names: Vec<&str> = settings.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(
        settings
            .iter()
            .any(|s| s["name"] == "base_url" && s["value"] == "https://gantry.example.com"),
        "{settings:?}"
    );
    assert!(
        !names.contains(&"automate_username_creation"),
        "hidden settings are not listed"
    );
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "ordered by name");

    Ok(())
}
