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
