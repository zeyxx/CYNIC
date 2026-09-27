//! Node identity — cryptographic fingerprint from ed25519 keypair.
//!
//! The node_id is NOT spoofable via env var. It is derived from a
//! persistent ed25519 keypair stored in the principal's Hermes directory.
//! `CYNIC_NODE_ID` override is reserved for tests/dev and logs a warning.

use blake3::Hasher;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::path::PathBuf;

const IDENTITY_DIR: &str = "node-identity";
const KEYPAIR_FILE: &str = "ed25519.key";
const NODE_ID_LEN: usize = 16; // 128-bit fingerprint

/// Path to the principal's Hermes config directory.
fn hermes_dir() -> PathBuf {
    std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".hermes")
        })
}

fn keypair_path() -> PathBuf {
    hermes_dir().join(IDENTITY_DIR).join(KEYPAIR_FILE)
}

/// Load or generate the node's persistent ed25519 keypair.
pub fn load_or_generate_keypair() -> SigningKey {
    let path = keypair_path();

    if path.exists() {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(%e, "Failed to read node identity keypair, regenerating");
                return generate_and_save(&path);
            }
        };
        if bytes.len() == 32 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes);
            SigningKey::from_bytes(&seed)
        } else {
            tracing::warn!("Invalid keypair length {}, regenerating", bytes.len());
            generate_and_save(&path)
        }
    } else {
        generate_and_save(&path)
    }
}

fn generate_and_save(path: &std::path::Path) -> SigningKey {
    let dir = path.parent().unwrap();
    let _ = std::fs::create_dir_all(dir);

    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("getrandom: failed to generate seed");
    let keypair = SigningKey::from_bytes(&seed);

    match std::fs::write(path, keypair.to_bytes()) {
        Ok(()) => {
            // Restrict permissions (Unix only) — private key must be owner-only.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
            }
        }
        Err(e) => {
            tracing::error!(%e, "Failed to persist node identity keypair");
        }
    }

    keypair
}

/// Compute the stable node_id as BLAKE3(pubkey)[..16] in hex.
pub fn crypto_node_id() -> String {
    let keypair = load_or_generate_keypair();
    let public: VerifyingKey = (&keypair).into();
    let pubkey_bytes = public.to_bytes();

    let mut hasher = Hasher::new();
    hasher.update(&pubkey_bytes);
    let hash = hasher.finalize();

    hash.as_bytes()[..NODE_ID_LEN]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Best-effort stable node_id — crypto primary, env override for dev/test.
pub fn stable_node_id() -> String {
    resolve_node_id(std::env::var("CYNIC_NODE_ID").ok().as_deref())
}

/// Pure decision: a non-empty env override wins (test/dev only), otherwise the
/// crypto fingerprint from the persistent ed25519 keypair is used.
fn resolve_node_id(override_id: Option<&str>) -> String {
    if let Some(raw) = override_id {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            tracing::warn!(
                "CYNIC_NODE_ID override active — node_id is NOT cryptographically bound. \
                 Use only in dev/test."
            );
            return trimmed.to_string();
        }
    }

    crypto_node_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_node_id_is_32_hex_chars() {
        let id = crypto_node_id();
        assert_eq!(id.len(), NODE_ID_LEN * 2); // 32 hex chars
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn crypto_node_id_is_stable() {
        let id1 = crypto_node_id();
        let id2 = crypto_node_id();
        assert_eq!(id1, id2, "node_id must be stable across calls");
    }

    #[test]
    fn override_takes_precedence() {
        assert_eq!(resolve_node_id(Some("test-node-override")), "test-node-override");
    }

    #[test]
    fn empty_override_falls_back_to_crypto() {
        let crypto = crypto_node_id();
        assert_eq!(resolve_node_id(None), crypto);
        assert_eq!(resolve_node_id(Some("")), crypto);
        assert_eq!(resolve_node_id(Some("   ")), crypto);
    }

    #[test]
    fn override_is_trimmed() {
        assert_eq!(resolve_node_id(Some("  padded  ")), "padded");
    }
}
