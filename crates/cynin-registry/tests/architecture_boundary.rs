use std::fs;
use std::path::PathBuf;

fn registry_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn registry_depends_on_protocol() {
    let cargo_toml =
        fs::read_to_string(registry_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    assert!(
        cargo_toml.contains("cynic-protocol"),
        "cynic-registry must depend on cynic-protocol for shared types"
    );
}

#[test]
fn registry_has_no_host_or_node_deps() {
    let cargo_toml =
        fs::read_to_string(registry_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden = ["cynic-host", "cynic-node", "cynic-cli"];
    for f in &forbidden {
        assert!(
            !cargo_toml.contains(f),
            "Architecture violation: cynic-registry must not depend on {f}"
        );
    }
}
