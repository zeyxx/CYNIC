//! Integration test for the Forgejo webhook server.
//!
//! Spawns the real TCP webhook server on a loopback port plus a stub node
//! builder webhook. Sends mock Forgejo push-event POSTs via raw TCP.
//! Verifies: HTTP parsing → HMAC verification → commit validation →
//! forward to node → workload_id read-back structure.

use cynic_host::forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

/// Forgejo push-event mock payload with a valid 40-char hex SHA on branch main.
const VALID_FORGEJO_PAYLOAD: &str = r#"{
  "ref": "refs/heads/main",
  "before": "0000000000000000000000000000000000000000",
  "after": "0123456789abcdef0123456789abcdef01234567",
  "repository": {
    "name": "CYNIC",
    "full_name": "localhost/CYNIC",
    "ssh_url": "git@127.0.0.1:CYNIC.git"
  },
  "commits": [
    {
      "id": "0123456789abcdef0123456789abcdef01234567",
      "message": "test commit",
      "author": {"name": "test", "email": "test@test.com"}
    }
  ]
}
"#;

/// Forgejo payload with an INVALID (short, non-hex) commit SHA.
/// handle_forgejo_payload treats this as Skipped (no commit found).
const INVALID_FORGEJO_PAYLOAD: &str = r#"{
  "ref": "refs/heads/main",
  "before": "0000000000000000000000000000000000000000",
  "after": "deadbeef",
  "repository": {"name": "CYNIC", "full_name": "localhost/CYNIC"},
  "commits": [{"id": "deadbeef", "message": "bad", "author": {"name": "x", "email": "x@x.com"}}]
}
"#;

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Stub node builder webhook: accepts one POST, records the raw body and the
/// signature header, then answers a fixed AcceptanceProof.
struct StubNode {
    url: String,
    seen_body: std::sync::Arc<std::sync::Mutex<String>>,
    seen_signature: std::sync::Arc<std::sync::Mutex<String>>,
}

