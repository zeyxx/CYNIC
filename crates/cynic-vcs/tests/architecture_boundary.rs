use std::fs;
use std::path::PathBuf;

fn vcs_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vcs_never_depends_on_high_layer_crates() {
    let cargo_toml_path = vcs_crate_root().join("Cargo.toml");
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
            "Architecture Violation: cynic-vcs must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn vcs_domain_modules_are_pure_zero_io() {
    // Pure domain modules must not perform direct OS/process I/O
    let domain_files = ["src/entry.rs", "src/parse.rs"];
    for rel_path in &domain_files {
        let path = vcs_crate_root().join(rel_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

        assert!(
            !content.contains("std::process::Command"),
            "Pure Domain Violation: {} contains direct `std::process::Command`! Move to an adapter or runner.",
            rel_path
        );
        assert!(
            !content.contains("reqwest"),
            "Pure Domain Violation: {} contains network HTTP dependencies!",
            rel_path
        );
        assert!(
            !content.contains("tokio::spawn"),
            "Pure Domain Violation: {} spawns async runtimes inside pure parsing/domain!",
            rel_path
        );
    }
}

#[test]
fn vcs_runners_are_isolated_behind_ports() {
    let runner_path = vcs_crate_root().join("src/runner.rs");
    let content = fs::read_to_string(&runner_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", runner_path, e));

    assert!(
        content.contains("pub trait VcsRunner: Send + Sync"),
        "Port Isolation Violation: cynic-vcs must declare a VcsRunner trait for dependency inversion"
    );
}
