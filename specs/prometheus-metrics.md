# Gantry Clean-Room Specification: Prometheus Metrics

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/advanced/instance_settings
  ("Expose Prometheus metrics for workers and servers on port 8001 at /metrics") and
  https://www.windmill.dev/docs/advanced/self_host (health metrics exposed "when metrics
  are enabled")
- AGPL backend in this repository: `windmill-common/src/lib.rs` (`METRICS_PORT`,
  `METRICS_ADDR`, `METRICS_ENABLED`, `IS_READY`, `serve_metrics`), `src/main.rs` (the
  metrics future, the `expose_metrics` setting listener), `src/monitor.rs`
  (`apply_metrics_enabled`, the metrics it records), `windmill-api/src/health.rs`,
  `windmill-worker/src/worker.rs`, the root `Cargo.toml` `prometheus` feature
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

With the `prometheus` cargo feature, the server, workers and monitor register their
counters, gauges and histograms in the default Prometheus registry and update them while
metrics are enabled. `serve_metrics` can serve that registry and a worker readiness probe.
Only starting it is missing: a process with metrics enabled logs that they are not
supported.

## 2. Enabling

Metrics are enabled when, at startup:

- `METRICS_PORT` or `METRICS_ADDR` is set in the environment, or
- the instance setting `expose_metrics` is `true` (it overrides the environment either
  way).

Changing `expose_metrics` already restarts the process, which picks up the new value.

The listen address is `METRICS_ADDR` (a socket address, or `true` for all interfaces),
else `0.0.0.0:METRICS_PORT`, else `0.0.0.0:8001`.

## 3. What is served

On that address, separate from the API port, while metrics are enabled:

- `GET /metrics`: every registered metric in the Prometheus text format.
- `GET /ready`, in processes that run workers: `200 ready` once a worker has started
  pulling jobs, `500 not ready` before.

The listener stops with the process's graceful shutdown. If the address cannot be bound,
the error is logged and the process carries on without metrics.

No endpoint resets metrics: the default registry cannot be reset in place, so the
`/reset` route `serve_metrics` declared (and left unimplemented) is removed.

## 4. Builds without the feature

A build without `prometheus` records no metrics. If metrics are enabled anyway, it logs
once that this build was compiled without the `prometheus` feature, and serves nothing.
