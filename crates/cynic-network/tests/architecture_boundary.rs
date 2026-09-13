use std::fs;
use std::path::{Path, PathBuf};

fn network_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn network_domain_has_no_orchestrator_dependencies() {
    let cargo_toml_path = network_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-host", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-network violates downward DAG: depends on orchestrator {}",
            forbidden
        );
    }
}

#[test]
fn network_domain_has_zero_direct_io_network_calls() {
    // References & Canonical Hexagonal Sources:
    // 1. Alistair Cockburn: Hexagonal Architecture (Ports and Adapters)
    //    https://alistair.cockburn.us/hexagonal-architecture/
    // 2. Domain-Driven Design (Eric Evans): Zero-I/O Domain Core Pattern
    //    The core domain must never spawn raw network I/O or HTTP clients directly;
    //    I/O must remain behind explicit ports.
    let cargo_toml_path = network_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    assert!(
        !content.contains("tokio"),
        "cynic-network core domain must not depend on tokio async runtime directly"
    );
}
