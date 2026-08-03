use crate::update_lifecycle::ArgvRunner;

use super::types::{FrameworkPropagationObservation, FrameworkPropagationProbe};

const RUNTIME_MARKER: &str = "/var/lib/cynic/runtime/current/.cynic-runtime-release.json";
const NODE_OBSERVATION: &str = "/var/lib/cynic/bootstrap/node-registry-observation.json";

pub(super) fn observe_framework_propagation(
    ssh_target: &str,
    runner: &dyn ArgvRunner,
) -> FrameworkPropagationObservation {
    let mut probes = Vec::new();
    let ssh = run_probe(
        "ssh_authentication",
        ssh_argv(ssh_target, &["true"]),
        runner,
    );
    let ssh_authenticated = ssh.status == "observed";
    probes.push(ssh);
    if !ssh_authenticated {
        return FrameworkPropagationObservation {
            schema: "cynic.framework_bootstrap.propagation_observation.v1",
            status: "blocked",
            ssh_authenticated: false,
            remote_principal_status: "unknown",
            remote_principal: None,
            runtime_marker_status: "unknown",
            runtime_marker: None,
            node_observation_status: "unknown",
            node_observation: None,
            probes,
            unknowns: vec![
                "runtime_marker_unobservable_without_ssh_authentication".to_string(),
                "node_observation_unobservable_without_ssh_authentication".to_string(),
            ],
        };
    }

    let principal_argv = ssh_argv(ssh_target, &["id", "-un"]);
    let principal_output = runner.run(
        &principal_argv
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let remote_principal = principal_output
        .success()
        .then(|| principal_output.stdout.trim().to_string())
        .filter(|value| !value.is_empty());
    let remote_principal_status = if remote_principal.is_some() {
        "observed"
    } else {
        "unknown"
    };
    probes.push(FrameworkPropagationProbe {
        phase: "remote_principal",
        argv: principal_argv,
        status: remote_principal_status,
        exit_code: principal_output.exit_code,
        stderr: principal_output.stderr,
    });

    let (runtime_marker_status, runtime_marker, marker_probe) =
        observe_json("runtime_marker", ssh_target, RUNTIME_MARKER, runner);
    probes.push(marker_probe);
    let (node_observation_status, node_observation, node_probe) =
        observe_json("node_observation", ssh_target, NODE_OBSERVATION, runner);
    probes.push(node_probe);

    let mut unknowns = [
        (runtime_marker_status, "runtime_marker_invalid"),
        (node_observation_status, "node_observation_invalid"),
    ]
    .into_iter()
    .filter(|(status, _)| *status == "invalid")
    .map(|(_, unknown)| unknown.to_string())
    .collect::<Vec<_>>();
    if remote_principal.is_none() {
        unknowns.push("remote_enrollment_principal_unobserved".to_string());
    }

    FrameworkPropagationObservation {
        schema: "cynic.framework_bootstrap.propagation_observation.v1",
        status: if unknowns.is_empty() {
            "observed"
        } else {
            "unknown"
        },
        ssh_authenticated,
        remote_principal_status,
        remote_principal,
        runtime_marker_status,
        runtime_marker,
        node_observation_status,
        node_observation,
        probes,
        unknowns,
    }
}

fn observe_json(
    phase: &'static str,
    ssh_target: &str,
    path: &str,
    runner: &dyn ArgvRunner,
) -> (
    &'static str,
    Option<serde_json::Value>,
    FrameworkPropagationProbe,
) {
    let argv = ssh_argv(ssh_target, &["cat", path]);
    let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
    let status = if output.success() {
        "observed"
    } else {
        "absent"
    };
    let evidence = output
        .success()
        .then(|| serde_json::from_str::<serde_json::Value>(&output.stdout).ok())
        .flatten();
    let status = if output.success() && evidence.is_none() {
        "invalid"
    } else {
        status
    };
    let probe = FrameworkPropagationProbe {
        phase,
        argv,
        status,
        exit_code: output.exit_code,
        stderr: output.stderr,
    };
    (status, evidence, probe)
}

pub(super) fn ssh_argv(ssh_target: &str, remote_argv: &[&str]) -> Vec<String> {
    let mut argv = vec![
        "ssh".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        ssh_target.to_string(),
    ];
    argv.extend(remote_argv.iter().map(|argument| (*argument).to_string()));
    argv
}

fn run_probe(
    phase: &'static str,
    argv: Vec<String>,
    runner: &dyn ArgvRunner,
) -> FrameworkPropagationProbe {
    let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
    FrameworkPropagationProbe {
        phase,
        argv,
        status: if output.success() {
            "observed"
        } else {
            "unavailable"
        },
        exit_code: output.exit_code,
        stderr: output.stderr,
    }
}
