//! Sign-in through Identity-Aware Proxy and with mock users (`specs/identity-aware-proxy.md`),
//! end to end, with a stand-in for IAP's key set.
//!
//! Run with:
//!   cargo test --test identity_proxy

use axum::{routing::get, Json, Router};
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::ApiServer;

const AUDIENCE: &str = "/projects/123/global/backendServices/456";
const KID: &str = "test-key";

// A P-256 key pair for tests only.
const PUBLIC_PEM: &str = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEzAfqyCh34iYOCW0vg4ejq/zzJlzLSZScjnVyPjLGTapEwo4gc6/y1Yudd/v54wKh0OdfTfzAKMPWx/2NWx/ugg==";
const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgu27S2DbSwUh8BmQb\n/i4/VhNdoXV7PJekhnoceMULYLihRANCAATMB+rIKHfiJg4JbS+Dh6Or/PMmXMtJ\nlJyOdXI+MsZNqkTCjiBzr/LVi513+/njAqHQ519N/MAow9bH/Y1bH+6C\n-----END PRIVATE KEY-----\n";

/// The test key as a JWK: the SPKI encoding ends with the uncompressed point's x and y.
fn public_jwk() -> Value {
    let der = base64::engine::general_purpose::STANDARD
        .decode(PUBLIC_PEM)
        .unwrap();
    let (x, y) = der[der.len() - 64..].split_at(32);
    let b64 = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
    json!({ "kty": "EC", "crv": "P-256", "alg": "ES256", "use": "sig", "kid": KID, "x": b64(x), "y": b64(y) })
}

/// Serves the test key as IAP's key set and points Gantry at it.
async fn serve_keys() {
    let app = Router::new().route(
        "/keys",
        get(|| async { Json(json!({ "keys": [public_jwk()] })) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    std::env::set_var(
        "IAP_JWKS_URL",
        format!("http://{}/keys", listener.local_addr().unwrap()),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
}

fn assertion(email: &str, audience: &str, expires_in: i64) -> String {
    let now = jsonwebtoken::get_current_timestamp() as i64;
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(KID.to_string());
    encode(
        &header,
        &json!({
            "iss": "https://cloud.google.com/iap",
            "aud": audience,
            "email": email,
            "sub": "accounts.google.com:1",
            "iat": now - 5,
            "exp": now + expires_in,
        }),
        &EncodingKey::from_ec_pem(PRIVATE_PEM.as_bytes()).unwrap(),
    )
    .unwrap()
}

async fn iap_login(api: &str, assertion: Option<&str>) -> (u16, String) {
    let mut req = reqwest::Client::new().post(format!("{api}/auth/iap"));
    if let Some(a) = assertion {
        req = req.header("x-goog-iap-jwt-assertion", a);
    }
    let res = req.send().await.unwrap();
    (res.status().as_u16(), res.text().await.unwrap())
}

async fn mock_login(api: &str, email: &str) -> (u16, String) {
    let res = reqwest::Client::new()
        .post(format!("{api}/auth/mock_login"))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    (res.status().as_u16(), res.text().await.unwrap())
}

async fn get_json(url: String, token: Option<&str>) -> Value {
    let mut req = reqwest::Client::new().get(url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    req.send().await.unwrap().json().await.unwrap()
}

async fn login_type(db: &Pool<Postgres>, email: &str) -> Option<String> {
    sqlx::query_scalar("SELECT login_type FROM password WHERE email = $1")
        .bind(email)
        .fetch_optional(db)
        .await
        .unwrap()
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn iap_signs_in_the_user_the_proxy_vouches_for(db: Pool<Postgres>) {
    serve_keys().await;
    std::env::set_var("IAP_AUDIENCE", AUDIENCE);
    std::env::set_var("GANTRY_MOCK_USERS", "dev@example.com");
    let server = ApiServer::start(db.clone()).await.unwrap();
    windmill_common::BASE_URL.store(std::sync::Arc::new("http://localhost:8000".to_string()));
    let api = format!("http://localhost:{}/api", server.addr.port());

    // IAP on: mock users are never offered, even on a local base URL.
    assert_eq!(
        get_json(format!("{api}/auth/sign_in_options"), None).await,
        json!({ "iap": true, "mock_users": [] })
    );
    assert_eq!(mock_login(&api, "dev@example.com").await.0, 404);

    // A first sign-in creates the account.
    let (status, token) = iap_login(&api, Some(&assertion("Ada@Example.com", AUDIENCE, 600))).await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(
        get_json(format!("{api}/users/whoami"), Some(&token)).await["email"],
        "ada@example.com"
    );
    assert_eq!(
        login_type(&db, "ada@example.com").await.as_deref(),
        Some("iap")
    );

    // An existing account keeps its rights and login type.
    let (status, token) =
        iap_login(&api, Some(&assertion("test@windmill.dev", AUDIENCE, 600))).await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(
        get_json(format!("{api}/users/whoami"), Some(&token)).await["super_admin"],
        true
    );
    assert_eq!(
        login_type(&db, "test@windmill.dev").await.as_deref(),
        Some("password")
    );

    // Refusals: no header, another audience, expired, disabled account.
    assert_eq!(iap_login(&api, None).await.0, 401);
    assert_eq!(
        iap_login(
            &api,
            Some(&assertion("ada@example.com", "/projects/9/apps/other", 600))
        )
        .await
        .0,
        401
    );
    assert_eq!(
        iap_login(&api, Some(&assertion("ada@example.com", AUDIENCE, -120)))
            .await
            .0,
        401
    );
    sqlx::query("UPDATE password SET disabled = true WHERE email = 'ada@example.com'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        iap_login(&api, Some(&assertion("ada@example.com", AUDIENCE, 600)))
            .await
            .0,
        403
    );

    std::env::remove_var("IAP_AUDIENCE");
    std::env::remove_var("GANTRY_MOCK_USERS");
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn mock_users_only_on_a_local_instance(db: Pool<Postgres>) {
    std::env::remove_var("IAP_AUDIENCE");
    std::env::set_var("GANTRY_MOCK_USERS", "dev@example.com, Test@windmill.dev");
    let server = ApiServer::start(db.clone()).await.unwrap();
    windmill_common::BASE_URL.store(std::sync::Arc::new("http://localhost:8000".to_string()));
    let api = format!("http://localhost:{}/api", server.addr.port());

    assert_eq!(
        get_json(format!("{api}/auth/sign_in_options"), None).await,
        json!({ "iap": false, "mock_users": ["dev@example.com", "test@windmill.dev"] })
    );
    assert_eq!(iap_login(&api, None).await.0, 404, "IAP is off");

    let (status, token) = mock_login(&api, "dev@example.com").await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(
        login_type(&db, "dev@example.com").await.as_deref(),
        Some("mock")
    );
    let (status, token) = mock_login(&api, "test@windmill.dev").await;
    assert_eq!(status, 200, "{token}");
    assert_eq!(
        get_json(format!("{api}/users/whoami"), Some(&token)).await["super_admin"],
        true
    );
    assert_eq!(mock_login(&api, "someone@example.com").await.0, 400);

    // A deployed base URL turns them off.
    windmill_common::BASE_URL.store(std::sync::Arc::new(
        "https://gantry.example.com".to_string(),
    ));
    assert_eq!(
        get_json(format!("{api}/auth/sign_in_options"), None).await,
        json!({ "iap": false, "mock_users": [] })
    );
    assert_eq!(mock_login(&api, "dev@example.com").await.0, 404);

    std::env::remove_var("GANTRY_MOCK_USERS");
}
