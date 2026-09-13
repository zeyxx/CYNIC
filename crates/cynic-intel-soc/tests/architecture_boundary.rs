use std::fs;
use std::path::{Path, PathBuf};

fn soc_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn soc_domain_has_no_orchestrator_dependencies() {
    let manifest_path = soc_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("read Cargo.toml");
    assert!(
        !content.contains("cynic-node"),
        "cynic-intel-soc must not depend on cynic-node"
    );
    assert!(
        !content.contains("cynic-cli"),
        "cynic-intel-soc must not depend on cynic-cli"
    );
    assert!(
        !content.contains("cynic-host"),
        "cynic-intel-soc must not depend on cynic-host"
    );
}
