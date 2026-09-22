use serde::{Deserialize, Serialize};

use crate::{LeaseGuard, SecurityEvidenceState};

pub const SOURCE_ADMISSION_REPORT_SCHEMA: &str = "cynic.source.admission.v1";
pub const SOURCE_CHANGE_PROPOSAL_SCHEMA: &str = "cynic.source.change_proposal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMatterClass {
    Genome,
    DesiredState,
    Adapter,
    Evidence,
    GeneratedArtifact,
    Secret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeProposal {
    pub schema: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub base_commit: String,
    pub actor: String,
    pub node_id: String,
    pub intent: String,
    pub matter: SourceMatterClass,
    pub paths: Vec<String>,
    pub validation: SecurityEvidenceState,
    pub rollback: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl SourceChangeProposal {
    pub fn lease_guard(&self) -> LeaseGuard {
        LeaseGuard {
            agent_id: self.actor.clone(),
            target: source_worktree_lease_target(&self.repository_id, &self.worktree_id),
            lease_id: self.lease_id.clone(),
            fencing_token: self.fencing_token,
        }
    }
}

pub fn source_worktree_lease_target(repository_id: &str, worktree_id: &str) -> String {
    format!("source-worktree:{repository_id}:{worktree_id}")
}

fn is_sensitive_source_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut components = normalized.split('/').filter(|part| !part.is_empty());
    let Some(file_name) = components.next_back() else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();

    if components.any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "secrets" | ".secrets" | "private-keys"
        )
    }) {
        return true;
    }

    file_name == ".env"
        || file_name.starts_with(".env.")
        || matches!(
            file_name.as_str(),
            "credentials.json"
                | "credentials.toml"
                | "id_ed25519"
                | "id_rsa"
                | "token"
                | "token.txt"
        )
        || [".key", ".pem", ".p12", ".pfx"]
            .iter()
            .any(|suffix| file_name.ends_with(suffix))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeAdmission {
    pub schema: String,
    pub status: String,
    pub proposal_id: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn admit_source_change(proposal: &SourceChangeProposal) -> SourceChangeAdmission {
    let mut blockers = Vec::new();
    if proposal.schema != SOURCE_CHANGE_PROPOSAL_SCHEMA {
        blockers.push("proposal_schema_mismatch".to_string());
    }
    if proposal.proposal_id.trim().is_empty() {
        blockers.push("proposal_id_missing".to_string());
    }
    if proposal.repository_id.trim().is_empty() {
        blockers.push("repository_id_missing".to_string());
    }
    if proposal.worktree_id.trim().is_empty() {
        blockers.push("worktree_id_missing".to_string());
    }
    if proposal.lease_id.trim().is_empty() {
        blockers.push("worktree_lease_id_missing".to_string());
    }
    if proposal.fencing_token == 0 {
        blockers.push("worktree_fencing_token_missing".to_string());
    }
    if proposal.worktree_id.contains('/') || proposal.worktree_id.contains('\\') {
        blockers.push("worktree_id_not_path_safe".to_string());
    }
    if proposal.base_commit.trim().is_empty() {
        blockers.push("base_commit_missing".to_string());
    }
    if proposal.actor.trim().is_empty() || proposal.node_id.trim().is_empty() {
        blockers.push("change_identity_incomplete".to_string());
    }
    if proposal.intent.trim().is_empty() {
        blockers.push("change_intent_missing".to_string());
    }
    if proposal.paths.is_empty() {
        blockers.push("change_paths_missing".to_string());
    }
    if proposal.rollback.trim().is_empty() {
        blockers.push("rollback_intent_missing".to_string());
    }
    if proposal.validation != SecurityEvidenceState::Proven {
        blockers.push("validation_unproven".to_string());
    }
    // Source-governance: commit intent must follow the agent-source protocol.
    // `intent` is the commit subject; an agent that cannot express its change
    // as a Conventional Commit has not met the contract.
    let commit_blockers = crate::source_policy::commit_conforms(
        &proposal.intent,
        &crate::source_policy::CommitPolicy::default(),
    );
    blockers.extend(commit_blockers);
    match proposal.matter {
        SourceMatterClass::Secret => blockers.push("secret_matter_never_versioned".to_string()),
        SourceMatterClass::GeneratedArtifact => {
            blockers.push("generated_artifact_requires_release_store".to_string())
        }
        SourceMatterClass::Genome
        | SourceMatterClass::DesiredState
        | SourceMatterClass::Adapter
        | SourceMatterClass::Evidence => {}
    }
    for path in &proposal.paths {
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            blockers.push(format!("path_outside_repository:{path}"));
        }
        if is_sensitive_source_path(path) {
            blockers.push(format!("sensitive_path_not_versioned:{path}"));
        }
    }
    SourceChangeAdmission {
        schema: "cynic.source.change_admission.v1".to_string(),
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        }
        .to_string(),
        proposal_id: proposal.proposal_id.clone(),
        blockers,
        evidence_refs: proposal.evidence_refs.clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePromotionStage {
    Worktree,
    CommitCandidate,
    SovereignCommit,
    ReleaseSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub upstream: Option<String>,
    pub staged_paths: Vec<String>,
    pub unstaged_paths: Vec<String>,
    pub untracked_paths: Vec<String>,
    pub unmerged_paths: Vec<String>,
    pub validation: SecurityEvidenceState,
    pub sovereign_transport_identity: SecurityEvidenceState,
    pub sovereign_authentication: SecurityEvidenceState,
    pub remote_commit_matches: SecurityEvidenceState,
    pub build_input_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAdmissionReport {
    pub schema: String,
    pub status: String,
    pub target_stage: SourcePromotionStage,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn admit_source_stage(
    snapshot: &SourceSnapshot,
    target_stage: SourcePromotionStage,
) -> SourceAdmissionReport {
    let mut blockers = Vec::new();
    if snapshot.branch.as_deref().is_none_or(str::is_empty) {
        blockers.push("branch_identity_missing".to_string());
    }
    if snapshot.head_commit.as_deref().is_none_or(str::is_empty) {
        blockers.push("head_commit_missing".to_string());
    }
    if !snapshot.unmerged_paths.is_empty() {
        blockers.push("unmerged_paths_present".to_string());
    }

    match target_stage {
        SourcePromotionStage::Worktree => {}
        SourcePromotionStage::CommitCandidate => {
            if snapshot.staged_paths.is_empty() {
                blockers.push("explicit_staged_path_set_required".to_string());
            }
            if snapshot.validation != SecurityEvidenceState::Proven {
                blockers.push("validation_unproven".to_string());
            }
        }
        SourcePromotionStage::SovereignCommit => {
            if !snapshot.staged_paths.is_empty()
                || !snapshot.unstaged_paths.is_empty()
                || !snapshot.untracked_paths.is_empty()
            {
                blockers.push("worktree_not_clean".to_string());
            }
            if snapshot.validation != SecurityEvidenceState::Proven {
                blockers.push("validation_unproven".to_string());
            }
            if snapshot.sovereign_transport_identity != SecurityEvidenceState::Proven {
                blockers.push("sovereign_transport_identity_unproven".to_string());
            }
            if snapshot.sovereign_authentication != SecurityEvidenceState::Proven {
                blockers.push("sovereign_authentication_unproven".to_string());
            }
            if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
                blockers.push("remote_commit_match_unproven".to_string());
            }
            // Source-governance: an agent may only promote to a branch it owns
            // (`agent/<id>/<scope>`), never to a protected trunk directly.
            if let Some(branch) = snapshot.branch.as_deref() {
                let policy = crate::source_policy::BranchPolicy::default();
                let branch_blockers = crate::source_policy::branch_conforms(branch, &policy);
                blockers.extend(branch_blockers);
                if crate::source_policy::ref_is_protected(branch, &policy.protected_refs) {
                    blockers.push("sovereign_promotion_to_protected_ref".to_string());
                }
            } else {
                blockers.push("branch_identity_missing".to_string());
            }
        }
        SourcePromotionStage::ReleaseSource => {
            if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
                blockers.push("remote_commit_match_unproven".to_string());
            }
            if snapshot
                .build_input_digest
                .as_deref()
                .is_none_or(str::is_empty)
            {
                blockers.push("build_input_digest_missing".to_string());
            }
            if snapshot.evidence_refs.is_empty() {
                blockers.push("source_evidence_missing".to_string());
            }
        }
    }

    SourceAdmissionReport {
        schema: SOURCE_ADMISSION_REPORT_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        target_stage,
        blockers,
        evidence_refs: snapshot.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_worktree_cannot_be_declared_sovereign() {
        let snapshot = SourceSnapshot {
            branch: Some("feature".to_string()),
            head_commit: Some("abc123".to_string()),
            upstream: Some("origin/feature".to_string()),
            staged_paths: Vec::new(),
            unstaged_paths: vec!["src/lib.rs".to_string()],
            untracked_paths: vec!["local-proof.json".to_string()],
            unmerged_paths: Vec::new(),
            validation: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Absent,
            sovereign_authentication: SecurityEvidenceState::Unknown,
            remote_commit_matches: SecurityEvidenceState::Unknown,
            build_input_digest: None,
            evidence_refs: Vec::new(),
        };

        let report = admit_source_stage(&snapshot, SourcePromotionStage::SovereignCommit);

        assert_eq!(report.status, "blocked");
        assert!(report.blockers.contains(&"worktree_not_clean".to_string()));
        assert!(
            report
                .blockers
                .contains(&"sovereign_transport_identity_unproven".to_string())
        );
    }

    #[test]
    fn secrets_and_generated_artifacts_cannot_enter_source_history() {
        let proposal = SourceChangeProposal {
            schema: SOURCE_CHANGE_PROPOSAL_SCHEMA.to_string(),
            proposal_id: "p-1".to_string(),
            repository_id: "cynic".to_string(),
            worktree_id: "agent-a".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 1,
            base_commit: "abc123".to_string(),
            actor: "agent-a".to_string(),
            node_id: "node-a".to_string(),
            intent: "checkpoint".to_string(),
            matter: SourceMatterClass::GeneratedArtifact,
            paths: vec![".env".to_string(), "target/release/cynic".to_string()],
            validation: SecurityEvidenceState::Proven,
            rollback: "revert proposal p-1".to_string(),
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = admit_source_change(&proposal);
        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .iter()
                .any(|v| v == "sensitive_path_not_versioned:.env")
        );
        assert!(
            report
                .blockers
                .iter()
                .any(|v| v == "generated_artifact_requires_release_store")
        );
    }

    #[test]
    fn secret_handling_source_names_are_not_secret_material() {
        for path in [
            "crates/cynic-host/src/runtime_secret.rs",
            "crates/cynic-protocol/src/secret_material.rs",
            "crates/cynic-host/src/secret_lint",
            "crates/cynic-protocol/src/token_policy.rs",
        ] {
            assert!(
                !is_sensitive_source_path(path),
                "unexpected rejection: {path}"
            );
        }
    }

    #[test]
    fn secret_material_paths_are_rejected_structurally() {
        for path in [
            ".env.local",
            "config/credentials.json",
            "infra/secrets/runtime.toml",
            "keys/node.key",
            "keys/node.PEM",
            "private-keys/node.pub",
            "bootstrap/token.txt",
        ] {
            assert!(
                is_sensitive_source_path(path),
                "unexpected admission: {path}"
            );
        }
    }

    #[test]
    fn a_valid_source_change_is_admitted_before_git() {
        let proposal = SourceChangeProposal {
            schema: SOURCE_CHANGE_PROPOSAL_SCHEMA.to_string(),
            proposal_id: "p-2".to_string(),
            repository_id: "cynic".to_string(),
            worktree_id: "agent-a".to_string(),
            lease_id: "lease-2".to_string(),
            fencing_token: 2,
            base_commit: "abc123".to_string(),
            actor: "agent-a".to_string(),
            node_id: "node-a".to_string(),
            intent: "feat(network): add observation proof".to_string(),
            matter: SourceMatterClass::Adapter,
            paths: vec!["crates/cynic-node/src/observe.rs".to_string()],
            validation: SecurityEvidenceState::Proven,
            rollback: "revert commit p-2".to_string(),
            evidence_refs: vec!["conformance-1".to_string()],
        };
        let report = admit_source_change(&proposal);
        assert_eq!(report.status, "admitted");
        assert!(report.blockers.is_empty());
    }
}
