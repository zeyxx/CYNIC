//! Architecture boundary — cynic-solana-pay.
//!
//! Leaf crate: depends only on cynic-protocol residue. Never on host, node,
//! kernel, cli, or any orchestrator crate.

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn no_orchestrator_dependencies() {
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    for forbidden in [
        "cynic-host",
        "cynic-node",
        "cynic-kernel",
        "cynic-cli",
        "cynic-mcp",
    ] {
        assert!(
            !cargo.contains(forbidden),
            "boundary violation: cynic-solana-pay must not depend on {forbidden}"
        );
    }
}

#[test]
fn domain_modules_are_pure() {
    for module in ["constants.rs", "ops.rs"] {
        let src = std::fs::read_to_string(manifest_dir().join("src").join(module)).unwrap();
        for forbidden in ["std::process", "reqwest", "tokio::spawn", "std::fs"] {
            assert!(
                !src.contains(forbidden),
                "purity violation in {module}: external I/O primitive {forbidden}"
            );
        }
    }
}
