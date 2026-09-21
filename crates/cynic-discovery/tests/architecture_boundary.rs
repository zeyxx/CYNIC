use std::fs;
use std::path::PathBuf;

fn discovery_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn discovery_never_depends_on_high_layer_crates() {
    let cargo_toml_path = discovery_crate_root().join("Cargo.toml");
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
            "Architecture Violation: cynic-discovery must NEVER depend on higher layer crate `{}`",
            dep
        );
    }
}

#[test]
fn discovery_is_pure_zero_io() {
    let src_dir = discovery_crate_root().join("src");
    for entry in fs::read_dir(src_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

            assert!(
                !content.contains("std::process::Command"),
                "Pure Domain Violation: {:?} contains `std::process::Command`!",
                path
            );
            assert!(
                !content.contains("reqwest"),
                "Pure Domain Violation: {:?} contains network HTTP dependencies!",
                path
            );
            assert!(
                !content.contains("tokio::spawn"),
                "Pure Domain Violation: {:?} spawns async runtimes!",
                path
            );
            assert!(
                !content.contains("std::fs::write"),
                "Pure Domain Violation: {:?} mutates filesystem!",
                path
            );
        }
    }
}
