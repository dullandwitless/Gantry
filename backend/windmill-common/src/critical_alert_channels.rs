//! Delivery of critical alerts to the instance's alert channels.
//!
//! Clean-room implementation of `specs/critical-alert-channels.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use std::sync::Arc;

use crate::{
    ee_oss::CriticalErrorChannel, email_oss::send_email_if_possible, error::Error,
    oauth2::GLOBAL_SLACK_BOT_TOKEN_PATH, utils::HTTP_CLIENT, variables::get_secret_value_as_admin,
    BASE_URL, CRITICAL_ERROR_CHANNELS, DB,
};

/// Where the instance's Slack bot token is kept.
const SLACK_TOKEN_WORKSPACE: &str = "admins";

pub const SLACK_API_BASE: &str = "https://slack.com/api";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Critical,
    Recovered,
    WorkspaceError,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Notification {
    pub subject: String,
    pub body: String,
}

impl Notification {
    /// One message for chat channels: the subject in bold, then the body.
    fn chat_text(&self) -> String {
        format!("*{}*\n{}", self.subject, self.body)
    }
}

pub fn compose(
    kind: AlertKind,
    message: &str,
    workspace_id: Option<&str>,
    resource: Option<&str>,
    base_url: &str,
) -> Notification {
    let title = match kind {
        AlertKind::Critical => "Critical alert",
        AlertKind::Recovered => "Recovered",
        AlertKind::WorkspaceError => "Workspace error",
    };
    let subject = match workspace_id {
        Some(w) => format!("[Gantry] {title} in workspace {w}"),
        None => format!("[Gantry] {title}"),
    };

    let mut body = message.to_string();
    for (label, value) in [("Workspace", workspace_id), ("Resource", resource)] {
        if let Some(value) = value {
            body.push_str(&format!("\n{label}: {value}"));
        }
    }
    if !base_url.is_empty() {
        body.push_str(&format!("\nInstance: {base_url}"));
    }
    Notification { subject, body }
}

/// Sends the alert to every configured channel in the background. Never fails the caller:
/// a channel that cannot be reached is logged and skipped.
pub fn notify_channels(
    db: &DB,
    kind: AlertKind,
    message: &str,
    workspace_id: Option<&str>,
    resource: Option<&str>,
) {
    let channels = CRITICAL_ERROR_CHANNELS.load_full();
    if channels.is_empty() {
        return;
    }
    let notification = compose(kind, message, workspace_id, resource, &BASE_URL.load());
    tokio::spawn(deliver(db.clone(), channels, notification));
}

async fn deliver(db: DB, channels: Arc<Vec<CriticalErrorChannel>>, notification: Notification) {
    let mut slack_token: Option<Option<String>> = None;
    for channel in channels.iter() {
        match channel {
            CriticalErrorChannel::Email { email } => {
                send_email_if_possible(&notification.subject, &notification.body, email);
            }
            CriticalErrorChannel::Slack { slack_channel } => {
                if slack_token.is_none() {
                    slack_token = Some(instance_slack_token(&db).await);
                }
                let Some(Some(token)) = &slack_token else {
                    tracing::warn!(
                        "Critical alert not sent to Slack channel {slack_channel}: Slack is not connected to the instance ({SLACK_TOKEN_WORKSPACE}/{GLOBAL_SLACK_BOT_TOKEN_PATH} is missing)"
                    );
                    continue;
                };
                if let Err(e) = post_slack_message(
                    SLACK_API_BASE,
                    token,
                    slack_channel,
                    &notification.chat_text(),
                )
                .await
                {
                    tracing::warn!(
                        "Critical alert not sent to Slack channel {slack_channel}: {e:#}"
                    );
                }
            }
            CriticalErrorChannel::Teams { teams_channel } => {
                tracing::warn!(
                    "Critical alert not sent to Microsoft Teams channel {}: Teams is not available in Gantry yet",
                    teams_channel.channel_name
                );
            }
        }
    }
}

/// The instance's Slack bot token, or `None` when Slack is not connected (or the token
/// cannot be read, which is logged).
async fn instance_slack_token(db: &DB) -> Option<String> {
    match get_secret_value_as_admin(db, SLACK_TOKEN_WORKSPACE, GLOBAL_SLACK_BOT_TOKEN_PATH).await {
        Ok(token) => Some(token),
        Err(Error::NotFound(_)) => None,
        Err(e) => {
            tracing::warn!("Could not read the instance Slack bot token: {e:#}");
            None
        }
    }
}

/// Posts `text` to a Slack channel. Slack answers 200 with `ok: false` on failure; that
/// error code is returned as the error.
pub async fn post_slack_message(
    api_base: &str,
    token: &str,
    channel: &str,
    text: &str,
) -> anyhow::Result<()> {
    let res: serde_json::Value = HTTP_CLIENT
        .post(format!("{api_base}/chat.postMessage"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "channel": channel, "text": text }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if res["ok"].as_bool() == Some(true) {
        Ok(())
    } else {
        anyhow::bail!(
            "Slack refused the message: {}",
            res["error"].as_str().unwrap_or("no error code")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_alert_names_workspace_resource_and_instance() {
        let n = compose(
            AlertKind::Critical,
            "Worker group default has no workers",
            Some("main"),
            Some("worker_group_default"),
            "https://gantry.example.com",
        );
        assert_eq!(n.subject, "[Gantry] Critical alert in workspace main");
        assert_eq!(
            n.body,
            "Worker group default has no workers\nWorkspace: main\n\
             Resource: worker_group_default\nInstance: https://gantry.example.com"
        );
    }

    #[test]
    fn instance_wide_recovery_has_only_the_message() {
        let n = compose(AlertKind::Recovered, "Disk space is back", None, None, "");
        assert_eq!(n.subject, "[Gantry] Recovered");
        assert_eq!(n.body, "Disk space is back");
        assert_eq!(n.chat_text(), "*[Gantry] Recovered*\nDisk space is back");
    }
}
