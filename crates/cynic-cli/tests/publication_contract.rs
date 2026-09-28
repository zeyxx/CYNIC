#![allow(clippy::expect_used)]

//! Executable invariants for `contracts/publication.json`.
//!
//! These run against the *private* tree. They prove the contract is complete and
//! self-consistent. They do not prove the public projection is clean — that is the
//! read-back, which runs against the generated tree, because redaction cannot be
//! verified from a tree that legitimately still contains the literals.
//!
//! The rule is the project's own: a contract that nothing executes decays into
//! documentation. So the first thing this file does is exist.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root()
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the workspace root")
        .to_path_buf()
}

fn contract() -> Value {
    let raw = fs::read_to_string(workspace_root().join("contracts/publication.json"))
        .expect("read contracts/publication.json");
    serde_json::from_str(&raw).expect("contracts/publication.json is valid JSON")
}

fn tracked_paths() -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files"])
        .current_dir(workspace_root())
        .output()
        .expect("git ls-files");
    assert!(
        out.status.success(),
        "git ls-files failed; the publication contract cannot be verified without the tracked tree"
    );
    String::from_utf8(out.stdout)
        .expect("git ls-files output is utf-8")
        .lines()
        .map(str::to_string)
        .collect()
}

/// Match a contract glob against a repository-relative path.
///
/// Supports `**` (any number of segments), `*` (within one segment) and a literal
/// segment. Implemented here rather than pulled in as a dependency: a new crate
/// dependency needs its own admission, and the semantics needed are three cases.
fn glob_matches(pattern: &str, path: &str) -> bool {
    fn segments(pattern: &str) -> Vec<&str> {
        pattern.split('/').collect()
    }

    let (p, v): (Vec<&str>, Vec<&str>) = (segments(pattern), segments(path));
    if p.first() == Some(&"**") {
        for skip in 0..=v.len() {
            if glob_matches(&p[1..].join("/"), &v[skip..].join("/")) {
                return true;
            }
        }
        return false;
    }
    if p.is_empty() {
        return v.is_empty();
    }
    if v.is_empty() {
        return false;
    }
    if p[0] == "*" || p[0] == v[0] {
        return glob_matches(&p[1..].join("/"), &v[1..].join("/"));
    }
    // A bare `**` tail also matches zero segments.
    p.len() > 1 && p[1] == "**" && (p[0] == v[0] || p[0] == "*")
}

fn rules() -> Vec<Value> {
    let c = contract();
    c["path_rules"]
        .as_array()
        .expect("path_rules is an array")
        .clone()
}

fn rule_for(path: &str) -> Option<Value> {
    rules()
        .into_iter()
        .find(|r| glob_matches(r["path"].as_str().unwrap_or_default(), path))
}

#[test]
fn publication_contract_is_declared_normative() {
    let c = contract();
    assert_eq!(c["schema"], "cynic.publication.v1");
    assert_eq!(
        c["status"], "normative",
        "a non-normative publication contract is a comment, not a contract"
    );
    let order = c["authority"]["conflict_order"]
        .as_array()
        .expect("authority.conflict_order is an array");
    assert!(
        order.len() >= 2,
        "a conflict order with fewer than two entries cannot resolve anything"
    );
    assert!(
        c["authority"]["conflict_order"][0] == "observed private tree state",
        "observed state must outrank the contract that describes it, or the contract is the assertion"
    );
}

