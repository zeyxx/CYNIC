use std::fs;
use std::path::{Path, PathBuf};

fn forensics_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn forensics_architecture_dag_downward() {
    let cargo_toml =
        fs::read_to_string(forensics_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-forensics must never depend on orchestrator or host crates ({forbidden})"
        );
    }
}

#[test]
fn forensics_probe_is_pure_zero_io() {
    let probe_rs =
        fs::read_to_string(forensics_crate_root().join("src/probe.rs")).expect("read probe.rs");

    let forbidden_io_patterns = [
        "std::process::Command",
        "reqwest",
        "tokio::spawn",
        "std::net::TcpStream",
    ];
    for pattern in &forbidden_io_patterns {
        assert!(
            !probe_rs.contains(pattern),
            "Architecture violation: cynic-forensics::probe must be 100% pure Zero-I/O (found {pattern})"
        );
    }
}

#[test]
fn forensics_types_is_pure_zero_io() {
    let types_rs =
        fs::read_to_string(forensics_crate_root().join("src/types.rs")).expect("read types.rs");

    let forbidden_io_patterns = [
        "std::process::Command",
        "reqwest",
        "tokio::spawn",
        "std::net::TcpStream",
    ];
    for pattern in &forbidden_io_patterns {
        assert!(
            !types_rs.contains(pattern),
            "Architecture violation: cynic-forensics::types must be 100% pure Zero-I/O (found {pattern})"
        );
    }
}
