#![allow(clippy::expect_used)]

use std::cell::RefCell;
use std::path::Path;

use cynic_build::{
    execute_canary_deploy, prepare_local_deploy_artifact, RemoteDeployReport,
};
use cynic_protocol::argv::{ArgvOutput, ArgvRunner};
use cynic_release::{
    DeployGateFailure, DeployLayout, DeployLocationsContract, DeployMachine, DeployReadback,
    MachineRoots, DEPLOY_LOCATIONS_SCHEMA,
};
use sha2::{Digest, Sha256};

const COMMIT: &str = "6488860424bfc89602642fb8d8fca20d1805004d";
const OLD_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

struct FakeFleetRunner {
    artifact_digest: String,
    baked_commit: String,
    fail_primary_host: Option<String>,
    fail_smoke_on_first: bool,
    swapped: RefCell<bool>,
    seen: RefCell<Vec<Vec<String>>>,
}

impl FakeFleetRunner {
    fn new(artifact_bytes: &[u8]) -> Self {
        Self {
            artifact_digest: hex::encode(Sha256::digest(artifact_bytes)),
            baked_commit: COMMIT.to_string(),
            fail_primary_host: None,
            fail_smoke_on_first: false,
            swapped: RefCell::new(false),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl ArgvRunner for FakeFleetRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let owned: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
        self.seen.borrow_mut().push(owned.clone());

        if let Some(bad_host) = &self.fail_primary_host {
            if argv.first() == Some(&"ssh") && argv.contains(&bad_host.as_str()) {
                return ArgvOutput {
                    exit_code: Some(255),
                    stdout: String::new(),
                    stderr: "Name or service not known".to_string(),
                    spawn_error: None,
                };
            }
        }

        // Local binary identity probe: `<local_bin> version --json`
        if argv.len() == 3 && argv[1] == "version" && argv[2] == "--json" {
            return ArgvOutput {
                exit_code: Some(0),
                stdout: format!(r#"{{"git_sha":"{}"}}"#, self.baked_commit),
                stderr: String::new(),
                spawn_error: None,
            };
        }

        let tail: &[&str] = if argv.first() == Some(&"ssh") {
            let idx = argv
                .iter()
                .position(|a| !a.starts_with('-') && *a != "ssh" && !a.contains('='))
                .map(|i| i + 1)
                .unwrap_or(argv.len());
            &argv[idx..]
        } else {
            argv
        };

        if tail.first() == Some(&"mv") {
            *self.swapped.borrow_mut() = true;
        }
        if tail.first() == Some(&"sha256sum") {
            let digest = if *self.swapped.borrow() {
                &self.artifact_digest
            } else {
                OLD_DIGEST
            };
            return ArgvOutput {
                exit_code: Some(0),
                stdout: format!("{digest}  /srv/fleet-state/bin/cynic\n"),
                stderr: String::new(),
                spawn_error: None,
            };
        }
        if tail.len() >= 2 && tail[1] == "version" {
            return ArgvOutput {
                exit_code: Some(0),
                stdout: format!(r#"{{"git_sha":"{}"}}"#, self.baked_commit),
                stderr: String::new(),
                spawn_error: None,
            };
        }
        if tail.len() >= 2 && tail[1] == "source" {
            let code = if self.fail_smoke_on_first { 3 } else { 0 };
            return ArgvOutput {
                exit_code: Some(code),
                stdout: "{}".to_string(),
                stderr: if code == 0 {
                    String::new()
                } else {
                    "smoke probe failed".to_string()
                },
                spawn_error: None,
            };
        }

        ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        }
    }
}

fn fleet_contract() -> DeployLocationsContract {
    DeployLocationsContract {
        schema: DEPLOY_LOCATIONS_SCHEMA.to_string(),
        status: "normative".to_string(),
        machines: vec![
            DeployMachine {
                hostname: "agent-node".to_string(),
                registry_node_id: "cynic-agent-node".to_string(),
                os: "Ubuntu".to_string(),
                ssh_target: "unreachable.mesh.internal".to_string(),
                ssh_target_fallback: Some("cynic@10.10.10.102".to_string()),
                principal: "cynic".to_string(),
                deploy_path: "${state_root}/bin/cynic".to_string(),
                deploy_path_resolved: "/srv/fleet-state/bin/cynic".to_string(),
                path_owner: "cynic:cynic".to_string(),
                layout: DeployLayout::Flat,
                backup_pattern: "binary.bak-<sha>".to_string(),
                readback: DeployReadback {
                    sha256_command: "sha256sum ${state_root}/bin/cynic".to_string(),
                    identity_command: "${state_root}/bin/cynic version --json".to_string(),
                    smoke_command: "${state_root}/bin/cynic source status --json".to_string(),
                    smoke_expect: "exit code 0".to_string(),
                },
                measured: None,
            },
            DeployMachine {
                hostname: "builder".to_string(),
                registry_node_id: "cynic-builder".to_string(),
                os: "Ubuntu".to_string(),
                ssh_target: "cynic-builder-ssh".to_string(),
                ssh_target_fallback: None,
                principal: "cynic".to_string(),
                deploy_path: "${state_root}/bin/cynic".to_string(),
                deploy_path_resolved: "/srv/fleet-state/bin/cynic".to_string(),
                path_owner: "cynic:cynic".to_string(),
                layout: DeployLayout::Symlink,
                backup_pattern: "binary.bak-<sha>".to_string(),
                readback: DeployReadback {
                    sha256_command: "sha256sum ${state_root}/bin/cynic".to_string(),
                    identity_command: "${state_root}/bin/cynic version --json".to_string(),
                    smoke_command: "${state_root}/bin/cynic source status --json".to_string(),
                    smoke_expect: "exit code 0".to_string(),
                },
                measured: None,
            },
        ],
    }
}

fn roots() -> MachineRoots {
    MachineRoots {
        state_root: Some("/srv/fleet-state".to_string()),
        home: Some("/tmp/op-home".to_string()),
    }
}

#[test]
fn local_artifact_refuses_mismatched_baked_commit_before_any_remote_touch() {
    let bytes = b"ELF-synthetic-cynic-binary";
    let mut runner = FakeFleetRunner::new(bytes);
    runner.baked_commit = "0000000000000000000000000000000000000000".to_string();
    let err = prepare_local_deploy_artifact(
        Path::new("/tmp/cynic-bin"),
        bytes,
        COMMIT,
        "git-64888604",
        &runner,
    )
    .expect_err("mismatched baked commit must be refused");
    assert!(matches!(err, DeployGateFailure::IdentityMismatch { .. }));
}

#[test]
fn canary_deploy_fails_over_to_fallback_endpoint_and_converges_across_flat_and_symlink() {
    let bytes = b"ELF-synthetic-cynic-binary";
    let mut runner = FakeFleetRunner::new(bytes);
    runner.fail_primary_host = Some("unreachable.mesh.internal".to_string());

    let artifact = prepare_local_deploy_artifact(
        Path::new("/tmp/cynic-bin"),
        bytes,
        COMMIT,
        "git-64888604",
        &runner,
    )
    .expect("valid local artifact");

    let order = vec!["agent-node".to_string(), "builder".to_string()];
    let report: RemoteDeployReport =
        execute_canary_deploy(&fleet_contract(), &order, &artifact, &roots(), &runner);

    assert!(report.converged(), "report must converge: {report:?}");
    assert_eq!(report.steps.len(), 2);
    assert_eq!(report.steps[0].endpoint, "cynic@10.10.10.102");
    assert_eq!(
        report.steps[0].backup_path.as_deref(),
        Some("/srv/fleet-state/bin/cynic.bak-111111111111")
    );
    assert!(
        runner
            .seen
            .borrow()
            .iter()
            .any(|argv| argv.first().map(String::as_str) == Some("scp")),
        "staging must use scp_argv from cynic-ssh"
    );
}

#[test]
fn smoke_failure_rolls_back_first_machine_and_halts_canary() {
    let bytes = b"ELF-synthetic-cynic-binary";
    let mut runner = FakeFleetRunner::new(bytes);
    runner.fail_smoke_on_first = true;

    let artifact = prepare_local_deploy_artifact(
        Path::new("/tmp/cynic-bin"),
        bytes,
        COMMIT,
        "git-64888604",
        &runner,
    )
    .expect("valid local artifact");

    let order = vec!["agent-node".to_string(), "builder".to_string()];
    let report = execute_canary_deploy(&fleet_contract(), &order, &artifact, &roots(), &runner);

    assert!(!report.converged());
    assert_eq!(report.stopped_at.as_deref(), Some("agent-node"));
    assert_eq!(report.steps.len(), 1, "second machine must not be touched");
    assert!(
        report.steps[0].rolled_back,
        "failing smoke gate must roll back from mandatory backup"
    );
    assert!(matches!(
        report.steps[0].failure,
        Some(DeployGateFailure::Smoke { exit_code: 3, .. })
    ));
}