fn spawn_stub_node() -> StubNode {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen_body = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let seen_signature = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let body_slot = seen_body.clone();
    let sig_slot = seen_signature.clone();
    thread::spawn(move || {
        for stream in listener.incoming().take(8) {
            let mut stream = match stream {
                Ok(stream) => stream,
                Err(_) => continue,
            };
            let mut buf = [0u8; 65536];
            let mut raw = Vec::new();
            while let Ok(n) = stream.read(&mut buf) {
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&raw).into_owned();
            let (head, body) = match text.split_once("\r\n\r\n") {
                Some((head, body)) => (head.to_string(), body.to_string()),
                None => (text, String::new()),
            };
            let signature = head
                .lines()
                .skip(1)
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.trim().eq_ignore_ascii_case("x-forgejo-signature") {
                        Some(value.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            *body_slot.lock().unwrap() = body;
            *sig_slot.lock().unwrap() = signature;
            let proof = r#"{"schema":"cynic.builder.acceptance_proof.v1","workload_id":"forgejo-test-1","commit_sha":"0123456789abcdef0123456789abcdef01234567","accepted_at_ms":1,"status":"pending"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                proof.len(),
                proof
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    StubNode {
        url: format!("http://127.0.0.1:{port}/webhook/forgejo"),
        seen_body,
        seen_signature,
    }
}

/// Send a POST request with optional extra headers, return (raw, body).
fn send_post(port: u16, headers: &[(&str, &str)], body: &str) -> (String, String) {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(5),
    )
    .expect("should connect to webhook server");

    let mut head = format!(
        "POST /webhook/forgejo HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("\r\n{body}"));

    stream
        .write_all(head.as_bytes())
        .expect("should write request");
    stream.flush().expect("should flush");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("should read response");

    let resp_body = response
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    (response, resp_body)
}

fn test_config(node_url: &str, secret: Option<String>) -> WebhookServerConfig {
    WebhookServerConfig {
        bind: format!("127.0.0.1:{}", find_free_port()),
        node_webhook_url: node_url.to_string(),
        webhook_secret: secret,
    }
}

fn hmac_hex(secret: &str, body: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[test]
fn webhook_forwards_valid_payload_to_node() {
    let stub = spawn_stub_node();
    let port: u16 = {
        let config = test_config(&stub.url, None);
        let port = config.bind.split(':').nth(1).unwrap().parse().unwrap();
        let _handle = spawn_webhook_server(config).expect("should spawn server");
        thread::sleep(Duration::from_millis(200));
        port
    };

    let (_resp, body) = send_post(port, &[], VALID_FORGEJO_PAYLOAD);

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    assert_eq!(result["status"].as_str().unwrap(), "triggered");
    assert_eq!(
        result["source_commit"].as_str().unwrap(),
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(result["workload_id"].as_str().unwrap(), "forgejo-test-1");
    assert_eq!(result["build_triggered"].as_bool().unwrap(), true);
    assert_eq!(result["signature_verified"].as_bool().unwrap(), false);
    // The stub saw the exact raw payload bytes.
    assert_eq!(*stub.seen_body.lock().unwrap(), VALID_FORGEJO_PAYLOAD);
}

#[test]
fn webhook_verifies_hmac_when_secret_configured() {
    let stub = spawn_stub_node();
    let secret = "e2e-secret".to_string();
    let port: u16 = {
        let config = test_config(&stub.url, Some(secret.clone()));
        let port = config.bind.split(':').nth(1).unwrap().parse().unwrap();
        let _handle = spawn_webhook_server(config).expect("should spawn server");
        thread::sleep(Duration::from_millis(200));
        port
    };

    // Missing signature → rejected, nothing forwarded.
    let (_resp, body) = send_post(port, &[], VALID_FORGEJO_PAYLOAD);
    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    assert_eq!(result["status"].as_str().unwrap(), "rejected");
    assert_eq!(result["build_triggered"].as_bool().unwrap(), false);
    assert_eq!(*stub.seen_body.lock().unwrap(), "");

    // Correct signature → forwarded with the header passed through.
    let signature = hmac_hex(&secret, VALID_FORGEJO_PAYLOAD);
    let (_resp, body) = send_post(
        port,
        &[("X-Forgejo-Signature", &signature)],
        VALID_FORGEJO_PAYLOAD,
    );
    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    assert_eq!(result["status"].as_str().unwrap(), "triggered");
    assert_eq!(result["signature_verified"].as_bool().unwrap(), true);
    assert_eq!(*stub.seen_body.lock().unwrap(), VALID_FORGEJO_PAYLOAD);
    assert_eq!(*stub.seen_signature.lock().unwrap(), signature);

    // Wrong signature → rejected.
    let (_resp, body) = send_post(
        port,
        &[("X-Forgejo-Signature", "deadbeef")],
        VALID_FORGEJO_PAYLOAD,
    );
    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    assert_eq!(result["status"].as_str().unwrap(), "rejected");
}

#[test]
fn webhook_server_skips_invalid_commit_sha() {
    let stub = spawn_stub_node();
    let port: u16 = {
        let config = test_config(&stub.url, None);
        let port = config.bind.split(':').nth(1).unwrap().parse().unwrap();
        let _handle = spawn_webhook_server(config).expect("should spawn server");
        thread::sleep(Duration::from_millis(200));
        port
    };

    let (_resp, body) = send_post(port, &[], INVALID_FORGEJO_PAYLOAD);

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");

    // Invalid SHA → extract_source_commit returns None → Skipped
    assert_eq!(result["status"].as_str().unwrap(), "skipped");
    assert!(!result["build_triggered"].as_bool().unwrap());
    assert_eq!(
        result["error"].as_str().unwrap(),
        "no_commit_sha_found",
        "error should be no_commit_sha_found"
    );
    // Nothing forwarded for a skipped push.
    assert_eq!(*stub.seen_body.lock().unwrap(), "");
}

#[test]
fn webhook_blocks_when_node_url_unconfigured() {
    let port: u16 = {
        let config = test_config("", None);
        let port = config.bind.split(':').nth(1).unwrap().parse().unwrap();
        let _handle = spawn_webhook_server(config).expect("should spawn server");
        thread::sleep(Duration::from_millis(200));
        port
    };

    let (_resp, body) = send_post(port, &[], VALID_FORGEJO_PAYLOAD);

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    assert_eq!(result["status"].as_str().unwrap(), "blocked");
    assert!(!result["build_triggered"].as_bool().unwrap());
}

#[test]
fn webhook_server_rejects_malformed_json() {
    let stub = spawn_stub_node();
    let port: u16 = {
        let config = test_config(&stub.url, None);
        let port = config.bind.split(':').nth(1).unwrap().parse().unwrap();
        let _handle = spawn_webhook_server(config).expect("should spawn server");
        thread::sleep(Duration::from_millis(200));
        port
    };

    let (_resp, body) = send_post(port, &[], "{not valid json}");

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");

    assert_eq!(result["status"].as_str().unwrap(), "error");
    assert!(
        result["error"].as_str().unwrap().contains("parse"),
        "error should mention parse failure"
    );
}
