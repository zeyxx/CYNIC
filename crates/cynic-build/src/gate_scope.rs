//! Gate scope: run only the gate the change deserves.
//!
//! A Python-only commit must never cost a Rust compile; a docs-only commit
//! must cost nothing. This module is pure: changed paths in, minimal gate
//! out. The trigger resolves the paths (local `git diff`, read-only) and
//! executes what the scope says — never more.
//!
//! Fail-open rule: anything unrecognized maps to [`GateScope::Full`]. Skipping
//! a needed gate is a silent lie; running an unneeded one is only wasted
//! energy. When in doubt, spend the energy.

use std::collections::BTreeSet;

/// Minimal gate for a change set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateScope {
    /// `cargo check -p <packages>` on the runner. Empty means the whole workspace.
    RustCheck { packages: Vec<String> },
    /// Local pytest for one `apps/<suite>` tree. No builder dispatch.
    PythonSuite { suite: String },
    /// Changed JSON is still well-formed. No compile, no tests.
    ValidateOnly { files: Vec<String> },
    /// Nothing to prove (docs, comments, metadata). The reason is the proof.
    Nothing { reason: String },
    /// Full gate: workspace roots, shared crates, CI, or anything unknown.
    Full { reason: String },
}

/// Crates whose change invalidates the whole workspace check
/// (measured: 34 of 44 crates depend on cynic-protocol).
const SHARED_CRATES: &[&str] = &["cynic-protocol"];

/// Decide the minimal gate for a set of changed repo-relative paths.
pub fn decide_gate_scope(paths: &[&str]) -> GateScope {
    let mut rust = BTreeSet::new();
    let mut suites = BTreeSet::new();
    let mut json_files = Vec::new();

    for path in paths {
        let path = path.trim().trim_start_matches("./");
        if path.is_empty() {
            continue;
        }
        if is_ignorable(path) {
            continue;
        }
        if let Some(rest) = path.strip_prefix("apps/") {
            let suite = rest.split('/').next().unwrap_or("");
            if path.ends_with(".py") && !suite.is_empty() {
                suites.insert(suite.to_string());
                continue;
            }
            return GateScope::Full {
                reason: format!("non-Python change under apps/: {path}"),
            };
        }
        if let Some(rest) = path.strip_prefix("crates/") {
            let krate = rest.split('/').next().unwrap_or("");
            if krate.is_empty() {
                return GateScope::Full {
                    reason: format!("unscoped crates change: {path}"),
                };
            }
            if SHARED_CRATES.contains(&krate) {
                return GateScope::Full {
                    reason: format!("shared crate touched: {krate}"),
                };
            }
            rust.insert(krate.to_string());
            continue;
        }
        if path == "Cargo.toml"
            || path == "Cargo.lock"
            || path == "Makefile"
            || path.starts_with("forgejo/")
            || path.starts_with(".github/")
        {
            return GateScope::Full {
                reason: format!("workspace root touched: {path}"),
            };
        }
        if path.starts_with("contracts/") || path.starts_with("infra/") {
            if path.ends_with(".json") {
                json_files.push(path.to_string());
                continue;
            }
            return GateScope::Full {
                reason: format!("non-JSON registry change: {path}"),
            };
        }
        return GateScope::Full {
            reason: format!("unclassified path: {path}"),
        };
    }

    if !rust.is_empty() && suites.is_empty() && json_files.is_empty() {
        return GateScope::RustCheck {
            packages: rust.into_iter().collect(),
        };
    }
    if rust.is_empty() && suites.len() == 1 && json_files.is_empty() {
        return GateScope::PythonSuite {
            suite: suites.into_iter().next().unwrap_or_default(),
        };
    }
    if rust.is_empty() && suites.is_empty() && !json_files.is_empty() {
        return GateScope::ValidateOnly { files: json_files };
    }
    if rust.is_empty() && suites.is_empty() && json_files.is_empty() {
        return GateScope::Nothing {
            reason: "docs/metadata only: nothing to compile, test, or validate"
                .to_string(),
        };
    }
    GateScope::Full {
        reason: "mixed change set: rust + python/json cannot share one gate".to_string(),
    }
}

