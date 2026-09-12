use std::fs;
use std::path::{Path, PathBuf};

fn market_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn market_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(market_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-market must never depend on orchestrator or host crates ({forbidden})"
        );
    }
}
