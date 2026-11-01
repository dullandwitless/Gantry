use crate::server::Smtp;

pub const SMTP_ENABLED: bool = false;

pub async fn send_email(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}

pub async fn send_email_html(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}

pub async fn send_email_plain_text(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}

pub fn send_email_if_possible(_subject: &str, _content: &str, _to: &str) {
    tracing::warn!(
        "send_email_if_possible is not implemented in Gantry"
    );
}
