use std::fs;
use std::path::PathBuf;

fn discovery_host_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn discovery_host_never_depends_on_high_layer_crates() {
    let cargo_toml_path = discovery_host_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", cargo_toml_path, e));

    let forbidden = [
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-mcp",
    ];
    for dep in &forbidden {
        assert!(
            !content.contains(dep),
            "Architecture Violation: cynic-discovery-host must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}
