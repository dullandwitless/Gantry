//! Slack delivery of critical alerts, per `specs/critical-alert-channels.md`, against a
//! local stand-in for the Slack Web API.
//!
//! Run with:
//!   cargo test -p windmill-common --test critical_alert_slack

use std::sync::{Arc, Mutex};

use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use serde_json::{json, Value};
use windmill_common::critical_alert_channels::post_slack_message;

type Seen = Arc<Mutex<Vec<(Option<String>, Value)>>>;

/// Serves `chat.postMessage`, records each request's bearer token and body, and answers
/// `ok: false` with `channel_not_found` for the channel `#missing`.
async fn slack_stub() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route(
            "/chat.postMessage",
            post(|State(seen): State<Seen>, headers: HeaderMap, Json(body): Json<Value>| async move {
                let token = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(str::to_string);
                let answer = if body["channel"] == "#missing" {
                    json!({ "ok": false, "error": "channel_not_found" })
                } else {
                    json!({ "ok": true })
                };
                seen.lock().unwrap().push((token, body));
                Json(answer)
            }),
        )
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    (base, seen)
}

#[tokio::test]
async fn posts_the_text_to_the_channel_with_the_bot_token() {
    let (base, seen) = slack_stub().await;

    post_slack_message(&base, "xoxb-test", "#alerts", "*[Gantry] Critical alert*\nDisk low")
        .await
        .expect("delivered");

    let seen = seen.lock().unwrap();
    assert_eq!(
        *seen,
        vec![(
            Some("xoxb-test".to_string()),
            json!({ "channel": "#alerts", "text": "*[Gantry] Critical alert*\nDisk low" })
        )]
    );
}

#[tokio::test]
async fn a_refusal_is_an_error_naming_slacks_code() {
    let (base, _) = slack_stub().await;

    let err = post_slack_message(&base, "xoxb-test", "#missing", "text")
        .await
        .expect_err("Slack refused it");
    assert!(err.to_string().contains("channel_not_found"), "{err}");
}
