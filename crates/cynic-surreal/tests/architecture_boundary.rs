use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn surreal_boundary_no_orchestrators() {
    let cargo_toml =
        fs::read_to_string(crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for name in &forbidden {
        assert!(
            !cargo_toml.contains(name),
            "Architecture violation: cynic-surreal must not depend on orchestrators ({name})"
        );
    }
}
