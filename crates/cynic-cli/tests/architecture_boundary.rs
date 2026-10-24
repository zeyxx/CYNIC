#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use clap::CommandFactory;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cli_main_is_a_thin_facade() {
    let main_rs = fs::read_to_string(crate_root().join("src/main.rs")).expect("read main.rs");
    let line_count = main_rs.lines().count();
    assert!(
        line_count <= 50,
        "Architecture Violation: cynic-cli/src/main.rs must remain a thin facade (<= 50 lines), found {}",
        line_count
    );
}

#[test]
fn cli_has_no_hidden_commands() {
    let cmd = cynic_cli::Cli::command();
    for sub in cmd.get_subcommands() {
        assert!(
            !sub.is_hide_set(),
            "Architecture Violation: Hidden CLI command `{}` detected! All commands must be visible and properly nested under an owning family.",
            sub.get_name()
        );
    }
}

#[test]
fn cli_audit_crate_verification_passes() {
    let report = cynic_cli::crate_audit::audit_core_crates();
    assert_eq!(
        report.blocked_count, 0,
        "Architecture Violation: {} crates failed architecture audit!",
        report.blocked_count
    );
    assert_eq!(
        report.crate_count, observed_workspace_crate_count(),
        "Architecture Violation: audit covered {} crates but the workspace holds {} — silent exclusion or drift!",
        report.crate_count,
        observed_workspace_crate_count()
    );
}

/// Observed workspace inventory: directories under `crates/` holding a
/// `Cargo.toml`. Never a literal — fusions and deletions (44 today, 46
/// yesterday) must not red the gate. The audit's own
/// `workspace_crate_names` observes the same way; this mirrors it from the
/// test side so the two cannot silently diverge.
fn observed_workspace_crate_count() -> usize {
    let crates_root = crate_root().join("..").join("..").join("crates");
    std::fs::read_dir(&crates_root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().join("Cargo.toml").is_file())
                .count()
        })
        .unwrap_or(0)
}
