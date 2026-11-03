//! Google Cloud Secret Manager as the secret backend (specs/gcp-secret-manager.md), against a
//! local stand-in for the metadata server and the Secret Manager API.
//!
//! Run with:
//!   cargo test -p windmill-common --test gcp_secret_manager

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    body::Bytes,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_common::{
    secret_backend::{
        gcp_secret_manager::{
            migrate_secrets_from_gcp_secret_manager, migrate_secrets_to_gcp_secret_manager,
            secret_id,
        },
        get_secret_value, GcpSecretManagerBackend, GcpSecretManagerSettings, SecretBackend,
    },
    variables::{build_crypt, encrypt},
};

const PROJECT: &str = "test-project";
const TOKEN: &str = "metadata-token";

/// Secret id -> versions (payload, destroyed), the last being the latest.
type Store = Arc<Mutex<HashMap<String, Vec<(String, bool)>>>>;

/// Serves the metadata token endpoint and the part of the Secret Manager API the backend uses.
async fn stand_in(store: State<Store>, req: Request) -> impl IntoResponse {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().unwrap_or_default().to_string();
    let body: Bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();

    if path == "/computeMetadata/v1/instance/service-accounts/default/token" {
        assert_eq!(parts.headers["metadata-flavor"], "Google");
        return (
            StatusCode::OK,
            Json(json!({ "access_token": TOKEN, "expires_in": 3600 })),
        );
    }
    assert_eq!(
        parts.headers["authorization"],
        format!("Bearer {TOKEN}").as_str()
    );
    let rest = path
        .strip_prefix(&format!("/v1/projects/{PROJECT}/"))
        .expect("project path");
    let mut store = store.lock().unwrap();
    let not_found = (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": { "message": "not found" } })),
    );

    match (parts.method, rest) {
        (Method::GET, "secrets") => (StatusCode::OK, Json(json!({ "secrets": [] }))),
        (Method::POST, "secrets") => {
            let id = query.strip_prefix("secretId=").unwrap().to_string();
            let created: Value = serde_json::from_slice(&body).unwrap();
            assert!(created["annotations"]["gantry-path"].is_string());
            if store.contains_key(&id) {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": { "message": "exists" } })),
                );
            }
            store.insert(id, vec![]);
            (StatusCode::OK, Json(json!({})))
        }
        (method, rest) => {
            let rest = rest.strip_prefix("secrets/").unwrap();
            if let Some(id) = rest.strip_suffix(":addVersion") {
                let Some(versions) = store.get_mut(id) else {
                    return not_found;
                };
                let added: Value = serde_json::from_slice(&body).unwrap();
                versions.push((
                    added["payload"]["data"].as_str().unwrap().to_string(),
                    false,
                ));
                let name = format!(
                    "projects/{PROJECT}/secrets/{id}/versions/{}",
                    versions.len()
                );
                return (StatusCode::OK, Json(json!({ "name": name })));
            }
            if let Some(id) = rest.strip_suffix("/versions/latest:access") {
                return match store.get(id).and_then(|v| v.last()) {
                    Some((data, false)) => {
                        (StatusCode::OK, Json(json!({ "payload": { "data": data } })))
                    }
                    _ => not_found,
                };
            }
            if let Some(id) = rest.strip_suffix("/versions") {
                let Some(versions) = store.get(id) else {
                    return not_found;
                };
                let listed: Vec<Value> = versions
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, destroyed))| !destroyed)
                    .map(|(i, _)| {
                        json!({ "name": format!("projects/{PROJECT}/secrets/{id}/versions/{}", i + 1) })
                    })
                    .collect();
                return (StatusCode::OK, Json(json!({ "versions": listed })));
            }
            if let Some(version) = rest.strip_suffix(":destroy") {
                let (id, n) = version.split_once("/versions/").unwrap();
                let n: usize = n.parse().unwrap();
                store.get_mut(id).unwrap()[n - 1].1 = true;
                return (StatusCode::OK, Json(json!({})));
            }
            assert_eq!(method, Method::DELETE, "unexpected call to {rest}");
            match store.remove(rest) {
                Some(_) => (StatusCode::OK, Json(json!({}))),
                None => not_found,
            }
        }
    }
}

