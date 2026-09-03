//! Forgejo webhook HTTP server.
//!
//! Receives Forgejo push-event POST requests, verifies the HMAC signature
//! when a secret is configured, validates the source commit, and forwards
//! the raw payload to the node builder webhook (`POST /webhook/forgejo`)
//! where admission, durable persistence, and scheduling live.
//!
//! This server owns no build policy and triggers no build itself. It is a
//! thin ingress projection: verify → validate → forward → report.
//! The single admitted webhook path is the node `builder_http` surface.
//!
//! No external HTTP dependency — std::net::TcpListener for ingress,
//! reqwest (blocking) only for the forward leg.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use crate::forgejo_webhook::{
    ForgejoHandlerConfig, handle_forgejo_payload, validate_source_commit,
};

/// Configuration for the Forgejo webhook server.
#[derive(Debug, Clone)]
pub struct WebhookServerConfig {
    /// Address to bind (e.g. "127.0.0.1:9147").
    pub bind: String,
    /// Node builder webhook URL (e.g. "http://127.0.0.1:3030/webhook/forgejo").
    /// Empty means forwarding is unconfigured: pushes validate but never
    /// forward (explicit blocked result, never a guessed target).
    pub node_webhook_url: String,
    /// Shared Forgejo webhook secret. When set, the
    /// `X-Forgejo-Signature` header (hex HMAC-SHA256 over the raw body)
    /// must verify or the push is rejected. When unset, pushes are
    /// accepted without signature proof and reported as such.
    /// Load from `CYNIC_WEBHOOK_SECRET`, never from CLI flags.
    pub webhook_secret: Option<String>,
}

/// Result of a webhook-forwarded build request.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookTriggerResult {
    pub status: String,
    pub source_commit: Option<String>,
    pub workload_id: Option<String>,
    pub build_triggered: bool,
    pub build_proof: Option<String>,
    pub signature_verified: bool,
    pub error: Option<String>,
}

/// Spawn the webhook server thread.
pub fn spawn_webhook_server(
    config: WebhookServerConfig,
) -> std::io::Result<thread::JoinHandle<()>> {
    let listener = TcpListener::bind(&config.bind)?;
    listener.set_nonblocking(false)?;

    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let result = handle_webhook_request(&mut stream, &config);
                respond_json(&mut stream, &result);
            }
        }
    });

    Ok(handle)
}

/// A parsed HTTP request: lowercase header map plus the raw body bytes.
/// The raw bytes are retained so the HMAC verifies exactly what Forgejo sent.
struct ParsedRequest {
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<ParsedRequest, String> {
    let mut buf = [0u8; 8192];
    let mut raw: Vec<u8> = Vec::new();
    // Bounded read: headers plus at most 1 MiB of body.
    while raw.len() < 1_100_000 {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return Err("failed to read request".to_string()),
        };
        raw.extend_from_slice(&buf[..n]);
        if let Some(idx) = find_header_end(&raw) {
            let body_len = content_length(&raw[..idx]);
            if raw.len() >= idx + body_len {
                let head = String::from_utf8_lossy(&raw[..idx]).into_owned();
                let body = raw[idx..idx + body_len.min(raw.len() - idx)].to_vec();
                return Ok(ParsedRequest {
                    headers: parse_headers(&head),
                    body,
                });
            }
        }
    }
    Err("incomplete HTTP request".to_string())
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn content_length(head: &[u8]) -> usize {
    let text = String::from_utf8_lossy(head);
    text.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn parse_headers(head: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in head.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    headers
}

fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let presented = match hex::decode(signature.trim()) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&presented).is_ok()
}

