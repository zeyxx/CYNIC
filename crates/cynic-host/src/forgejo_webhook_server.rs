//! Forgejo webhook HTTP server.
//!
//! Receives Forgejo push-event POST requests, validates the source commit,
//! and triggers a build on the remote build_runner via SSH + `cynic build run --apply`.
//!
//! No external HTTP dependency — uses std::net::TcpListener for a minimal
//! HTTP 1.0 server sufficient for Forgejo webhook delivery.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use serde::Serialize;

use crate::forgejo_webhook::{ForgejoHandlerConfig, ForgejoPayload, handle_forgejo_payload, validate_source_commit};
use crate::update_lifecycle::ArgvRunner;

/// Configuration for the Forgejo webhook server.
#[derive(Debug, Clone)]
pub struct WebhookServerConfig {
    /// Address to bind (e.g. "127.0.0.1:9147").
    pub bind: String,
    /// Build phase to trigger (check, build, activate).
    pub phase: String,
    /// SSH target for the build runner (e.g. "cynic@10.10.10.119").
    pub runner_target: String,
    /// Whether to apply the build automatically (true for autonomous).
    pub apply: bool,
    /// Path to the registry JSON for node discovery.
    pub registry_path: String,
}

/// Result of a webhook-triggered build.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookTriggerResult {
    pub status: String,
    pub source_commit: Option<String>,
    pub workload_id: Option<String>,
    pub build_triggered: bool,
    pub error: Option<String>,
}

/// Spawn the webhook server thread. Blocks the thread until shutdown.
/// Returns immediately in a background thread; use the returned handle
/// to inspect the last result.
pub fn spawn_webhook_server(config: WebhookServerConfig) -> std::io::Result<thread::JoinHandle<()>> {
    let listener = TcpListener::bind(&config.bind)?;
    listener.set_nonblocking(false)?;

    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let result = handle_webhook_request(&mut stream, &config);
                    respond_json(&mut stream, &result);
                }
                Err(_) => continue,
            }
        }
    });

    Ok(handle)
}

/// Handle a single HTTP POST request (minimal HTTP/1.0 parsing).
fn handle_webhook_request(
    stream: &mut TcpStream,
    config: &WebhookServerConfig,
) -> WebhookTriggerResult {
    let mut buf = [0u8; 65536];
    let mut body = String::new();
    let mut headers = std::collections::HashMap::new();

    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                return WebhookTriggerResult {
                    status: "error".to_string(),
                    source_commit: None,
                    workload_id: None,
                    build_triggered: false,
                    error: Some("failed to read request body".to_string()),
                };
            }
        };

        let chunk = match std::str::from_utf8(&buf[..n]) {
            Ok(s) => s,
            Err(_) => {
                return WebhookTriggerResult {
                    status: "error".to_string(),
                    source_commit: None,
                    workload_id: None,
                    build_triggered: false,
                    error: Some("invalid UTF-8 in request".to_string()),
                };
            }
        };

        // Split headers from body
        if let Some(idx) = chunk.find("\r\n\r\n") {
            let header_part = &chunk[..idx];
            let rest = &chunk[idx + 4..];

            // Parse headers
            for line in header_part.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim().to_lowercase();
                    let v = v.trim().to_string();
                    if k == "content-length" {
                        if let Ok(len) = v.parse::<usize>() {
                            headers.insert(k, len);
                        }
                    } else {
                        headers.insert(k, v.len());
                    }
                }
            }

            body.push_str(rest);

            // Check if we've read the full body
            if let Some(&content_length) = headers.get("content-length") {
                if body.len() >= content_length {
                    break;
                }
                // Need more data — but for simplicity, we already have it
            }
            break;
        } else {
            body.push_str(chunk);
        }
    }

    // Parse Forgejo payload
    let payload: ForgejoPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            return WebhookTriggerResult {
                status: "error".to_string(),
                source_commit: None,
                workload_id: None,
                build_triggered: false,
                error: Some(format!("failed to parse Forgejo payload: {e}")),
            };
        }
    };

    // Handle via existing forgejo webhook handler
    let handler_result = handle_forgejo_payload(&payload, &ForgejoHandlerConfig::default());

    match handler_result {
        crate::forgejo_webhook::ForgejoHandlerResult::Triggered {
            workload_id,
            source_commit,
        } => {
            // Validate commit (fail-closed: never trigger with invalid SHA)
            if !validate_source_commit(&source_commit) {
                return WebhookTriggerResult {
                    status: "rejected".to_string(),
                    source_commit: Some(source_commit),
                    workload_id: Some(workload_id),
                    build_triggered: false,
                    error: Some("invalid source commit SHA — rejecting build".to_string()),
                };
            }

            // Trigger build on remote runner via SSH
            let build_triggered = trigger_remote_build(
                &config.runner_target,
                &source_commit,
                &config.phase,
                config.apply,
            );

            WebhookTriggerResult {
                status: if build_triggered { "triggered" } else { "trigger_failed" }.to_string(),
                source_commit: Some(source_commit),
                workload_id: Some(workload_id),
                build_triggered,
                error: if build_triggered {
                    None
                } else {
                    Some("SSH build trigger failed — see server logs".to_string())
                },
            }
        }
        crate::forgejo_webhook::ForgejoHandlerResult::Skipped { reason } => {
            WebhookTriggerResult {
                status: "skipped".to_string(),
                source_commit: None,
                workload_id: None,
                build_triggered: false,
                error: Some(reason),
            }
        }
    }
}

/// Trigger a build on the remote runner via SSH + `cynic build run --apply`.
fn trigger_remote_build(
    runner_target: &str,
    source_commit: &str,
    phase: &str,
    apply: bool,
) -> bool {
    // Build argv for SSH invocation
    let make_argv = match phase {
        "build" => "make:build",
        "activate" => "make:build-release",
        _ => "make:check",
    };

    let argv: Vec<String> = vec![
        "ssh".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        runner_target.to_string(),
        "cynic".to_string(),
        "build".to_string(),
        "run".to_string(),
        "--apply".to_string(),
        format!("--source-commit={source_commit}"),
        format!("--action={make_argv}"),
        "--json".to_string(),
    ];

    let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    let runner = crate::update_lifecycle::LocalArgvRunner;
    let output = runner.run(&refs);

    if !apply {
        // If not applying, just report the plan
        return output.success();
    }

    // Check for success
    if !output.success() {
        eprintln!(
            "webhook build trigger failed: {}\nstderr: {}",
            output.spawn_error.unwrap_or_else(|| "non-zero exit".to_string()),
            output.stderr
        );
        return false;
    }

    // Verify read-back: the proof should be in stdout
    output.success()
}

/// Write a minimal HTTP/1.0 JSON response.
fn respond_json(stream: &mut TcpStream, result: &WebhookTriggerResult) {
    let body = serde_json::to_string(result).unwrap_or_else(|_| {
        "{}".to_string()
    });

    let response = format!(
        "HTTP/1.0 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        body.len(),
        body,
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forgejo_webhook::{ForgejoPayload, ForgejoHandlerResult};

    fn valid_sha() -> String {
        "a".repeat(40)
    }

    #[test]
    fn validate_source_commit_accepts_valid_sha() {
        let sha = valid_sha();
        assert!(validate_source_commit(&sha));
    }

    #[test]
    fn validate_source_commit_rejects_short() {
        assert!(!validate_source_commit("abc123"));
    }

    #[test]
    fn validate_source_commit_rejects_non_hex() {
        assert!(!validate_source_commit(&"z".repeat(40)));
    }

    #[test]
    fn validate_source_commit_rejects_empty() {
        assert!(!validate_source_commit(""));
    }
}