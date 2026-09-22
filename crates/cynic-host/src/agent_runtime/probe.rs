use crate::{
    adapters::agent_access::observe_linux_uid, runtime_authority::default_runtime_authority_paths,
};
use cynic_lifecycle::ArgvRunner;

use super::{
    specs::RUNTIME_SPECS,
    types::{
        AGENT_RUNTIME_INVENTORY_SCHEMA, AgentRuntimeActivation, AgentRuntimeInstallChannel,
        AgentRuntimeInstallOwnership, AgentRuntimeInstallProvenance, AgentRuntimeInventoryReport,
        AgentRuntimeObservation, AgentRuntimeSupervision, CapabilityNode, CapabilityState,
        StatusProjection,
    },
};

/// Observe declaratively listed external agent CLIs without starting services
/// or accepting work. Command output is reduced to the first version line so
/// diagnostic output cannot become an accidental secret projection.
pub fn observe_agent_runtimes(runner: &dyn ArgvRunner) -> AgentRuntimeInventoryReport {
    let runtimes = RUNTIME_SPECS
        .iter()
        .map(|spec| {
            let version_output = runner.run(spec.version_argv);
            let binary_ready = version_output.success();
            let version = binary_ready.then(|| first_line(&version_output.stdout));
            let provenance = observe_binary_provenance(runner, spec.version_argv[0], binary_ready);
            let install_provenance =
                observe_install_provenance(runner, spec.version_argv[0], binary_ready);
            let mut graph = vec![CapabilityNode {
                id: "binary_identity",
                state: if binary_ready {
                    CapabilityState::Observed
                } else {
                    CapabilityState::Blocked
                },
                depends_on: Vec::new(),
                retained_capabilities: binary_ready
                    .then_some("read_only_audit")
                    .into_iter()
                    .collect(),
                reason: command_reason(&version_output, "version"),
            }];
            graph.push(provenance);
            graph.push(install_provenance_node(&install_provenance));
            graph.push(observe_declared_runtime_principal());
            let mut status_projection = Vec::new();
            let status_node = match (binary_ready, spec.status_argv) {
                (false, _) => CapabilityNode {
                    id: "runtime_status",
                    state: CapabilityState::Blocked,
                    depends_on: vec!["binary_identity"],
                    retained_capabilities: Vec::new(),
                    reason: "binary_identity_unobserved".into(),
                },
                (true, Some(argv)) => {
                    let status = runner.run(argv);
                    let node = CapabilityNode {
                        id: "runtime_status",
                        state: if status.success() {
                            CapabilityState::Observed
                        } else {
                            CapabilityState::Degraded
                        },
                        depends_on: vec!["binary_identity"],
                        retained_capabilities: if status.success() {
                            vec!["read_only_audit", "run_observation"]
                        } else {
                            vec!["read_only_audit"]
                        },
                        reason: command_reason(&status, "status"),
                    };
                    status_projection = project_status(spec.status_projection, &status.stdout);
                    node
                }
                (true, None) => CapabilityNode {
                    id: "runtime_status",
                    state: CapabilityState::Unknown,
                    depends_on: vec!["binary_identity"],
                    retained_capabilities: vec!["read_only_audit"],
                    reason: "no_read_only_status_adapter_declared".into(),
                },
            };
            graph.push(status_node);
            graph.extend(status_projection);
            graph.push(observe_supervisor(spec.status_projection, binary_ready));
            AgentRuntimeObservation {
                id: spec.id,
                mode: spec.mode,
                version,
                install_provenance,
                supervision: derive_supervision(&graph),
                graph,
                adoption: None,
            }
        })
        .collect();
    AgentRuntimeInventoryReport {
        schema: AGENT_RUNTIME_INVENTORY_SCHEMA,
        authority: "read_only_host_observation",
        runtimes,
    }
}

fn observe_supervisor(projection: StatusProjection, binary_ready: bool) -> CapabilityNode {
    match (projection, binary_ready) {
        (_, false) => CapabilityNode {
            id: "runtime_supervisor",
            state: CapabilityState::Blocked,
            depends_on: vec!["binary_identity"],
            retained_capabilities: Vec::new(),
            reason: "binary_identity_unobserved".into(),
        },
        (StatusProjection::Hermes, true) => CapabilityNode {
            id: "runtime_supervisor",
            state: CapabilityState::Unknown,
            depends_on: vec!["runtime_status"],
            retained_capabilities: Vec::new(),
            reason: "external_supervisor_kind_unclassified".into(),
        },
        (StatusProjection::None, true) => CapabilityNode {
            id: "runtime_supervisor",
            state: CapabilityState::Unknown,
            depends_on: vec!["binary_identity"],
            retained_capabilities: Vec::new(),
            reason: "no_supervisor_observation_adapter_declared".into(),
        },
    }
}

