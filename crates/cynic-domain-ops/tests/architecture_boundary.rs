//! Architecture boundary tests for cynic-domain-ops.
//!
//! Enforces the CYNIC crate invariants (AGENTS.md Architecture & Crate Invariants):
//!   1. Strict DAG: cynic-domain-ops depends ONLY on cynic-protocol (pure types).
//!      Must NOT depend on cynic-host, cynic-kernel, cynic-node, cynic-cli, or
//!      cynic-mcp.
//!   2. Zero-I/O Domain Core: the `domain/` module is 100% pure — no
//!      `std::process::Command`, no `reqwest`, no `tokio::spawn`.
//!   3. Facade & Ratchet: `lib.rs` is thin — no `std::fs`, no `std::process`.

use std::path::Path;

const FORBIDDEN_DEPS: &[&str] = &[
    "cynic-host",
    "cynic-kernel",
    "cynic-node",
    "cynic-cli",
    "cynic-mcp",
    "cynic-askesis",
];

#[test]
fn no_dependency_on_higher_layer_crates() {
    let cargo_toml = include_str!("../Cargo.toml");
    for dep in FORBIDDEN_DEPS {
        assert!(
            !cargo_toml.contains(dep),
            "Architecture Violation: cynic-domain-ops must not depend on {dep} \
             (violates strict DAG — higher-layer crates cannot be dependencies)"
        );
    }
}

#[test]
fn domain_module_is_pure_zero_io() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain_dir = base.join("src/domain");

    assert!(
        domain_dir.exists(),
        "domain/ directory must exist"
    );

    for entry in std::fs::read_dir(&domain_dir).expect("read domain dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

            assert!(
                !content.contains("std::process::Command"),
                "Pure Domain Violation in {}: domain logic must not use std::process::Command",
                path.file_name().unwrap().to_str().unwrap()
            );
            assert!(
                !content.contains("reqwest"),
                "Pure Domain Violation in {}: domain logic must not use reqwest",
                path.file_name().unwrap().to_str().unwrap()
            );
            assert!(
                !content.contains("tokio::spawn"),
                "Pure Domain Violation in {}: domain logic must not use tokio::spawn",
                path.file_name().unwrap().to_str().unwrap()
            );
        }
    }
}

#[test]
fn lib_rs_is_thin_facade() {
    let content = include_str!("../src/lib.rs");

    assert!(
        !content.contains("std::fs"),
        "Facade Violation: lib.rs must not use std::fs directly"
    );
    assert!(
        !content.contains("std::process"),
        "Facade Violation: lib.rs must not use std::process directly"
    );
}

#[test]
fn only_protocol_reexported_not_executed() {
    let content = include_str!("../src/lib.rs");
    // lib.rs should re-export from domain/ports/runner, not define business logic
    assert!(
        content.contains("pub mod") || content.contains("pub use"),
        "lib.rs must be a thin facade with module declarations or re-exports"
    );
}
