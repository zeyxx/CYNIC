use thiserror::Error;

/// Error type for Slack alerting operations.
#[derive(Debug, Error)]
pub enum AlertError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Slack API error (status {status}): {body}")]
    SlackError { status: u16, body: String },

    #[error("Slack webhook request timed out")]
    Timeout,
}

/// Slack alerter — sends messages to a webhook.
#[derive(Debug)]
pub struct SlackAlerter {
    client: reqwest::Client,
    webhook_url: String,
}

impl SlackAlerter {
    /// Create a new SlackAlerter with the given webhook URL.
    pub fn new(webhook_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            webhook_url,
        }
    }

    /// Create a new SlackAlerter from the CYNIC_SLACK_WEBHOOK environment variable.
    /// Returns `None` if the env var is not set or empty.
    pub fn from_env() -> Option<Self> {
        let webhook_url = std::env::var("CYNIC_SLACK_WEBHOOK")
            .ok()?
            .trim()
            .to_string();

        if webhook_url.is_empty() {
            return None;
        }

        Some(Self::new(webhook_url))
    }

    /// Send a message to Slack. Non-blocking on failure — logs warn but does not error out.
    pub async fn send(&self, message: &str) -> Result<(), AlertError> {
        let payload = serde_json::json!({
            "text": message,
        });

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            self.client.post(&self.webhook_url).json(&payload).send(),
        )
        .await
        .map_err(|_e| AlertError::Timeout)?
        .map_err(AlertError::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "(could not read response)".to_string());
            return Err(AlertError::SlackError { status, body });
        }

        Ok(())
    }
}

/// Format a ContractDelta event as a Slack message.
pub fn format_contract_delta(missing: &[String], expected: usize, actual: usize) -> String {
    let dog_list = if missing.is_empty() {
        "all Dogs healthy".to_string()
    } else {
        format!("missing: {}", missing.join(", "))
    };

    format!(":warning: Contract breach: expected {expected}, got {actual} ({dog_list})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_contract_delta_missing() {
        let msg = format_contract_delta(
            &["qwen35-9b-gpu".to_string(), "gemma-4b-core".to_string()],
            4,
            2,
        );
        assert!(msg.contains("expected 4"));
        assert!(msg.contains("got 2"));
        assert!(msg.contains("qwen35-9b-gpu"));
    }

    #[test]
    fn format_contract_delta_all_healthy() {
        let msg = format_contract_delta(&[], 4, 4);
        assert!(msg.contains("all Dogs healthy"));
    }

    #[test]
    fn new_creates_alerter() {
        let alerter = SlackAlerter::new("https://hooks.slack.com/test".to_string());
        assert_eq!(alerter.webhook_url, "https://hooks.slack.com/test");
    }
}