fn observe_install_provenance(
    runner: &dyn ArgvRunner,
    binary: &str,
    binary_ready: bool,
) -> AgentRuntimeInstallProvenance {
    if !binary_ready {
        return AgentRuntimeInstallProvenance {
            payload_owner: AgentRuntimeInstallOwnership::Unknown,
            install_channel: AgentRuntimeInstallChannel::Unknown,
            reason: "binary_identity_unobserved".into(),
        };
    }
    let located = runner.run(&["which", binary]);
    let resolved = located
        .success()
        .then(|| runner.run(&["readlink", "-f", first_line(&located.stdout).as_str()]))
        .filter(|output| output.success())
        .map(|output| first_line(&output.stdout));
    let owner = resolved.as_deref().and_then(|resolved| {
        let output = runner.run(&["stat", "--format=%u", resolved]);
        output.success().then(|| first_line(&output.stdout))
    });
    let expected = default_runtime_authority_paths().principal.runtime_user;
    let expected_uid = observe_linux_uid(&expected).ok().map(|uid| uid.to_string());
    let payload_owner = match (owner.as_deref(), expected_uid.as_deref()) {
        (Some(owner), Some(expected)) if owner == expected => {
            AgentRuntimeInstallOwnership::RuntimePrincipal
        }
        (Some(_), Some(_)) => AgentRuntimeInstallOwnership::ExternalPrincipal,
        _ => AgentRuntimeInstallOwnership::Unknown,
    };
    AgentRuntimeInstallProvenance {
        payload_owner,
        install_channel: classify_install_channel(resolved.as_deref()),
        reason: match payload_owner {
            AgentRuntimeInstallOwnership::RuntimePrincipal => {
                "resolved_payload_owner_matches_runtime_principal".into()
            }
            AgentRuntimeInstallOwnership::ExternalPrincipal => {
                "resolved_payload_owner_differs_from_runtime_principal".into()
            }
            AgentRuntimeInstallOwnership::Unknown => "payload_owner_unobserved".into(),
        },
    }
}

/// Classify the installation channel from the resolved binary path.
/// Rules (AGENTS.md §4 — detection before specialization):
///   /usr/bin | /usr/local/bin          → DistroManaged
///   ~/.cargo/bin | ~/.local/bin | ~/.rustup → Scripted
///   autre chemin hors système           → Unclassified
fn classify_install_channel(resolved: Option<&str>) -> AgentRuntimeInstallChannel {
    let Some(path) = resolved else {
        return AgentRuntimeInstallChannel::Unknown;
    };
    if path.starts_with("/usr/bin/") || path.starts_with("/usr/local/bin/") {
        AgentRuntimeInstallChannel::DistroManaged
    } else if path.contains("/.cargo/bin/")
        || path.contains("/.local/bin/")
        || path.contains("/.rustup/")
    {
        AgentRuntimeInstallChannel::Scripted
    } else if !path.starts_with('/') {
        // Relative path — likely a distributed/remote binary
        AgentRuntimeInstallChannel::Distributed
    } else {
        AgentRuntimeInstallChannel::Unclassified
    }
}

fn install_provenance_node(provenance: &AgentRuntimeInstallProvenance) -> CapabilityNode {
    CapabilityNode {
        id: "binary_install_provenance",
        state: match provenance.payload_owner {
            AgentRuntimeInstallOwnership::RuntimePrincipal => CapabilityState::Observed,
            AgentRuntimeInstallOwnership::ExternalPrincipal => CapabilityState::Degraded,
            AgentRuntimeInstallOwnership::Unknown => CapabilityState::Unknown,
        },
        depends_on: vec!["binary_provenance", "runtime_principal"],
        retained_capabilities: matches!(
            provenance.payload_owner,
            AgentRuntimeInstallOwnership::RuntimePrincipal
        )
        .then_some("runtime_payload_inventory")
        .into_iter()
        .collect(),
        reason: provenance.reason.clone(),
    }
}

