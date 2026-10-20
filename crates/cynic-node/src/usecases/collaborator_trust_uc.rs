//! Collaborator trust tier promotion/demotion usecase.
//!
//! WHY: OQ-4 policy (docs/design/COLLABORATOR_TRUST_PROMOTION.md) defines
//! promotion criteria for external project collaborators. This usecase
//! enforces those criteria by validating promotion/demotion requests against
//! the current collaborator registry and the trust-tier admission rules in
//! `cynic-vcs::external_source::admit_external_source_operation`.
//!
//! HOW: Pure logic in this module (no I/O). The caller provides the current
//! collaborator registry and external projects registry as typed values;
//! this module validates the request and returns an updated registry or
//! blockers. I/O (reading/writing infra/collaborators.json) lives in the
//! adapter or caller.
//!
//! NEGATIVE BOUNDARY: does not read or write files, does not perform network
//! I/O, does not spawn processes. Pure data in, pure data out.

use serde::{Deserialize, Serialize};

use cynic_vcs::external_source::admit_external_source_operation;

/// Schema identifier for collaborator promotion reports.
pub const COLLABORATOR_TRUST_SCHEMA: &str = "cynic.node.collaborator_trust.v1";

/// A collaborator entry from `infra/collaborators.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collaborator {
    pub collaborator_id: String,
    pub display_name: String,
    pub github_handle: String,
    pub trust_level: String,
    pub role: String,
    pub projects: Vec<String>,
    pub owns: Vec<String>,
    pub maintains: Vec<String>,
    pub notes: String,
}

/// External project entry from `infra/external-projects.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalProject {
    pub project_id: String,
    pub display_name: String,
    pub repo_url: String,
    pub default_branch: String,
    pub collaborator_ids: Vec<String>,
    pub mutation_policy: String,
    pub sensitivity: String,
}

/// Request to promote or demote a collaborator.
#[derive(Debug, Clone, PartialEq)]
pub struct TrustPromotionRequest {
    pub collaborator_id: String,
    pub target_tier: u8,
    pub operator_approval: bool,
    pub evidence_refs: Vec<String>,
}

/// Result of a trust promotion/demotion evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustPromotionReport {
    pub schema: &'static str,
    pub collaborator_id: String,
    pub action: TrustAction,
    pub target_tier: u8,
    pub admitted: bool,
    pub blockers: Vec<String>,
    pub updated_collaborator: Option<Collaborator>,
}

/// Whether this is a promotion or demotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustAction {
    Promote,
    Demote,
}

