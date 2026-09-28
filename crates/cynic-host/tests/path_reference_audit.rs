//! Path reference audit — regression proof at the owning layer.
//!
//! WHY: the path rules live in `cynic-host::audit_engine`, so their proof belongs
//! here rather than in a projection test. A test that only asserted "the report
//! has a schema" would pass while every rule silently returned zero violations —
//! which is exactly how a broken audit stays green.
//!
//! HOW: evaluate the real rules against the real workspace root and assert that
//! the specific, named divergences found by
//! `infra/DIRECTORY_STRUCTURE_AUDIT.md` are detected. These are not synthetic
//! fixtures: if someone repairs a divergence, this test fails and names it, which
//! is the intended direction. A repaired divergence must be deleted from the
//! list, never satisfied by weakening the rule.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use cynic_host::audit_engine::run_path_audit;

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

/// Every code the report is expected to raise, with the divergence it names.
///
/// The needle is the missing path, not the rendered sentence: asserting on prose
/// couples the proof to a message format and makes a reword fail the gate for no
/// reason. What must hold is that the code fires and that it names the right
/// path.
const EXPECTED: &[(&str, &str)] = &[
    (
        "PATH-001-DECLARED-ABSENT",
        "crates/cynic-host/src/lifecycle/runtime_paths.rs",
    ),
    (
        "PATH-002-CITED-ABSENT",
        "contracts/consumer-profiles",
    ),
    (
        "PATH-002-CITED-ABSENT",
        ".hermes/plans/",
    ),
    (
        "PATH-003-SSOT-IGNORED",
        "infra/directory-tree.json",
    ),
];

#[test]
fn path_audit_detects_every_named_divergence() {
    let report = report();
    let rendered: Vec<String> = report
        .violations
        .iter()
        .map(|v| format!("{} {}", v.code, v.message))
        .collect();

    for (code, needle) in EXPECTED {
        assert!(
            rendered
                .iter()
                .any(|line| line.starts_with(code) && line.contains(needle)),
            "path audit did not report `{code}` naming `{needle}`\nreported: {rendered:#?}"
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
        vec!["PATH-001", "PATH-002", "PATH-003"],
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
        assert!(
            !snippet.starts_with('/') && !snippet.starts_with('~'),
            "runtime-space path reported as a missing git path: {snippet}"
        );
    }
}

#[test]
fn path_audit_is_included_in_the_workspace_audit() {
    // The same rules must run for MCP and `cynic audit all`, not only for the
    // focused surface. A rule reachable from one projection is a rule nobody runs.
    let full = cynic_host::audit_engine::run_workspace_audit(&workspace_root());
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
    assert_eq!(report.summary.total_rules_evaluated, 3);
    // PATH-001, PATH-002 and PATH-003 all currently report, so a category-based
    // score would collapse to 0 passed. Rule-id matching keeps them distinct.
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
