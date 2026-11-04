//! Layout-aware fleet canary deployment executor over `cynic-ssh` SSOT and
//! `ArgvRunner`.
//!
//! Zero direct OS calls: byte hashing is pure in-memory, and every local or
//! remote process invocation flows through `ArgvRunner` and `cynic-ssh`.

use std::path::{Path, PathBuf};

use cynic_protocol::argv::ArgvRunner;
use cynic_release::{
    backup_path_for, build_backup_argv, build_rollback_commands, evaluate_readback,
    identity_matches, parse_identity_git_sha, parse_sha256, plan_machine_deploy,
    DeployGateFailure, DeployLayout, DeployLocationsContract, DeployReadbackObservation,
    MachineDeploySpec, MachineRoots, REMOTE_DEPLOY_REPORT_SCHEMA,
};
use cynic_ssh::{
    scp_argv, RemoteCommand, RemoteExecutorPort, RemoteOutcome, SshRemoteExecutor, TransportForm,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Verified local binary artifact ready for canary deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedDeployArtifact {
    pub commit: String,
    pub sha256: String,
    pub local_path: PathBuf,
    pub tag: String,
}

/// Per-machine execution report emitted by `cynic build deploy --apply`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineDeployStepReport {
    pub machine: String,
    pub registry_node_id: String,
    pub layout: DeployLayout,
    pub transport: TransportForm,
    pub endpoint: String,
    pub deploy_path: String,
    pub backup_path: Option<String>,
    pub readback: Option<DeployReadbackObservation>,
    pub swapped: bool,
    pub rolled_back: bool,
    pub failure: Option<DeployGateFailure>,
}

impl MachineDeployStepReport {
    pub fn succeeded(&self) -> bool {
        self.swapped && self.failure.is_none()
    }
}

/// Fleet-wide canary deploy report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeployReport {
    pub schema: String,
    pub status: String,
    pub commit: String,
    pub artifact_sha256: String,
    pub tag: String,
    pub steps: Vec<MachineDeployStepReport>,
    pub stopped_at: Option<String>,
}

impl RemoteDeployReport {
    pub fn converged(&self) -> bool {
        self.stopped_at.is_none() && self.steps.iter().all(MachineDeployStepReport::succeeded)
    }
}

/// Driven port carrying staged bytes and typed argv to one machine.
pub trait DeployTargetTransportPort {
    fn transport_form(&self) -> TransportForm;
    fn endpoint(&self) -> &str;
    fn stage_binary(
        &self,
        local_binary: &Path,
        staging_dir: &str,
        staged_binary_path: &str,
    ) -> Result<(), DeployGateFailure>;
    fn run_argv(&self, argv: &[String]) -> RemoteOutcome;
}

/// Local workstation transport (`ssh_target == "local"`).
pub struct LocalDeployTransport<'a, R: ArgvRunner> {
    runner: &'a R,
}

impl<'a, R: ArgvRunner> LocalDeployTransport<'a, R> {
    pub fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<R: ArgvRunner> DeployTargetTransportPort for LocalDeployTransport<'_, R> {
    fn transport_form(&self) -> TransportForm {
        TransportForm::Local
    }

    fn endpoint(&self) -> &str {
        "local"
    }

    fn stage_binary(
        &self,
        local_binary: &Path,
        staging_dir: &str,
        staged_binary_path: &str,
    ) -> Result<(), DeployGateFailure> {
        let mkdir_out = self.runner.run(&["mkdir", "-p", staging_dir]);
        if mkdir_out.exit_code != Some(0) || mkdir_out.spawn_error.is_some() {
            return Err(DeployGateFailure::Stage {
                detail: format!(
                    "local staging mkdir failed: {}",
                    mkdir_out.spawn_error.unwrap_or(mkdir_out.stderr).trim()
                ),
            });
        }
        let src = local_binary.display().to_string();
        let cp_out = self.runner.run(&["cp", "-f", &src, staged_binary_path]);
        if cp_out.exit_code != Some(0) || cp_out.spawn_error.is_some() {
            return Err(DeployGateFailure::Stage {
                detail: format!(
                    "local staging copy failed: {}",
                    cp_out.spawn_error.unwrap_or(cp_out.stderr).trim()
                ),
            });
        }
        Ok(())
    }

