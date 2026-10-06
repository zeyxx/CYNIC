//! Artefact contract rule.
//!
//! WHY. The workspace built 13 binaries while `infra/runtime-basket.json`
//! declared 5, and nothing compared the two. Three sets existed -- built,
//! declared, installed -- and none was checked against another. `autobins` was
//! declared nowhere, so a file dropped in `src/bin/` became a deployment
//! artefact with no declaration in any manifest, registry, or contract.
//!
//! This rule is the comparator. It runs inside `cynic audit all`, which is
//! `make audit` -- the canonical gate named by AGENTS.md -- so the comparison
//! executes rather than existing as a test nobody invokes.
//!
//! OBSERVATION. Everything comes from the scanner's `WorkspaceSourceIndex`:
//! the manifests already parsed, the JSON files already read. This rule
//! performs no I/O, so it is deterministic and answers the same on any host.
//!
//! NEGATIVE BOUNDARY. It does not build, install, or classify. It compares the
//! declared set with the produced set and names the difference in both
//! directions.

use std::collections::BTreeSet;

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const RULE_ID: &str = "ART-001";
const BASKET_PATH: &str = "infra/runtime-basket.json";

pub struct ArtefactContractRule;

impl ArtefactContractRule {
    /// Binary names declared by `[[bin]]` sections across the workspace.
    ///
    /// With `autobins = false` in every crate (enforced by `autobins_frozen`)
    /// this is exhaustive: a binary absent here is not built, and a binary
    /// built but absent here is impossible.
    fn declared_binaries(index: &WorkspaceSourceIndex) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for manifest in &index.manifests {
            let mut in_bin = false;
            for line in manifest.content.lines() {
                let trimmed = line.trim();
                if trimmed == "[[bin]]" {
                    in_bin = true;
                    continue;
                }
                if trimmed.starts_with('[') {
                    in_bin = false;
                    continue;
                }
                if in_bin {
                    if let Some(rest) = trimmed.strip_prefix("name = ") {
                        let name = rest.trim().trim_matches('"');
                        if !name.is_empty() {
                            found.insert(name.to_string());
                        }
                    }
                }
            }
        }
        found
    }

    /// Binary names the contract accounts for: shipped artefacts, tooling, and
    /// the runtime seed (deliberately not a basket member).
    fn contracted_binaries(index: &WorkspaceSourceIndex) -> Option<BTreeSet<String>> {
        let basket = index
            .json_files
            .iter()
            .find(|f| f.relative_path == BASKET_PATH)?;
        let doc: serde_json::Value = serde_json::from_str(&basket.content).ok()?;

        let mut found = BTreeSet::new();
        for key in ["artifacts", "tooling"] {
            for entry in doc.get(key)?.as_array()? {
                found.insert(entry.get("binary")?.as_str()?.to_string());
            }
        }
        found.insert(
            doc.get("runtime_seed")?
                .get("binary")?
                .as_str()?
                .to_string(),
        );
        Some(found)
    }
}

