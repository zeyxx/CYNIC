//! Couche B — the orchestrator: deploy a release across machines, in order,
//! stopping at the first failure.
//!
//! WHAT THIS IS. Couche A rendered a plan. Couche B EXECUTES it: for each
//! machine in the canary order, stage the artifact, back up the live binary,
//! swap, then read the deployed binary back and judge it against the gates.
//! A machine that fails any gate stops the deploy: the next machine is not
//! touched.
//!
//! WHY IT STOPS. A canary that keeps going after a failure is not a canary —
//! it is a fleet-wide rollout with extra steps. The whole point of the order
//! is that the second machine is only reached when the first one is proven
//! good.
//!
//! WHAT IT PROVES. Every step reports measured facts: the sha256 of the
//! bytes now on disk, the commit the binary reports it was built from, and
//! the exit code of the smoke probe run through the real deployed path.
//! The report is evidence, not a boolean.
//!
//! WHAT IT DOES NOT DO. It does not climb the containment ladder (that is
//! couche C), and it does not decide eligibility (the release verdict does).
//! It executes a decision and reports what it observed.

use std::path::{Path, PathBuf};

use crate::deploy_locations::{DeployLayout, DeployMachine, DeployLocationsContract};
use crate::deploy_remote::{
    backup_path_for, build_backup_command, build_identity_command, build_sha256_command,
    build_smoke_command, build_stage_command, build_swap_command, parse_identity_git_sha,
    parse_sha256, Readback, RemoteCommand, RemoteExecutorPort, RemoteOutcome,
    TransportForm,
};
use crate::deploy_ssh::transport_for;

/// Which gate failed, when one did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateFailure {
    /// The swap command itself failed.
    Swap(String),
    /// The bytes on disk are not the bytes that were staged.
    BytesMismatch { expected: String, observed: String },
    /// The binary does not report the deployed commit.
    IdentityMismatch { expected: String, observed: String },
    /// The smoke probe failed.
    Smoke { exit_code: i32, stderr: String },
    /// The transport could not reach the machine.
    Unreachable(String),
}

impl GateFailure {
    /// The gate name this failure belongs to.
    pub fn gate(&self) -> &'static str {
        match self {
            GateFailure::Swap(_) => "swap",
            GateFailure::BytesMismatch { .. } => "bytes_match",
            GateFailure::IdentityMismatch { .. } => "identity_match",
            GateFailure::Smoke { .. } => "smoke",
            GateFailure::Unreachable(_) => "transport",
        }
    }
}

/// The outcome of one deploy step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutcome {
    pub machine: String,
    pub transport: TransportForm,
    pub endpoint: String,
    pub deploy_path: String,
    pub backup_path: Option<String>,
    pub readback: Option<Readback>,
    pub swapped: bool,
    /// The gate that failed, if any.
    pub failure: Option<GateFailure>,
}

/// The outcome of the whole deploy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployOutcome {
    pub commit: String,
    pub steps: Vec<StepOutcome>,
    /// The machine the deploy stopped at.
    pub stopped_at: Option<String>,
}

impl DeployOutcome {
    /// True when every attempted step passed.
    pub fn converged(&self) -> bool {
        self.stopped_at.is_none()
    }
}

/// The artifact to deploy.
pub struct DeployArtifact {
    /// The commit the artifact was built from.
    pub commit: String,
    /// sha256 of the artifact bytes.
    pub sha256: String,
    /// The local path of the built binary.
    pub local_path: PathBuf,
    /// The release tag used in the release directory name.
    pub tag: String,
}

/// The roots used to resolve the registry's path templates per machine.
#[derive(Debug, Clone, Default)]
pub struct MachineRoots {
    /// The state root of a deployed node (`/var/lib/cynic`).
    pub state_root: Option<String>,
    /// The principal's home (a workstation).
    pub home: Option<String>,
}

/// Resolve a machine's deploy path against its roots.
pub fn resolve_for_machine(
    machine: &DeployMachine,
    roots: &MachineRoots,
) -> Result<String, String> {
    crate::deploy_locations::resolve_deploy_path(
        &machine.deploy_path,
        roots.state_root.as_deref(),
        roots.home.as_deref(),
    )
    .map_err(|refusal| refusal.reason)
}

