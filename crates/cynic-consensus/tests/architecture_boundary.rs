use std::fs;
use std::path::{Path, PathBuf};

fn consensus_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn consensus_domain_has_no_orchestrator_dependencies() {
    let manifest_path = consensus_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("read Cargo.toml");
    assert!(
        !content.contains("cynic-node"),
        "cynic-consensus must not depend on cynic-node"
    );
    assert!(
        !content.contains("cynic-cli"),
        "cynic-consensus must not depend on cynic-cli"
    );
    assert!(
        !content.contains("cynic-host"),
        "cynic-consensus must not depend on cynic-host"
    );
}

#[test]
fn peer_domain_is_pure_zero_io() {
    let domain_files = [
        "src/peer/accounts.rs",
        "src/peer/engine.rs",
        "src/peer/genesis.rs",
        "src/peer/mod.rs",
        "src/peer/state.rs",
        "src/peer/transactions.rs",
        "src/peer/wal.rs",
    ];
    for rel_path in &domain_files {
        let path = consensus_crate_root().join(rel_path);
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