    fn run_argv(&self, argv: &[String]) -> RemoteOutcome {
        let borrowed: Vec<&str> = argv.iter().map(String::as_str).collect();
        let out = self.runner.run(&borrowed);
        if let Some(err) = out.spawn_error {
            return RemoteOutcome {
                exit_code: 255,
                stdout: out.stdout,
                stderr: err,
                transport_failed: true,
            };
        }
        RemoteOutcome {
            exit_code: out.exit_code.unwrap_or(255),
            stdout: out.stdout,
            stderr: out.stderr,
            transport_failed: false,
        }
    }
}

/// SSH transport backed exclusively by `cynic-ssh` with primary -> fallback failover.
pub struct SshDeployTransport<'a, R: ArgvRunner> {
    endpoint: String,
    runner: &'a R,
}

#[derive(Clone, Copy)]
struct BorrowedRunner<'a, R: ArgvRunner>(&'a R);

impl<R: ArgvRunner> ArgvRunner for BorrowedRunner<'_, R> {
    fn run(&self, argv: &[&str]) -> cynic_protocol::argv::ArgvOutput {
        self.0.run(argv)
    }
}

impl<'a, R: ArgvRunner> SshDeployTransport<'a, R> {
    pub fn connect(
        primary_endpoint: &str,
        fallback_endpoint: Option<&str>,
        runner: &'a R,
    ) -> Result<Self, DeployGateFailure> {
        let probe = RemoteCommand {
            argv: vec!["true".to_string()],
        };
        let primary_exec = SshRemoteExecutor::new(primary_endpoint, BorrowedRunner(runner));
        let first = primary_exec.run(primary_endpoint, &probe);
        if first.success() {
            return Ok(Self {
                endpoint: primary_endpoint.to_string(),
                runner,
            });
        }
        if let Some(fallback) = fallback_endpoint.filter(|f| !f.trim().is_empty()) {
            let fallback_exec = SshRemoteExecutor::new(fallback, BorrowedRunner(runner));
            let second = fallback_exec.run(fallback, &probe);
            if second.success() {
                return Ok(Self {
                    endpoint: fallback.to_string(),
                    runner,
                });
            }
            return Err(DeployGateFailure::Transport {
                detail: format!(
                    "primary {} ({}) and fallback {} ({}) unreachable",
                    primary_endpoint,
                    first.stderr.trim(),
                    fallback,
                    second.stderr.trim()
                ),
            });
        }
        Err(DeployGateFailure::Transport {
            detail: format!(
                "endpoint {} unreachable: {}",
                primary_endpoint,
                first.stderr.trim()
            ),
        })
    }
}

impl<R: ArgvRunner> DeployTargetTransportPort for SshDeployTransport<'_, R> {
    fn transport_form(&self) -> TransportForm {
        TransportForm::Ssh
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn stage_binary(
        &self,
        local_binary: &Path,
        staging_dir: &str,
        staged_binary_path: &str,
    ) -> Result<(), DeployGateFailure> {
        let mkdir_out = self.run_argv(&[
            "mkdir".to_string(),
            "-p".to_string(),
            staging_dir.to_string(),
        ]);
        if !mkdir_out.success() {
            return Err(DeployGateFailure::Stage {
                detail: format!("remote mkdir failed: {}", mkdir_out.stderr.trim()),
            });
        }
        let src = local_binary.display().to_string();
        let dest = format!("{}:{staged_binary_path}", self.endpoint);
        let scp = scp_argv(&src, &dest).map_err(|e| DeployGateFailure::Stage { detail: e })?;
        let borrowed: Vec<&str> = scp.iter().map(String::as_str).collect();
        let copy_out = self.runner.run(&borrowed);
        if copy_out.exit_code != Some(0) || copy_out.spawn_error.is_some() {
            return Err(DeployGateFailure::Stage {
                detail: format!(
                    "scp staging failed: {}",
                    copy_out.spawn_error.unwrap_or(copy_out.stderr).trim()
                ),
            });
        }
        Ok(())
    }

    fn run_argv(&self, argv: &[String]) -> RemoteOutcome {
        let exec = SshRemoteExecutor::new(&self.endpoint, BorrowedRunner(self.runner));
        exec.run(
            &self.endpoint,
            &RemoteCommand {
                argv: argv.to_vec(),
            },
        )
    }
}