#[test]
fn every_tracked_path_is_declared() {
    let undeclared: Vec<String> = tracked_paths()
        .into_iter()
        .filter(|p| rule_for(p).is_none())
        .collect();
    assert!(
        undeclared.is_empty(),
        "Publication Violation: {} tracked path(s) have no rule in contracts/publication.json. \
         An undeclared path would be copied by omission rather than by decision. First offenders:\n  {}",
        undeclared.len(),
        undeclared
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_exclude_carries_a_reason() {
    for r in rules() {
        if r["status"] == "exclude" {
            let reason = r["reason"].as_str().unwrap_or_default();
            assert!(
                reason.len() > 20,
                "Publication Violation: `{}` is excluded without a substantive reason. \
                 An exclusion without a stated reason is indistinguishable from an oversight.",
                r["path"]
            );
        }
    }
}

#[test]
fn every_temporary_rule_states_its_removal_condition() {
    for r in rules() {
        if r["status"] == "temporary" {
            assert!(
                r.get("until").and_then(Value::as_str).is_some_and(|s| s.len() > 20),
                "Publication Violation: `{}` is temporary without a removal condition. \
                 Temporary is only meaningful with an end.",
                r["path"]
            );
        }
    }
}

#[test]
fn statuses_are_known() {
    let c = contract();
    let declared: BTreeSet<&str> = c["path_statuses"]
        .as_object()
        .expect("path_statuses is an object")
        .keys()
        .map(String::as_str)
        .collect();
    for r in rules() {
        let status = r["status"].as_str().expect("every rule has a status");
        assert!(
            declared.contains(status),
            "Publication Violation: rule `{}` uses unknown status `{status}`. Declared: {declared:?}",
            r["path"]
        );
    }
}

#[test]
fn rules_do_not_silently_shadow_each_other() {
    // Two literal rules on the same path are an unresolvable ambiguity. `**` rules may
    // overlap a specific rule only when the specific one is a redaction, which is the
    // intended layering: broad public, narrow redact.
    let literals: Vec<&Value> = rules()
        .iter()
        .filter(|r| !r["path"].as_str().unwrap_or_default().contains(['*', '?']))
        .collect();
    let mut seen = BTreeSet::new();
    for r in &literals {
        let p = r["path"].as_str().unwrap();
        assert!(
            seen.insert(p.to_string()),
            "Publication Violation: `{p}` is declared by more than one literal rule, so \
             which one applies depends on file order"
        );
    }
}

#[test]
fn every_public_and_redact_rule_matches_a_tracked_path() {
    let tracked = tracked_paths();
    for r in rules() {
        let status = r["status"].as_str().unwrap_or_default();
        if status != "public" && status != "redact" && status != "temporary" {
            continue;
        }
        let pattern = r["path"].as_str().unwrap();
        assert!(
            tracked.iter().any(|p| glob_matches(pattern, p)),
            "Publication Violation: rule `{pattern}` ({status}) matches no tracked path. \
             A rule that matches nothing is either dead or misnamed, and both are drift."
        );
    }
}

#[test]
fn redaction_substitutions_are_declared_and_audited() {
    let c = contract();
    let redactions = c["redactions"]
        .as_object()
        .expect("redactions is an object");
    assert!(
        !redactions.is_empty(),
        "a contract that redacts nothing while the tree holds 105 files of private hostnames \
         is not a redaction contract"
    );
    for (name, spec) in redactions {
        assert_eq!(spec["kind"], "literal_substitution");
        assert_eq!(
            spec["must_preserve_compilation"], true,
            "redaction `{name}` must declare that the rendered tree still compiles"
        );
        let subs = spec["substitutions"]
            .as_array()
            .unwrap_or_else(|| panic!("redaction `{name}` declares no substitution"));
        assert!(!subs.is_empty());
        for s in subs {
            let replace = s["replace"].as_str().unwrap_or_default();
            assert!(
                replace.contains("example.invalid"),
                "Publication Violation: substitution `{}` -> `{replace}` does not target a \
                 reserved example domain. A replacement that could resolve turns a redacted \
                 identity into a live one.",
                s["match"].as_str().unwrap_or_default()
            );
        }
    }
}

#[test]
fn redaction_applies_to_every_published_path() {
    // Visibility and redaction are orthogonal. Modelling redaction as a path status
    // was wrong and provably so: 76 of the 105 contaminated files live under
    // `crates/`, which must be published, and `contracts/relay-policy.json` is a
    // normative contract that must also be published. One status cannot hold both.
    let c = contract();
    let declared = c["redactions"].as_object().expect("redactions");
    assert!(
        !declared.is_empty(),
        "a contract that redacts nothing while the tree holds a private hostname in every \
         host substrate file is not a redaction contract"
    );
    for (name, spec) in declared {
        assert_eq!(
            spec["applies_to"], "every published path, without exception",
            "redaction `{name}` is scoped. A scoped redaction misses the next literal, \
             which is how 76 crates would have been published verbatim."
        );
    }
}

#[test]
fn read_back_has_execution_steps_not_only_content_steps() {
    let c = contract();
    let steps = c["read_back"]["steps"]
        .as_array()
        .expect("read_back.steps is an array");
    let ids: Vec<&str> = steps.iter().map(|s| s["id"].as_str().unwrap_or_default()).collect();
    for required in [
        "no_forbidden_literal",
        "no_undeclared_path",
        "secrets_clean",
        "workspace_compiles",
        "tests_green",
        "canonical_gate_runs",
    ] {
        assert!(
            ids.contains(&required),
            "Publication Violation: read_back is missing `{required}`. A read_back without an \
             execution step proves only that the file looks right, not that it runs."
        );
    }
    assert!(
    let c = contract();
    c["read_back"]["on_any_failure"]
            .as_str()
            .is_some_and(|s| s.contains("refused")),
        "read_back must state what happens on failure; a soft failure is a wish"
    );
}

/// The invariant that closes the class, not the instance.
///
/// A redaction that only covers the paths somebody thought of fails silently on the
/// next literal. This measures the contamination on every published path and requires
/// it to be non-zero, so that the day the tree is clean the assertion has to be
/// re-argued rather than silently passing.
#[test]
fn contamination_of_published_content_stays_measured() {
    let c = contract();
    let redactions = c["redactions"].as_object().expect("redactions").clone();
    let stems: Vec<String> = redactions
        .values()
        .filter_map(|s| s["substitutions"].as_array())
        .flatten()
        .filter_map(|s| s["match"].as_str().map(str::to_string))
        .filter_map(|p| {
            let stem = p
                .split(['\\', '.', '[', '(', '$'])
                .next()
                .unwrap_or_default()
                .to_string();
            (stem.len() > 4).then_some(stem)
        })
        .collect();
    assert!(!stems.is_empty(), "no usable stem could be derived from the redaction patterns");

    let contaminated: Vec<String> = tracked_paths()
        .into_iter()
        .filter(|p| rule_for(p).map(|r| r["status"] != "exclude").unwrap_or(false))
        .filter(|p| {
            fs::read_to_string(workspace_root().join(p))
                .map(|body| stems.iter().any(|s| body.contains(s.as_str())))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !contaminated.is_empty(),
        "no published file holds a private literal any more. If that is real, remove the \
         redactions and record the date here; if it is not real, this test is vacuous and \
         the read-back `no_forbidden_literal` step is the only thing still checking."
    );
    let declared = redactions["infra_identities"]["measured_occurrences"]["files"]
        .as_u64()
        .expect("measured_occurrences.files");
    assert!(
        declared >= contaminated.len() as u64,
        "measured_occurrences.files ({declared}) is below the live count ({}) — the contract \
         is stale and would under-report what publication actually rewrites",
        contaminated.len()
    );
}
