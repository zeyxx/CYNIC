//! Materialization boundary for agent-owned Git worktrees.
//!
//! A worktree is a disposable execution surface. The source repository and
//! Forge authority remain outside this module; mutation requires a verified
//! coordination lease and an explicitly requested apply.

use std::path::{Path, PathBuf};

use cynic_protocol::{LeaseGuard, source_worktree_lease_target};
use serde::Serialize;

use crate::source_control::{SourceWorktreeObservation, observe_source_worktree};
use crate::source_lease::SourceLeaseVerifier;
#[cfg(test)]
use crate::source_lease::UnavailableSourceLeaseVerifier;
use crate::update_lifecycle::ArgvRunner;

pub const SOURCE_WORKTREE_REPORT_SCHEMA: &str = "cynic.host.source_worktree_report.v1";

#[derive(Debug, Clone)]
pub struct SourceWorktreeMaterializationRequest {
    pub repository_id: String,
    pub worktree_id: String,
    pub source_root: PathBuf,
    pub target_root: PathBuf,
    pub base_ref: String,
    pub expected_owner: String,
    pub lease: LeaseGuard,
    pub apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceWorktreeMaterializationReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub repository_id: String,
    pub worktree_id: String,
    pub source: SourceWorktreeObservation,
    pub target: SourceWorktreeObservation,
    pub lease_target: String,
    pub lease_verified: bool,
    pub command: Option<SourceWorktreeCommandProof>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceWorktreeCommandProof {
    pub argv: Vec<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn materialize_source_worktree(
    request: &SourceWorktreeMaterializationRequest,
    runner: &impl ArgvRunner,
    verifier: &impl SourceLeaseVerifier,
) -> SourceWorktreeMaterializationReport {
    let source = observe_source_worktree(
        &request.source_root,
        Some(request.expected_owner.as_str()),
    );
    let target = observe_source_worktree(
        &request.target_root,
        Some(request.expected_owner.as_str()),
    );
    let lease_target = source_worktree_lease_target(&request.repository_id, &request.worktree_id);
    let mut blockers = Vec::new();
    if request.lease.target != lease_target {
        blockers.push("worktree_lease_target_mismatch".to_string());
    }
    if request.lease.agent_id.trim().is_empty() || request.lease.lease_id.trim().is_empty() {
        blockers.push("worktree_lease_identity_incomplete".to_string());
    }
    if request.lease.fencing_token == 0 {
        blockers.push("worktree_fencing_token_missing".to_string());
    }
    if !source.git_dir_exists {
        blockers.push("source_git_dir_missing".to_string());
    }
    if target.root_exists {
        blockers.push("target_worktree_already_exists".to_string());
    }
    let parent = request.target_root.parent().unwrap_or_else(|| Path::new("."));
    let parent_observation = observe_source_worktree(parent, Some(request.expected_owner.as_str()));
    if parent_observation.ownership_status != "matched" {
        blockers.push("target_parent_owner_mismatch".to_string());
    }
    if !parent_observation.declared_writable {
        blockers.push("target_parent_not_declared_writable".to_string());
    }
    let lease_verified = match verifier.verify(&request.lease) {
        Ok(()) => true,
        Err(error) => {
            blockers.push(error);
            false
        }
    };
    if !request.apply {
        blockers.push("explicit_apply_required".to_string());
    }
    if !blockers.is_empty() {
        return report(request, source, target, lease_target, lease_verified, None, blockers);
    }
    let source_text = request.source_root.to_string_lossy().to_string();
    let target_text = request.target_root.to_string_lossy().to_string();
    let argv = vec![
        "git".to_string(), "-C".to_string(), source_text,
        "worktree".to_string(), "add".to_string(), "--detach".to_string(),
        target_text, request.base_ref.clone(),
    ];
    let args = argv.iter().map(String::as_str).collect::<Vec<_>>();
    let output = runner.run(&args);
    let proof = SourceWorktreeCommandProof {
        argv, success: output.success(), exit_code: output.exit_code,
        stdout: output.stdout, stderr: output.spawn_error.unwrap_or(output.stderr),
    };
    let after = observe_source_worktree(&request.target_root, Some(request.expected_owner.as_str()));
    let mut blockers = Vec::new();
    if !proof.success { blockers.push("worktree_materialization_failed".to_string()); }
    if proof.success && after.status != "ready" { blockers.push("worktree_readback_unproven".to_string()); }
    report(request, source, after, lease_target, lease_verified, Some(proof), blockers)
}

fn report(
    request: &SourceWorktreeMaterializationRequest,
    source: SourceWorktreeObservation,
    target: SourceWorktreeObservation,
    lease_target: String,
    lease_verified: bool,
    command: Option<SourceWorktreeCommandProof>,
    blockers: Vec<String>,
) -> SourceWorktreeMaterializationReport {
    SourceWorktreeMaterializationReport {
        schema: SOURCE_WORKTREE_REPORT_SCHEMA,
        status: if blockers.is_empty() { "proved" } else if command.is_none() { "blocked" } else { "failed" },
        mutates: command.is_some(),
        repository_id: request.repository_id.clone(),
        worktree_id: request.worktree_id.clone(),
        source, target, lease_target, lease_verified, command, blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    struct Verified;
    impl SourceLeaseVerifier for Verified {
        fn verify(&self, _guard: &LeaseGuard) -> Result<(), String> { Ok(()) }
    }

    #[test]
    fn unavailable_lease_authority_blocks_materialization_before_git() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("worktree");
        std::fs::create_dir_all(source.join(".git")).unwrap();
        let request = SourceWorktreeMaterializationRequest {
            repository_id: "cynic".into(), worktree_id: "agent-a".into(),
            source_root: source, target_root: target, base_ref: "HEAD".into(),
            expected_owner: "user".into(),
            lease: LeaseGuard { agent_id: "agent-a".into(), target: "source-worktree:cynic:agent-a".into(), lease_id: "lease".into(), fencing_token: 1 },
            apply: true,
        };
        let runner = MapArgvRunner { responses: Vec::<(Vec<String>, ArgvOutput)>::new() };
        let report = materialize_source_worktree(
            &request,
            &runner,
            &UnavailableSourceLeaseVerifier::default(),
        );
        assert_eq!(report.status, "blocked");
        assert!(report.blockers.contains(&"coordination_lease_authority_unavailable".to_string()));
        assert!(report.command.is_none());
        let _ = Verified;
    }
}