/// Hash local artifact bytes and verify the binary's baked `git_sha` matches `commit`.
pub fn prepare_local_deploy_artifact<R: ArgvRunner>(
    local_path: &Path,
    binary_bytes: &[u8],
    commit: &str,
    tag: &str,
    runner: &R,
) -> Result<VerifiedDeployArtifact, DeployGateFailure> {
    if binary_bytes.is_empty() {
        return Err(DeployGateFailure::Refused {
            reason: "local binary is empty".to_string(),
        });
    }
    let sha256 = hex::encode(Sha256::digest(binary_bytes));
    let bin_str = local_path.display().to_string();
    let probe = runner.run(&[&bin_str, "version", "--json"]);
    if let Some(err) = probe.spawn_error {
        return Err(DeployGateFailure::Refused {
            reason: format!("cannot execute local binary identity probe: {err}"),
        });
    }
    if probe.exit_code != Some(0) {
        return Err(DeployGateFailure::Refused {
            reason: format!(
                "local binary identity probe failed (exit {:?}): {}",
                probe.exit_code,
                probe.stderr.trim()
            ),
        });
    }
    let observed_git_sha = parse_identity_git_sha(&probe.stdout).unwrap_or_default();
    if !identity_matches(&observed_git_sha, commit) {
        return Err(DeployGateFailure::IdentityMismatch {
            expected: commit.to_string(),
            observed: observed_git_sha,
        });
    }
    Ok(VerifiedDeployArtifact {
        commit: commit.to_string(),
        sha256,
        local_path: local_path.to_path_buf(),
        tag: tag.to_string(),
    })
}

fn run_rollback<T: DeployTargetTransportPort>(
    transport: &T,
    spec: &MachineDeploySpec,
    backup_path: &Path,
    roots: &MachineRoots,
    live_sha256: &str,
) -> bool {
    let commands = build_rollback_commands(spec, backup_path, roots, live_sha256);
    if commands.is_empty() {
        return false;
    }
    commands
        .iter()
        .all(|argv| transport.run_argv(argv).success())
}

/// Execute stage -> mandatory backup -> atomic swap -> 3 read-backs (+ rollback on failure)
/// on a single resolved machine target.
pub fn deploy_machine_with_transport<T: DeployTargetTransportPort>(
    transport: &T,
    spec: &MachineDeploySpec,
    artifact: &VerifiedDeployArtifact,
    roots: &MachineRoots,
) -> MachineDeployStepReport {
    let mut report = MachineDeployStepReport {
        machine: spec.machine.clone(),
        registry_node_id: spec.registry_node_id.clone(),
        layout: spec.layout,
        transport: transport.transport_form(),
        endpoint: transport.endpoint().to_string(),
        deploy_path: spec.deploy_path.clone(),
        backup_path: None,
        readback: None,
        swapped: false,
        rolled_back: false,
        failure: None,
    };

    if let Err(err) = transport.stage_binary(
        &artifact.local_path,
        &spec.staging_dir,
        &spec.staged_binary_path,
    ) {
        report.failure = Some(err);
        return report;
    }

    let live_sha_out = transport.run_argv(&spec.sha256_argv);
    let Some(live_sha) = live_sha_out
        .success()
        .then(|| parse_sha256(&live_sha_out.stdout))
        .flatten()
    else {
        report.failure = Some(DeployGateFailure::Backup {
            detail: format!(
                "cannot measure live binary digest before backup: {}",
                live_sha_out.stderr.trim()
            ),
        });
        return report;
    };

    let backup = backup_path_for(&spec.backup_pattern, Path::new(&spec.deploy_path), &live_sha);
    let backup_out = transport.run_argv(&build_backup_argv(&spec.deploy_path, &backup));
    if !backup_out.success() {
        report.failure = Some(DeployGateFailure::Backup {
            detail: format!("mandatory backup failed: {}", backup_out.stderr.trim()),
        });
        return report;
    }
    report.backup_path = Some(backup.display().to_string());

    for cmd in &spec.swap_commands {
        let out = transport.run_argv(cmd);
        if !out.success() {
            report.rolled_back = run_rollback(transport, spec, &backup, roots, &live_sha);
            report.failure = Some(DeployGateFailure::Swap {
                detail: format!("swap step {:?} failed: {}", cmd, out.stderr.trim()),
            });
            return report;
        }
    }
    report.swapped = true;

    let sha_after = transport.run_argv(&spec.sha256_argv);
    let id_after = transport.run_argv(&spec.identity_argv);
    let smoke_after = transport.run_argv(&spec.smoke_argv);

    match evaluate_readback(
        &artifact.sha256,
        &artifact.commit,
        &sha_after.stdout,
        &id_after.stdout,
        smoke_after.exit_code,
        &smoke_after.stdout,
        &smoke_after.stderr,
    ) {
        Ok(obs) => {
            report.readback = Some(obs);
        }
        Err(gate_failure) => {
            report.rolled_back = run_rollback(transport, spec, &backup, roots, &live_sha);
            report.failure = Some(gate_failure);
        }
    }
    report
}

