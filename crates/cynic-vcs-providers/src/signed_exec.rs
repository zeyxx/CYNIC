//! Signed builder /exec transport.
//!
//! WHY. The builder's `/exec` runs shell as the daemon user. An unsigned
//! call is remote execution for anyone who reaches the port — the 2026-10-23
//! audit closed that hole server-side, so every client must sign. There is
//! one signing helper; forgejo.rs and github.rs both use it, no second
//! implementation.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Read the exec signing secret. Fail closed: no env, no remote shell.
pub fn exec_secret() -> Result<String, String> {
    match std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET") {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(
            "exec refused: CYNIC_FORGEJO_WEBHOOK_SECRET is not set; refusing unsigned remote shell"
                .to_string(),
        ),
    }
}

/// Hex HMAC-SHA256 over the exact body bytes. Same wire format as the
/// admin signals (`x-cynic-signature`), so the server verifies both with
/// one function.
pub fn exec_signature(secret: &str, body: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac accepts any key length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_deterministic_hex() {
        let first = exec_signature("secret", b"{\"command\":\"id\"}");
        let second = exec_signature("secret", b"{\"command\":\"id\"}");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_bodies_sign_differently() {
        assert_ne!(
            exec_signature("secret", b"a"),
            exec_signature("secret", b"b")
        );
    }

    #[test]
    fn secret_follows_the_env_fail_closed() {
        // No env mutation (unsafe-code is denied): branch on whatever the
        // ambient env holds. Either way the real function is exercised and
        // the fail-closed side is asserted, never assumed.
        match std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET") {
            Ok(value) if !value.trim().is_empty() => {
                assert_eq!(exec_secret().unwrap(), value.trim());
            }
            _ => {
                let error = exec_secret().expect_err("must refuse without secret");
                assert!(error.contains("refusing unsigned remote shell"), "got: {error}");
            }
        }
    }
}
