//! Architecture boundary tests for cynic-domain-askesis.
//!
//! Enforces: strict DAG — this crate may only depend on `cynic-askesis`
//! and `cynic-protocol` (not higher-layer crates). cynic-askesis is a
//! pure domain crate (no I/O), so depending on it is safe.

const FORBIDDEN_DEPS: &[&str] = &[
    "cynic-host",
    "cynic-kernel",
    "cynic-node",
    "cynic-cli",
    "cynic-mcp",
    "cynic-network",
    "cynic-inference",
    "cynic-market",
    "cynic-observer",
    "cynic-storage",
    "cynic-zeroclaw-solana",
    "cynic-release",
    "cynic-build",
    "cynic-sandbox",
    "cynic-lifecycle",
    "cynic-intel",
    "cynic-intel-escore",
    "cynic-intel-futarchy",
    "cynic-intel-soc",
    "cynic-consensus",
    "cynic-channel",
    "cynic-workload",
    "cynic-forensics",
    "cynic-lxc",
    "cynic-pkg",
    "cynic-pool",
    "cynic-vcs",
    "cynic-domain-ops", // askesis is a leaf adapter — no need for the ops crate
];

#[test]
fn no_dependency_on_higher_layer_crates() {
    let cargo_toml = include_str!("../Cargo.toml");
    for dep in FORBIDDEN_DEPS {
        assert!(
            !cargo_toml.contains(dep),
            "Architecture Violation: cynic-domain-askesis must not depend on \
             {dep} (violates strict DAG)"
        );
    }
}

#[test]
fn lib_rs_is_thin_facade() {
    let content = include_str!("../src/lib.rs");
    assert!(
        content.contains("pub mod") || content.contains("pub use"),
        "lib.rs must be a thin facade with module declarations or re-exports"
    );
}
