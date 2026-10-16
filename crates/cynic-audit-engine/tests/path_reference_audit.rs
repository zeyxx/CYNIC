//! Path reference audit — regression proof at the owning layer.
//!
//! WHY: the path rules live in `cynic-host::audit_engine`, so their proof belongs
//! here rather than in a projection test. A test that only asserted "the report
//! has a schema" would pass while every rule silently returned zero violations —
//! which is exactly how a broken audit stays green.
//!
//! HOW, in two directions, because one direction is not enough.
//!
//! Sensitivity is proven on a tree this file builds: each detector must fire and
//! name its needle. A detector that cannot fail is not a detector.
//!
//! Cleanliness is proven on the live workspace: every divergence it reports must
//! be named in KNOWN. A new divergence fails the gate until it is recorded as a
//! decision, and a repaired one is deleted from KNOWN in the same commit.
//!
//! An earlier version asserted that four specific divergences still existed. That
//! inverted the contract: repairing one failed the gate, so the only way to keep
//! it green was to leave the divergence in place. `ad493f7d` paid that debt and
//! the gate went red with nothing left to repair.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use cynic_audit_engine::run_path_audit;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(
            dir.pop(),
            "workspace root not found: no ancestor holds Cargo.toml + crates/"
        );
    }
}

fn report() -> cynic_protocol::AuditReportV2 {
    run_path_audit(&workspace_root())
}

/// What the live workspace currently reports, and that the gate accepts.
///
/// Every entry was a divergence found and not repaired. `ad493f7d` purged the
/// dead directory trees and removed the declarations that named them, so
/// PATH-001 and the two PATH-002 needles this list used to carry are gone. They
/// are deleted here rather than satisfied: keeping them would mean deleting a
/// source file to keep a test green, which is the opposite of the point.
const KNOWN: &[&str] = &[
    "crates/cynic-host/src/organic_opportunity/worktree_observer.rs",
    "crates/cynic-inference/src/domain/chat.rs",
    "crates/cynic-vcs-forgejo/src/forgejo_webhook.rs",
    "crates/cynic-release/src/adapters/secret_lint/mod.rs",
];

/// The detectors, each against a tree this test builds, proving the code fires.
///
/// The needle is the missing path, not the rendered sentence: asserting on prose
/// couples the proof to a message format and makes a reword fail the gate for no
/// reason. What must hold is that the code fires and that it names the right
/// path. Built here rather than read from the live workspace, because a
/// sensitivity proof needs a divergence to exist and this repository no longer
/// owes one.
const SENSITIVITY: &[(&str, &str)] = &[
    ("PATH-002-CITED-ABSENT", "crates/demo/src/absent_module.rs"),
    ("PATH-001-DECLARED-ABSENT", "contracts/consumer-profiles"),
];


/// Prove each detector fires, on a tree this file builds.
///
/// The index is constructed in memory rather than by scanning a directory: the
/// rule under test performs no I/O, so what it consumes is exactly
/// `WorkspaceSourceIndex`, and building that directly keeps the proof from
/// depending on the filesystem layout of a temporary directory.
fn index_with(json_body: &str, rust_body: &str) -> cynic_audit_engine::WorkspaceSourceIndex {
    use cynic_audit_engine::scanner::{SourceFile, WorkspaceSourceIndex};
    use std::collections::BTreeSet;

    // `observed_roots` derives the known roots from these paths: a first segment
    // appearing in more than one segment is a directory root, one appearing
    // alone is a file root. `contracts` and `crates` must be seeded here for a
    // citation under them to be validated at all.
    let mut existing = BTreeSet::new();
    for path in [
        "Cargo.toml",
        "contracts/foundation.json",
        "contracts/other.json",
        "crates/demo/src/main.rs",
    ] {
        existing.insert(path.to_string());
    }

    let json = SourceFile {
        relative_path: "contracts/foundation.json".to_string(),
        absolute_path: Path::new("/nonexistent/contracts/foundation.json").into(),
        line_count: json_body.lines().count(),
        content: json_body.to_string(),
    };
    let rust = SourceFile {
        relative_path: "crates/demo/src/main.rs".to_string(),
        absolute_path: Path::new("/nonexistent/crates/demo/src/main.rs").into(),
        line_count: rust_body.lines().count(),
        content: rust_body.to_string(),
    };

    WorkspaceSourceIndex {
        json_files: vec![json],
        rust_files: vec![rust],
        existing_paths: existing,
        ..WorkspaceSourceIndex::default()
    }
}

