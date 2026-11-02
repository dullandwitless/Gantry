# Gantry Clean-Room Specification: Critical Alert Channels

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/core_concepts/critical_alerts
  (what raises critical alerts; email needs SMTP and a channel address in the instance
  settings; Slack and Microsoft Teams channels by name; "Mute critical alerts in UI"
  disables in-app alerts; recovery notices)
- AGPL backend in this repository: `windmill-common/src/utils.rs`
  (`report_critical_error`, `report_recovered_critical_error`,
  `send_workspace_error_to_instance_channels`), `windmill-common/src/ee_oss.rs`
  (`CriticalErrorChannel`), `windmill-common/src/lib.rs` (`CRITICAL_ERROR_CHANNELS`,
  `BASE_URL`), `src/monitor.rs` (`apply_critical_error_channels_setting`),
  `windmill-common/src/oauth2.rs` (`GLOBAL_SLACK_BOT_TOKEN_PATH`),
  `windmill-common/src/email_oss.rs` (`send_email_if_possible`),
  `windmill-common/src/variables.rs` (`get_secret_value_as_admin`), the `alerts` table
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

Critical errors and their recoveries are recorded in `alerts`, where the UI lists and
acknowledges them. The instance setting `critical_error_channels` is parsed into
`CRITICAL_ERROR_CHANNELS`, a list of channels, each one of:

- `{"email": "<address>"}`
- `{"slack_channel": "<channel name or id>"}`
- `{"teams_channel": {...}}`

Nothing is sent to them.

## 2. What is sent, and when

Every configured channel receives a notification for:

- a **critical error** (`report_critical_error`), whether or not it is muted in the UI:
  muting hides alerts in the app, the channels belong to the instance operator;
- a **recovery** (`report_recovered_critical_error`);
- a **workspace error routed to the instance** (`send_workspace_error_to_instance_channels`,
  which callers invoke only for workspaces that opted in). Never on cloud-hosted
  instances, where the channels' owner is not the workspace's.

The text names the kind (critical alert, recovered, workspace error), the workspace and
resource when there is one, the message, and the instance's base URL when it is set.

Sending never delays or fails the caller: it happens in the background, and a channel that
cannot be reached is logged and skipped.

## 3. Channels

- **Email**: through the instance SMTP configuration, like other instance email. Without
  SMTP configured, or in a build without the `smtp` feature, it is logged instead.
- **Slack**: `chat.postMessage` to the named channel with the instance's Slack bot token,
  the secret variable `f/slack_bot/global_bot_token` in the `admins` workspace. Without that
  variable it is logged as not connected. A Slack answer with `ok: false` is logged with
  its error code.
- **Microsoft Teams**: not yet available in Gantry (Teams integration is on the backlog);
  logged as such.
