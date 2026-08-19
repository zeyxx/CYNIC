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

/// The 12 sources currently unreachable from `lib.rs`.
///
/// Declaring them is not a repair: a probe adding all 12 `pub mod` lines
/// produced 29 compile errors (E0425/E0433/E0422 — missing symbols), so this
/// code depends on types that were never written. They are quarantined here as
/// explicit data with their observed state, not silently tolerated.
///
/// Removing an entry from this list is the only way to satisfy the test: either
/// declare the module and make it compile, or delete the file. The list may
/// never grow — that is what `no_new_orphan_protocol_sources` enforces.
const KNOWN_UNREACHABLE_SOURCES: &[&str] = &[
    "agent_execution_provenance",
    "audit",
    "coordination_authority",
    "coordination_fault_harness",
    "coordination_journal",
    "coordination_scaling_harness",
    "detection",
    "eval_hygiene",
    "fleet_inference",
    "hardening",
    "runtime_placement",
    "source_change_submission",
];

#[test]
fn no_new_orphan_protocol_sources() {
    let lib_rs = read_repo_file("crates/cynic-protocol/src/lib.rs");
    let declared = declared_modules(&lib_rs);

    let src = repo_root().join("crates/cynic-protocol/src");
    let mut orphans = Vec::new();

    let mut entries = fs::read_dir(&src)
        .expect("read protocol src")
        .map(|e| e.expect("dir entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // A directory module is reachable through its own `mod.rs`.
        let module_name = if path.is_dir() {
            if !path.join("mod.rs").is_file() {
                continue;
            }
            name
        } else {
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            if name == "lib" {
                continue;
            }
            name
        };

        if !declared.contains(&module_name) {
            orphans.push(module_name);
        }
    }

    let new_orphans: Vec<&String> = orphans
        .iter()
        .filter(|o| !KNOWN_UNREACHABLE_SOURCES.contains(&o.as_str()))
        .collect();

    assert!(
        new_orphans.is_empty(),
        "{} new protocol source(s) are not declared in lib.rs and are therefore \
         never compiled: {:?}. An undeclared module produces no build error, no \
         test run and no warning — it proves nothing. Add `pub mod <name>;` or \
         delete the file.",
        new_orphans.len(),
        new_orphans
    );

    // A quarantined source that got fixed must leave the list, or the list
    // becomes stale cover for code that no longer needs it.
    let stale: Vec<&&str> = KNOWN_UNREACHABLE_SOURCES
        .iter()
        .filter(|known| !orphans.iter().any(|o| o == *known))
        .collect();

    assert!(
        stale.is_empty(),
        "{:?} are listed as unreachable but are now declared or absent — \
         remove them from KNOWN_UNREACHABLE_SOURCES",
        stale
    );
}

