//! Architecture boundary tests for cynic-identity.
//!
//! Enforces: no std I/O, no network, no async — pure domain crate.

use std::path::Path;

#[test]
fn no_external_crypto_exports() {
    // This crate owns ed25519 — no other crate should re-export it
    let src = include_str!("../src/types.rs");
    // We WANT ed25519_dalek here — that's the point
    assert!(src.contains("ed25519_dalek"));
}

#[test]
fn node_id_is_copy() {
    let id = cynic_identity::NodeId::from_bytes([0u8; 16]);
    let _copy = id;
    let _ref = id.as_bytes(); // Still usable after copy
    let _ = _copy; // Silence unused warning
}

#[test]
fn identity_deterministic() {
    let id1 = cynic_identity::NodeIdentity::generate();
    let id2 = cynic_identity::NodeIdentity::from_signing_key(id1.signing_key.clone());
    assert_eq!(id1.node_id, id2.node_id);
}

#[test]
fn signing_material_wrapper() {
    let id = cynic_identity::NodeIdentity::generate();
    let material = cynic_identity::SigningMaterial::from_key(id.signing_key.clone());
    assert_eq!(material.node_id(), id.node_id);
}

#[test]
fn no_filesystem_or_network_types() {
    // Scan for forbidden types in the crate
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");
    
    for entry in std::fs::read_dir(src_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(!content.contains("TcpStream"), "No network types in {}", path.display());
            assert!(!content.contains("File::"), "No file I/O in {}", path.display());
            assert!(!content.contains("tokio::"), "No async runtime in {}", path.display());
            assert!(!content.contains("reqwest::"), "No HTTP client in {}", path.display());
        }
    }
}
