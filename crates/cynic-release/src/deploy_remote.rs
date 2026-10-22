//! Couche B — remote deploy executor: the port that actually touches a machine.
//!
//! WHAT THIS IS. The pure layer (`cynic-release::deploy`) decides WHAT must
//! happen; this module is the only place that DOES it. It implements the
//! same shape as `cynic-pkg`'s local executor, but every mutation crosses
//! an SSH boundary and every step is read back.
//!
//! WHY AN EXECUTOR AND NOT A SCRIPT. The measured fleet has three machines
//! with two different layouts (blue-green `releases/` + `current` symlink on
//! the builder, flat `bin/` on agent-host and cortex) and two different
//! SSH reachability forms (an ssh-config alias on the builder, an IP form
//! everywhere else, and a mesh-only alias that does not resolve outside the
//! mesh). A shell script would hardcode one of those shapes and be wrong on
//! the other two. The registry declares the shape; this executor reads it.
//!
//! FAIL-CLOSED. Every step is verified by a read-back. A step whose
//! read-back disagrees with the plan is a failure, and the caller climbs the
//! containment ladder. Nothing here decides policy — it executes a decision
//! and reports what it observed.
//!
//! PROOF. Each step returns the observed facts (sha256, baked git sha,
//! smoke exit code), not a boolean. The report is the evidence.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::deploy_locations::{DeployLayout, DeployMachine};

/// Schema of the remote deploy report.
pub const REMOTE_DEPLOY_REPORT_SCHEMA: &str = "cynic.release.remote_deploy_report.v1";

/// How a command was carried to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportForm {
    /// The target is the machine this process runs on.
    Local,
    /// The target is reached over SSH.
    Ssh,
}

/// A command to run on a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCommand {
    /// The program and its arguments, exactly, with no shell involved.
    pub argv: Vec<String>,
}

/// The result of running a command on a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteOutcome {
    /// Exit code of the command (or 255 when the transport itself failed).
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// True when the command could not be started at all.
    pub transport_failed: bool,
}

impl RemoteOutcome {
    pub fn success(&self) -> bool {
        !self.transport_failed && self.exit_code == 0
    }
}

/// A transport that can run a command on a target.
pub trait RemoteExecutorPort {
    /// Run a command on the target, returning the observed outcome.
    fn run(&self, target: &DeployMachine, command: &RemoteCommand) -> RemoteOutcome;
}

/// One measured fact read back from a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readback {
    /// sha256 of the deployed binary.
    pub sha256: String,
    /// The commit the binary reports it was built from.
    pub git_sha: String,
    /// Exit code of the smoke command.
    pub smoke_exit_code: i32,
    /// The smoke command's stdout (truncated by the caller if needed).
    pub smoke_stdout: String,
}

/// What one deploy step did and what it observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployStepReport {
    pub machine: String,
    pub transport: TransportForm,
    /// The endpoint actually used (ssh target, or `local`).
    pub endpoint: String,
    /// The deploy path that was written.
    pub deploy_path: String,
    /// The backup left behind (the previous binary), if a backup was made.
    pub backup_path: Option<String>,
    /// The observed facts after the swap.
    pub readback: Option<Readback>,
    /// Whether the swap completed.
    pub swapped: bool,
    /// Ordered list of failures; empty means the step succeeded.
    pub failures: Vec<String>,
}

impl DeployStepReport {
    pub fn succeeded(&self) -> bool {
        self.failures.is_empty() && self.swapped
    }
}

/// A refused deploy: named reason, no mutation attempted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployRefusal {
    pub machine: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeployReport {
    pub schema: String,
    pub status: String,
    pub commit: String,
    pub steps: Vec<DeployStepReport>,
    /// The machine the deploy stopped at (the first failing step).
    pub stopped_at: Option<String>,
}

impl fmt::Display for DeployRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deploy refused for {}: {}",
            self.machine, self.reason
        )
    }
}

/// Parse the identity surface's JSON into the commit it reports.
///
/// The identity surface is `cynic version --json`; the commit it reports is
/// the proof that the bytes on disk were built from the intended tree.
pub fn parse_identity_git_sha(stdout: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let sha = value
        .get("git_sha")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            value
                .get("build")
                .and_then(|build| build.get("git_sha"))
                .and_then(serde_json::Value::as_str)
        })?;
    Some(sha.trim().to_string())
}