/// Evaluate a trust promotion/demotion request against the current registries.
///
/// Pure: no I/O. The caller is responsible for reading the registries and
/// writing the updated collaborator back.
///
/// # Arguments
/// * `collaborators` — current collaborator registry entries.
/// * `projects` — current external projects registry entries.
/// * `request` — the promotion/demotion request to evaluate.
/// * `operation` — the external source operation to check trust tier against
///   (e.g. "clone", "push", "create_pr").
///
/// # Returns
/// A `TrustPromotionReport` with `admitted: true` and the updated collaborator
/// if the request is valid, or `admitted: false` with blockers.
pub fn evaluate_trust_promotion(
    collaborators: &[Collaborator],
    projects: &[ExternalProject],
    request: &TrustPromotionRequest,
    operation: &str,
) -> TrustPromotionReport {
    let mut blockers = Vec::new();

    // Find the collaborator.
    let collaborator = collaborators
        .iter()
        .find(|c| c.collaborator_id == request.collaborator_id);

    let collaborator = match collaborator {
        Some(c) => c,
        None => {
            blockers.push(format!(
                "collaborator '{}' not found in registry",
                request.collaborator_id
            ));
            return TrustPromotionReport {
                schema: COLLABORATOR_TRUST_SCHEMA,
                collaborator_id: request.collaborator_id.clone(),
                action: if request.target_tier < tier_from_level(&collaborator_trust_level(
                    &request.collaborator_id,
                    collaborators,
                )) {
                    TrustAction::Promote
                } else {
                    TrustAction::Demote
                },
                target_tier: request.target_tier,
                admitted: false,
                blockers,
                updated_collaborator: None,
            };
        }
    };

    // Determine current tier from trust_level.
    let current_tier = tier_from_level(&collaborator.trust_level);

    // Validate target tier.
    if request.target_tier > 3 {
        blockers.push(format!(
            "target_tier {} is invalid (must be 1-3)",
            request.target_tier
        ));
    }

    // Operator approval is required for any tier change.
    if !request.operator_approval {
        blockers.push("operator approval required (human-in-the-loop)".to_string());
    }

    // Evidence refs required for tier changes.
    if request.evidence_refs.is_empty() {
        blockers.push("at least one evidence reference required".to_string());
    }

    // Check trust tier admission for the requested operation.
    // Find the first project this collaborator works on.
    let project = projects.iter().find(|p| {
        p.collaborator_ids
            .iter()
            .any(|id| id == &request.collaborator_id)
    });

    let (mutation_policy, sensitivity, project_id) = match project {
        Some(p) => (p.mutation_policy.as_str(), p.sensitivity.as_str(), p.project_id.as_str()),
        None => ("", "", ""),
    };

    let admission = admit_external_source_operation(
        project_id,
        mutation_policy,
        sensitivity,
        operation,
        request.target_tier,
    );

    if !admission.admitted {
        blockers.extend(admission.blockers.iter().cloned());
    }

    // Tier 2 → Tier 1: require at least 3 evidence refs (PR track record).
    if current_tier == 2 && request.target_tier == 1 && request.evidence_refs.len() < 3 {
        blockers.push(
            "tier 2 → tier 1 requires at least 3 evidence references (PR track record)"
                .to_string(),
        );
    }

    // Tier 3 → Tier 2: require at least 1 evidence ref (read-back proof).
    if current_tier == 3 && request.target_tier == 2 && request.evidence_refs.is_empty() {
        blockers.push(
            "tier 3 → tier 2 requires at least 1 evidence reference (read-back proof)"
                .to_string(),
        );
    }

    let admitted = blockers.is_empty();

    let updated_collaborator = if admitted {
        let new_level = level_from_tier(request.target_tier);
        Some(Collaborator {
            collaborator_id: collaborator.collaborator_id.clone(),
            display_name: collaborator.display_name.clone(),
            github_handle: collaborator.github_handle.clone(),
            trust_level: new_level,
            role: collaborator.role.clone(),
            projects: collaborator.projects.clone(),
            owns: collaborator.owns.clone(),
            maintains: collaborator.maintains.clone(),
            notes: collaborator.notes.clone(),
        })
    } else {
        None
    };

    TrustPromotionReport {
        schema: COLLABORATOR_TRUST_SCHEMA,
        collaborator_id: request.collaborator_id.clone(),
        action: if request.target_tier < current_tier {
            TrustAction::Promote
        } else {
            TrustAction::Demote
        },
        target_tier: request.target_tier,
        admitted,
        blockers,
        updated_collaborator,
    }
}

/// Convert a trust_level string to a numeric tier.
fn tier_from_level(level: &str) -> u8 {
    match level {
        "core" => 1,
        "trusted" => 2,
        "observed" => 3,
        _ => 3,
    }
}

/// Convert a numeric tier to a trust_level string.
fn level_from_tier(tier: u8) -> String {
    match tier {
        1 => "core".to_string(),
        2 => "trusted".to_string(),
        _ => "observed".to_string(),
    }
}

