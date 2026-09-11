//! Policy formality audit: measures whether each declarative CYNIC policy is
//! typed law (code an agent can consume) or informal prose (text an agent must
//! interpret).
//!
//! Authority sources scanned: `crates/*/src`. A policy's status is DERIVED from
//! token evidence found in source files at run time; the table below carries
//! expectations and scan tokens, never statuses. Excluding this module from its
//! own scan keeps the table literals from self-certifying.
//!
//! Schema: `cynic.cli.policy_formality_audit.v1`

use serde::Serialize;

use crate::rust_source::{relative_path, rust_files_under, workspace_crate_names, workspace_root};

/// Crates searched after `cynic-protocol`, in workspace order. Protocol hits
/// are reported as canonical typed law; later hits as runtime-crate-only
/// typing. Observed from the workspace — never a hardcoded list.
fn runtime_crates() -> Vec<String> {
    workspace_crate_names()
        .into_iter()
        .filter(|name| name != "cynic-protocol")
        .collect()
}

struct PolicyFormalitySpec {
    /// Stable policy identifier (snake_case).
    policy: &'static str,
    /// Where the declarative law currently lives when it is NOT code.
    authority_prose_source: &'static str,
    /// Crate that should own the typed form per one-boundary-one-reason.
    expected_owner_crate: &'static str,
    /// Distinctive identifiers whose presence proves a typed form exists.
    scan_tokens: &'static [&'static str],
}

const POLICY_SURFACES: &[PolicyFormalitySpec] = &[
    PolicyFormalitySpec {
        policy: "secret_material_governance",
        authority_prose_source: "AGENTS.md §Secrets (class table)",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["SecretClass", "SecretReference"],
    },
    PolicyFormalitySpec {
        policy: "membership_liveness_policy",
        authority_prose_source: "protocols/machine-enrollment-federation.md D3",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["MembershipPolicy", "membership_liveness_transition"],
    },
    PolicyFormalitySpec {
        policy: "install_provenance",
        authority_prose_source: "AGENTS.md §Structural Invariants 4",
        expected_owner_crate: "cynic-host",
        scan_tokens: &["ToolingInstallProvenance", "InstallProvenance"],
    },
    PolicyFormalitySpec {
        policy: "surface_mutation_policy",
        authority_prose_source: "infra/surface-registry.json mutation_policy",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["SurfaceMutationPolicy"],
    },
    PolicyFormalitySpec {
        policy: "agent_coordination_claims",
        authority_prose_source: "AGENTS.md §Agent Coordination Claims",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["agent_claims.v2"],
    },
    PolicyFormalitySpec {
        policy: "agent_delegation_envelope",
        authority_prose_source: "AGENTS.md §Fractal agent recursion",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["agent_delegation_envelope"],
    },
    PolicyFormalitySpec {
        policy: "autonomy_decision_law",
        authority_prose_source: "protocols + AGENTS.md Autonomous mode",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["decide_autonomy"],
    },
    PolicyFormalitySpec {
        policy: "runtime_harness_dimensions",
        authority_prose_source: "AGENTS.md §Runtime Harness",
        expected_owner_crate: "cynic-host",
        scan_tokens: &["RuntimeHarnessKind", "isolation_boundary"],
    },
    PolicyFormalitySpec {
        policy: "metric_target_residual_risk",
        authority_prose_source: "AGENTS.md §Runtime Harness metric rule",
        expected_owner_crate: "cynic-askesis",
        scan_tokens: &["target_percent", "residual_risk"],
    },
    PolicyFormalitySpec {
        policy: "publication_disclosure_consent",
        authority_prose_source: "AGENTS.md §Long-Horizon Direction disclosure guardrail",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &[
            "PublicationConsent",
            "project_for_publication",
            "PublicProjection",
        ],
    },
    PolicyFormalitySpec {
        policy: "machine_identity_orphan_rule",
        authority_prose_source: "protocols/machine-enrollment-federation.md D1",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["deletion_rule"],
    },
    PolicyFormalitySpec {
        policy: "enrollment_voucher_anchor_l0",
        authority_prose_source: "protocols/machine-enrollment-federation.md §1bis",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["EnrollmentVoucher"],
    },
    PolicyFormalitySpec {
        policy: "transport_readback_contract",
        authority_prose_source: "protocols/machine-enrollment-federation.md step 2",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["TransportReadback", "transport_readback"],
    },
    PolicyFormalitySpec {
        policy: "agent_workload_admission_envelope",
        authority_prose_source: "AGENTS.md §Agent Session Practice",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["agent_workload_envelope"],
    },
    PolicyFormalitySpec {
        policy: "foundation_evolutive_policies",
        authority_prose_source: "contracts/foundation.json evolutive_policies (prose strings)",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["EvolutivePolicy", "default_gate"],
    },
    PolicyFormalitySpec {
        policy: "organic_opportunities_compass",
        authority_prose_source: "AGENTS.md §Organic opportunities — Organic always wins",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &[
            "OrganicOpportunity",
            "rank_in_value_frame",
            "OpportunityDecisionRecord",
        ],
    },
    PolicyFormalitySpec {
        policy: "one_shot_capability_bounded_budget",
        authority_prose_source: "AGENTS.md §Agent Coordination Claims & Sec1",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["OneShotCapability", "satisfies_monotonic_budget"],
    },
    PolicyFormalitySpec {
        policy: "constitutional_authority_hierarchy",
        authority_prose_source: "contracts/foundation.json §authority_order & AGENTS.md §Authority",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &[
            "AuthorityLevel",
            "resolve_precedence",
            "ObservedRuntimeState",
        ],
    },
    PolicyFormalitySpec {
        policy: "cynic_coherence_pattern",
        authority_prose_source: "AGENTS.md §CYNIC Coherence Pattern (5 terms x 5 scales)",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["CoherencePattern", "CoherenceScale", "is_coherent"],
    },
    PolicyFormalitySpec {
        policy: "bottom_up_design_gate",
        authority_prose_source: "AGENTS.md §Design gate (6 mandatory questions)",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &[
            "DesignGateAnswers",
            "observation_port",
            "pure_reconciliation_rule",
        ],
    },
    PolicyFormalitySpec {
        policy: "hard_constitutional_boundaries",
        authority_prose_source: "contracts/foundation.json §hard_boundaries (11 invariants)",
        expected_owner_crate: "cynic-protocol",
        scan_tokens: &["ConstitutionalBoundary", "rule_text", "LowLayerFirst"],
    },
];

