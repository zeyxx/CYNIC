//! Governed source-material operations.
//!
//! Git is an adapter here, not an authority. The protocol crate owns the
//! classification and admission rule; this module owns worktree locking,
//! argv execution, and read-back evidence.

use std::path::{Path, PathBuf};

use cynic_protocol::{
    SecurityEvidenceState, SourceChangeAdmission, SourceChangeProposal, SourceMatterClass,
    SourceSnapshot, admit_source_change,
};
use serde::Serialize;

use crate::repo_access::current_repo_access_status;
use crate::source_lease::SourceLeaseVerifier;
use crate::update_lifecycle::{ArgvOutput, ArgvRunner};

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

pub fn observe_source_worktree(
    root: &Path,
    expected_owner: Option<&str>,
) -> SourceWorktreeObservation {
    let expected_owner = expected_owner
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("CYNIC_RUNTIME_PRINCIPAL").ok())
        .unwrap_or_else(|| "cynic".to_string());
    let root_metadata = std::fs::metadata(root).ok();
    let git_marker = root.join(".git");
    let git_dir = if git_marker.is_dir() {
        Some(git_marker.clone())
    } else {
        std::fs::read_to_string(&git_marker)
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
fn owner_group_mode(metadata: &std::fs::Metadata) -> (Option<String>, Option<String>, Option<u32>) {
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
    _metadata: &std::fs::Metadata,
) -> (Option<String>, Option<String>, Option<u32>) {
    (None, None, None)
}

#[cfg(unix)]
fn passwd_name(id: u32) -> Option<String> {
    std::fs::read_to_string("/etc/passwd")
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
    std::fs::read_to_string("/etc/group")
        .ok()?
        .lines()
        .find_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            (fields.len() > 2 && fields[2].parse::<u32>().ok() == Some(id))
                .then(|| fields[0].to_string())
        })
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

pub fn observe_source_push_target(repository_id: &str, remote: &str) -> SecurityEvidenceState {
    let report = current_repo_access_status();
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
        let stderr = output.stderr;
        SourceCommandProof {
            argv,
            success,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.spawn_error.map_or(stderr.clone(), |error| {
                if stderr.is_empty() {
                    error
                } else {
                    format!("{error}: {stderr}")
                }
            }),
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
        use std::fs::OpenOptions;
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
        let _ = std::fs::remove_file(&self.path);
    }
}

fn parse_snapshot(
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
mod tests {
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
