use std::fs;
use std::path::PathBuf;

fn build_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn build_never_depends_on_high_layer_crates() {
    let cargo_toml_path = build_crate_root().join("Cargo.toml");
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
            "Architecture Violation: cynic-build must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn build_compressor_is_pure_zero_io() {
    let path = build_crate_root().join("src/compressor.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    assert!(
        !content.contains("std::process::Command"),
        "Pure Domain Violation: compressor contains `std::process::Command`!"
    );
    assert!(
        !content.contains("reqwest"),
        "Pure Domain Violation: compressor contains network HTTP dependencies!"
    );
    assert!(
        !content.contains("tokio::spawn"),
        "Pure Domain Violation: compressor spawns async runtimes!"
    );
}

#[test]
fn build_runner_depends_on_abstract_argv_runner() {
    let path = build_crate_root().join("src/lib.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    assert!(
        content.contains("runner: &dyn BuildRunner"),
        "Port Inversion Violation: run_build must take abstract `&dyn BuildRunner`"
    );
}
