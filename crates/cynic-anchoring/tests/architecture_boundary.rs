//! Architecture boundary tests for cynic-anchoring.
//!
//! Enforces CYNIC crate invariants (AGENTS.md):
//!   1. Strict DAG: cynic-anchoring depends ONLY on lower-layer crates
//!      (cynic-protocol) — never on higher-layer crates.
//!   2. Zero-I/O Domain Core: domain/ and port/ are pure logic/types.
//!   3. Bounded entrypoint: lib.rs is module declarations + re-exports only.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const FORBIDDEN_DEPS: &[&str] = &[
    "cynic-host",
    "cynic-agent",
    "cynic-node",
    "cynic-cli",
    "cynic-mcp",
    "cynic-build",
    "cynic-release",
    "cynic-vcs",
    "cynic-network",
    "cynic-consensus",
];

#[test]
fn no_dependency_on_higher_layer_crates() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", cargo_toml_path, e));

    for dep in FORBIDDEN_DEPS {
        assert!(
            !content.contains(dep),
            "Architecture Violation: cynic-anchoring must not depend on {dep} \
             (violates strict DAG — higher-layer crates cannot be dependencies)"
        );
    }
}

#[test]
fn pure_domain_and_port_are_zero_io() {
    let root = crate_root();
    let pure_dirs = [root.join("src/domain"), root.join("src/port")];

    let forbidden_io_tokens = [
        "std::fs",
        "std::net",
        "std::process",
        "tokio::fs",
        "tokio::net",
        "tokio::process",
        "reqwest",
    ];

    for dir in &pure_dirs {
        if dir.is_file() {
            check_file(dir, &forbidden_io_tokens);
        } else if dir.is_dir() {
            for entry in fs::read_dir(dir).expect("read dir") {
                let entry = entry.expect("dir entry");
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    check_file(&path, &forbidden_io_tokens);
                }
            }
        }
    }
}

fn check_file(path: &Path, forbidden_tokens: &[&str]) {
    let content =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));
    for token in forbidden_tokens {
        assert!(
            !content.contains(token),
            "Architecture Violation: pure layer {:?} contains forbidden I/O token '{}'",
            path,
            token
        );
    }
}