#[derive(Serialize)]
pub(crate) struct PolicyFormalityReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) summary: PolicyFormalitySummary,
    pub(crate) policies: Vec<PolicyFormalityEntry>,
}

#[derive(Serialize)]
pub(crate) struct PolicyFormalitySummary {
    pub(crate) policy_count: usize,
    pub(crate) typed_in_protocol_count: usize,
    pub(crate) typed_in_runtime_crate_count: usize,
    pub(crate) prose_only_count: usize,
    pub(crate) blocked_unobserved_count: usize,
}

#[derive(Serialize)]
pub(crate) struct PolicyFormalityEntry {
    pub(crate) policy: &'static str,
    pub(crate) authority_prose_source: &'static str,
    pub(crate) expected_owner_crate: &'static str,
    pub(crate) scan_tokens: Vec<&'static str>,
    /// typed_in_protocol | typed_in_runtime_crate | prose_only | blocked_unobserved
    pub(crate) status: String,
    /// Source files carrying the token evidence (crate-relative paths).
    pub(crate) evidence_paths: Vec<String>,
}

pub(crate) fn audit_policy_formality() -> PolicyFormalityReport {
    let mut typed_in_protocol_count = 0usize;
    let mut typed_in_runtime_crate_count = 0usize;
    let mut prose_only_count = 0usize;
    let mut blocked_unobserved_count = 0usize;

    let policies = POLICY_SURFACES
        .iter()
        .map(|spec| {
            let protocol_evidence = find_token_evidence("cynic-protocol", spec.scan_tokens);
            let (mut status, evidence_paths) = if protocol_evidence.is_empty() {
                let runtime_evidence = runtime_crates()
                    .iter()
                    .flat_map(|crate_name| {
                        find_token_evidence(crate_name, spec.scan_tokens)
                            .into_iter()
                            .map(move |path| format!("{crate_name}/{path}"))
                    })
                    .collect::<Vec<_>>();
                if runtime_evidence.is_empty() {
                    ("prose_only".to_string(), runtime_evidence)
                } else {
                    ("typed_in_runtime_crate".to_string(), runtime_evidence)
                }
            } else {
                ("typed_in_protocol".to_string(), protocol_evidence)
            };

            // Unknown is data: an unresolvable workspace root must not read as
            // prose debt nor as typed law.
            if !workspace_root_exists() && evidence_paths.is_empty() {
                status = "blocked_unobserved".to_string();
            }

            match status.as_str() {
                "typed_in_protocol" => typed_in_protocol_count += 1,
                "typed_in_runtime_crate" => typed_in_runtime_crate_count += 1,
                "prose_only" => prose_only_count += 1,
                "blocked_unobserved" => blocked_unobserved_count += 1,
                _ => {}
            }

            PolicyFormalityEntry {
                policy: spec.policy,
                authority_prose_source: spec.authority_prose_source,
                expected_owner_crate: spec.expected_owner_crate,
                scan_tokens: spec.scan_tokens.to_vec(),
                status,
                evidence_paths,
            }
        })
        .collect::<Vec<_>>();

    let status = if prose_only_count == 0 {
        "ok"
    } else {
        "formalization_debt"
    };

    PolicyFormalityReport {
        schema: "cynic.cli.policy_formality_audit.v1",
        status,
        summary: PolicyFormalitySummary {
            policy_count: POLICY_SURFACES.len(),
            typed_in_protocol_count,
            typed_in_runtime_crate_count,
            prose_only_count,
            blocked_unobserved_count: blocked_unobserved_count,
        },
        policies,
    }
}

