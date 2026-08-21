//! Integration test for the Forgejo webhook server.
//!
//! Spawns the real TCP webhook server on a loopback port and sends a
//! mock Forgejo push-event POST via raw TCP (no external HTTP dependency).
//! Verifies: HTTP parsing → Forgejo payload parsing → commit validation →
//! build trigger attempt → JSON read-back proof structure.

use cynic_host::forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};
use std::io::{Read, Write};
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

/// Send a POST request and return (raw_response, body).
fn send_post(port: u16, body: &str) -> (String, String) {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(5),
    )
    .expect("should connect to webhook server");

    let request = format!(
        "POST /webhook/forgejo HTTP/1.0\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n{}",
        body.len(),
        body,
    );

    stream
        .write_all(request.as_bytes())
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

fn test_config(port: u16) -> WebhookServerConfig {
    WebhookServerConfig {
        bind: format!("127.0.0.1:{port}"),
        phase: "check".to_string(),
        runner_target: "cynic@10.10.10.119".to_string(),
        apply: true,
        registry_path: "infra/registry.json".to_string(),
    }
}

#[test]
fn webhook_server_accepts_valid_payload_and_attempts_trigger() {
    let port = find_free_port();
    let config = test_config(port);

    let _handle = spawn_webhook_server(config).expect("should spawn server");
    thread::sleep(Duration::from_millis(200));

    let (_resp, body) = send_post(port, VALID_FORGEJO_PAYLOAD);

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");
    let status = result["status"].as_str().expect("status field");

    // With no SSH key configured, the build attempt will fail, but the webhook
    // should still return a structured result (fail-closed, not crash).
    assert!(
        status == "triggered" || status == "trigger_failed",
        "expected triggered or trigger_failed, got {status}"
    );
    assert_eq!(
        result["source_commit"].as_str().unwrap(),
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert!(result["workload_id"].is_string());
    assert_eq!(result["build_triggered"].as_bool().unwrap(), false);
}

#[test]
fn webhook_server_skips_invalid_commit_sha() {
    let port = find_free_port();
    let config = test_config(port);

    let _handle = spawn_webhook_server(config).expect("should spawn server");
    thread::sleep(Duration::from_millis(200));

    let (_resp, body) = send_post(port, INVALID_FORGEJO_PAYLOAD);

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");

    // Invalid SHA → extract_source_commit returns None → Skipped
    assert_eq!(result["status"].as_str().unwrap(), "skipped");
    assert!(!result["build_triggered"].as_bool().unwrap());
    assert_eq!(
        result["error"].as_str().unwrap(),
        "no_commit_sha_found",
        "error should be no_commit_sha_found"
    );
}

#[test]
fn webhook_server_rejects_malformed_json() {
    let port = find_free_port();
    let config = test_config(port);

    let _handle = spawn_webhook_server(config).expect("should spawn server");
    thread::sleep(Duration::from_millis(200));

    let (_resp, body) = send_post(port, "{not valid json}");

    let result: serde_json::Value = serde_json::from_str(&body).expect("should parse JSON");

    assert_eq!(result["status"].as_str().unwrap(), "error");
    assert!(
        result["error"].as_str().unwrap().contains("parse"),
        "error should mention parse failure"
    );
}