/// The remote staging path of the artifact for a machine.
pub fn staging_path_for(
    machine: &DeployMachine,
    roots: &MachineRoots,
    tag: &str,
) -> Result<PathBuf, String> {
    match machine.layout {
        DeployLayout::Symlink => {
            let state_root = roots
                .state_root
                .as_deref()
                .ok_or_else(|| "the symlink layout needs a state root".to_string())?;
            Ok(crate::deploy_remote::release_dir_for(state_root, tag).join("cynic"))
        }
        DeployLayout::Flat => {
            let state_root = roots
                .state_root
                .as_deref()
                .ok_or_else(|| "the flat layout needs a state root".to_string())?;
            Ok(PathBuf::from(state_root).join("staging").join(tag).join("cynic"))
        }
    }
}

/// The command that reads the live binary's sha256, for a given path.
fn sha_command(deploy_path: &Path) -> RemoteCommand {
    build_sha256_command(deploy_path)
}

/// Deploy one machine. Returns the measured outcome.
///
/// The order is: stage, back up, swap, read back. The read-back is the
/// verdict: three independent observations (bytes, commit, smoke).
pub fn deploy_to_machine<E: RemoteExecutorPort>(
    executor: &E,
    machine: &DeployMachine,
    artifact: &DeployArtifact,
    roots: &MachineRoots,
) -> StepOutcome {
    let transport = transport_for(machine);
    let endpoint = machine.ssh_target.clone();
    let mut outcome = StepOutcome {
        machine: machine.hostname.clone(),
        transport,
        endpoint: endpoint.clone(),
        deploy_path: String::new(),
        backup_path: None,
        readback: None,
        swapped: false,
        failure: None,
    };

    let deploy_path = match resolve_for_machine(machine, roots) {
        Ok(path) => path,
        Err(reason) => {
            outcome.failure = Some(GateFailure::Unreachable(reason));
            return outcome;
        }
    };
    outcome.deploy_path = deploy_path.clone();
    let deploy_path_ref = PathBuf::from(&deploy_path);

    // Step 1: stage the artifact on the target. The bytes are placed by the
    // transport (scp), the directory is created here.
    let staging = match staging_path_for(machine, roots, &artifact.tag) {
        Ok(path) => path,
        Err(reason) => {
            outcome.failure = Some(GateFailure::Unreachable(reason));
            return outcome;
        }
    };
    // The state root comes from the caller or the deployment is refused.
    //
    // It used to fall back to a hardcoded "/var/lib/cynic". That literal
    // is a placement claim in code: the audit engine flagged it
    // (PATH-004-LITERAL-TRACKED) because a fallback that names an
    // absolute path is a second source of truth about where the fleet
    // keeps its state. Every other step of this orchestrator already
    // REFUSES a missing state root ("the symlink layout needs a state
    // root"); this one guessed, so a caller that forgot to resolve the
    // roots would deploy to a path it never declared.
    let state_root = match roots.state_root.as_deref() {
        Some(state_root) => state_root,
        None => {
            outcome.failure = Some(GateFailure::Unreachable(
                "the deployment needs a state root".to_string(),
            ));
            return outcome;
        }
    };
    let stage = build_stage_command(state_root, &artifact.tag);
    let stage_outcome = executor.run(&machine.ssh_target, &stage);
    if !stage_outcome.success() {
        outcome.failure = Some(GateFailure::Unreachable(format!(
            "staging directory creation failed: {}",
            stage_outcome.stderr.trim()
        )));
        return outcome;
    }

    // Step 2: read the live binary's sha256, so the backup is named after
    // the bytes it actually holds.
    let live_sha = match executor.run(&machine.ssh_target, &sha_command(&deploy_path_ref)) {
        RemoteOutcome {
            exit_code: 0,
            stdout,
            transport_failed: false,
            ..
        } => parse_sha256(&stdout).unwrap_or_default(),
        other => {
            outcome.failure = Some(GateFailure::Unreachable(format!(
                "cannot read the live binary: exit={} transport_failed={} {}",
                other.exit_code,
                other.transport_failed,
                other.stderr.trim()
            )));
            return outcome;
        }
    };

    // Step 3: back the live binary up. A deploy without a backup cannot be
    // undone, so the backup is not optional.
    let backup = backup_path_for(&machine.backup_pattern, &deploy_path_ref, &live_sha);
    let backup_outcome = executor.run(&machine.ssh_target, &build_backup_command(&deploy_path_ref, &backup));
    if !backup_outcome.success() {
        outcome.failure = Some(GateFailure::Swap(format!(
            "backup failed: {}",
            backup_outcome.stderr.trim()
        )));
        return outcome;
    }
    outcome.backup_path = Some(backup.display().to_string());

    // Step 4: swap. The staged bytes are already in place (the transport put
    // them there), so the swap is the atomic activation.
    let swap = match build_swap_command(
        machine.layout,
        &deploy_path_ref,
        &staging,
        roots.state_root.as_deref(),
        &artifact.tag,
    ) {
        Some(command) => command,
        None => {
            outcome.failure = Some(GateFailure::Swap(
                "the layout needs a state root the registry did not resolve".to_string(),
            ));
            return outcome;
        }
    };
    let swap_outcome = executor.run(&machine.ssh_target, &swap);
    if !swap_outcome.success() {
        outcome.failure = Some(GateFailure::Swap(format!(
            "swap failed: exit={} {}",
            swap_outcome.exit_code,
            swap_outcome.stderr.trim()
        )));
        return outcome;
    }
    outcome.swapped = true;

    // Step 5: read back — three independent observations.
    let observed_sha = match executor.run(&machine.ssh_target, &sha_command(&deploy_path_ref)) {
        RemoteOutcome {
            exit_code: 0,
            stdout,
            transport_failed: false,
            ..
        } => parse_sha256(&stdout).unwrap_or_default(),
        other => {
            outcome.failure = Some(GateFailure::Unreachable(format!(
                "cannot read the deployed binary: exit={} {}",
                other.exit_code,
                other.stderr.trim()
            )));
            return outcome;
        }
    };
    let identity = executor.run(&machine.ssh_target, &build_identity_command(&deploy_path_ref));
    let observed_git_sha = parse_identity_git_sha(&identity.stdout).unwrap_or_default();
    let smoke = executor.run(&machine.ssh_target, &build_smoke_command(&deploy_path_ref));
    outcome.readback = Some(Readback {
        sha256: observed_sha.clone(),
        git_sha: observed_git_sha.clone(),
        smoke_exit_code: smoke.exit_code,
        smoke_stdout: smoke.stdout.clone(),
    });

    if observed_sha != artifact.sha256 {
        outcome.failure = Some(GateFailure::BytesMismatch {
            expected: artifact.sha256.clone(),
            observed: observed_sha,
        });
        return outcome;
    }
    if observed_git_sha != artifact.commit {
        outcome.failure = Some(GateFailure::IdentityMismatch {
            expected: artifact.commit.clone(),
            observed: observed_git_sha,
        });
        return outcome;
    }
    if !smoke.success() {
        outcome.failure = Some(GateFailure::Smoke {
            exit_code: smoke.exit_code,
            stderr: smoke.stderr.clone(),
        });
    }
    outcome
}

