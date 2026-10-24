use cynic_vcs::providers::forgejo_webhook::validate_source_commit;
use cynic_vcs::providers::forgejo_webhook_server::*;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

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