/// Handle a single HTTP POST request: verify → validate → forward → report.
fn handle_webhook_request(
    stream: &mut TcpStream,
    config: &WebhookServerConfig,
) -> WebhookTriggerResult {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => return error_result(&error),
    };

    let signature_verified = match config.webhook_secret.as_deref() {
        Some(secret) => {
            let presented = request
                .headers
                .get("x-forgejo-signature")
                .map(String::as_str)
                .unwrap_or("");
            if !verify_signature(secret, &request.body, presented) {
                return WebhookTriggerResult {
                    status: "rejected".to_string(),
                    source_commit: None,
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified: false,
                    error: Some("invalid webhook signature".to_string()),
                };
            }
            true
        }
        None => false,
    };

    // Parse Forgejo payload from the exact bytes that were verified.
    let body_text = match std::str::from_utf8(&request.body) {
        Ok(text) => text,
        Err(_) => return error_result("invalid UTF-8 in request body"),
    };
    let payload: crate::forgejo_webhook::ForgejoPayload = match serde_json::from_str(body_text) {
        Ok(payload) => payload,
        Err(error) => {
            return error_result(&format!("failed to parse Forgejo payload: {error}"));
        }
    };

    // Handle via existing forgejo webhook handler
    let handler_result = handle_forgejo_payload(&payload, &ForgejoHandlerConfig::default());

    match handler_result {
        crate::forgejo_webhook::ForgejoHandlerResult::Triggered {
            workload_id: _,
            source_commit,
        } => {
            // Fail-closed: never forward with invalid SHA
            if !validate_source_commit(&source_commit) {
                return WebhookTriggerResult {
                    status: "rejected".to_string(),
                    source_commit: Some(source_commit),
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified,
                    error: Some("invalid source commit SHA — rejecting build".to_string()),
                };
            }

            if config.node_webhook_url.trim().is_empty() {
                return WebhookTriggerResult {
                    status: "blocked".to_string(),
                    source_commit: Some(source_commit),
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified,
                    error: Some(
                        "node webhook URL unconfigured — refusing to guess a target".to_string(),
                    ),
                };
            }

            forward_to_node(config, &request.body, &source_commit, signature_verified)
        }
        crate::forgejo_webhook::ForgejoHandlerResult::Skipped { reason } => WebhookTriggerResult {
            status: "skipped".to_string(),
            source_commit: None,
            workload_id: None,
            build_triggered: false,
            build_proof: None,
            signature_verified,
            error: Some(reason),
        },
    }
}

/// Forward the verified raw payload to the node builder webhook, which owns
/// admission, durable persistence, and scheduling. The signature header is
/// passed through untouched so the node verifies the same bytes.
fn forward_to_node(
    config: &WebhookServerConfig,
    raw_body: &[u8],
    source_commit: &str,
    signature_verified: bool,
) -> WebhookTriggerResult {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let client = match client {
        Ok(client) => client,
        Err(error) => {
            return error_result(&format!("webhook forward client failed: {error}"));
        }
    };
    let response = client
        .post(config.node_webhook_url.as_str())
        .header("Content-Type", "application/json")
        .body(raw_body.to_vec())
        .send();
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return error_result(&format!("webhook forward failed: {error}"));
        }
    };
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        return WebhookTriggerResult {
            status: "trigger_failed".to_string(),
            source_commit: Some(source_commit.to_string()),
            workload_id: None,
            build_triggered: false,
            build_proof: Some(text.clone()),
            signature_verified,
            error: Some(format!("node webhook refused: HTTP {}", status.as_u16())),
        };
    }
    let proof: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let workload_id = proof
        .get("workload_id")
        .and_then(|id| id.as_str())
        .map(String::from);
    WebhookTriggerResult {
        status: "triggered".to_string(),
        source_commit: Some(source_commit.to_string()),
        workload_id,
        build_triggered: true,
        build_proof: Some(text),
        signature_verified,
        error: None,
    }
}

/// Helper for the error path.
fn error_result(msg: &str) -> WebhookTriggerResult {
    WebhookTriggerResult {
        status: "error".to_string(),
        source_commit: None,
        workload_id: None,
        build_triggered: false,
        build_proof: None,
        signature_verified: false,
        error: Some(msg.to_string()),
    }
}

/// Write a minimal HTTP/1.0 JSON response.
fn respond_json(stream: &mut TcpStream, result: &WebhookTriggerResult) {
    let body = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());

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

    fn valid_sha() -> String {
        "a".repeat(40)
    }

    #[test]
    fn validate_source_commit_accepts_valid_sha() {
        assert!(validate_source_commit(&valid_sha()));
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

    #[test]
    fn error_result_has_correct_fields() {
        let r = error_result("test error");
        assert_eq!(r.status, "error");
        assert!(!r.build_triggered);
        assert_eq!(r.build_proof, None);
        assert!(!r.signature_verified);
        assert_eq!(r.source_commit, None);
        assert_eq!(r.workload_id, None);
    }

    #[test]
    fn hmac_signature_verifies_and_rejects() {
        let secret = "test-secret";
        let body = br#"{"after":"abc"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let good = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(secret, body, &good));
        assert!(!verify_signature(secret, body, "deadbeef"));
        assert!(!verify_signature(secret, b"other", &good));
        assert!(!verify_signature("wrong", body, &good));
    }

    #[test]
    fn content_length_parses_case_insensitively() {
        assert_eq!(
            content_length(b"POST /x HTTP/1.0\r\nContent-Length: 12"),
            12
        );
        assert_eq!(content_length(b"POST /x HTTP/1.0\r\ncontent-length: 5"), 5);
        assert_eq!(content_length(b"POST /x HTTP/1.0\r\nHost: y"), 0);
    }
}
