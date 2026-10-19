//! Architecture boundary tests for cynic-tui.
//!
//! The kit owns the terminal but never domain files: no filesystem reads, no
//! processes, no network, no higher-layer cynic crates. Domains inject data
//! through the refresh closure; the kit only renders it.

use std::path::Path;

#[test]
fn no_dependency_on_higher_layer_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-fleet",
        "cynic-inference",
        "cynic-intel",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "cynic-tui must not depend on {forbidden}"
        );
    }
}

#[test]
fn kit_reads_no_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "src/types.rs",
        "src/app.rs",
        "src/conversation.rs",
        "src/lib.rs",
    ] {
        let content = std::fs::read_to_string(Path::new(manifest_dir).join(file)).unwrap();
        // `std::process::ExitCode` (return-code type) is allowed; only
        // process *spawning* is forbidden.
        for needle in ["std::fs", "std::process::Command", "reqwest"] {
            assert!(
                !content.contains(needle),
                "{file} must not contain {needle}"
            );
        }
    }
}
