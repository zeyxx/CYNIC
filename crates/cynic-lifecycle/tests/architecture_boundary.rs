use std::fs;
use std::path::{Path, PathBuf};

fn lifecycle_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn lifecycle_domain_is_pure_zero_io() {
    let domain_files = [
        "src/gate.rs",
        "src/plan.rs",
        "src/policy.rs",
        "src/types.rs",
        "src/kernel_release/types.rs",
        "src/kernel_release/proof.rs",
    ];

    for rel_path in &domain_files {
        let path = lifecycle_crate_root().join(rel_path);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let forbidden = ["tokio::spawn", "reqwest::", "std::net::TcpStream"];
        for pattern in &forbidden {
            assert!(
                !content.contains(pattern),
                "Architecture violation in {rel_path}: domain module contains forbidden pattern '{pattern}'"
            );
        }
    }
}

#[test]
fn lifecycle_never_depends_on_high_layer_crates() {
    let cargo_toml =
        fs::read_to_string(lifecycle_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-lifecycle must never depend on orchestrators or CLI ({forbidden})"
        );
    }
}
