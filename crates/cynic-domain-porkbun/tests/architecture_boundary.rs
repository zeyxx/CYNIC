//! Architecture boundary tests for cynic-domain-porkbun.
//!
//! Enforces: strict DAG — this adapter crate may only depend on
//! `cynic-domain-ops` and `cynic-protocol` (not higher-layer crates).
//! lib.rs is a thin facade with no direct I/O.

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
];

#[test]
fn no_dependency_on_higher_layer_crates() {
    let cargo_toml = include_str!("../Cargo.toml");
    for dep in FORBIDDEN_DEPS {
        assert!(
            !cargo_toml.contains(dep),
            "Architecture Violation: cynic-domain-porkbun must not depend on \
             {dep} (violates strict DAG — adapter crate cannot bind upward)"
        );
    }
}

#[test]
fn lib_rs_is_thin_facade() {
    let content = include_str!("../src/lib.rs");
    assert!(
        !content.contains("std::fs::read") && !content.contains("std::process::Command"),
        "Facade Violation: lib.rs must not perform I/O directly"
    );
    assert!(
        content.contains("pub mod") || content.contains("pub use"),
        "lib.rs must be a thin facade with module declarations or re-exports"
    );
}

#[test]
fn provider_implements_registrar_port() {
    // This test ensures the trait is implemented — compile-time check.
    fn assert_registrar_port<T: cynic_domain_ops::RegistrarPort>() {}
    assert_registrar_port::<cynic_domain_porkbun::PorkbunRegistrar>();
}