fn workspace_root_exists() -> bool {
    workspace_root().join("Cargo.toml").exists()
}

fn find_token_evidence(crate_name: &str, tokens: &[&str]) -> Vec<String> {
    let crate_root = workspace_root().join("crates").join(crate_name).join("src");
    if !crate_root.exists() {
        return Vec::new();
    }

    let mut matches = std::collections::BTreeSet::new();
    for file in rust_files_under(&crate_root) {
        // Never certify ourselves: the token table lives here verbatim.
        if file
            .file_name()
            .is_some_and(|name| name == "policy_formality_audit.rs")
        {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        if tokens.iter().any(|token| content.contains(token)) {
            matches.insert(relative_path(&crate_root, &file));
        }
    }

    matches.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::audit_policy_formality;

    #[test]
    fn reports_expected_schema_and_counts_all_policies() {
        let report = audit_policy_formality();
        assert_eq!(report.schema, "cynic.cli.policy_formality_audit.v1");
        assert_eq!(report.summary.policy_count, report.policies.len());
        assert_eq!(
            report.summary.policy_count,
            report.summary.typed_in_protocol_count
                + report.summary.typed_in_runtime_crate_count
                + report.summary.prose_only_count
                + report.summary.blocked_unobserved_count
        );
    }

    #[test]
    fn every_policy_declares_tokens_and_known_expected_owner() {
        let report = audit_policy_formality();
        for entry in &report.policies {
            assert!(
                !entry.scan_tokens.is_empty(),
                "{} has no tokens",
                entry.policy
            );
            assert!(
                entry.expected_owner_crate.starts_with("cynic-"),
                "{} expects unknown crate {}",
                entry.policy,
                entry.expected_owner_crate
            );
        }
    }

    #[test]
    fn secret_governance_is_canonical_typed_law() {
        let report = audit_policy_formality();
        let entry = report
            .policies
            .iter()
            .find(|entry| entry.policy == "secret_material_governance")
            .expect("secret_material_governance entry");
        assert_eq!(
            entry.status, "typed_in_protocol",
            "SecretClass/SecretReference must stay canonical typed law in cynic-protocol"
        );
    }

    #[test]
    fn publication_disclosure_is_now_typed_law() {
        let report = audit_policy_formality();
        let entry = report
            .policies
            .iter()
            .find(|entry| entry.policy == "publication_disclosure_consent")
            .expect("publication_disclosure_consent entry");
        assert_eq!(
            entry.status, "typed_in_protocol",
            "PublicationConsent/project_for_publication live in \
             cynic_protocol::publication_policy — reverting to prose would \
             reopen the disclosure leak risk"
        );
    }

    #[test]
    fn prose_debt_fails_the_report_status() {
        let report = audit_policy_formality();
        if report.summary.prose_only_count > 0 {
            assert_eq!(report.status, "formalization_debt");
        } else {
            assert_eq!(report.status, "ok");
        }
    }
}
