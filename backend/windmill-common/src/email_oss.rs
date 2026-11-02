/*
 * Gantry clean-room implementation of outbound email.
 * Specification: specs/users-and-email.md (section 4).
 * This file and its contents are licensed under the AGPLv3 License.
 */

use crate::server::Smtp;

/// Whether this build can send email (the `smtp` cargo feature).
pub const SMTP_ENABLED: bool = cfg!(feature = "smtp");

// Without `smtp` the bodies are built but never read.
#[cfg_attr(not(feature = "smtp"), allow(dead_code))]
enum Body<'a> {
    Plain(&'a str),
    Html(&'a str),
}

pub async fn send_email(
    subject: &str,
    content: &str,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    send(subject, Body::Plain(content), to, smtp, client_timeout).await
}

pub async fn send_email_html(
    subject: &str,
    content: &str,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    send(subject, Body::Html(content), to, smtp, client_timeout).await
}

pub async fn send_email_plain_text(
    subject: &str,
    content: &str,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    send(subject, Body::Plain(content), to, smtp, client_timeout).await
}

/// Sends a plain-text email in the background with the instance's SMTP settings, when
/// they are configured. Returns at once; failures are logged.
pub fn send_email_if_possible(subject: &str, content: &str, to: &str) {
    if !SMTP_ENABLED {
        tracing::info!("built without the smtp feature, not sending {subject:?} to {to}");
        return;
    }
    let Some(smtp) = (**crate::worker::SMTP_CONFIG.load()).clone() else {
        tracing::info!("SMTP is not configured, not sending {subject:?} to {to}");
        return;
    };
    let (subject, content, to) = (subject.to_string(), content.to_string(), to.to_string());
    tokio::spawn(async move {
        if let Err(e) = send_email_plain_text(
            &subject,
            &content,
            vec![to.clone()],
            smtp,
            Some(DEFAULT_TIMEOUT),
        )
        .await
        {
            tracing::error!("sending {subject:?} to {to} failed: {e:#}");
        }
    });
}

/// Bounds `send_email_if_possible`, which has no caller waiting on it to set one.
const DEFAULT_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(30);

#[cfg(not(feature = "smtp"))]
async fn send(
    _subject: &str,
    _body: Body<'_>,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Err(crate::error::Error::BadRequest(
        "this Gantry build cannot send email: it was compiled without the smtp feature".to_string(),
    ))
}

#[cfg(feature = "smtp")]
async fn send(
    subject: &str,
    body: Body<'_>,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    use crate::error::Error;
    use mail_send::mail_builder::{headers::raw::Raw, MessageBuilder};
    use mail_send::SmtpClientBuilder;

    if to.is_empty() {
        return Ok(());
    }

    // mail-send builds its TLS config from the process-wide rustls provider and panics
    // without one. The server binary installs ring at startup; tests and other entry
    // points may not.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let mut message = MessageBuilder::new()
        .from(smtp.from.as_str())
        .to(to.iter().map(String::as_str).collect::<Vec<_>>())
        .subject(subject);
    message = match body {
        Body::Plain(text) => message.text_body(text),
        Body::Html(html) => message.html_body(html),
    };
    if smtp.clicktracking_off.unwrap_or(false) {
        // SendGrid's documented per-message setting; other servers ignore the header.
        message = message.header(
            "X-SMTPAPI",
            Raw::new(r#"{"filters":{"clicktrack":{"settings":{"enable":0}}}}"#),
        );
    }

    let mut client = SmtpClientBuilder::new(smtp.host.as_str(), smtp.port)
        .implicit_tls(smtp.tls_implicit.unwrap_or(smtp.port == 465));
    if let Some(username) = smtp.username.as_deref().filter(|u| !u.is_empty()) {
        client = client.credentials((username, smtp.password.as_deref().unwrap_or("")));
    }
    if let Some(timeout) = client_timeout {
        client = client.timeout(timeout);
    }

    let sent = async {
        if smtp.disable_tls.unwrap_or(false) {
            client.connect_plain().await?.send(message).await
        } else {
            client.connect().await?.send(message).await
        }
    };
    let result = match client_timeout {
        Some(timeout) => tokio::time::timeout(timeout, sent)
            .await
            .unwrap_or(Err(mail_send::Error::Timeout)),
        None => sent.await,
    };
    result.map_err(|e| {
        Error::internal_err(format!(
            "sending email through {}:{}: {e}",
            smtp.host, smtp.port
        ))
    })
}