fn observe_declared_runtime_principal() -> CapabilityNode {
    let principal = default_runtime_authority_paths().principal.runtime_user;
    match observe_linux_uid(&principal) {
        Ok(_) => CapabilityNode {
            id: "runtime_principal",
            state: CapabilityState::Observed,
            depends_on: vec!["binary_provenance"],
            retained_capabilities: Vec::new(),
            reason: "declared_runtime_principal_uid_observed".into(),
        },
        Err(_) => CapabilityNode {
            id: "runtime_principal",
            state: CapabilityState::Unknown,
            depends_on: vec!["binary_provenance"],
            retained_capabilities: Vec::new(),
            reason: "declared_runtime_principal_uid_unobserved".into(),
        },
    }
}

fn derive_supervision(graph: &[CapabilityNode]) -> AgentRuntimeSupervision {
    let binary = graph.iter().find(|node| node.id == "binary_identity");
    let runtime = graph.iter().find(|node| node.id == "runtime_status");
    let provenance = graph.iter().find(|node| node.id == "binary_provenance");
    let mut blockers = graph
        .iter()
        .filter(|node| {
            matches!(
                node.state,
                CapabilityState::Blocked | CapabilityState::Degraded
            )
        })
        .map(|node| format!("{}:{}", node.id, node.reason))
        .collect::<Vec<_>>();
    let activation = if binary.is_none_or(|node| matches!(node.state, CapabilityState::Blocked)) {
        AgentRuntimeActivation::Blocked
    } else if provenance.is_none_or(|node| !matches!(node.state, CapabilityState::Observed)) {
        AgentRuntimeActivation::Discovered
    } else if runtime.is_none_or(|node| !matches!(node.state, CapabilityState::Observed)) {
        // Binary + provenance both observed, but no read-only status adapter
        // available. Runtime is discoverable but not yet supervision-ready.
        AgentRuntimeActivation::AutonomousCandidate
    } else {
        // All capability nodes observed: binary identity, path digest,
        // install provenance, and runtime status are all confirmed.
        // Escalate to AutonomousCandidate so adoption.rs:apply_runtime_ownership
        // can push toward SupervisedReadOnly when gateway is ready.
        AgentRuntimeActivation::AutonomousCandidate
    };
    if !matches!(activation, AgentRuntimeActivation::SupervisedReadOnly) && blockers.is_empty() {
        blockers.push("supervision_proof_incomplete".into());
    }
    let retained_operations = match activation {
        AgentRuntimeActivation::Blocked => Vec::new(),
        AgentRuntimeActivation::Discovered => vec!["binary_inventory"],
        AgentRuntimeActivation::SupervisedReadOnly => vec!["read_only_audit", "run_observation"],
        AgentRuntimeActivation::InteractiveAdmitted => {
            vec!["read_only_audit", "human_approved_workload"]
        }
        AgentRuntimeActivation::AutonomousCandidate | AgentRuntimeActivation::AutonomousActive => {
            Vec::new()
        }
    };
    AgentRuntimeSupervision {
        activation,
        retained_operations,
        blocked_operations: vec![
            "workload_execution",
            "scheduler_authority",
            "completion_publication",
        ],
        blockers,
    }
}

fn observe_binary_provenance(
    runner: &dyn ArgvRunner,
    binary: &str,
    binary_ready: bool,
) -> CapabilityNode {
    if !binary_ready {
        return CapabilityNode {
            id: "binary_provenance",
            state: CapabilityState::Blocked,
            depends_on: vec!["binary_identity"],
            retained_capabilities: Vec::new(),
            reason: "binary_identity_unobserved".into(),
        };
    }
    let path = runner.run(&["which", binary]);
    let resolved = first_line(&path.stdout);
    let digest = path
        .success()
        .then(|| runner.run(&["sha256sum", resolved.as_str()]))
        .filter(|output| output.success());
    CapabilityNode {
        id: "binary_provenance",
        state: if digest.is_some() {
            CapabilityState::Observed
        } else {
            CapabilityState::Degraded
        },
        depends_on: vec!["binary_identity"],
        retained_capabilities: vec!["binary_inventory"],
        reason: if digest.is_some() {
            "resolved_path_and_digest_observed".into()
        } else {
            "binary_digest_unobserved".into()
        },
    }
}

fn project_status(projection: StatusProjection, status: &str) -> Vec<CapabilityNode> {
    match projection {
        StatusProjection::None => Vec::new(),
        StatusProjection::Hermes => hermes_status_graph(status),
    }
}

