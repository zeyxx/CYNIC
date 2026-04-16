//! CliBackend — ChatPort adapter for CLI-based LLM tools (e.g. `gemini` CLI).
//! Spawns a subprocess per request; stdout is the response text.
//! No HTTP — pure subprocess I/O. Used when the inference tool is a binary,
//! not a server (e.g. `gemini --prompt "..."` from the Gemini CLI).

use crate::domain::chat::{ChatError, ChatPort, ChatResponse, InferenceProfile};
use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;
use std::time::Duration;

/// Backend that delegates inference to a CLI binary via subprocess.
///
/// For CLI backends, `base_url` in `BackendConfig` holds the binary path/name
/// (e.g. `"gemini"` or `"/usr/local/bin/gemini"`).
#[derive(Debug)]
pub struct CliBackend {
    name: String,
    binary: String,
    timeout: Duration,
}

impl CliBackend {
    /// Create a new CLI backend.
    /// - `name`: human-readable Dog name (e.g. `"gemini-2.5-flash"`)
    /// - `binary`: path or name of the CLI binary (e.g. `"gemini"`)
    /// - `timeout_secs`: per-call timeout in seconds
    pub fn new(name: &str, binary: &str, timeout_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }
}

// ── BackendPort ──────────────────────────────────────────────

#[async_trait]
impl BackendPort for CliBackend {
    fn name(&self) -> &str {
        &self.name
    }

    async fn health(&self) -> BackendStatus {
        let probe = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new(&self.binary)
                .arg("--version")
                .output(),
        )
        .await;

        match probe {
            Ok(Ok(out)) if out.status.success() => BackendStatus::Healthy,
            Ok(Ok(_)) => {
                tracing::warn!(backend = %self.name, "CLI --version returned non-zero");
                BackendStatus::Critical
            }
            Ok(Err(e)) => {
                tracing::error!(backend = %self.name, error = %e, "CLI binary not found or not executable");
                BackendStatus::Critical
            }
            Err(_elapsed) => {
                tracing::warn!(backend = %self.name, "CLI --version timed out after 5s");
                BackendStatus::Critical
            }
        }
    }
}

// ── ChatPort ─────────────────────────────────────────────────

#[async_trait]
impl ChatPort for CliBackend {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        _profile: InferenceProfile,
        _request_id: Option<&str>,
    ) -> Result<ChatResponse, ChatError> {
        // Combine system + user into a single prompt string for the CLI.
        let prompt = if system.is_empty() {
            user.to_string()
        } else {
            format!("{system}\n\n{user}")
        };

        let run = tokio::time::timeout(
            self.timeout,
            tokio::process::Command::new(&self.binary)
                .arg("--prompt")
                .arg(&prompt)
                .output(),
        )
        .await;

        match run {
            Err(_elapsed) => Err(ChatError::Timeout {
                ms: self.timeout.as_millis() as u64,
            }),
            Ok(Err(e)) => Err(ChatError::Unreachable(format!(
                "{}: spawn failed: {}",
                self.name, e
            ))),
            Ok(Ok(out)) => {
                if !out.status.success() {
                    let stderr_excerpt = String::from_utf8_lossy(&out.stderr)
                        .chars()
                        .take(256)
                        .collect::<String>();
                    return Err(ChatError::Protocol(format!(
                        "{}: exit {}: {}",
                        self.name,
                        out.status.code().unwrap_or(-1),
                        stderr_excerpt
                    )));
                }

                let text = String::from_utf8_lossy(&out.stdout).trim().to_string();

                if text.is_empty() {
                    return Err(ChatError::Protocol(format!("{}: empty stdout", self.name)));
                }

                Ok(ChatResponse {
                    text,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                })
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_configured_name() {
        let backend = CliBackend::new("gemini-2.5-flash", "/usr/bin/gemini", 60);
        assert_eq!(backend.name(), "gemini-2.5-flash");
    }

    // WHY: runtime skip notice when gemini CLI is absent — operator-facing message
    // for an #[ignore]d test invoked explicitly. stderr avoids polluting test output capture.
    #[allow(clippy::print_stderr)]
    #[tokio::test]
    #[ignore] // Requires working gemini CLI — times out in sandboxed environments
    async fn chat_returns_text_from_cli_stdout() {
        // Skip if gemini not installed
        let which = tokio::process::Command::new("which")
            .arg("gemini")
            .output()
            .await;
        if which.is_err() || !which.unwrap().status.success() {
            eprintln!("SKIP: gemini CLI not found");
            return;
        }
        let backend = CliBackend::new("gemini-test", "gemini", 30);
        let resp = backend
            .chat(
                "You are a test.",
                "Reply with exactly: PONG",
                crate::domain::chat::InferenceProfile::Scoring,
                None,
            )
            .await;
        assert!(resp.is_ok(), "gemini CLI should return a response");
        assert!(
            !resp.unwrap().text.is_empty(),
            "response should not be empty"
        );
    }
}