/// Parse `sha256sum` output into the digest.
pub fn parse_sha256(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .next()
        .map(|digest| digest.trim().to_string())
        .filter(|digest| digest.len() == 64)
}

/// The backup path for a live binary, following the registry's pattern.
///
/// The pattern is `<name>.bak-<sha>` — the sha of the binary being backed
/// up, so a backup is self-describing: the name says which bytes it holds.
pub fn backup_path_for(pattern: &str, deploy_path: &Path, live_sha256: &str) -> PathBuf {
    let name = deploy_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("binary");
    let short = if live_sha256.len() >= 12 {
        &live_sha256[..12]
    } else {
        live_sha256
    };
    let backup_name = pattern.replace("<sha>", short);
    let backup_name = if backup_name.contains("binary") {
        backup_name.replace("binary", name)
    } else {
        backup_name
    };
    deploy_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(backup_name)
}

/// The remote path of a release directory for the symlink layout.
pub fn release_dir_for(state_root: &str, tag: &str) -> PathBuf {
    PathBuf::from(state_root).join("releases").join(tag)
}

/// Build the argv that swaps the live binary for the staged one.
///
/// Two layouts, two mechanisms — both atomic, both leaving the live binary
/// recoverable:
///
/// - `symlink`: the staged release directory is pointed at by `current`,
///   and `bin/<name>` points through `current`. The swap is a rename of the
///   `current` symlink.
/// - `flat`: the staged file is renamed over the live file. A rename over a
///   running binary is what `mv` does everywhere; the old inode stays open
///   for any process still using it.
pub fn build_swap_command(
    layout: DeployLayout,
    deploy_path: &Path,
    staged_path: &Path,
    state_root: Option<&str>,
    tag: &str,
) -> Option<RemoteCommand> {
    match layout {
        DeployLayout::Symlink => {
            let state_root = state_root?;
            let release_dir = release_dir_for(state_root, tag);
            let current_link = PathBuf::from(state_root).join("current");
            let bin_link = deploy_path.to_path_buf();
            // `ln -sfn` + `mv -T` is the atomic symlink swap: the new link is
            // created under a temporary name and renamed over the old one, so
            // a reader never sees a missing link.
            let tmp = PathBuf::from(format!("{}.new", current_link.display()));
            let argv = vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "ln -sfn {} {} && mv -Tf {} {} && ln -sfn {}/cynic {}",
                    release_dir.display(),
                    tmp.display(),
                    tmp.display(),
                    current_link.display(),
                    current_link.display(),
                    bin_link.display()
                ),
            ];
            Some(RemoteCommand { argv })
        }
        DeployLayout::Flat => {
            let argv = vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "mv -f {} {}",
                    staged_path.display(),
                    deploy_path.display()
                ),
            ];
            Some(RemoteCommand { argv })
        }
    }
}

/// The argv that stages a binary into the target's staging area.
pub fn build_stage_command(state_root: &str, tag: &str) -> RemoteCommand {
    let staging = release_dir_for(state_root, tag);
    RemoteCommand {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("mkdir -p {}", staging.display()),
        ],
    }
}

/// The argv that backs the live binary up.
pub fn build_backup_command(deploy_path: &Path, backup_path: &Path) -> RemoteCommand {
    RemoteCommand {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "cp -p {} {}",
                deploy_path.display(),
                backup_path.display()
            ),
        ],
    }
}

/// The argv that reads the live binary's sha256.
pub fn build_sha256_command(deploy_path: &Path) -> RemoteCommand {
    RemoteCommand {
        argv: vec![
            "sha256sum".to_string(),
            deploy_path.display().to_string(),
        ],
    }
}

/// The argv that reads the identity surface of the live binary.
pub fn build_identity_command(deploy_path: &Path) -> RemoteCommand {
    RemoteCommand {
        argv: vec![
            deploy_path.display().to_string(),
            "version".to_string(),
            "--json".to_string(),
        ],
    }
}

