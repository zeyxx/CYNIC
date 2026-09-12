use std::fs;
use std::path::{Path, PathBuf};

fn mcp_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn mcp_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(mcp_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-node"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-mcp must never depend on host or node orchestrators ({forbidden})"
        );
    }
}