/// Execute canary deployment across `machine_order`, halting immediately at the first failure.
pub fn execute_canary_deploy<R: ArgvRunner>(
    locations: &DeployLocationsContract,
    machine_order: &[String],
    artifact: &VerifiedDeployArtifact,
    roots: &MachineRoots,
    runner: &R,
) -> RemoteDeployReport {
    let mut steps = Vec::new();
    let mut stopped_at = None;

    for machine_key in machine_order {
        let spec = match plan_machine_deploy(
            locations,
            machine_key,
            &artifact.commit,
            &artifact.tag,
            roots,
        ) {
            Ok(spec) => spec,
            Err(refusal) => {
                steps.push(MachineDeployStepReport {
                    machine: refusal.machine.clone(),
                    registry_node_id: machine_key.clone(),
                    layout: DeployLayout::Flat,
                    transport: TransportForm::Ssh,
                    endpoint: String::new(),
                    deploy_path: String::new(),
                    backup_path: None,
                    readback: None,
                    swapped: false,
                    rolled_back: false,
                    failure: Some(DeployGateFailure::Refused {
                        reason: refusal.reason,
                    }),
                });
                stopped_at = Some(machine_key.clone());
                break;
            }
        };

        let step = if spec.primary_endpoint == "local" {
            let transport = LocalDeployTransport::new(runner);
            deploy_machine_with_transport(&transport, &spec, artifact, roots)
        } else {
            match SshDeployTransport::connect(
                &spec.primary_endpoint,
                spec.fallback_endpoint.as_deref(),
                runner,
            ) {
                Ok(transport) => deploy_machine_with_transport(&transport, &spec, artifact, roots),
                Err(err) => MachineDeployStepReport {
                    machine: spec.machine,
                    registry_node_id: spec.registry_node_id,
                    layout: spec.layout,
                    transport: TransportForm::Ssh,
                    endpoint: spec.primary_endpoint,
                    deploy_path: spec.deploy_path,
                    backup_path: None,
                    readback: None,
                    swapped: false,
                    rolled_back: false,
                    failure: Some(err),
                },
            }
        };

        let failed = step.failure.is_some();
        let halted_name = step.machine.clone();
        steps.push(step);
        if failed {
            stopped_at = Some(halted_name);
            break;
        }
    }

    let status = if stopped_at.is_none() {
        "converged"
    } else {
        "halted"
    };
    RemoteDeployReport {
        schema: REMOTE_DEPLOY_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        commit: artifact.commit.clone(),
        artifact_sha256: artifact.sha256.clone(),
        tag: artifact.tag.clone(),
        steps,
        stopped_at,
    }
}