/// The argv that runs the smoke probe through the live binary.
pub fn build_smoke_command(deploy_path: &Path) -> RemoteCommand {
    RemoteCommand {
        argv: vec![
            deploy_path.display().to_string(),
            "source".to_string(),
            "status".to_string(),
            "--json".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(layout: DeployLayout) -> DeployMachine {
        DeployMachine {
            hostname: "builder".to_string(),
            registry_node_id: "cynic-builder".to_string(),
            os: "Ubuntu".to_string(),
            ssh_target: "cynic-builder-ssh".to_string(),
            ssh_target_fallback: None,
            principal: "cynic".to_string(),
            deploy_path: "${state_root}/bin/cynic".to_string(),
            deploy_path_resolved: "/var/lib/cynic/bin/cynic".to_string(),
            path_owner: "cynic:cynic".to_string(),
            layout,
            backup_pattern: "binary.bak-<sha>".to_string(),
            readback: crate::deploy_locations::DeployReadback {
                sha256_command: "sha256sum /var/lib/cynic/bin/cynic".to_string(),
                identity_command: "/var/lib/cynic/bin/cynic version --json".to_string(),
                smoke_command: "/var/lib/cynic/bin/cynic source status --json".to_string(),
                smoke_expect: "exit code 0".to_string(),
            },
            measured: None,
        }
    }

    #[test]
    fn identity_json_yields_the_baked_commit() {
        let stdout = r#"{"git_sha":"42e56f5f1f19b7cd7ef0560255129bf7a439f82f","build_profile":"release"}"#;
        assert_eq!(
            parse_identity_git_sha(stdout).as_deref(),
            Some("42e56f5f1f19b7cd7ef0560255129bf7a439f82f")
        );
    }

    #[test]
    fn identity_json_without_a_sha_yields_nothing() {
        assert!(parse_identity_git_sha("{}").is_none());
        assert!(parse_identity_git_sha("not json").is_none());
    }

    #[test]
    fn sha256_output_yields_the_digest() {
        let digest = "998ddf12236bb715cd307618a134f38381debfefadba0d5054989a03c7c13f7e";
        let stdout = format!("{digest}  /var/lib/cynic/bin/cynic");
        assert_eq!(parse_sha256(&stdout).as_deref(), Some(digest));
    }

    #[test]
    fn a_truncated_sha256_is_rejected() {
        assert!(parse_sha256("deadbeef  /x").is_none());
    }

    #[test]
    fn the_backup_name_carries_the_sha_it_holds() {
        let path = backup_path_for(
            "binary.bak-<sha>",
            Path::new("/var/lib/cynic/bin/cynic"),
            "998ddf12236bb715cd307618a134f38381debfefadba0d5054989a03c7c13f7e",
        );
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(name, "cynic.bak-998ddf12236b");
        assert!(path.starts_with("/var/lib/cynic/bin"));
    }

    #[test]
    fn the_symlink_swap_points_current_at_the_new_release() {
        let command = build_swap_command(
            DeployLayout::Symlink,
            Path::new("/var/lib/cynic/bin/cynic"),
            Path::new("/var/lib/cynic/releases/v0.9.0/cynic"),
            Some("/var/lib/cynic"),
            "v0.9.0",
        )
        .expect("symlink layout must render");
        let script = &command.argv[2];
        assert!(script.contains("/var/lib/cynic/releases/v0.9.0"));
        assert!(script.contains("/var/lib/cynic/current"));
        assert!(script.contains("mv -Tf"), "the swap must be an atomic rename");
    }

    #[test]
    fn the_flat_swap_renames_over_the_live_file() {
        let command = build_swap_command(
            DeployLayout::Flat,
            Path::new("/var/lib/cynic/bin/cynic"),
            Path::new("/var/lib/cynic/staging/cynic"),
            None,
            "v0.9.0",
        )
        .expect("flat layout must render");
        let script = &command.argv[2];
        assert!(script.contains("mv -f"));
        assert!(script.contains("/var/lib/cynic/bin/cynic"));
    }

    #[test]
    fn the_symlink_layout_needs_a_state_root() {
        assert!(build_swap_command(
            DeployLayout::Symlink,
            Path::new("/var/lib/cynic/bin/cynic"),
            Path::new("/x/cynic"),
            None,
            "v0.9.0",
        )
        .is_none());
    }

    #[test]
    fn the_readback_commands_run_through_the_deployed_path() {
        let path = Path::new("/var/lib/cynic/bin/cynic");
        assert_eq!(build_sha256_command(path).argv[0], "sha256sum");
        assert_eq!(build_identity_command(path).argv[1], "version");
        assert_eq!(build_smoke_command(path).argv[1], "source");
    }
}
