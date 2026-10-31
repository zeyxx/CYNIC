use std::path::{Path, PathBuf};
use std::fs;

use crate::ports::RepoAccessPort;
use crate::source_lease::SourceLeaseVerifier;
use cynic_process::{ArgvOutput, ArgvRunner};
use cynic_protocol::{LeaseGuard, SecurityEvidenceState};
use serde::{Deserialize, Serialize};

pub const SOURCE_ADMISSION_REPORT_SCHEMA: &str = "cynic.source.admission.v1";
pub const SOURCE_CHANGE_PROPOSAL_SCHEMA: &str = "cynic.source.change_proposal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMatterClass {
    Baseline,
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
        SourceMatterClass::Baseline
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

pub use cynic_protocol::vcs::SourceSnapshot;

// --- Pure source control data models (extracted from cynic-host) ---

pub const SOURCE_CONTROL_PROOF_SCHEMA: &str = "cynic.host.source_control_proof.v1";

#[derive(Debug, Clone, Serialize)]
pub struct SourceWorktreeObservation {
    pub schema: &'static str,
    pub root: String,
    pub root_exists: bool,
    pub git_dir: Option<String>,
    pub git_dir_exists: bool,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub expected_owner: String,
    pub ownership_status: &'static str,
    pub mode: Option<u32>,
    pub declared_writable: bool,
    pub write_capability_status: &'static str,
    pub status: &'static str,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceControlProof {
    pub schema: &'static str,
    pub status: &'static str,
    pub operation: &'static str,
    pub admission: SourceChangeAdmission,
    pub before: SourceSnapshot,
    pub after: Option<SourceSnapshot>,
    pub commands: Vec<SourceCommandProof>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceCommandProof {
    pub argv: Vec<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SourcePushAuthorization {
    pub remote_target: SecurityEvidenceState,
    pub transport_identity: SecurityEvidenceState,
    pub authentication: SecurityEvidenceState,
    pub remote_commit_matches: SecurityEvidenceState,
}

pub trait SourceControlPort {
    fn observe(&self) -> SourceSnapshot;
    fn admit(&self, proposal: &SourceChangeProposal) -> SourceChangeAdmission;
    fn commit(&self, proposal: &SourceChangeProposal) -> SourceControlProof;
    fn push(
        &self,
        proposal: &SourceChangeProposal,
        remote: &str,
        branch: &str,
        authorization: SourcePushAuthorization,
    ) -> SourceControlProof;
}

pub fn parse_snapshot(
    branch: &str,
    head: &str,
    upstream: Option<String>,
    status: &str,
) -> SourceSnapshot {
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();
    let mut unmerged = Vec::new();
    for line in status.lines() {
        if line.len() < 3 {
            continue;
        }
        let code = &line[..2];
        let path = line[3..].to_string();
        if code == "??" {
            untracked.push(path);
            continue;
        }
        if code.chars().next().is_some_and(|value| value != ' ') {
            staged.push(path.clone());
        }
        if code.chars().nth(1).is_some_and(|value| value != ' ') {
            unstaged.push(path.clone());
        }
        if code.contains('U') {
            unmerged.push(path);
        }
    }
    SourceSnapshot {
        branch: (!branch.is_empty()).then(|| branch.to_string()),
        head_commit: (!head.is_empty()).then(|| head.to_string()),
        upstream,
        staged_paths: staged,
        unstaged_paths: unstaged,
        untracked_paths: untracked,
        unmerged_paths: unmerged,
        validation: SecurityEvidenceState::Unknown,
        sovereign_transport_identity: SecurityEvidenceState::Unknown,
        sovereign_authentication: SecurityEvidenceState::Unknown,
        remote_commit_matches: SecurityEvidenceState::Unknown,
        build_input_digest: None,
        evidence_refs: Vec::new(),
    }
}

#[cfg(test)]
mod source_control_tests {
    use super::*;

    #[test]
    fn porcelain_snapshot_keeps_staged_and_untracked_distinct() {
        let snapshot = parse_snapshot(
            "main",
            "abc",
            Some("origin/main".into()),
            "M  staged.rs\n M unstaged.rs\n?? new.rs\nUU conflict.rs\n",
        );
        assert_eq!(snapshot.staged_paths, vec!["staged.rs", "conflict.rs"]);
        assert_eq!(snapshot.unstaged_paths, vec!["unstaged.rs", "conflict.rs"]);
        assert_eq!(snapshot.untracked_paths, vec!["new.rs"]);
        assert_eq!(snapshot.unmerged_paths, vec!["conflict.rs"]);
    }
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

pub fn observe_source_worktree(
    root: &Path,
    expected_owner: Option<&str>,
) -> SourceWorktreeObservation {
    let expected_owner = expected_owner
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("CYNIC_RUNTIME_PRINCIPAL").ok())
        .unwrap_or_else(|| "cynic".to_string());
    let root_metadata = fs::metadata(root).ok();
    let git_marker = root.join(".git");
    let git_dir = if git_marker.is_dir() {
        Some(git_marker.clone())
    } else {
        fs::read_to_string(&git_marker)
            .ok()
            .and_then(|value| {
                value
                    .trim()
                    .strip_prefix("gitdir:")
                    .map(|item| item.trim().to_string())
            })
            .map(|value| root.join(value))
    };
    let git_dir_exists = git_dir.as_ref().is_some_and(|path| path.is_dir());
    let (owner, group, mode) = root_metadata
        .as_ref()
        .map(owner_group_mode)
        .unwrap_or((None, None, None));
    let declared_writable = mode.is_some_and(|value| value & 0o222 != 0);
    let ownership_status = if owner.as_deref() == Some(expected_owner.as_str()) {
        "matched"
    } else if owner.is_some() {
        "mismatch"
    } else {
        "unknown"
    };
    let mut blockers = Vec::new();
    if !root.exists() {
        blockers.push("worktree_missing".to_string());
    }
    if !git_dir_exists {
        blockers.push("git_dir_missing".to_string());
    }
    if ownership_status != "matched" {
        blockers.push("runtime_owner_mismatch".to_string());
    }
    if !declared_writable {
        blockers.push("worktree_not_declared_writable".to_string());
    }
    SourceWorktreeObservation {
        schema: "cynic.host.source_worktree_observation.v1",
        root: root.display().to_string(),
        root_exists: root.exists(),
        git_dir: git_dir.map(|path| path.display().to_string()),
        git_dir_exists,
        owner,
        group,
        expected_owner,
        ownership_status,
        mode,
        declared_writable,
        // A managed sandbox can reject .git writes despite permissive mode bits.
        write_capability_status: "unknown",
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        },
        blockers,
    }
}

#[cfg(unix)]
fn owner_group_mode(metadata: &fs::Metadata) -> (Option<String>, Option<String>, Option<u32>) {
    use std::os::unix::fs::MetadataExt;
    let uid = metadata.uid();
    let gid = metadata.gid();
    (
        passwd_name(uid),
        group_name(gid),
        Some(metadata.mode() & 0o777),
    )
}

#[cfg(not(unix))]
fn owner_group_mode(
    _metadata: &fs::Metadata,
) -> (Option<String>, Option<String>, Option<u32>) {
    (None, None, None)
}

#[cfg(unix)]
fn passwd_name(id: u32) -> Option<String> {
    fs::read_to_string("/etc/passwd")
        .ok()?
        .lines()
        .find_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            (fields.len() > 2 && fields[2].parse::<u32>().ok() == Some(id))
                .then(|| fields[0].to_string())
        })
}

