use std::fs;
use std::path::PathBuf;

fn pool_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn pool_never_depends_on_high_layer_crates() {
    let cargo_toml_path = pool_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", cargo_toml_path, e));

    let forbidden = [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-mcp",
    ];
    for dep in &forbidden {
        assert!(
            !content.contains(dep),
            "Architecture Violation: cynic-pool must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn pool_domain_is_pure_zero_io() {
    let forbidden_io = ["std::process::Command", "reqwest"];
    for file in &["difficulty.rs", "merkle.rs", "reward.rs", "types.rs"] {
        let path = pool_crate_root().join("src").join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));
        for io in &forbidden_io {
            assert!(
                !content.contains(io),
                "Pure Domain Violation: cynic-pool/src/{} contains `{}`!",
                file,
                io
            );
        }
    }
}
