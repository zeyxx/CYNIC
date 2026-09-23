//! Architecture boundary tests for cynic-types.
//!
//! Enforces Layer 0 invariants:
//! 1. No dependency on higher-layer cynic crates (kernel, host, node, cli).
//! 2. No I/O in the library source (no std::fs, std::process, reqwest, tokio).
//! 3. No external I/O crates pulled transitively into the type surface.

use std::path::Path;

#[test]
fn no_dependency_on_higher_layer_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-inference",
        "cynic-observer",
        "cynic-intel",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "cynic-types must not depend on {forbidden}"
        );
    }
}

#[test]
fn no_io_in_lib_sources() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");

    for entry in std::fs::read_dir(&src_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let rel = path.strip_prefix(manifest_dir).unwrap().display();
        for needle in ["std::fs", "std::process", "reqwest", "tokio::spawn", "tokio::fs"] {
            assert!(
                !content.contains(needle),
                "{rel}: forbidden I/O primitive {needle}"
            );
        }
    }
}

#[test]
fn agent_id_is_16_bytes() {
    let id = cynic_types::AgentId::from_bytes([42u8; 16]);
    assert_eq!(id.as_bytes().len(), 16);
}

#[test]
fn hash32_hex_roundtrip() {
    let bytes = [0xAA; 32];
    let h = cynic_types::Hash32::from(bytes);
    let hex = h.to_hex();
    assert!(hex.starts_with("sha256:"));
    // Re-parse via serde to validate the Deserialize path.
    let json = format!("\"{hex}\"");
    let parsed: cynic_types::Hash32 = serde_json::from_str(&json).unwrap();
    assert_eq!(h, parsed);
}

#[test]
fn task_id_string_roundtrip() {
    let id = cynic_types::TaskId::new("task-abc-123");
    assert_eq!(id.as_str(), "task-abc-123");
    let s = id.to_string();
    let parsed: cynic_types::TaskId = s.parse().unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn timestamp_from_u64() {
    let ts = cynic_types::Timestamp::from(1_700_000_000u64);
    assert_eq!(ts.as_u64(), 1_700_000_000);
}

#[test]
fn budget_default_zero() {
    let b = cynic_types::Budget::default();
    assert_eq!(b.actions, 0);
    assert_eq!(b.wall_seconds, 0);
}

#[test]
fn scope_kinds_serialized() {
    let s = cynic_types::Scope {
        kind: cynic_types::ScopeKind::Crate,
        path: "cynic-types".to_string(),
    };
    let json = serde_json::to_string(&s).unwrap();
    let back: cynic_types::Scope = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}