/// Closure cap: beyond this many crates the workspace check is cheaper
/// than N narrowed ones (shared deps recompile anyway) — fall back to Full.
pub const CLOSURE_CAP: usize = 6;

/// Transitive consumers closure: every crate that (directly or indirectly)
/// depends on a touched crate must re-prove — an API break hides there.
/// `consumers_of` maps a crate to its direct consumers. Pure; the caller
/// loads the topology (infra/crate-topology.json) and passes the lookup.
pub fn expand_with_consumers(
    packages: &[String],
    consumers_of: &dyn Fn(&str) -> Vec<String>,
) -> Vec<String> {
    let mut seen: BTreeSet<String> = packages.iter().cloned().collect();
    let mut stack: Vec<String> = packages.to_vec();
    while let Some(next) = stack.pop() {
        for consumer in consumers_of(&next) {
            if seen.insert(consumer.clone()) {
                stack.push(consumer);
            }
        }
    }
    seen.into_iter().collect()
}

/// Client-call matrix for the proof system: every (`scope`, `package`,
/// `phase`, `filter`) combination maps to exactly one cell. The table is
/// the contract; the tests below pin each cell. Adding a call shape means
/// adding a row here first.
///
/// | scope | package        | phase | filter | cell              |
/// |-------|----------------|-------|--------|-------------------|
/// | full  | none           | check | none   | workspace check   |
/// | full  | none           | test  | any    | workspace suite   |
/// | full  | one            | check | none   | package check     |
/// | full  | one            | test  | any    | package suite     |
/// | full  | several        | any   | none   | narrowed checks   |
/// | full  | any            | any   | set+check | REFUSED (filter needs test) |
/// | full  | several        | test  | set    | REFUSED (filter is single-package) |
/// | auto  | any            | any   | any    | diff decides (see GateScope) |
/// | auto+explicit package | wins over diff; scope only fills unset package |
///
/// `auto` resolves the commit diff through [`decide_gate_scope`], then:
/// Nothing/ValidateOnly/PythonSuite run locally with no builder dispatch;
/// RustCheck proves the closed package set; Full falls back to the `full`
/// row for the same (package, phase, filter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPlan {
    /// No builder dispatch; the reason is the proof.
    Local {
        kind: LocalProofKind,
        reason: String,
    },
    /// One builder proof per package, same phase and filter.
    Proofs {
        packages: Vec<String>,
        phase: ProofPhaseKind,
        filter: Option<String>,
    },
    /// The call shape is invalid; the reason names the fix.
    Refused { reason: String },
}

/// Local (no-builder) proof legs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalProofKind {
    Nothing,
    ValidateJson { files: Vec<String> },
    PythonSuite { suite: String },
}

/// Phase without scope or filter attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPhaseKind {
    Check,
    Test,
}