/// Starts the stand-in, points the metadata host at it, and answers backend settings for it.
async fn start(store: Store) -> GcpSecretManagerSettings {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    std::env::set_var("GCE_METADATA_HOST", addr.to_string());
    let app = Router::new().fallback(stand_in).with_state(store);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    GcpSecretManagerSettings {
        project_id: PROJECT.to_string(),
        prefix: Some("gantry-".to_string()),
        endpoint: Some(format!("http://{addr}")),
    }
}

/// Gives the fixture's placeholder secrets real ciphertext; answers (workspace, path, plaintext).
async fn seed(db: &Pool<Postgres>) -> Vec<(String, String, String)> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT workspace_id, path FROM variable WHERE is_secret ORDER BY path")
            .fetch_all(db)
            .await
            .unwrap();
    let mut seeded = vec![];
    for (w, path) in rows {
        let plain = format!("value of {path}");
        let mc = build_crypt(db, &w).await.unwrap();
        sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
            .bind(encrypt(&mc, &plain))
            .bind(&w)
            .bind(&path)
            .execute(db)
            .await
            .unwrap();
        seeded.push((w, path, plain));
    }
    seeded
}

async fn configure(db: &Pool<Postgres>, settings: &GcpSecretManagerSettings) {
    let mut value = serde_json::to_value(settings).unwrap();
    value["type"] = json!("GcpSecretManager");
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('secret_backend', $1)
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(value)
    .execute(db)
    .await
    .unwrap();
}

async fn stored(db: &Pool<Postgres>, w: &str, path: &str) -> String {
    sqlx::query_scalar("SELECT value FROM variable WHERE workspace_id = $1 AND path = $2")
        .bind(w)
        .bind(path)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
async fn secrets_move_to_secret_manager_and_back(db: Pool<Postgres>) {
    let store: Store = Arc::default();
    let settings = start(store.clone()).await;
    let seeded = seed(&db).await;
    assert!(seeded.len() >= 3);
    configure(&db, &settings).await;

    // Before migrating, database secrets still read with Secret Manager configured.
    let (w, path, plain) = &seeded[0];
    assert_eq!(
        &get_secret_value(&db, w, path, &stored(&db, w, path).await)
            .await
            .unwrap(),
        plain
    );

    let report = migrate_secrets_to_gcp_secret_manager(&db, &settings)
        .await
        .unwrap();
    assert_eq!(
        (
            report.total_secrets,
            report.migrated_count,
            report.failed_count
        ),
        (seeded.len(), seeded.len(), 0)
    );
    for (w, path, plain) in &seeded {
        let marker = stored(&db, w, path).await;
        assert_eq!(marker, format!("$gcp_sm:{path}"));
        assert_eq!(
            &get_secret_value(&db, w, path, &marker).await.unwrap(),
            plain
        );
        let id = secret_id(Some("gantry-"), w, path).unwrap();
        assert!(
            store.lock().unwrap().contains_key(&id),
            "{id} in Secret Manager"
        );
    }

    // Saving again leaves only the new version readable.
    let backend = GcpSecretManagerBackend::new(settings.clone());
    backend.set_secret(w, path, "rotated").await.unwrap();
    let id = secret_id(Some("gantry-"), w, path).unwrap();
    let live: Vec<String> = store.lock().unwrap()[&id]
        .iter()
        .filter(|(_, destroyed)| !destroyed)
        .map(|(data, _)| data.clone())
        .collect();
    assert_eq!(live, vec![BASE64.encode("rotated")]);
    assert_eq!(backend.get_secret(w, path).await.unwrap(), "rotated");

    let report = migrate_secrets_from_gcp_secret_manager(&db, &settings)
        .await
        .unwrap();
    assert_eq!(
        (report.migrated_count, report.failed_count),
        (seeded.len(), 0)
    );
    assert!(
        store.lock().unwrap().is_empty(),
        "secrets deleted from Secret Manager"
    );
    for (w, p, plain) in &seeded {
        let value = stored(&db, w, p).await;
        assert!(!value.starts_with('$'));
        let expected = if p == path { "rotated" } else { plain.as_str() };
        assert_eq!(get_secret_value(&db, w, p, &value).await.unwrap(), expected);
    }

    // Deleting a secret that is already gone is fine.
    backend.delete_secret(w, path).await.unwrap();
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
async fn a_marker_without_the_backend_says_so(db: Pool<Postgres>) {
    let err = get_secret_value(
        &db,
        "test-workspace",
        "u/test-user/db_password",
        "$gcp_sm:u/test-user/db_password",
    )
    .await
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("not the configured secret backend"),
        "{err}"
    );
}