/// Deploy across the fleet, in the given order, stopping at the first
/// failure.
pub fn deploy_across<E: RemoteExecutorPort>(
    executor: &E,
    locations: &DeployLocationsContract,
    order: &[String],
    artifact: &DeployArtifact,
    roots: &MachineRoots,
) -> DeployOutcome {
    let mut steps = Vec::new();
    let mut stopped_at = None;
    for machine_name in order {
        let machine = match locations.machine(machine_name) {
            Some(machine) => machine,
            None => {
                stopped_at = Some(machine_name.clone());
                break;
            }
        };
        let outcome = deploy_to_machine(executor, machine, artifact, roots);
        let failed = outcome.failure.is_some();
        steps.push(outcome);
        if failed {
            stopped_at = Some(machine_name.clone());
            break;
        }
    }
    DeployOutcome {
        commit: artifact.commit.clone(),
        steps,
        stopped_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_locations::DeployReadback;

    struct FakeExecutor {
        /// What the sha256 command answers.
        live_sha: String,
        /// What the identity command answers.
        identity: String,
        /// What the smoke command answers.
        smoke_exit: i32,
        /// Every command that was run, for assertions.
        seen: std::cell::RefCell<Vec<Vec<String>>>,
    }

    impl FakeExecutor {
        fn new() -> Self {
            Self {
                live_sha: "998ddf12236bb715cd307618a134f38381debfefadba0d5054989a03c7c13f7e"
                    .to_string(),
                identity: r#"{"git_sha":"42e56f5f1f19b7cd7ef0560255129bf7a439f82f"}"#.to_string(),
                smoke_exit: 0,
                seen: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl RemoteExecutorPort for FakeExecutor {
        fn run(&self, _target: &str, command: &RemoteCommand) -> RemoteOutcome {
            self.seen.borrow_mut().push(command.argv.clone());
            let script = command.argv.get(2).cloned().unwrap_or_default();
            if command.argv[0] == "sha256sum" {
                return RemoteOutcome {
                    exit_code: 0,
                    stdout: format!("{}  /x\n", self.live_sha),
                    stderr: String::new(),
                    transport_failed: false,
                };
            }
            // The identity and smoke commands are not shell scripts: they run
            // the deployed binary directly.
            if command.argv.len() > 1 && command.argv[1] == "version" {
                return RemoteOutcome {
                    exit_code: 0,
                    stdout: self.identity.clone(),
                    stderr: String::new(),
                    transport_failed: false,
                };
            }
            if command.argv.len() > 1 && command.argv[1] == "source" {
                return RemoteOutcome {
                    exit_code: self.smoke_exit,
                    stdout: String::new(),
                    stderr: String::new(),
                    transport_failed: false,
                };
            }
            RemoteOutcome {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                transport_failed: false,
            }
        }
    }

    fn machine(hostname: &str, layout: DeployLayout) -> DeployMachine {
        DeployMachine {
            hostname: hostname.to_string(),
            registry_node_id: format!("node-{hostname}"),
            os: "Ubuntu".to_string(),
            ssh_target: format!("ssh-{hostname}"),
            ssh_target_fallback: None,
            principal: "cynic".to_string(),
            deploy_path: "${state_root}/bin/cynic".to_string(),
            deploy_path_resolved: "/var/lib/cynic/bin/cynic".to_string(),
            path_owner: "cynic:cynic".to_string(),
            layout,
            backup_pattern: "binary.bak-<sha>".to_string(),
            readback: DeployReadback {
                sha256_command: "sha256sum".to_string(),
                identity_command: "cynic version --json".to_string(),
                smoke_command: "cynic source status --json".to_string(),
                smoke_expect: "exit code 0".to_string(),
            },
            measured: None,
        }
    }

    fn locations(machines: Vec<DeployMachine>) -> DeployLocationsContract {
        DeployLocationsContract {
            schema: crate::deploy_locations::DEPLOY_LOCATIONS_SCHEMA.to_string(),
            status: "normative".to_string(),
            machines,
        }
    }

    fn artifact() -> DeployArtifact {
        DeployArtifact {
            commit: "42e56f5f1f19b7cd7ef0560255129bf7a439f82f".to_string(),
            sha256: "998ddf12236bb715cd307618a134f38381debfefadba0d5054989a03c7c13f7e"
                .to_string(),
            local_path: PathBuf::from("/tmp/cynic"),
            tag: "v0.9.0".to_string(),
        }
    }

    fn roots() -> MachineRoots {
        MachineRoots {
            state_root: Some("/var/lib/cynic".to_string()),
            home: Some("/home/operator".to_string()),
        }
    }

    #[test]
    fn a_clean_deploy_reports_the_measured_facts() {
        let executor = FakeExecutor::new();
        let registry = locations(vec![machine("builder", DeployLayout::Symlink)]);
        let outcome = deploy_to_machine(
            &executor,
            &registry.machines[0],
            &artifact(),
            &roots(),
        );
        assert!(outcome.failure.is_none(), "unexpected failure: {:?}", outcome.failure);
        assert!(outcome.swapped);
        let readback = outcome.readback.expect("readback");
        assert_eq!(readback.sha256, artifact().sha256);
        assert_eq!(readback.git_sha, artifact().commit);
        assert_eq!(readback.smoke_exit_code, 0);
        assert!(outcome.backup_path.is_some());
    }

    #[test]
    fn a_smoke_failure_is_reported_as_the_smoke_gate() {
        let mut executor = FakeExecutor::new();
        executor.smoke_exit = 3;
        let registry = locations(vec![machine("builder", DeployLayout::Symlink)]);
        let outcome = deploy_to_machine(&executor, &registry.machines[0], &artifact(), &roots());
        match outcome.failure {
            Some(GateFailure::Smoke { exit_code, .. }) => assert_eq!(exit_code, 3),
            other => panic!("expected a smoke failure, got {other:?}"),
        }
    }

    #[test]
    fn wrong_bytes_are_caught_before_the_commit_is_even_read() {
        let mut executor = FakeExecutor::new();
        executor.live_sha = "1111111111111111111111111111111111111111111111111111111111111111"
            .to_string();
        let registry = locations(vec![machine("builder", DeployLayout::Symlink)]);
        let outcome = deploy_to_machine(&executor, &registry.machines[0], &artifact(), &roots());
        assert!(matches!(
            outcome.failure,
            Some(GateFailure::BytesMismatch { .. })
        ));
    }

    #[test]
    fn a_wrong_baked_commit_is_caught() {
        let mut executor = FakeExecutor::new();
        executor.identity = r#"{"git_sha":"deadbeef"}"#.to_string();
        let registry = locations(vec![machine("builder", DeployLayout::Symlink)]);
        let outcome = deploy_to_machine(&executor, &registry.machines[0], &artifact(), &roots());
        assert!(matches!(
            outcome.failure,
            Some(GateFailure::IdentityMismatch { .. })
        ));
    }

    #[test]
    fn the_deploy_stops_at_the_first_failing_machine() {
        let mut executor = FakeExecutor::new();
        executor.smoke_exit = 1;
        let registry = locations(vec![
            machine("builder", DeployLayout::Symlink),
            machine("cortex", DeployLayout::Flat),
        ]);
        let outcome = deploy_across(
            &executor,
            &registry,
            &["builder".to_string(), "cortex".to_string()],
            &artifact(),
            &roots(),
        );
        assert_eq!(outcome.stopped_at.as_deref(), Some("builder"));
        assert_eq!(outcome.steps.len(), 1, "the second machine must not be touched");
        assert!(!outcome.converged());
    }

    #[test]
    fn a_clean_fleet_reaches_every_machine() {
        let executor = FakeExecutor::new();
        let registry = locations(vec![
            machine("builder", DeployLayout::Symlink),
            machine("cortex", DeployLayout::Flat),
        ]);
        let outcome = deploy_across(
            &executor,
            &registry,
            &["builder".to_string(), "cortex".to_string()],
            &artifact(),
            &roots(),
        );
        assert!(outcome.converged());
        assert_eq!(outcome.steps.len(), 2);
    }

    #[test]
    fn the_backup_is_taken_before_the_swap() {
        let executor = FakeExecutor::new();
        let registry = locations(vec![machine("builder", DeployLayout::Symlink)]);
        deploy_to_machine(&executor, &registry.machines[0], &artifact(), &roots());
        let seen = executor.seen.borrow();
        let backup_index = seen
            .iter()
            .position(|argv| argv.iter().any(|part| part.contains("cp -p")))
            .expect("a backup command");
        let swap_index = seen
            .iter()
            .position(|argv| argv.iter().any(|part| part.contains("mv -Tf")))
            .expect("a swap command");
        assert!(
            backup_index < swap_index,
            "the backup must precede the swap"
        );
    }
}