/// Get the trust level for a collaborator by id.
fn collaborator_trust_level(collaborator_id: &str, collaborators: &[Collaborator]) -> String {
    collaborators
        .iter()
        .find(|c| c.collaborator_id == collaborator_id)
        .map(|c| c.trust_level.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_collaborators() -> Vec<Collaborator> {
        vec![
            Collaborator {
                collaborator_id: "titouan".to_string(),
                display_name: "Titouan".to_string(),
                github_handle: "zeyxx".to_string(),
                trust_level: "core".to_string(),
                role: "maintainer".to_string(),
                projects: vec!["cynic".to_string(), "aegisscan".to_string()],
                owns: vec!["cynic".to_string()],
                maintains: vec![],
                notes: "".to_string(),
            },
            Collaborator {
                collaborator_id: "robinho100".to_string(),
                display_name: "Robinho".to_string(),
                github_handle: "Robinho100".to_string(),
                trust_level: "observed".to_string(),
                role: "observer".to_string(),
                projects: vec!["aegisscan".to_string()],
                owns: vec![],
                maintains: vec![],
                notes: "".to_string(),
            },
        ]
    }

    fn test_projects() -> Vec<ExternalProject> {
        vec![ExternalProject {
            project_id: "aegisscan".to_string(),
            display_name: "AegisScan".to_string(),
            repo_url: "https://example.com/aegisscan".to_string(),
            default_branch: "main".to_string(),
            collaborator_ids: vec!["robinho100".to_string(), "titouan".to_string()],
            mutation_policy: "branch_pr".to_string(),
            sensitivity: "public".to_string(),
        }]
    }

    #[test]
    fn promote_tier3_to_tier2_succeeds() {
        let req = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "clone");
        assert!(report.admitted, "expected admitted, got: {:?}", report.blockers);
        assert_eq!(report.updated_collaborator.as_ref().unwrap().trust_level, "trusted");
    }

    #[test]
    fn promote_tier2_to_tier1_requires_3_evidence_refs() {
        // First promote to tier 2.
        let collabs = test_collaborators();
        let promote_to_2 = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&collabs, &test_projects(), &promote_to_2, "clone");
        assert!(report.admitted);

        // Now try to promote to tier 1 with only 1 evidence ref — should fail.
        let mut updated = collabs.clone();
        updated[1].trust_level = "trusted".to_string();
        let promote_to_1 = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 1,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&updated, &test_projects(), &promote_to_1, "push");
        assert!(!report.admitted);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("3 evidence references")));
    }

    #[test]
    fn promote_tier2_to_tier1_with_3_refs_succeeds() {
        let collabs = test_collaborators();
        let promote_to_2 = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&collabs, &test_projects(), &promote_to_2, "clone");
        assert!(report.admitted);

        let mut updated = collabs.clone();
        updated[1].trust_level = "trusted".to_string();
        let promote_to_1 = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 1,
            operator_approval: true,
            evidence_refs: vec![
                "pr-1".to_string(),
                "pr-2".to_string(),
                "pr-3".to_string(),
            ],
        };
        let report = evaluate_trust_promotion(&updated, &test_projects(), &promote_to_1, "push");
        assert!(report.admitted, "expected admitted, got: {:?}", report.blockers);
    }

    #[test]
    fn missing_operator_approval_blocks() {
        let req = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: false,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "clone");
        assert!(!report.admitted);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("operator approval")));
    }

    #[test]
    fn missing_evidence_refs_blocks() {
        let req = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec![],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "clone");
        assert!(!report.admitted);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("evidence reference")));
    }

    #[test]
    fn invalid_target_tier_blocks() {
        let req = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 5,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "clone");
        assert!(!report.admitted);
        assert!(report.blockers.iter().any(|b| b.contains("invalid")));
    }

    #[test]
    fn unknown_collaborator_blocks() {
        let req = TrustPromotionRequest {
            collaborator_id: "unknown".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "clone");
        assert!(!report.admitted);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("not found")));
    }

    #[test]
    fn demote_tier2_to_tier3_succeeds() {
        let collabs = test_collaborators();
        // First promote to tier 2.
        let promote = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 2,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&collabs, &test_projects(), &promote, "clone");
        assert!(report.admitted);

        let mut updated = collabs.clone();
        updated[1].trust_level = "trusted".to_string();
        let demote = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 3,
            operator_approval: true,
            evidence_refs: vec!["incident-1".to_string()],
        };
        let report = evaluate_trust_promotion(&updated, &test_projects(), &demote, "clone");
        assert!(report.admitted, "expected admitted, got: {:?}", report.blockers);
        assert_eq!(report.updated_collaborator.as_ref().unwrap().trust_level, "observed");
    }

    #[test]
    fn trust_tier_admission_checked_for_operation() {
        // Tier 3 should not allow push.
        let req = TrustPromotionRequest {
            collaborator_id: "robinho100".to_string(),
            target_tier: 3,
            operator_approval: true,
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = evaluate_trust_promotion(&test_collaborators(), &test_projects(), &req, "push");
        assert!(!report.admitted);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("trust_tier 3")));
    }
}
