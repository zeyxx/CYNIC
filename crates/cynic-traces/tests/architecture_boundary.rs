//! Architecture boundary tests for cynic-traces.
//!
//! Tranche 1 owns zero I/O: pure domain entities plus the session aggregate.
//! Adapters (filesystem, SQLite, processes) land in tranche 2 behind ports.

use std::path::Path;

#[test]
fn no_dependency_on_higher_layer_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-inference",
        "cynic-intel",
        "cynic-knowledge",
        // rusqlite lives here legitimately (adapters own I/O, checked above);
        // tokio stays out everywhere.
        "tokio",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "cynic-traces (tranche 1) must not depend on {forbidden}"
        );
    }
}

#[test]
fn no_io_in_pure_layers() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "src/domain.rs",
        "src/session.rs",
        "src/conservation.rs",
        "src/ports.rs",
        "src/kernel/contract.rs",
        "src/kernel/errors.rs",
        "src/kernel/ids.rs",
        "src/kernel/mod.rs",
        "src/lib.rs",
    ] {
        let content = std::fs::read_to_string(Path::new(manifest_dir).join(file)).unwrap();
        for needle in [
            "std::fs",
            "std::process",
            "std::net",
            "tokio::spawn",
            "tokio::fs",
            "rusqlite",
        ] {
            assert!(!content.contains(needle), "{file} must not contain {needle}");
        }
    }
}

#[test]
fn adapters_own_all_io() {
    // Tranche 3 owns I/O, and ONLY inside adapters/: the pure layers above
    // must stay free of it (checked by no_io_in_pure_layers). Adapters may
    // use rusqlite and std::fs — nothing else performs I/O.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "src/adapters/mod.rs",
        "src/adapters/hermes/mod.rs",
        "src/adapters/hermes/contract.rs",
        "src/adapters/hermes/human.rs",
        "src/adapters/hermes/normalizer.rs",
        "src/adapters/content_store.rs",
        "src/adapters/store_schema.rs",
        "src/adapters/store_sqlite.rs",
        "src/usecases.rs",
    ] {
        let content = std::fs::read_to_string(Path::new(manifest_dir).join(file)).unwrap();
        for needle in ["std::process", "std::net", "tokio::spawn", "tokio::fs"] {
            assert!(!content.contains(needle), "{file} must not contain {needle}");
        }
    }
}
