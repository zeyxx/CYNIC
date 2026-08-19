use std::fs;
use std::path::{Path, PathBuf};

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

fn rust_sources_under(relative_dir: &str) -> Vec<(String, String)> {
    let root = repo_root().join(relative_dir);
    let mut files = Vec::new();
    collect_rust_sources(&root, &root, &mut files);
    files
}

fn collect_rust_sources(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    let mut entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", dir.display()))
        .map(|entry| entry.expect("directory entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(root, &path, files);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("relative path")
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        files.push((relative, content));
    }
}

#[test]
fn protocol_lib_stays_a_contract_focused_entrypoint() {
    let lib_rs = read_repo_file("crates/cynic-protocol/src/lib.rs");
    let line_count = lib_rs.lines().count();

    assert!(
        line_count <= 220,
        "protocol lib.rs should stay compact, got {line_count} lines"
    );

    for expected in [
        "pub mod coord;",
        "pub mod development;",
        "pub mod endpoint;",
        "pub mod framework;",
        "pub mod network;",
        "pub mod observe;",
        "pub mod outcomes;",
        "pub mod runtime;",
        "pub mod supervision;",
        "pub mod tasks;",
    ] {
        assert!(
            lib_rs.contains(expected),
            "protocol lib.rs should export `{expected}`"
        );
    }

    for forbidden in [
        "tokio::main",
        "clap::Parser",
        "reqwest::",
        "std::fs::",
        "std::process::Command",
        "tokio::process::Command",
    ] {
        assert!(
            !lib_rs.contains(forbidden),
            "protocol lib.rs must not own adapter concern `{forbidden}`"
        );
    }
}

#[test]
fn protocol_sources_do_not_pull_in_runtime_adapters() {
    let sources = rust_sources_under("crates/cynic-protocol/src");

    for (relative_path, content) in &sources {
        for forbidden in [
            "reqwest::",
            "tokio::process::Command",
            "std::process::Command",
            "clap::Parser",
            "#[tokio::main]",
        ] {
            assert!(
                !content.contains(forbidden),
                "protocol source {} must stay free of adapter concern `{forbidden}`",
                relative_path
            );
        }
    }
}

#[test]
fn protocol_keeps_runtime_and_development_contracts_explicit() {
    let runtime_rs = read_repo_file("crates/cynic-protocol/src/runtime.rs");
    let development_rs = read_repo_file("crates/cynic-protocol/src/development.rs");
    let outcomes_rs = read_repo_file("crates/cynic-protocol/src/outcomes.rs");

    for expected in [
        "pub mod layers;",
        "pub mod planner;",
        "pub mod proofs;",
        "pub mod types;",
        "build_runtime_execution_plan_from_gates",
        "runtime_initial_observed_gates",
        "runtime_missing_gates",
    ] {
        assert!(
            runtime_rs.contains(expected),
            "runtime.rs should contain `{expected}`"
        );
    }

    for expected in [
        "DEVELOPMENT_SURFACE_MANIFEST_SCHEMA",
        "owner: &'static str",
        "\"cynic-protocol\"",
        "\"Rust protocol types and pure functions only\"",
        "\"read host filesystem\"",
        "\"execute OS probes\"",
    ] {
        assert!(
            development_rs.contains(expected),
            "development.rs should contain `{expected}`"
        );
    }

    for expected in [
        "OUTCOME_SPEC_SCHEMA",
        "KPI_SCORE_SCHEMA",
        "\"hexagonal_crate_boundaries\"",
        "\"protocol_owns_contracts\"",
        "\"runtime planner and development/outcome specs live in cynic-protocol\"",
    ] {
        assert!(
            outcomes_rs.contains(expected),
            "outcomes.rs should contain `{expected}`"
        );
    }
}

/// Every Rust source in the crate must be reachable from `lib.rs`.
///
/// An undeclared `.rs` file is never compiled: no build error, no test run, no
/// warning. It is invisible rather than broken, so `cargo check` cannot detect
/// it. Bulk integration commits have repeatedly dropped files into this crate
/// without adding the matching `pub mod`, leaving contract code that never
/// executed once.
///
/// This test owns that contract at the layer that can express it: the crate
/// root. It reports every orphan at once rather than the first, so the operator
/// does not repair one and hit the next blind.
fn declared_modules(lib_rs: &str) -> Vec<String> {
    lib_rs
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let rest = line
                .strip_prefix("pub mod ")
                .or_else(|| line.strip_prefix("pub(crate) mod "))
                .or_else(|| line.strip_prefix("mod "))?;
            // `pub mod foo;` declares foo; `pub mod foo {` is inline, not a file.
            rest.strip_suffix(';').map(|name| name.trim().to_string())
        })
        .collect()
}

/// Orphan detection for this crate now lives in `orphan_sources_ratchet.rs`,
/// which shares one detector with every other crate.
///
/// The earlier version of this test lived here and scanned only the top level of
/// `src/`, matching module NAMES against `lib.rs`. That missed nested orphans
/// (`build/bootstrap_recovery.rs`, `runtime/degradation.rs`, all of
/// `fleet_inference/`) and knew nothing about `include!`, which is how
/// `cynic-mcp` reaches its 2221-line `handler.rs`.
///
/// Keeping both would give one concern two decision sources that disagree, so
/// the weaker one was removed rather than left as a second opinion.
#[test]
fn orphan_detection_is_owned_by_the_shared_ratchet() {
    let ratchet = repo_root()
        .join("crates/cynic-protocol/tests/orphan_sources_ratchet.rs")
        .is_file();
    assert!(
        ratchet,
        "orphan_sources_ratchet.rs is missing: this crate would silently lose \
         orphan detection, which is exactly the failure mode the ratchet exists \
         to prevent"
    );

    let detector = repo_root().join("tests/support/orphan_sources.rs");
    assert!(
        detector.is_file(),
        "the shared detector at tests/support/orphan_sources.rs is missing; \
         every crate ratchet includes it by path"
    );
}

