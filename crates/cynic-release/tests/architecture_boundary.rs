use std::fs;
use std::path::PathBuf;

fn release_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_never_depends_on_high_layer_crates() {
    let cargo_toml_path = release_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", cargo_toml_path, e));

    let forbidden = [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-mcp",
    ];
    for dep in &forbidden {
        assert!(
            !content.contains(dep),
            "Architecture Violation: cynic-release must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn release_evaluator_is_pure_zero_io() {
    let path = release_crate_root().join("src/evaluator.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    assert!(
        !content.contains("std::process::Command"),
        "Pure Domain Violation: evaluator contains `std::process::Command`!"
    );
    assert!(
        !content.contains("reqwest"),
        "Pure Domain Violation: evaluator contains network HTTP dependencies!"
    );
    assert!(
        !content.contains("tokio::spawn"),
        "Pure Domain Violation: evaluator spawns async runtimes!"
    );
    assert!(
        !content.contains("std::fs::write"),
        "Pure Domain Violation: evaluator mutates filesystem!"
    );
}
