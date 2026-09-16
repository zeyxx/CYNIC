use std::fs;
use std::path::PathBuf;

fn workload_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn workload_never_depends_on_high_layer_crates() {
    let cargo_toml_path = workload_crate_root().join("Cargo.toml");
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
            "Architecture Violation: cynic-workload must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn workload_domain_modules_are_pure_zero_io() {
    let domain_files = [
        "src/admission.rs",
        "src/agent_scheduler.rs",
        "src/delivery.rs",
        "src/durability.rs",
        "src/lifecycle.rs",
        "src/settlement.rs",
        "src/workspace_claim.rs",
        "src/workspace_lifecycle.rs",
    ];
    for rel_path in &domain_files {
        let path = workload_crate_root().join(rel_path);
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