/// Redacts Hermes' human status output into a small, stable capability graph.
/// It ignores provider names, credentials, paths, models, URLs, account expiry,
/// and session content.
fn hermes_status_graph(status: &str) -> Vec<CapabilityNode> {
    vec![
        contains_node(
            status,
            "Terminal Backend",
            "terminal_backend",
            "bounded_terminal_observation",
        ),
        contains_node(
            status,
            "Sudo:         ✗ disabled",
            "privileged_terminal",
            "sudo_disabled",
        ),
        gateway_service_node(status),
        contains_node(
            status,
            "◆ Scheduled Jobs",
            "external_scheduler",
            "external_schedule_observation_only",
        ),
        contains_node(
            status,
            "◆ Sessions",
            "session_inventory",
            "session_observation",
        ),
        tool_gateway_node(status),
    ]
}

fn contains_node(
    status: &str,
    marker: &str,
    id: &'static str,
    retained: &'static str,
) -> CapabilityNode {
    let observed = status.contains(marker);
    CapabilityNode {
        id,
        state: if observed {
            CapabilityState::Observed
        } else {
            CapabilityState::Unknown
        },
        depends_on: vec!["runtime_status"],
        retained_capabilities: observed.then_some(retained).into_iter().collect(),
        reason: if observed {
            "status_field_observed".into()
        } else {
            "status_field_missing".into()
        },
    }
}

fn gateway_service_node(status: &str) -> CapabilityNode {
    let running = status.contains("Status:       ✓ running");
    let stopped = status.contains("Status:       ✗ stopped");
    CapabilityNode {
        id: "gateway_service",
        state: if running {
            CapabilityState::Observed
        } else if stopped {
            CapabilityState::Degraded
        } else {
            CapabilityState::Unknown
        },
        depends_on: vec!["runtime_status"],
        retained_capabilities: if running {
            vec!["agent_api_observation"]
        } else {
            vec!["local_cli_audit"]
        },
        reason: if running {
            "gateway_running".into()
        } else if stopped {
            "gateway_stopped".into()
        } else {
            "gateway_state_unobserved".into()
        },
    }
}

fn tool_gateway_node(status: &str) -> CapabilityNode {
    let unavailable =
        status.contains("no usable paid credits") || status.contains("tools is unavailable");
    CapabilityNode {
        id: "managed_tool_gateway",
        state: if unavailable {
            CapabilityState::Degraded
        } else {
            CapabilityState::Unknown
        },
        depends_on: vec!["runtime_status"],
        retained_capabilities: Vec::new(),
        reason: if unavailable {
            "managed_tools_unavailable".into()
        } else {
            "managed_tool_state_unobserved".into()
        },
    }
}

fn first_line(output: &str) -> String {
    output
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(256)
        .collect()
}

fn command_reason(output: &cynic_lifecycle::ArgvOutput, label: &str) -> String {
    if output.success() {
        format!("{label}_command_observed")
    } else if let Some(error) = &output.spawn_error {
        format!("{label}_spawn_error:{error}")
    } else {
        format!("{label}_exit_code:{:?}", output.exit_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    #[test]
    fn mercury_status_failure_degrades_without_promoting_execution() {
        let runner = MapArgvRunner {
            responses: vec![
                (
                    vec!["mercury".into(), "--version".into()],
                    ArgvOutput {
                        exit_code: Some(0),
                        stdout: "mercury 1.2.2\n".into(),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
                (
                    vec!["mercury".into(), "status".into()],
                    ArgvOutput {
                        exit_code: Some(1),
                        stdout: String::new(),
                        stderr: "offline".into(),
                        spawn_error: None,
                    },
                ),
            ],
        };
        let report = observe_agent_runtimes(&runner);
        let mercury = report
            .runtimes
            .iter()
            .find(|runtime| runtime.id == "mercury-agent")
            .expect("mercury runtime observed");
        assert_eq!(mercury.version.as_deref(), Some("mercury 1.2.2"));
        // Look the node up by id: indexing into graph[1] silently asserted
        // against binary_provenance (which retains binary_inventory) instead of
        // the status node, and any reordering of the graph would move it again.
        let status = mercury
            .graph
            .iter()
            .find(|node| node.id == "runtime_status")
            .expect("runtime_status node present");
        assert!(
            matches!(status.state, CapabilityState::Degraded),
            "status node: {status:?}"
        );
        // A failing status must not promote execution: only read-only audit
        // survives, never run_observation or workload capabilities.
        assert_eq!(status.retained_capabilities, vec!["read_only_audit"]);
    }
}
