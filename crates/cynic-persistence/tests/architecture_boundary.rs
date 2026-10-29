use std::fs;
use std::path::{Path, PathBuf};

fn storage_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn storage_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(storage_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-storage must never depend on orchestrator or host crates ({forbidden})"
        );
    }
}

#[test]
fn storage_domain_modules_are_pure_zero_io() {
    let domain_files = [
        "src/client_policy.rs",
        "src/controller.rs",
        "src/credential_sync.rs",
        "src/identity.rs",
        "src/lifecycle.rs",
        "src/topology.rs",
    ];
    for rel_path in &domain_files {
        let path = storage_crate_root().join(rel_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

        assert!(
            !content.contains("std::process::Command"),
            "Pure Domain Violation: {} contains direct std::process::Command!",
            rel_path
        );
        assert!(
            !content.contains("reqwest"),
            "Pure Domain Violation: {} contains network HTTP dependencies!",
            rel_path
        );
        assert!(
            !content.contains("tokio::spawn"),
            "Pure Domain Violation: {} spawns async runtimes inside pure domain!",
            rel_path
        );
    }
}
