use serde::{Deserialize, Serialize};

use crate::{SecurityEvidenceState, SourceSnapshot};

pub const BUILD_INPUT_ADMISSION_SCHEMA: &str = "cynic.build.input_admission.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildInputAdmission {
    pub schema: String,
    pub status: String,
    pub source_commit: Option<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn admit_build_input(snapshot: &SourceSnapshot) -> BuildInputAdmission {
    let mut blockers = Vec::new();
    if snapshot.head_commit.as_deref().is_none_or(str::is_empty) {
        blockers.push("source_commit_missing".to_string());
    }
    if !snapshot.staged_paths.is_empty()
        || !snapshot.unstaged_paths.is_empty()
        || !snapshot.untracked_paths.is_empty()
    {
        blockers.push("source_worktree_not_clean".to_string());
    }
    if !snapshot.unmerged_paths.is_empty() {
        blockers.push("source_unmerged_paths_present".to_string());
    }
    if snapshot.validation != SecurityEvidenceState::Proven {
        blockers.push("source_validation_unproven".to_string());
    }
    if snapshot.sovereign_transport_identity != SecurityEvidenceState::Proven {
        blockers.push("source_transport_identity_unproven".to_string());
    }
    if snapshot.sovereign_authentication != SecurityEvidenceState::Proven {
        blockers.push("source_authentication_unproven".to_string());
    }
    if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
        blockers.push("source_remote_commit_match_unproven".to_string());
    }
    if snapshot.evidence_refs.is_empty() {
        blockers.push("source_evidence_missing".to_string());
    }
    BuildInputAdmission {
        schema: BUILD_INPUT_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() { "admitted" } else { "blocked" }.to_string(),
        source_commit: snapshot.head_commit.clone(),
        blockers,
        evidence_refs: snapshot.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_requires_post_push_source_proof() {
        let snapshot = SourceSnapshot {
            branch: Some("main".into()),
            head_commit: Some("abc".into()),
            upstream: Some("forgejo/main".into()),
            staged_paths: Vec::new(),
            unstaged_paths: Vec::new(),
            untracked_paths: Vec::new(),
            unmerged_paths: Vec::new(),
            validation: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Proven,
            sovereign_authentication: SecurityEvidenceState::Proven,
            remote_commit_matches: SecurityEvidenceState::Unknown,
            build_input_digest: None,
            evidence_refs: vec!["git_push_readback".into()],
        };
        let admission = admit_build_input(&snapshot);
        assert_eq!(admission.status, "blocked");
        assert!(admission.blockers.contains(&"source_remote_commit_match_unproven".into()));
    }
}
