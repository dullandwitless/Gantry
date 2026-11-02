# What Gantry does not implement yet

Upstream implements these in proprietary code. Gantry's tree has only the
open-source stand-ins (`*_oss.rs`) or refusals ("... is not supported in
Gantry"), so each is clean-room work: spec it here from public material, then
implement it under the rules in [README.md](README.md).

Compiled from the AGPL tree on 2026-11-02 (the stand-in modules, the refusal
messages, and the tests ignored with a `gantry: ... is not implemented`
reason). It names features, not how upstream builds them.

## Implemented since this list was compiled

- **Users and passwords**, **email** and the **global settings listing**:
  [users-and-email.md](users-and-email.md). Email needs the `smtp` cargo feature;
  a build without it says so instead of sending.
- **Audit logs**: writing and listing ([audit-logs.md](audit-logs.md)). Export to
  object storage and OpenTelemetry is not implemented.
- **Concurrency limits**: [concurrency-limits.md](concurrency-limits.md).
- **Settable poll delay** for job update streams:
  [job-update-poll-delay.md](job-update-poll-delay.md).

## Identity and access

- SSO login: SAML, SCIM provisioning, OAuth login providers.
- Service accounts; custom workspace and instance roles; instance groups.
- Workload identity through Gantry's OIDC provider (AWS roles, Azure, Anthropic).
- Data table roles, ACLs, permissions and replay.

## Operations and observability

- Audit log export (object storage, OpenTelemetry).
- Prometheus metrics (`METRICS_ADDR`); OpenTelemetry export and the tracing proxy.
- Critical alerts and alert channels; usage stats.

## Execution

- Dedicated workers (flow loop squashing); agent workers (`MODE=agent`);
  worker autoscaling.
- Private package registries (npm, PyPI, Maven and others); worker object-store
  cache.
- SSH executor; volumes; job memory.

## Data and storage

- The workspace S3 proxy (DuckLake writes), S3 job helpers, storage listing.
- External instance databases; IAM RDS authentication; DuckLake maintenance;
  the Azure proxy.

## Secrets

- External secret backends: HashiCorp Vault, AWS Secrets Manager, Azure Key
  Vault, HTTP secret store.

## Deployment and git

- Git sync (including automatic pull and multiple repositories).
- Deployment requests and the deployment UI; deploying to another instance;
  promotion mode; fork attribution and notes; a workspace default app.

## Pipelines

- Advanced asset-trigger dispatch: joins, partitions, debounce, retry.
- Backfill, cascade, freshness watchdog.

## Integrations

- Microsoft Teams (including approvals); email triggers (inbound SMTP);
  Slack OAuth; MCP OAuth.

## Removed rather than stubbed

- Full-text search over jobs and service logs (the indexer), and with it
  retrieval of service logs older than the raw files a host keeps.
- Kafka, NATS, SQS, GCP Pub/Sub and Azure triggers.
