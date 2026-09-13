use std::fs;
use std::path::{Path, PathBuf};

fn channel_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn channel_domain_has_no_orchestrator_dependencies() {
    let cargo_toml_path = channel_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-host", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-channel violates downward DAG: depends on orchestrator {}",
            forbidden
        );
    }
}

#[test]
fn channel_domain_is_zero_io_pure() {
    // References:
    // - RFC-002: CYNIC 5-Layer Decentralized Architecture (Layer 2)
    // - Bitcoin Lightning Network & HTLC (Poon & Dryja 2016)
    // - Multi-Party Virtual State Channels (Dziembowski et al. EUROCRYPT 2019)
    let cargo_toml_path = channel_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    assert!(
        !content.contains("tokio"),
        "cynic-channel core domain must not depend on tokio async runtime directly"
    );
    assert!(
        !content.contains("reqwest"),
        "cynic-channel core domain must not depend on reqwest HTTP client directly"
    );
}
