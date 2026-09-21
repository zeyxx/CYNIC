use std::fs;
use std::path::{Path, PathBuf};

fn escore_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// E-Score is a pure domain crate. It MUST NOT depend on orchestrator or host crates.
#[test]
fn escore_domain_has_no_orchestrator_dependencies() {
    let cargo_toml =
        fs::read_to_string(escore_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden = [
        "cynic-host", "cynic-kernel", "cynic-node", "cynic-cli", "cynic-intel-soc",
    ];
    for f in &forbidden {
        assert!(
            !cargo_toml.contains(f),
            "Architecture violation: cynic-intel-escore must not depend on {f}"
        );
    }
}

/// E-Score MUST depend on cynic-intel (its domain parent).
#[test]
fn escore_depends_on_cynic_intel() {
    let cargo_toml =
        fs::read_to_string(escore_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    assert!(
        cargo_toml.contains("cynic-intel"),
        "cynic-intel-escore must depend on cynic-intel"
    );
}
