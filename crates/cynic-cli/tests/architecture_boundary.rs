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
        report.crate_count, 47,
        "Expected 47 crates in architecture audit, found {}",
        report.crate_count
    );
}