#[cfg(unix)]
fn group_name(id: u32) -> Option<String> {
    fs::read_to_string("/etc/group")
        .ok()?
        .lines()
        .find_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            (fields.len() > 2 && fields[2].parse::<u32>().ok() == Some(id))
                .then(|| fields[0].to_string())
        })
}

pub fn observe_source_push_target(
    port: &dyn RepoAccessPort,
    repository_id: &str,
    remote: &str,
) -> SecurityEvidenceState {
    let report = port.current_repo_access_status();
    report
        .sovereign_remotes
        .iter()
        .find(|candidate| {
            candidate.role == "sovereign_candidate"
                && candidate
                    .repository
                    .as_deref()
                    .is_some_and(|repository| repository.eq_ignore_ascii_case(repository_id))
                && candidate.desired_git_url.as_deref() == Some(remote)
        })
        .map(|candidate| {
            if candidate.desired_git_resolution_status == "resolved" {
                SecurityEvidenceState::Proven
            } else {
                SecurityEvidenceState::Unknown
            }
        })
        .unwrap_or(SecurityEvidenceState::Absent)
}

pub struct GitSourceControlAdapter<'a, R: ArgvRunner> {
    root: PathBuf,
    runner: &'a R,
    lease_verifier: &'a dyn SourceLeaseVerifier,
}

impl<'a, R: ArgvRunner> GitSourceControlAdapter<'a, R> {
    pub fn new(
        root: impl Into<PathBuf>,
        runner: &'a R,
        lease_verifier: &'a dyn SourceLeaseVerifier,
    ) -> Self {
        Self {
            root: root.into(),
            runner,
            lease_verifier,
        }
    }

