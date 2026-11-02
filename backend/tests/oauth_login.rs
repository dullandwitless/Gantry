//! Sign-in with an OAuth provider, end to end against a stand-in provider
//! (`specs/sso-oauth-login.md`).
//!
//! Run with:
//!   cargo test --features oauth2 --test oauth_login
#![cfg(feature = "oauth2")]

use std::sync::atomic::Ordering;

use axum::{
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_api::{oauth2_oss::build_oauth_clients, OAUTH_CLIENTS};
use windmill_common::oauth2::REQUIRE_PREEXISTING_USER_FOR_OAUTH;
use windmill_test_utils::ApiServer;

const ACCESS_TOKEN: &str = "provider-access-token";

/// A provider whose token endpoint hands out `ACCESS_TOKEN` and whose userinfo endpoint,
/// given it, answers `userinfo`.
async fn provider(userinfo: Value) -> String {
    let app = Router::new()
        .route(
            "/token",
            post(|| async {
                Json(json!({ "access_token": ACCESS_TOKEN, "token_type": "bearer" }))
            }),
        )
        .route(
            "/userinfo",
            get(move |headers: HeaderMap| async move {
                let bearer = format!("Bearer {ACCESS_TOKEN}");
                if headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(&bearer) {
                    Ok(Json(userinfo))
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    base
}

/// Configures the provider `stub` at `provider_base` as the instance's only sign-in provider.
async fn configure(db: &Pool<Postgres>, provider_base: &str, allowed_domains: Option<Vec<&str>>) {
    let oauths = serde_json::from_value(json!({
        "stub": {
            "id": "client-id",
            "secret": "client-secret",
            "display_name": "Stub SSO",
            "allowed_domains": allowed_domains,
            "login_config": {
                "auth_url": format!("{provider_base}/authorize"),
                "token_url": format!("{provider_base}/token"),
                "userinfo_url": format!("{provider_base}/userinfo"),
                "scopes": ["email"]
            }
        }
    }))
    .unwrap();
    let clients = build_oauth_clients("http://gantry.test", Some(oauths), db)
        .await
        .unwrap();
    OAUTH_CLIENTS.store(std::sync::Arc::new(clients));
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

/// Starts a sign-in and returns the `state` the server put in both the redirect and the cookie.
async fn start(api: &str) -> String {
    let res = http()
        .get(format!("{api}/oauth/login/stub"))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_redirection(), "{}", res.status());
    let location = res.headers()["location"].to_str().unwrap().to_string();
    let cookie = res.headers()["set-cookie"].to_str().unwrap().to_string();
    let state = url::Url::parse(&location)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .expect("state in the authorize URL");
    assert!(location.contains("/authorize?"), "{location}");
    assert!(cookie.starts_with(&format!("csrf={state}")), "{cookie}");
    state
}

/// Completes the sign-in as the browser would, with `cookie_state` in the CSRF cookie.
async fn callback(api: &str, state: &str, cookie_state: &str) -> (u16, String) {
    let res = http()
        .post(format!("{api}/oauth/login_callback/stub"))
        .header("cookie", format!("csrf={cookie_state}"))
        .json(&json!({ "code": "the-code", "state": state }))
        .send()
        .await
        .unwrap();
    (res.status().as_u16(), res.text().await.unwrap())
}

async fn whoami(api: &str, token: &str) -> Value {
    http()
        .get(format!("{api}/users/whoami"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn first_sign_in_creates_the_account_and_signs_in(db: Pool<Postgres>) {
    REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(false, Ordering::Relaxed);
    let server = ApiServer::start(db.clone()).await.unwrap();
    let api = format!("http://localhost:{}/api", server.addr.port());
    configure(
        &db,
        &provider(json!({ "email": "Ada@Example.com", "name": "Ada" })).await,
        None,
    )
    .await;

    let logins: Value = http()
        .get(format!("{api}/oauth/list_logins"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        logins,
        json!({ "oauth": [{ "type": "stub", "display_name": "Stub SSO" }], "saml": null, "auto_login": null })
    );

    let state = start(&api).await;
    let (status, token) = callback(&api, &state, &state).await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(whoami(&api, &token).await["email"], "ada@example.com");

    let (login_type, name, username, super_admin): (String, Option<String>, Option<String>, bool) =
        sqlx::query_as(
            "SELECT login_type, name, username, super_admin FROM password WHERE email = $1",
        )
        .bind("ada@example.com")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(
        (login_type.as_str(), name.as_deref(), super_admin),
        ("stub", Some("Ada"), false)
    );
    assert!(
        username.is_some(),
        "the new account gets an instance username"
    );

    // Signing in again reuses the account.
    let state = start(&api).await;
    assert_eq!(callback(&api, &state, &state).await.0, 200);
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM password WHERE email = $1")
        .bind("ada@example.com")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(accounts, 1);
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn an_existing_account_keeps_its_rights_and_login_type(db: Pool<Postgres>) {
    REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(true, Ordering::Relaxed);
    let server = ApiServer::start(db.clone()).await.unwrap();
    let api = format!("http://localhost:{}/api", server.addr.port());
    configure(
        &db,
        &provider(json!({ "email": "test@windmill.dev" })).await,
        None,
    )
    .await;

    let state = start(&api).await;
    let (status, token) = callback(&api, &state, &state).await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(whoami(&api, &token).await["super_admin"], true);
    let login_type: String =
        sqlx::query_scalar("SELECT login_type FROM password WHERE email = 'test@windmill.dev'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(login_type, "password");
    REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(false, Ordering::Relaxed);
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn refusals(db: Pool<Postgres>) {
    let server = ApiServer::start(db.clone()).await.unwrap();
    let api = format!("http://localhost:{}/api", server.addr.port());
    let unknown = provider(json!({ "email": "new@example.com" })).await;

    // A forged state.
    configure(&db, &unknown, None).await;
    let state = start(&api).await;
    assert_eq!(callback(&api, &state, "another-state").await.0, 400);

    // No account, and accounts must exist beforehand.
    REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(true, Ordering::Relaxed);
    let state = start(&api).await;
    let (status, body) = callback(&api, &state, &state).await;
    REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(false, Ordering::Relaxed);
    assert_eq!(status, 403, "{body}");

    // A domain outside allowed_domains.
    configure(&db, &unknown, Some(vec!["windmill.dev"])).await;
    let state = start(&api).await;
    assert_eq!(callback(&api, &state, &state).await.0, 403);

    // An unverified email.
    configure(
        &db,
        &provider(json!({ "email": "x@example.com", "email_verified": false })).await,
        None,
    )
    .await;
    let state = start(&api).await;
    assert_eq!(callback(&api, &state, &state).await.0, 400);

    // A disabled account.
    sqlx::query("UPDATE password SET disabled = true WHERE email = 'test@windmill.dev'")
        .execute(&db)
        .await
        .unwrap();
    configure(
        &db,
        &provider(json!({ "email": "test@windmill.dev" })).await,
        None,
    )
    .await;
    let state = start(&api).await;
    assert_eq!(callback(&api, &state, &state).await.0, 403);

    let created: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM password WHERE email IN ('new@example.com', 'x@example.com')",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(created, 0, "no refused sign-in creates an account");
}
