//! Architecture boundary tests for cynic-secrets.
//!
//! Enforces CYNIC crate invariants (AGENTS.md):
//!   1. Strict DAG: cynic-secrets depends ONLY on cynic-protocol.
//!   2. Zero-I/O Protocol Core: protocol.rs and ports.rs are 100% pure.
//!   3. Thin Facade: lib.rs is module declarations + re-exports only.

use std::path::Path;

const FORBIDDEN_DEPS: &[&str] = &[
    "cynic-host",
    "cynic-kernel",
    "cynic-node",
    "cynic-cli",
    "cynic-mcp",
    "cynic-askesis",
    "cynic-domain-ops",
    "cynic-domain-porkbun",
    "cynic-domain-cloudflare",
];

const PURE_FILES: &[&str] = &["src/protocol.rs", "src/ports.rs", "src/composite.rs"];

#[test]
fn no_dependency_on_higher_layer_crates() {
    let cargo_toml = include_str!("../Cargo.toml");
    for dep in FORBIDDEN_DEPS {
        assert!(
            !cargo_toml.contains(dep),
            "Architecture Violation: cynic-secrets must not depend on {dep} \
             (violates strict DAG — higher-layer crates cannot be dependencies)"
        );
    }
}

#[test]
fn pure_layers_are_zero_io() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel_path in PURE_FILES {
        let path = base.join(rel_path);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

        assert!(
            !content.contains("std::process::Command"),
            "Pure Layer Violation in {rel_path}: must not use std::process::Command"
        );
        assert!(
            !content.contains("reqwest"),
            "Pure Layer Violation in {rel_path}: must not use reqwest"
        );
        assert!(
            !content.contains("tokio::spawn"),
            "Pure Layer Violation in {rel_path}: must not use tokio::spawn"
        );
    }
}

#[test]
fn lib_rs_is_thin_facade() {
    let content = include_str!("../src/lib.rs");

    assert!(
        !content.contains("std::fs::"),
        "Facade Violation: lib.rs must not use std::fs directly"
    );
    assert!(
        !content.contains("std::process::Command"),
        "Facade Violation: lib.rs must not use std::process::Command directly"
    );
    assert!(
        content.contains("pub mod") || content.contains("pub use"),
        "lib.rs must be a thin facade with module declarations or re-exports"
    );
}

#[test]
fn adapters_directory_exists() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapters_dir = base.join("src/adapters");
    assert!(
        adapters_dir.exists(),
        "adapters/ directory must exist — all I/O is bounded there"
    );
}
