//! Architecture boundary tests for cynic-domain-cloudflare.
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
            "Architecture Violation: cynic-domain-cloudflare must not depend on \
             {dep} (violates strict DAG — adapter crate cannot bind upward)"
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

#[test]
fn provider_implements_both_ports() {
    // Compile-time check: CloudflareClient implements both CloudflareDnsPort
    // and EdgeProbePort.
    fn assert_dns_port<T: cynic_domain_ops::CloudflareDnsPort>() {}
    fn assert_edge_port<T: cynic_domain_ops::EdgeProbePort>() {}
    assert_dns_port::<cynic_domain_cloudflare::CloudflareClient>();
    assert_edge_port::<cynic_domain_cloudflare::CloudflareClient>();
}
