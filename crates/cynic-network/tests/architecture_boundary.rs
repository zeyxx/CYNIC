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
    assert!(
        !content.contains("reqwest"),
        "cynic-network core domain must not depend on reqwest HTTP client directly (Hexagonal SoC)"
    );
}

#[test]
fn network_extracted_domain_is_pure_zero_io() {
    let domain_files = [
        "src/types.rs",
        "src/transport_readback.rs",
    ];
    for rel_path in &domain_files {
        let path = network_crate_root().join(rel_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

        assert!(
            !content.contains("std::process::Command"),
            "Pure Domain Violation: {} contains direct std::process::Command!",
            rel_path
        );
        assert!(
            !content.contains("reqwest"),
            "Pure Domain Violation: {} contains network HTTP dependencies!",
            rel_path
        );
        assert!(
            !content.contains("tokio::spawn"),
            "Pure Domain Violation: {} spawns async runtimes inside pure domain!",
            rel_path
        );
    }
}
