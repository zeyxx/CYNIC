//! Architecture boundary — cynic-eval.
//!
//! Domain crate: depends only on cynic-protocol residue. Never on host,
//! node, kernel, cli, mcp, or any orchestrator crate.

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
            "boundary violation: cynic-eval must not depend on {forbidden}"
        );
    }
}

#[test]
fn domain_modules_are_pure() {
    let src_dir = manifest_dir().join("src");
    let mut stack = vec![src_dir.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let src = std::fs::read_to_string(&path).unwrap();
                for forbidden in ["std::process", "reqwest", "tokio::spawn"] {
                    assert!(
                        !src.contains(forbidden),
                        "purity violation in {}: external I/O primitive {}",
                        path.display(),
                        forbidden
                    );
                }
            }
        }
    }
}
