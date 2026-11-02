//! The Prometheus endpoint, per `specs/prometheus-metrics.md`.
//!
//! Run with:
//!   cargo test -p windmill-common --features prometheus --test metrics_endpoint
#![cfg(feature = "prometheus")]

use std::{net::SocketAddr, sync::atomic::Ordering, time::Duration};

use windmill_common::{serve_metrics, IS_READY};

/// A loopback address with a port free at the time of asking.
fn free_addr() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("free port")
}

async fn get(url: &str) -> (u16, String) {
    // The listener starts on a spawned task; retry until it accepts.
    for _ in 0..50 {
        if let Ok(res) = reqwest::get(url).await {
            let status = res.status().as_u16();
            return (status, res.text().await.expect("body"));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{url} never answered");
}

#[tokio::test]
async fn serves_metrics_and_readiness_until_shutdown() {
    let counter = prometheus::register_int_counter!(
        "gantry_metrics_endpoint_test_total",
        "Registered by the metrics endpoint test"
    )
    .expect("register");
    counter.inc_by(3);

    let addr = free_addr();
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    let server = tokio::spawn(serve_metrics(addr, rx, true, true));

    let (status, body) = get(&format!("http://{addr}/metrics")).await;
    assert_eq!(status, 200);
    assert!(
        body.contains("gantry_metrics_endpoint_test_total 3"),
        "registered metrics are exported: {body}"
    );

    IS_READY.store(false, Ordering::Relaxed);
    assert_eq!(get(&format!("http://{addr}/ready")).await.0, 500);
    IS_READY.store(true, Ordering::Relaxed);
    assert_eq!(
        get(&format!("http://{addr}/ready")).await,
        (200, "ready".to_string())
    );

    assert_eq!(
        get(&format!("http://{addr}/reset")).await.0,
        404,
        "there is no reset"
    );

    tx.send(()).expect("shutdown");
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("stops on shutdown")
        .expect("join")
        .expect("serve");
}

#[tokio::test]
async fn readiness_is_not_served_without_workers() {
    let addr = free_addr();
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    let server = tokio::spawn(serve_metrics(addr, rx, false, true));

    assert_eq!(get(&format!("http://{addr}/metrics")).await.0, 200);
    assert_eq!(get(&format!("http://{addr}/ready")).await.0, 404);

    tx.send(()).expect("shutdown");
    server.await.expect("join").expect("serve");
}
