//! Admin signal client — send HMAC-signed signals to a node, no SSH.
//!
//! WHY: `/admin/drain` and `/admin/restart` close the loop on autonomous
//! restarts, but a signal nobody can send is a dead endpoint. This module
//! signs the exact body bytes with the shared secret (HMAC-SHA256, header
//! `x-cynic-signature`, format `sha256=<hex>`) and POSTs them blocking.
//! NEGATIVE BOUNDARY: the secret arrives as a function argument — the CLI
//! reads it from `CYNIC_ADMIN_SECRET`, never from argv (process lists
//! leak argv) and never into logs.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign exact body bytes. Output matches what the server verifies
/// (`sha256=<hex>`, prefix stripped server-side before compare).
pub fn sign_admin_body(secret: &str, body: &[u8]) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| "hmac_key_rejected".to_string())?;
    mac.update(body);
    Ok(format!(
        "sha256={}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

/// POST a signed JSON body to an admin path. Returns the decoded reply.
///
/// Fails closed on transport error, timeout, non-2xx status, and
/// unparsable replies — a refused signal never reads as sent.
pub fn send_admin_signal(
    base_url: &str,
    path: &str,
    secret: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw = serde_json::to_vec(body).map_err(|e| format!("admin_body_unserializable: {e}"))?;
    let signature = sign_admin_body(secret, &raw)?;
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let reply = std::thread::spawn(move || {
        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        else {
            return Err("admin_client_build_failed".to_string());
        };
        let response = client
            .post(&url)
            .header("x-cynic-signature", &signature)
            .header("content-type", "application/json")
            .body(raw)
            .send()
            .map_err(|e| format!("admin_signal_unreachable: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("admin_signal_refused: http {}", response.status()));
        }
        response
            .json::<serde_json::Value>()
            .map_err(|e| format!("admin_reply_unparsable: {e}"))
    })
    .join()
    .map_err(|_| "admin_signal_thread_failed".to_string())??;
    Ok(reply)
}

/// Drain request body. The server ignores content — only the signature
/// matters — but the body must be byte-stable for HMAC.
pub fn drain_body() -> serde_json::Value {
    serde_json::json!({})
}

/// Restart request body naming the desired installed binary by sha256.
pub fn restart_body(desired_sha256: &str) -> serde_json::Value {
    serde_json::json!({ "desired_sha256": desired_sha256 })
}

/// Send a drain signal. See `/admin/drain` semantics: stop admitting,
/// finish in-flight, then exit for supervised relaunch.
pub fn send_drain(base_url: &str, secret: &str) -> Result<serde_json::Value, String> {
    send_admin_signal(base_url, "/admin/drain", secret, &drain_body())
}

/// Send a restart signal. The node bounces only when the installed
/// binary already matches `desired_sha256` (install first, then signal).
pub fn send_restart(
    base_url: &str,
    secret: &str,
    desired_sha256: &str,
) -> Result<serde_json::Value, String> {
    send_admin_signal(
        base_url,
        "/admin/restart",
        secret,
        &restart_body(desired_sha256),
    )
}

/// Extract the HTTP status from a `send_admin_signal` refusal
/// (`admin_signal_refused: http NNN`). Pure so the ceremony can branch on
/// proof outcomes without parsing strings at the call site.
pub fn refused_status_code(error: &str) -> Option<u16> {
    error
        .strip_prefix("admin_signal_refused: http ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|code| code.parse().ok())
}

/// Fetch the public capacity gauge. No signature: proves the HTTP surface
/// is up after a restart, before any signed proof is attempted.
pub fn fetch_capacity(base_url: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}/capacity", base_url.trim_end_matches('/'));
    std::thread::spawn(move || {
        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        else {
            return Err("capacity_client_build_failed".to_string());
        };
        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("capacity_unreachable: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("capacity_refused: http {}", response.status()));
        }
        response
            .json::<serde_json::Value>()
            .map_err(|e| format!("capacity_unparsable: {e}"))
    })
    .join()
    .map_err(|_| "capacity_thread_failed".to_string())?
}

/// Prove the freshly provisioned secret is live without disrupting the
/// node: a signed restart naming an impossible digest must be REFUSED
/// (409, signature verified, digest rejected) — never UNAUTHORIZED (401,
/// secret missing or wrong). Any 409 means the secret works end to end.
pub fn prove_secret_live(base_url: &str, secret: &str) -> Result<(), String> {
    let impossible = format!("sha256:{}", "0".repeat(64));
    match send_restart(base_url, secret, &impossible) {
        Ok(_) => Err("secret_proof_unexpected_success: impossible digest accepted".to_string()),
        Err(error) => match refused_status_code(&error) {
            Some(409) => Ok(()),
            Some(401) => Err("secret_not_live: server refused the signature".to_string()),
            Some(code) => Err(format!("secret_proof_unexpected_status: http {code}")),
            None => Err(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_rfc4231_test_case_1() {
        // RFC 4231 §4.2, HMAC-SHA-256 test case 1: key 0x0b * 20,
        // data "Hi There".
        let key = vec![0x0bu8; 20];
        let key_str = String::from_utf8(vec![0x0bu8; 20]).unwrap();
        assert_eq!(key.len(), 20);
        let signature = sign_admin_body(&key_str, b"Hi There").unwrap();
        assert_eq!(
            signature,
            "sha256=b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn signature_covers_exact_body_bytes() {
        let first = sign_admin_body("secret", b"{}").unwrap();
        let second = sign_admin_body("secret", b"{ }").unwrap();
        assert_ne!(first, second, "one whitespace byte must change the MAC");
        assert!(first.starts_with("sha256="));
        assert_eq!(first.len(), "sha256=".len() + 64);
    }

    #[test]
    fn restart_body_names_the_desired_digest() {
        let digest = "b".repeat(64);
        let body = restart_body(&digest);
        assert_eq!(
            body.get("desired_sha256").and_then(|v| v.as_str()),
            Some(digest.as_str())
        );
    }

    #[test]
    fn unreachable_node_fails_closed_not_silent() {
        let error = send_drain("http://127.0.0.1:9", "secret")
            .expect_err("unreachable admin must fail, not pretend sent");
        assert!(
            error.contains("admin_signal_unreachable"),
            "expected precise unreachable code, got: {error}"
        );
    }

    #[test]
    fn refusal_status_parses_for_ceremony_branching() {
        assert_eq!(
            refused_status_code("admin_signal_refused: http 409 Conflict"),
            Some(409)
        );
        assert_eq!(
            refused_status_code("admin_signal_refused: http 401 Unauthorized"),
            Some(401)
        );
        assert_eq!(refused_status_code("admin_signal_unreachable: foo"), None);
        assert_eq!(refused_status_code("garbage"), None);
    }

    #[test]
    fn impossible_digest_proof_fails_closed_offline() {
        let error = prove_secret_live("http://127.0.0.1:9", "secret")
            .expect_err("offline proof must fail, not pretend live");
        assert!(
            error.contains("admin_signal_unreachable"),
            "expected unreachable code, got: {error}"
        );
    }
}
