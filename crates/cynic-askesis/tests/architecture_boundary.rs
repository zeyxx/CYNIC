use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|err| panic!("failed to read {relative_path}: {err}"))
}

#[test]
fn askesis_binary_stays_a_thin_cli_adapter() {
    let main_rs = read_repo_file("crates/cynic-askesis/src/main.rs");
    let line_count = main_rs.lines().count();

    assert!(
        line_count <= 220,
        "askesis main.rs should stay bounded, got {line_count} lines"
    );

    for expected in [
        "use clap::Parser;",
        "use cynic_askesis::audit::kernel_rest::KernelRestAudit;",
        "use cynic_askesis::log::jsonl::JsonlLog;",
        "#[tokio::main]",
        "enum Command {",
        "Command::parse()",
    ] {
        assert!(
            main_rs.contains(expected),
            "askesis main.rs should contain `{expected}`"
        );
    }

    for forbidden in [
        "reqwest::Client",
        "async_trait::async_trait",
        "trait AuditEngine",
    ] {
        assert!(
            !main_rs.contains(forbidden),
            "askesis main.rs must not own lower-layer detail `{forbidden}`"
        );
    }
}

#[test]
fn askesis_lib_exports_ports_and_domain_types_not_cli_details() {
    let lib_rs = read_repo_file("crates/cynic-askesis/src/lib.rs");

    for expected in [
        "pub mod anchor;",
        "pub mod audit;",
        "pub mod domains;",
        "pub mod error;",
        "pub mod log;",
        "pub mod reflection;",
        "pub use error::{AskesisError, Result};",
        "pub use reflection::{Reflection, Verdict};",
    ] {
        assert!(
            lib_rs.contains(expected),
            "askesis lib.rs should contain `{expected}`"
        );
    }

    for forbidden in ["clap::Parser", "#[tokio::main]", "std::fs::read_to_string"] {
        assert!(
            !lib_rs.contains(forbidden),
            "askesis lib.rs must not own CLI-only detail `{forbidden}`"
        );
    }
}

#[test]
fn askesis_keeps_ports_and_adapters_separate() {
    let audit_mod = read_repo_file("crates/cynic-askesis/src/audit/mod.rs");
    let kernel_rest = read_repo_file("crates/cynic-askesis/src/audit/kernel_rest.rs");
    let log_mod = read_repo_file("crates/cynic-askesis/src/log/mod.rs");
    let reflection_rs = read_repo_file("crates/cynic-askesis/src/reflection.rs");
    let gcal_rs = read_repo_file("crates/cynic-askesis/src/anchor/gcal.rs");

    for expected in [
        "pub trait AuditEngine: Send + Sync",
        "pub mod kernel_rest;",
        "default_phase2_directives",
        "Reflection::degraded",
    ] {
        assert!(
            audit_mod.contains(expected),
            "audit/mod.rs should contain `{expected}`"
        );
    }

    for expected in [
        "use reqwest::Client;",
        "pub struct KernelRestAudit",
        "impl AuditEngine for KernelRestAudit",
        "env::var(\"CYNIC_REST_ADDR\")",
    ] {
        assert!(
            kernel_rest.contains(expected),
            "audit/kernel_rest.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub trait LogStore",
        "pub struct ObservationEvent",
        "pub mod jsonl;",
    ] {
        assert!(
            log_mod.contains(expected),
            "log/mod.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub enum Verdict",
        "pub struct Reflection",
        "pub fn degraded",
        "pub fn to_markdown",
    ] {
        assert!(
            reflection_rs.contains(expected),
            "reflection.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub struct GoogleCalendarAnchor",
        "reqwest::Client",
        "yup_oauth2::InstalledFlowAuthenticator",
        "impl AnchorProvider for GoogleCalendarAnchor",
    ] {
        assert!(
            gcal_rs.contains(expected),
            "anchor/gcal.rs should contain `{expected}`"
        );
    }
}
