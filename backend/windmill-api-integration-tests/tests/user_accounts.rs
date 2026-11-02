//! Global user creation, password setting and the global settings listing
//! (specs/users-and-email.md, sections 1, 2 and 5).

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn bearer(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

async fn login(base: &str, email: &str, password: &str) -> anyhow::Result<reqwest::StatusCode> {
    Ok(client()
        .post(format!("{base}/auth/login"))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await?
        .status())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_user_then_change_password(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    // Only a superadmin creates users.
    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({ "email": "new@example.com", "password": "pw-1", "super_admin": false }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "non-superadmin create: {}",
        resp.text().await?
    );

    // A password account needs a password.
    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "email": "new@example.com", "super_admin": false }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "create without password: {}",
        resp.text().await?
    );

    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "email": "  New@Example.com ",
        "password": "pw-1",
        "super_admin": false,
        "name": "New User",
        "skip_email": true,
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let (login_type, hash, verified, username): (String, String, bool, Option<String>) =
        sqlx::query_as(
            "SELECT login_type, password_hash, verified, username FROM password WHERE email = $1",
        )
        .bind("new@example.com")
        .fetch_one(&db)
        .await?;
    assert_eq!(login_type, "password");
    assert!(hash.starts_with("$argon2id$"), "{hash}");
    assert!(verified);
    assert_eq!(username.as_deref(), Some("new"));

    // The address is stored lowercased, so a second create is a conflict.
    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "email": "new@example.com", "password": "pw-2", "super_admin": false }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "duplicate create: {}",
        resp.text().await?
    );

    assert_eq!(login(&base, "new@example.com", "pw-1").await?, 200);
    assert_eq!(login(&base, "new@example.com", "wrong").await?, 400);

    // A superadmin sets someone else's password.
    let resp = bearer(
        client().post(format!("{base}/users/set_password_of/new@example.com")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "password": "pw-2" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "set_password_of: {}",
        resp.text().await?
    );
    assert_eq!(login(&base, "new@example.com", "pw-1").await?, 400);
    assert_eq!(login(&base, "new@example.com", "pw-2").await?, 200);

    // An empty password is refused.
    let resp = bearer(
        client().post(format!("{base}/users/set_password_of/new@example.com")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "password": "" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 400, "empty password: {}", resp.text().await?);

    // A user sets their own password.
    let resp = bearer(
        client().post(format!("{base}/users/setpassword")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({ "password": "own-pw" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "setpassword: {}", resp.text().await?);
    assert_eq!(login(&base, "test2@windmill.dev", "own-pw").await?, 200);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_user_adopts_reserved_username(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());

    // Invited to a workspace before having an account: the username is reserved.
    sqlx::query("INSERT INTO pending_user (email, username) VALUES ($1, $2)")
        .bind("invited@example.com")
        .bind("invited_reserved")
        .execute(&db)
        .await?;

    let resp = bearer(
        client().post(format!("{base}/users/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "email": "invited@example.com",
        "login_type": "pending_oauth",
        "super_admin": false,
        "skip_email": true,
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let (login_type, hash, username): (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT login_type, password_hash, username FROM password WHERE email = $1")
            .bind("invited@example.com")
            .fetch_one(&db)
            .await?;
    assert_eq!(login_type, "pending_oauth");
    assert_eq!(hash, None);
    assert_eq!(username.as_deref(), Some("invited_reserved"));

    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pending_user WHERE email = $1")
        .bind("invited@example.com")
        .fetch_one(&db)
        .await?;
    assert_eq!(pending, 0, "the reservation is consumed by the account");

    // Setting a password turns the pending account into a password account.
    let resp = bearer(
        client().post(format!("{base}/users/set_password_of/invited@example.com")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "password": "now-a-password" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "set_password_of: {}",
        resp.text().await?
    );
    assert_eq!(
        login(&base, "invited@example.com", "now-a-password").await?,
        200
    );

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
