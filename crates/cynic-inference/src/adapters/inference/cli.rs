//! CliBackend — ChatPort adapter for CLI-based LLM tools (e.g. `gemini` CLI).
//! Spawns a subprocess per request; stdout is the response text.

use async_trait::async_trait;
use std::time::Duration;

use crate::domain::chat::{ChatError, ChatResponse, InferenceProfile};
use crate::domain::inference::BackendStatus;
use crate::ports::chat_port::ChatPort;
use crate::ports::inference_port::BackendPort;

/// Backend that delegates inference to a CLI binary via subprocess.
#[derive(Debug)]
pub struct CliBackend {
    name: String,
    binary: String,
    timeout: Duration,
    extra_args: Vec<String>,
}

impl CliBackend {
    pub fn new(name: &str, binary: &str, timeout_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            timeout: Duration::from_secs(timeout_secs),
            extra_args: Vec::new(),
        }
    }

    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    fn build_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.binary);

        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        } else {
            cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");
        }
        if let Ok(home) = std::env::var("HOME") {
            cmd.env("HOME", home);
        }
        if let Ok(user) = std::env::var("USER") {
            cmd.env("USER", user);
        }
        if let Ok(shell) = std::env::var("SHELL") {
            cmd.env("SHELL", shell);
        }
        if let Ok(npm_global) = std::env::var("npm_config_prefix") {
            cmd.env("npm_config_prefix", npm_global);
        }

        cmd
    }
}

#[async_trait]
impl BackendPort for CliBackend {
    fn name(&self) -> &str {
        &self.name
    }

    async fn health(&self) -> BackendStatus {
        let probe = tokio::time::timeout(
            Duration::from_secs(5),
            self.build_command().arg("--version").output(),
        )
        .await;

        match probe {
            Ok(Ok(out)) if out.status.success() => BackendStatus::Healthy,
            Ok(Ok(_)) => {
                tracing::warn!(backend = %self.name, "CLI --version returned non-zero");
                BackendStatus::Critical
            }
            Ok(Err(error)) => {
                tracing::error!(backend = %self.name, error = %error, "CLI binary not found or not executable");
                BackendStatus::Critical
            }
            Err(_) => {
                tracing::warn!(backend = %self.name, "CLI --version timed out after 5s");
                BackendStatus::Critical
            }
        }
    }
}

#[async_trait]
impl ChatPort for CliBackend {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        _profile: InferenceProfile,
        _request_id: Option<&str>,
    ) -> Result<ChatResponse, ChatError> {
        let prompt = if system.is_empty() {
            user.to_string()
        } else {
            format!("{system}\n\n{user}")
        };

        let mut cmd = self.build_command();
        for arg in &self.extra_args {
            cmd.arg(arg);
        }
        cmd.arg("--prompt").arg(&prompt);
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|error| {
            ChatError::Unreachable(format!("{}: spawn failed: {}", self.name, error))
        })?;

        let run = tokio::time::timeout(self.timeout, child.wait_with_output()).await;

        match run {
            Err(_) => Err(ChatError::Timeout {
                ms: self.timeout.as_millis() as u64,
            }),
            Ok(Err(error)) => Err(ChatError::Unreachable(format!(
                "{}: wait failed: {}",
                self.name, error
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

                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();

                if raw.is_empty() {
                    return Err(ChatError::Protocol(format!("{}: empty stdout", self.name)));
                }

                if raw.starts_with('{')
                    && let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
                {
                    let text = value
                        .get("response")
                        .and_then(|response| response.as_str())
                        .unwrap_or(&raw)
                        .to_string();

                    let reasoning = value
                        .get("thinking")
                        .or_else(|| value.get("thought"))
                        .and_then(|reasoning| reasoning.as_str())
                        .map(|reasoning| reasoning.to_string());

                    let (prompt_tokens, completion_tokens, thinking_tokens) = value
                        .get("stats")
                        .and_then(|stats| stats.get("models"))
                        .and_then(|models| models.as_object())
                        .and_then(|models| models.values().next())
                        .and_then(|model| model.get("tokens"))
                        .map(|tokens| {
                            let input = tokens
                                .get("input")
                                .and_then(|entry| entry.as_u64())
                                .unwrap_or(0);
                            let candidates = tokens
                                .get("candidates")
                                .and_then(|entry| entry.as_u64())
                                .unwrap_or(0);
                            let thoughts = tokens
                                .get("thoughts")
                                .and_then(|entry| entry.as_u64())
                                .unwrap_or(0);
                            (input as u32, candidates as u32, thoughts as u32)
                        })
                        .unwrap_or((0, 0, 0));

                    return Ok(ChatResponse {
                        text,
                        reasoning,
                        prompt_tokens,
                        completion_tokens,
                        thinking_tokens,
                    });
                }

                Ok(ChatResponse {
                    text: raw,
                    reasoning: None,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_configured_name() {
        let backend = CliBackend::new("gemini-2.5-flash", "/usr/bin/gemini", 60);
        assert_eq!(backend.name(), "gemini-2.5-flash");
    }

    #[allow(clippy::print_stderr)]
    #[tokio::test]
    #[ignore]
    async fn chat_returns_text_from_cli_stdout() {
        let which = tokio::process::Command::new("which")
            .arg("gemini")
            .output()
            .await;
        if which.is_err() || !which.expect("which output").status.success() {
            eprintln!("SKIP: gemini CLI not found");
            return;
        }
        let backend = CliBackend::new("gemini-test", "gemini", 30);
        let resp = backend
            .chat(
                "You are a test.",
                "Reply with exactly: PONG",
                InferenceProfile::SCORING,
                None,
            )
            .await;
        assert!(resp.is_ok(), "gemini CLI should return a response");
        assert!(
            !resp.expect("response").text.is_empty(),
            "response should not be empty"
        );
    }
}