    fn git(&self, args: &[&str]) -> (Vec<String>, ArgvOutput) {
        let root = self.root.to_string_lossy().to_string();
        let mut argv = vec!["git", "-C", root.as_str()];
        argv.extend(args.iter().copied());
        let output = self.runner.run(&argv);
        (argv.into_iter().map(str::to_string).collect(), output)
    }

    fn command_proof(argv: Vec<String>, output: ArgvOutput) -> SourceCommandProof {
        let success = output.success();
        SourceCommandProof {
            argv,
            success,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: stderr_with_spawn_error(output.stderr, output.spawn_error),
        }
    }

    fn lock(&self, worktree_id: &str) -> Result<SourceLock, String> {
        let git_dir = self
            .git_dir()
            .ok_or_else(|| "git_dir_unavailable".to_string())?;
        SourceLock::acquire(&git_dir.join(format!("cynic-source-control-{worktree_id}.lock")))
    }

    fn git_dir(&self) -> Option<PathBuf> {
        let (_, output) = self.git(&["rev-parse", "--git-dir"]);
        if !output.success() {
            return None;
        }
        let value = output.stdout.trim();
        let path = Path::new(value);
        Some(if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        })
    }
}

impl<'a, R: ArgvRunner> SourceControlPort for GitSourceControlAdapter<'a, R> {
    fn observe(&self) -> SourceSnapshot {
        let (_, branch) = self.git(&["branch", "--show-current"]);
        let (_, head) = self.git(&["rev-parse", "HEAD"]);
        let (_, upstream) =
            self.git(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
        let (_, status) = self.git(&["status", "--porcelain=v1"]);
        parse_snapshot(
            branch.stdout.trim(),
            head.stdout.trim(),
            upstream
                .success()
                .then(|| upstream.stdout.trim().to_string()),
            &status.stdout,
        )
    }

    fn admit(&self, proposal: &SourceChangeProposal) -> SourceChangeAdmission {
        admit_source_change(proposal)
    }

    fn commit(&self, proposal: &SourceChangeProposal) -> SourceControlProof {
        self.mutate(proposal, "commit", || {
            let (argv, output) = self.git(&["add", "--", &proposal.paths[0]]);
            // One command per path avoids shell expansion and keeps the staged set explicit.
            let mut commands = vec![Self::command_proof(argv, output)];
            for path in proposal.paths.iter().skip(1) {
                let (argv, output) = self.git(&["add", "--", path]);
                commands.push(Self::command_proof(argv, output));
            }
            let (argv, output) = self.git(&["commit", "-m", &proposal.intent]);
            commands.push(Self::command_proof(argv, output));
            commands
        })
    }

    fn push(
        &self,
        proposal: &SourceChangeProposal,
        remote: &str,
        branch: &str,
        authorization: SourcePushAuthorization,
    ) -> SourceControlProof {
        let before = self.observe();
        let admission = self.admit(proposal);
        let mut blockers = admission.blockers.clone();
        if authorization.remote_target != SecurityEvidenceState::Proven {
            blockers.push("source_remote_target_not_registry_admitted".to_string());
        }
        if remote.trim().is_empty() || branch.trim().is_empty() {
            blockers.push("push_target_incomplete".to_string());
        }
        if !blockers.is_empty() {
            return SourceControlProof {
                schema: SOURCE_CONTROL_PROOF_SCHEMA,
                status: "blocked",
                operation: "push",
                admission,
                before,
                after: None,
                commands: Vec::new(),
                blockers,
            };
        }

        let refspec = format!("refs/heads/{branch}");
        let (probe_argv, probe_output) = self.git(&["ls-remote", remote, refspec.as_str()]);
        let probe_command = Self::command_proof(probe_argv, probe_output);
        let mut commands = vec![probe_command.clone()];
        if !probe_command.success {
            blockers.push("sovereign_transport_or_authentication_unproven".to_string());
            return SourceControlProof {
                schema: SOURCE_CONTROL_PROOF_SCHEMA,
                status: "blocked",
                operation: "push",
                admission,
                before,
                after: None,
                commands,
                blockers,
            };
        }
        if proposal.base_commit != before.head_commit.clone().unwrap_or_default() {
            blockers.push("base_commit_mismatch".to_string());
        }
        if !before.unmerged_paths.is_empty() {
            blockers.push("unmerged_paths_present".to_string());
        }
        if proposal.fencing_token == 0 {
            blockers.push("worktree_fencing_token_missing".to_string());
        }
        if blockers.is_empty() {
            if let Err(blocker) = self.lease_verifier.verify(&proposal.lease_guard()) {
                blockers.push(blocker);
            }
        }
        let _lock = if blockers.is_empty() {
            self.lock(&proposal.worktree_id).ok()
        } else {
            None
        };
        if blockers.is_empty() && _lock.is_none() {
            blockers.push("source_lock_unavailable".to_string());
        }
        if !blockers.is_empty() {
            return SourceControlProof {
                schema: SOURCE_CONTROL_PROOF_SCHEMA,
                status: "blocked",
                operation: "push",
                admission,
                before,
                after: None,
                commands,
                blockers,
            };
        }

        let (push_argv, push_output) = self.git(&["push", remote, &format!("HEAD:{branch}")]);
        let push_command = Self::command_proof(push_argv, push_output);
        commands.push(push_command.clone());
        if !push_command.success {
            blockers.push("source_command_failed".to_string());
            return SourceControlProof {
                schema: SOURCE_CONTROL_PROOF_SCHEMA,
                status: "failed",
                operation: "push",
                admission,
                before,
                after: None,
                commands,
                blockers,
            };
        }

        let (readback_argv, readback_output) = self.git(&["ls-remote", remote, refspec.as_str()]);
        let readback_command = Self::command_proof(readback_argv, readback_output);
        let remote_head = readback_command.stdout.split_whitespace().next();
        let remote_matches = readback_command.success
            && before
                .head_commit
                .as_deref()
                .is_some_and(|head| remote_head == Some(head));
        commands.push(readback_command);
        if !remote_matches {
            blockers.push("remote_commit_match_unproven".to_string());
        }
        let mut after = self.observe();
        after.validation = proposal.validation;
        after.sovereign_transport_identity = SecurityEvidenceState::Proven;
        after.sovereign_authentication = SecurityEvidenceState::Proven;
        after.remote_commit_matches = if remote_matches {
            SecurityEvidenceState::Proven
        } else {
            SecurityEvidenceState::Unknown
        };
        after.evidence_refs.push("git_push_readback".to_string());
        SourceControlProof {
            schema: SOURCE_CONTROL_PROOF_SCHEMA,
            status: if blockers.is_empty() {
                "proved"
            } else {
                "failed"
            },
            operation: "push",
            admission,
            before,
            after: Some(after),
            commands,
            blockers,
        }
    }
}

impl<'a, R: ArgvRunner> GitSourceControlAdapter<'a, R> {
    fn mutate<F>(
        &self,
        proposal: &SourceChangeProposal,
        operation: &'static str,
        action: F,
    ) -> SourceControlProof
    where
        F: FnOnce() -> Vec<SourceCommandProof>,
    {
        let before = self.observe();
        let admission = self.admit(proposal);
        let mut blockers = admission.blockers.clone();
        if proposal.base_commit != before.head_commit.clone().unwrap_or_default() {
            blockers.push("base_commit_mismatch".to_string());
        }
        if before.unmerged_paths.len() > 0 {
            blockers.push("unmerged_paths_present".to_string());
        }
        if proposal.fencing_token == 0 {
            blockers.push("worktree_fencing_token_missing".to_string());
        }
        if blockers.is_empty() {
            if let Err(blocker) = self.lease_verifier.verify(&proposal.lease_guard()) {
                blockers.push(blocker);
            }
        }
        if proposal.matter == SourceMatterClass::Evidence
            && proposal.paths.iter().any(|p| p.ends_with(".json"))
        {
            // Evidence may be versioned, but only after validation; the protocol admission already enforces it.
        }
        let _lock = if blockers.is_empty() {
            self.lock(&proposal.worktree_id).ok()
        } else {
            None
        };
        if blockers.is_empty() && _lock.is_none() {
            blockers.push("source_lock_unavailable".to_string());
        }
        if !blockers.is_empty() {
            return SourceControlProof {
                schema: SOURCE_CONTROL_PROOF_SCHEMA,
                status: "blocked",
                operation,
                admission,
                before,
                after: None,
                commands: Vec::new(),
                blockers,
            };
        }
        let commands = action();
        let failed = commands.iter().any(|command| !command.success);
        let after = (!failed).then(|| self.observe());
        if failed {
            blockers.push("source_command_failed".to_string());
        }
        SourceControlProof {
            schema: SOURCE_CONTROL_PROOF_SCHEMA,
            status: if blockers.is_empty() {
                "proved"
            } else {
                "failed"
            },
            operation,
            admission,
            before,
            after,
            commands,
            blockers,
        }
    }
}

struct SourceLock {
    path: PathBuf,
}

impl SourceLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        use fs::OpenOptions;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map(|_| Self {
                path: path.to_path_buf(),
            })
            .map_err(|error| format!("source_lock:{error}"))
    }
}

impl Drop for SourceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn stderr_with_spawn_error(mut stderr: String, spawn_error: Option<String>) -> String {
    if let Some(error) = spawn_error {
        if stderr.is_empty() {
            stderr = error;
        } else {
            stderr = format!("{error}: {stderr}");
        }
    }
    stderr
}