impl AuditRule for ArtefactContractRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: RULE_ID.to_string(),
            name: "artefact_contract".to_string(),
            description: format!(
                "Every binary the workspace declares must be accounted for in {BASKET_PATH} \
                 (artifacts or tooling), and every entry there must correspond to a declared \
                 binary. An undeclared binary is a deployment artefact nobody agreed to ship; \
                 a phantom entry makes a role bundle select an artefact that cannot exist."
            ),
            category: AuditCategory::MutationContract,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        let Some(contracted) = Self::contracted_binaries(index) else {
            // An unreadable basket is an explicit unknown, not a pass. Reporting
            // it as a violation keeps the gate honest: the comparator cannot
            // compare, and saying so is the correct outcome.
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-BASKET-UNREADABLE"),
                message: format!(
                    "{BASKET_PATH} could not be read or parsed; the artefact contract \
                     comparator could not run. This is an explicit unknown, not a pass."
                ),
                severity: AuditSeverity::Critical,
                category: AuditCategory::ResidualUnknown,
                span: None,
                remediation: Some(format!(
                    "Restore {BASKET_PATH} as valid JSON with `artifacts`, `tooling`, \
                     and `runtime_seed.binary` keys."
                )),
            });
            return violations;
        };

        let declared = Self::declared_binaries(index);
        if declared.is_empty() {
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-NO-BINARIES-OBSERVED"),
                message: "No [[bin]] section was observed in any workspace manifest. \
                          The comparator would pass vacuously, which means the observation \
                          failed rather than the contract being satisfied."
                    .to_string(),
                severity: AuditSeverity::Critical,
                category: AuditCategory::ResidualUnknown,
                span: None,
                remediation: Some(
                    "Verify the scanner reads crates/*/Cargo.toml into `manifests`.".to_string(),
                ),
            });
            return violations;
        }

        for name in declared.difference(&contracted) {
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-UNDECLARED-ARTIFACT"),
                message: format!(
                    "The workspace declares a `[[bin]]` named `{name}` that {BASKET_PATH} \
                     does not account for. A binary produced by the compiler but absent from \
                     the contract is a deployment artefact nobody agreed to ship."
                ),
                severity: AuditSeverity::Critical,
                category: AuditCategory::MutationContract,
                span: None,
                remediation: Some(format!(
                    "Add `{name}` to `artifacts` (deployed, needs roles and install_path) \
                     or to `tooling` (never deployed) in {BASKET_PATH}."
                )),
            });
        }

        for name in contracted.difference(&declared) {
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-PHANTOM-ARTIFACT"),
                message: format!(
                    "{BASKET_PATH} accounts for a binary named `{name}` that no workspace \
                     manifest declares. A role bundle selecting this artefact cannot activate, \
                     and `missing_required_artifact` is `blocked`."
                ),
                severity: AuditSeverity::Critical,
                category: AuditCategory::MutationContract,
                span: Some(AuditSpan {
                    file_path: BASKET_PATH.to_string(),
                    line_start: 1,
                    line_end: 1,
                    snippet: Some(name.clone()),
                }),
                remediation: Some(format!(
                    "Remove `{name}` from {BASKET_PATH}, or restore the `[[bin]]` target \
                     that produced it."
                )),
            });
        }

        violations
    }
}

/// The `autobins` freeze, asserted rather than assumed.
///
/// WHY. `autobins` is a `[package]` key and does NOT inherit from
/// `[workspace.package]` (verified: `autobins.workspace = true` is a manifest
/// error). So the freeze holds only if every crate sets it individually. A
/// single new crate without it reintroduces silent artefact creation, and
/// nothing would notice.
///
/// Separate rule from `ArtefactContractRule` on purpose: one asserts the
/// observation is exhaustive, the other asserts the contract matches. A failure
/// in the first makes the second's verdict unreliable, so they must be able to
/// fail independently.
pub struct AutobinsFrozenRule;

impl AuditRule for AutobinsFrozenRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "ART-002".to_string(),
            name: "autobins_frozen".to_string(),
            description: "Every crate must set `autobins = false`. Without it, any file in \
                          src/bin/ becomes a build artefact with no declaration anywhere, \
                          which is how three undeclared executables appeared in cynic-host."
                .to_string(),
            category: AuditCategory::MutationContract,
            severity: AuditSeverity::Critical,
            automated_remediation_available: true,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();
        for manifest in &index.manifests {
            // Only crate manifests carry [[package]]; the root workspace manifest
            // cannot set autobins for its members.
            if !manifest.relative_path.starts_with("crates/") {
                continue;
            }
            let has_package = manifest.content.lines().any(|l| l.trim() == "[package]");
            if !has_package {
                continue;
            }
            let frozen = manifest
                .content
                .lines()
                .any(|l| l.trim().replace(' ', "") == "autobins=false");
            if !frozen {
                violations.push(AuditViolation {
                    code: "ART-002-AUTOBINS-NOT-FROZEN".to_string(),
                    message: format!(
                        "{} does not set `autobins = false`. Cargo will autodiscover every \
                         src/main.rs and src/bin/*.rs in this crate as a deployment artefact.",
                        manifest.relative_path
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::MutationContract,
                    span: None,
                    remediation: Some(format!(
                        "Add `autobins = false` under [package] in {}, then declare each \
                         binary with an explicit [[bin]] section.",
                        manifest.relative_path
                    )),
                });
            }
        }
        violations
    }
}