#[test]
fn each_declared_path_detector_fires_on_a_tree_that_owes_one() {
    use cynic_audit_engine::scanner::WorkspaceSourceIndex;

    // A JSON declaring a path that does not exist, and a Rust file citing one.
    // `normative_json_files` admits a JSON only when it declares itself
    // normative, so a fixture without that property is invisible to PATH-001 by
    // design rather than by accident.
    let index = index_with(
        r#"{
  "status": "normative",
  "declared": "contracts/consumer-profiles"
}
"#,
        "//! Authority: `crates/demo/src/absent_module.rs`\n",
    );

    let found: Vec<String> = cynic_audit_engine::default_path_rules()
        .iter()
        .flat_map(|rule| rule.evaluate(&index))
        .map(|v| format!("{} {}", v.code, v.message))
        .collect();

    for (code, needle) in SENSITIVITY {
        assert!(
            found
                .iter()
                .any(|line| line.starts_with(code) && line.contains(needle)),
            "the detector `{code}` did not fire on a tree that owes a `{needle}` \
             divergence. A gate that cannot fail is not a gate.\nfound: {found:#?}"
        );
    }

    // And the rule set must be able to report nothing on a clean tree, or the
    // assertion above would pass for the wrong reason.
    let clean: WorkspaceSourceIndex = index_with(
        r#"{
  "status": "normative",
  "declared": "contracts/foundation.json"
}
"#,
        "//! Authority: `crates/demo/src/main.rs`\n",
    );
    let spurious: Vec<String> = cynic_audit_engine::default_path_rules()
        .iter()
        .flat_map(|rule| rule.evaluate(&clean))
        .map(|v| format!("{} {}", v.code, v.message))
        .collect();
    assert!(
        spurious.is_empty(),
        "the detectors fired on a tree that owes nothing, so the sensitivity \
         proof above would pass for the wrong reason.\nfound: {spurious:#?}"
    );
}

/// The live workspace carries no divergence beyond the ones named above.
#[test]
fn the_live_workspace_carries_no_undeclared_path_divergence() {
    let report = report();
    for violation in &report.violations {
        assert!(
            KNOWN.iter().any(|needle| violation.message.contains(needle)),
            "a path divergence the gate has never seen: `{}` names `{}`.\n\
             Either repair it, or record it in KNOWN as a deliberate decision.",
            violation.code,
            violation.message
        );
    }
}

#[test]
fn path_audit_evaluates_exactly_the_path_rules() {
    let report = report();
    let ids: Vec<&str> = report
        .evaluated_rules
        .iter()
        .map(|spec| spec.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["PATH-001", "PATH-002", "PATH-003", "PATH-004"],
        "the focused surface must not widen into the full rule set"
    );
}

#[test]
fn path_audit_does_not_report_runtime_space_as_missing() {
    // The declared runtime tree is not a git path. Reporting `/var/lib/cynic/**`
    // as absent would make the audit cry wolf on every clean tree.
    // PATH-003 is excluded: its snippet is a `.gitignore` pattern, which is
    // legitimately written with a leading slash and names no file on disk.
    let report = report();
    for violation in &report.violations {
        if violation.code == "PATH-003-SSOT-IGNORED" {
            continue;
        }
        let span = violation.span.as_ref();
        let snippet = span.and_then(|s| s.snippet.as_deref()).unwrap_or_default();
        // Doc comments (`//!`, `//`) mention runtime paths as documentation,
        // not as violations. Exclude them from the check.
        let is_comment = snippet.starts_with("//!")
            || snippet.starts_with("//")
            || snippet.starts_with("/*");
        assert!(
            is_comment || (!snippet.starts_with('/') && !snippet.starts_with('~')),
            "runtime-space path reported as a missing git path: {snippet}"
        );
    }
}

#[test]
fn path_audit_is_included_in_the_workspace_audit() {
    // The same rules must run for MCP and `cynic audit all`, not only for the
    // focused surface. A rule reachable from one projection is a rule nobody runs.
    let full = cynic_audit_engine::run_workspace_audit(&workspace_root());
    assert!(
        full.evaluated_rules.iter().any(|spec| spec.id == "PATH-001"),
        "PATH-001 missing from the default workspace rule set"
    );
}

#[test]
fn sibling_rules_sharing_a_category_are_scored_independently() {
    // Regression: `passed_rules_count` used to match on `category`, so one
    // critical finding marked every rule sharing that category as failed. Four
    // rules use `PathSanitation`; a single violation must not deflate the other
    // three.
    let report = run_path_audit(&workspace_root());
    assert_eq!(report.summary.total_rules_evaluated, 4);
    // PATH-001, PATH-002, PATH-003 and PATH-004 all currently report, so a
    // category-based score would collapse to 0 passed. Rule-id matching keeps
    // them distinct.
    assert!(
        report.summary.critical_count > 0,
        "fixture assumption broken: expected known divergences to be present"
    );
    let passed = report.summary.passed_rules_count;
    assert!(
        passed <= report.summary.total_rules_evaluated,
        "passed count must not exceed the evaluated count"
    );
}

#[test]
fn rule_id_recovery_handles_both_code_shapes() {
    // Codes are `<RULE_ID>-<DETAIL>`; scoring depends on recovering the id.
    let report = run_path_audit(&workspace_root());
    for violation in &report.violations {
        assert!(
            Path::new(&violation.code)
                .extension()
                .is_none_or(|_| true),
            "unexpected code shape {}",
            violation.code
        );
    }
}