/// Resolve the client-call matrix. Pure: inputs in, one cell out.
/// `explicit_package` is the caller's `--package` (comma-joined allowed);
/// `diff_paths` is the commit diff (repo-relative paths), consulted only
/// when `scope_auto` is true and no explicit package is set.
/// `consumers_of` maps a crate to its direct consumers (topology lookup,
/// injected to keep this pure); the transitive closure is capped at
/// [`CLOSURE_CAP`], beyond which the set falls back to Full.
pub fn resolve_proof_plan(
    scope_auto: bool,
    explicit_package: Option<&str>,
    phase: ProofPhaseKind,
    filter: Option<&str>,
    diff_paths: &[&str],
    consumers_of: &dyn Fn(&str) -> Vec<String>,
) -> ProofPlan {
    if phase == ProofPhaseKind::Check && filter.is_some() {
        return ProofPlan::Refused {
            reason: "filter needs --phase test; check ignores filters".to_string(),
        };
    }
    let packages: Vec<String> = explicit_package
        .map(|p| {
            p.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if phase == ProofPhaseKind::Test && filter.is_some() && packages.len() > 1 {
        return ProofPlan::Refused {
            reason: "one filter cannot scope several packages; narrow --package to one".to_string(),
        };
    }
    if !scope_auto || !packages.is_empty() {
        return ProofPlan::Proofs {
            packages,
            phase,
            filter: filter.map(str::to_string),
        };
    }
    match decide_gate_scope(diff_paths) {
        GateScope::Nothing { reason } => ProofPlan::Local {
            kind: LocalProofKind::Nothing,
            reason,
        },
        GateScope::ValidateOnly { files } => ProofPlan::Local {
            kind: LocalProofKind::ValidateJson {
                files: files.clone(),
            },
            reason: "json-only change: well-formedness is the proof".to_string(),
        },
        GateScope::PythonSuite { suite } => ProofPlan::Local {
            kind: LocalProofKind::PythonSuite {
                suite: suite.clone(),
            },
            reason: "python-only change: suite run is the proof".to_string(),
        },
        GateScope::RustCheck { packages, .. } => {
            let closed = expand_with_consumers(&packages, consumers_of);
            if closed.len() > CLOSURE_CAP {
                ProofPlan::Proofs {
                    packages: vec![],
                    phase,
                    filter: filter.map(str::to_string),
                }
            } else {
                ProofPlan::Proofs {
                    packages: closed,
                    phase,
                    filter: filter.map(str::to_string),
                }
            }
        }
        GateScope::Full { reason } => {
            let _ = reason;
            ProofPlan::Proofs {
                packages: vec![],
                phase,
                filter: filter.map(str::to_string),
            }
        }
    }
}
/// Paths that never affect any gate: docs, comments, editor noise.
fn is_ignorable(path: &str) -> bool {
    path.ends_with(".md")
        || path.starts_with("docs/")
        || path.ends_with(".gitignore")
        || path.ends_with(".editorconfig")
        || path.starts_with(".vscode/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_only_selects_its_suite() {
        let scope = decide_gate_scope(&[
            "apps/cynic-data/src/cynic_data/foo.py",
            "apps/cynic-data/tests/test_foo.py",
            "docs/notes.md",
        ]);
        assert_eq!(
            scope,
            GateScope::PythonSuite {
                suite: "cynic-data".to_string()
            }
        );
    }

    #[test]
    fn single_crate_narrows_the_check() {
        let scope = decide_gate_scope(&["crates/cynic-build/src/place.rs"]);
        assert_eq!(
            scope,
            GateScope::RustCheck {
                packages: vec!["cynic-build".to_string()]
            }
        );
    }

    #[test]
    fn shared_crate_forces_full() {
        let scope = decide_gate_scope(&["crates/cynic-protocol/src/lib.rs"]);
        assert!(matches!(scope, GateScope::Full { .. }));
    }

    #[test]
    fn workspace_roots_force_full() {
        for path in ["Cargo.toml", "Cargo.lock", "Makefile"] {
            assert!(matches!(decide_gate_scope(&[path]), GateScope::Full { .. }), "{path}");
        }
    }

    #[test]
    fn json_registry_validates_only() {
        let scope = decide_gate_scope(&["infra/registry.json"]);
        assert_eq!(
            scope,
            GateScope::ValidateOnly {
                files: vec!["infra/registry.json".to_string()]
            }
        );
    }

    #[test]
    fn docs_only_costs_nothing() {
        let scope = decide_gate_scope(&["README.md", "docs/specs/x.md"]);
        assert!(matches!(scope, GateScope::Nothing { .. }));
    }

    #[test]
    fn mixed_rust_and_python_falls_back_to_full() {
        let scope = decide_gate_scope(&[
            "crates/cynic-build/src/lib.rs",
            "apps/cynic-data/src/x.py",
        ]);
        assert!(matches!(scope, GateScope::Full { .. }));
    }

    #[test]
    fn closure_covers_transitive_consumers() {
        let consumers = |name: &str| match name {
            "leaf" => vec!["mid".to_string()],
            "mid" => vec!["top".to_string()],
            _ => vec![],
        };
        assert_eq!(
            expand_with_consumers(&["leaf".to_string()], &consumers),
            vec![
                "leaf".to_string(),
                "mid".to_string(),
                "top".to_string()
            ]
        );
    }

    #[test]
    fn closure_dedupes_diamonds() {
        let consumers = |name: &str| match name {
            "base" => vec!["left".to_string(), "right".to_string()],
            "left" | "right" => vec!["top".to_string()],
            _ => vec![],
        };
        let closed = expand_with_consumers(&["base".to_string()], &consumers);
        assert_eq!(closed.len(), 4);
    }

    #[test]
    fn unknown_paths_fail_open_to_full() {
        let scope = decide_gate_scope(&["scripts/mystery.sh"]);
        assert!(matches!(scope, GateScope::Full { .. }));
    }

    fn no_consumers(_: &str) -> Vec<String> {
        vec![]
    }

    fn plan(
        scope_auto: bool,
        package: Option<&str>,
        phase: ProofPhaseKind,
        filter: Option<&str>,
        diff: &[&str],
    ) -> ProofPlan {
        resolve_proof_plan(scope_auto, package, phase, filter, diff, &no_consumers)
    }

    #[test]
    fn full_check_without_package_is_workspace_check() {
        assert_eq!(
            plan(false, None, ProofPhaseKind::Check, None, &[]),
            ProofPlan::Proofs {
                packages: vec![],
                phase: ProofPhaseKind::Check,
                filter: None,
            }
        );
    }

    #[test]
    fn full_test_single_package_keeps_filter() {
        assert_eq!(
            plan(
                false,
                Some("cynic-node"),
                ProofPhaseKind::Test,
                Some("fleet_outbox"),
                &[]
            ),
            ProofPlan::Proofs {
                packages: vec!["cynic-node".to_string()],
                phase: ProofPhaseKind::Test,
                filter: Some("fleet_outbox".to_string()),
            }
        );
    }

    #[test]
    fn filter_with_check_is_refused() {
        assert!(matches!(
            plan(
                false,
                Some("cynic-node"),
                ProofPhaseKind::Check,
                Some("x"),
                &[]
            ),
            ProofPlan::Refused { .. }
        ));
    }

    #[test]
    fn filter_with_several_packages_is_refused() {
        assert!(matches!(
            plan(
                false,
                Some("cynic-node,cynic-cli"),
                ProofPhaseKind::Test,
                Some("x"),
                &[]
            ),
            ProofPlan::Refused { .. }
        ));
    }

    #[test]
    fn explicit_package_wins_over_auto_diff() {
        assert_eq!(
            plan(
                true,
                Some("cynic-build"),
                ProofPhaseKind::Check,
                None,
                &["docs/notes.md"]
            ),
            ProofPlan::Proofs {
                packages: vec!["cynic-build".to_string()],
                phase: ProofPhaseKind::Check,
                filter: None,
            }
        );
    }

    #[test]
    fn auto_docs_only_needs_nothing() {
        assert!(matches!(
            plan(true, None, ProofPhaseKind::Check, None, &["README.md"]),
            ProofPlan::Local {
                kind: LocalProofKind::Nothing,
                ..
            }
        ));
    }

    #[test]
    fn auto_json_only_validates_locally() {
        assert!(matches!(
            plan(
                true,
                None,
                ProofPhaseKind::Check,
                None,
                &["infra/registry.json"]
            ),
            ProofPlan::Local {
                kind: LocalProofKind::ValidateJson { .. },
                ..
            }
        ));
    }

    #[test]
    fn auto_single_crate_proves_it() {
        assert_eq!(
            plan(
                true,
                None,
                ProofPhaseKind::Check,
                None,
                &["crates/cynic-build/src/place.rs"]
            ),
            ProofPlan::Proofs {
                packages: vec!["cynic-build".to_string()],
                phase: ProofPhaseKind::Check,
                filter: None,
            }
        );
    }

    #[test]
    fn auto_shared_crate_falls_back_to_full() {
        assert_eq!(
            plan(
                true,
                None,
                ProofPhaseKind::Check,
                None,
                &["crates/cynic-protocol/src/lib.rs"]
            ),
            ProofPlan::Proofs {
                packages: vec![],
                phase: ProofPhaseKind::Check,
                filter: None,
            }
        );
    }

    #[test]
    fn auto_closure_over_cap_falls_back_to_full() {
        let fan = |_: &str| {
            (0..CLOSURE_CAP + 1)
                .map(|i| format!("consumer-{i}"))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            resolve_proof_plan(
                true,
                None,
                ProofPhaseKind::Check,
                None,
                &["crates/cynic-build/src/lib.rs"],
                &fan
            ),
            ProofPlan::Proofs {
                packages: vec![],
                phase: ProofPhaseKind::Check,
                filter: None,
            }
        );
    }
}
