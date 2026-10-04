//! Architecture boundary guards for `cynic-core`.
//!
//! `cynic-core` is the pure domain crate: entities, value objects, and ports.
//! It is the leaf of the domain DAG. These tests are executable statements of
//! that contract — the same spirit as Cockburn's Hexagonal Architecture and
//! Evans' Zero-I/O Domain Core.

use std::fs;
use std::path::{Path, PathBuf};

fn core_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn manifest() -> String {
    let path = core_crate_root().join("Cargo.toml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {:?}: {}", path, e))
}

/// The domain leaf never depends on an orchestrator or a host-substrate crate.
/// An upward edge would invert the DAG and let infrastructure leak into the
/// domain.
#[test]
fn core_domain_has_no_orchestrator_or_host_dependencies() {
    let content = manifest();
    let forbidden = [
        // orchestrators
        "cynic-node",
        "cynic-kernel",
        "cynic-cli",
        "cynic-host",
        "cynic-mcp",
        // host substrate (L1) — the domain must not reach into infrastructure
        "cynic-pkg",
        "cynic-lifecycle",
        "cynic-lxc",
        "cynic-sandbox",
    ];
    for name in forbidden {
        assert!(
            !content.contains(&format!("\"{}\"", name)),
            "cynic-core violates the downward DAG: declares dependency on {}",
            name
        );
    }
}

/// `cynic-core` is Zero-I/O. No async runtime, no HTTP client, no process
/// spawner may be declared as a direct dependency. Side effects live in the
/// adapters that implement the ports declared here.
#[test]
fn core_domain_has_zero_io_dependencies() {
    let content = manifest();
    for forbidden in ["tokio", "reqwest", "hyper", "ureq"] {
        assert!(
            !content.contains(forbidden),
            "cynic-core must stay Zero-I/O: declares dependency on {}",
            forbidden
        );
    }
}

/// Every source file under `src/` must be free of direct side effects. The
/// domain declares *what* must happen through ports; it never does it.
#[test]
fn core_domain_sources_contain_no_direct_io() {
    let src = core_crate_root().join("src");
    let forbidden = [
        "std::process::Command",
        "std::net::TcpStream",
        "std::fs::write",
        "std::fs::File",
    ];
    let mut violations = Vec::new();
    collect_rust_files(&src, &mut violations);
    assert!(
        !violations.is_empty(),
        "no Rust sources found under {:?} — the guard would pass vacuously",
        src
    );
    for path in violations {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let rel = path.strip_prefix(core_crate_root()).unwrap_or(&path);
        for needle in forbidden {
            assert!(
                !content.contains(needle),
                "Zero-I/O Domain Violation: {} contains {}",
                rel.display(),
                needle
            );
        }
    }
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// The mesh types are canonical here. `cynic-network` re-exports them rather
/// than declaring its own — one responsibility, one owning crate.
#[test]
fn mesh_types_are_declared_in_core_not_reexported_from_an_adapter() {
    let mesh = core_crate_root().join("src/fleet_mesh.rs");
    let content = fs::read_to_string(&mesh).unwrap_or_else(|e| panic!("read {:?}: {}", mesh, e));
    for ty in [
        "pub struct MeshNode",
        "pub struct MeshRoute",
        "pub enum MeshError",
        "pub trait FleetMeshPort",
    ] {
        assert!(
            content.contains(ty),
            "cynic-core must own the canonical mesh type `{}`",
            ty
        );
    }
    assert!(
        !content.contains("pub use cynic_network"),
        "cynic-core must not re-export mesh types from the cynic-network adapter"
    );
}
