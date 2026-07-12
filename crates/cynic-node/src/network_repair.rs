use chrono::Utc;
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

use crate::domain::node_observation::{
    NodeRuntimeAction, NodeRuntimeBlocker, NodeRuntimeRemediation, NodeRuntimeSeverity,
    NodeRuntimeStatus,
};
use crate::network_command_surface::{NetworkCommandSurface, argv_command, expectation};
use crate::network_root_policy::{NetworkRootPolicy, build_root_policy};
use crate::observe::NodeObservation;

#[derive(Debug, Clone)]
struct NetworkRepairRuntimeContext {
    route_command: String,
    network_actuator_bundle_dir: String,
    provider_fallback_bundle_dir: String,
    dns_bundle_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkRepairPlan {
    schema: &'static str,
    generated_at: String,
    dry_run: bool,
    mutates_host: bool,
    requires_root: bool,
    root_policy: Option<NetworkRootPolicy>,
    observation: RepairObservationSummary,
    steps: Vec<NetworkRepairStep>,
}

#[derive(Debug, Clone, Serialize)]
struct RepairObservationSummary {
    status: NodeRuntimeStatus,
    severity: NodeRuntimeSeverity,
    blockers: Vec<NodeRuntimeBlocker>,
}

#[derive(Debug, Clone, Serialize)]
struct NetworkRepairStep {
    action: NodeRuntimeAction,
    reason: &'static str,
    mutates_host: bool,
    requires_root: bool,
    canonical_surface: &'static str,
    fallback_surface: String,
    command_preview: Vec<NetworkCommandSurface>,
}

pub(crate) fn build_dry_run_plan(observation: &NodeObservation) -> NetworkRepairPlan {
    let steps = observation
        .remediation
        .iter()
        .map(|remediation| repair_step(observation, remediation))
        .collect::<Vec<_>>();

    NetworkRepairPlan {
        schema: "cynic.node_network_repair_plan.v1",
        generated_at: Utc::now().to_rfc3339(),
        dry_run: true,
        mutates_host: false,
        requires_root: steps.iter().any(|step| step.requires_root),
        root_policy: steps
            .iter()
            .any(|step| step.requires_root)
            .then(|| build_root_policy(observation)),
        observation: RepairObservationSummary {
            status: observation.status.clone(),
            severity: observation.severity.clone(),
            blockers: observation.blockers.clone(),
        },
        steps,
    }
}

fn repair_step(
    observation: &NodeObservation,
    remediation: &NodeRuntimeRemediation,
) -> NetworkRepairStep {
    NetworkRepairStep {
        action: remediation.action.clone(),
        reason: remediation.reason,
        mutates_host: remediation.mutates_host,
        requires_root: remediation.requires_root,
        canonical_surface: remediation.canonical_surface,
        fallback_surface: remediation.fallback_surface.clone(),
        command_preview: command_preview(observation, &remediation.action),
    }
}

fn command_preview(
    observation: &NodeObservation,
    action: &NodeRuntimeAction,
) -> Vec<NetworkCommandSurface> {
    let runtime_layout = current_runtime_layout();
    let context = NetworkRepairRuntimeContext {
        route_command: runtime_layout.capabilities().route_tool.command.clone(),
        network_actuator_bundle_dir: runtime_layout.network_actuator_bundle_dir.clone(),
        provider_fallback_bundle_dir: runtime_layout.provider_fallback_bundle_dir,
        dns_bundle_dir: runtime_layout.dns_bundle_dir,
    };
    command_preview_with_context(observation, action, &context)
}

fn command_preview_with_context(
    observation: &NodeObservation,
    action: &NodeRuntimeAction,
    context: &NetworkRepairRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    match action {
        NodeRuntimeAction::RestoreProviderFallbackRoute => observation
            .topology
            .provider_fallback_gateway
            .as_ref()
            .map(|gateway| {
                vec![argv_command(
                    "replace_provider_fallback_route",
                    vec![
                        context.route_command.clone(),
                        "route".to_string(),
                        "replace".to_string(),
                        observation.topology.provider_fallback_cidr.clone(),
                        "via".to_string(),
                        gateway.clone(),
                    ],
                )]
            })
            .unwrap_or_else(|| {
                vec![expectation(
                    "replace_provider_fallback_route",
                    format!(
                        "{} route replace {} via <registry.hypervisor.network.local>",
                        context.route_command, observation.topology.provider_fallback_cidr
                    ),
                )]
            }),
        NodeRuntimeAction::RestoreCriticalDns => vec![
            argv_command(
                "mesh_status",
                vec![
                    "cynic".to_string(),
                    "mesh".to_string(),
                    "status".to_string(),
                    "--json".to_string(),
                ],
            ),
            argv_command(
                "provider_fallback_hosts_bundle",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--".to_string(),
                    "network".to_string(),
                    "provider-fallback-hosts".to_string(),
                    "--json".to_string(),
                    "--registry".to_string(),
                    DEFAULT_REGISTRY_PATH.to_string(),
                    "--output-dir".to_string(),
                    context.provider_fallback_bundle_dir.clone(),
                ],
            ),
        ],
        NodeRuntimeAction::RestoreCanonicalDnsProjection => vec![
            argv_command(
                "dns_status",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "dns".to_string(),
                    "status".to_string(),
                    "--registry".to_string(),
                    DEFAULT_REGISTRY_PATH.to_string(),
                    "--resolve-dns".to_string(),
                ],
            ),
            argv_command(
                "dns_bundle",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "dns".to_string(),
                    "bundle".to_string(),
                    "--registry".to_string(),
                    DEFAULT_REGISTRY_PATH.to_string(),
                    "--output-dir".to_string(),
                    context.dns_bundle_dir.clone(),
                ],
            ),
            argv_command(
                "observe",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--".to_string(),
                    "observe".to_string(),
                    "--json".to_string(),
                ],
            ),
        ],
        NodeRuntimeAction::RestoreCriticalTcp => vec![
            argv_command(
                "fleet_status",
                vec![
                    "cynic".to_string(),
                    "fleet".to_string(),
                    "status".to_string(),
                    "--json".to_string(),
                ],
            ),
            expectation(
                "failed_tcp_probe_followup",
                "ssh/curl probe failed hosts from network.observation.services.tcp",
            ),
        ],
        NodeRuntimeAction::ActivateSynapseRoute => vec![
            argv_command(
                "observe",
                vec![
                    "cynic-node".to_string(),
                    "observe".to_string(),
                    "--json".to_string(),
                    "--fail-on-warning".to_string(),
                ],
            ),
            argv_command(
                "actuator_plan",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "actuator".to_string(),
                    "plan".to_string(),
                    "--bundle-dir".to_string(),
                    context.network_actuator_bundle_dir.clone(),
                ],
            ),
            argv_command(
                "build_actuator",
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-q".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--bin".to_string(),
                    "cynic-network-actuator".to_string(),
                ],
            ),
            argv_command(
                "bundle_actuator",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "actuator".to_string(),
                    "bundle".to_string(),
                    "--output-dir".to_string(),
                    context.network_actuator_bundle_dir.clone(),
                ],
            ),
        ],
        NodeRuntimeAction::ActivateWireguardMesh => vec![
            argv_command(
                "observe",
                vec![
                    "cynic-node".to_string(),
                    "observe".to_string(),
                    "--json".to_string(),
                    "--fail-on-warning".to_string(),
                ],
            ),
            argv_command(
                "actuator_plan",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "actuator".to_string(),
                    "plan".to_string(),
                    "--bundle-dir".to_string(),
                    context.network_actuator_bundle_dir.clone(),
                ],
            ),
            argv_command(
                "build_actuator",
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-q".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--bin".to_string(),
                    "cynic-network-actuator".to_string(),
                ],
            ),
            argv_command(
                "bundle_actuator",
                vec![
                    "cynic".to_string(),
                    "network".to_string(),
                    "actuator".to_string(),
                    "bundle".to_string(),
                    "--output-dir".to_string(),
                    context.network_actuator_bundle_dir.clone(),
                ],
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::node_observation::NodeRuntimeAction;
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_command_surface::{NetworkCommandInvocation, render_command};

    #[test]
    fn dry_run_plan_never_mutates_host_even_when_steps_would() {
        let observation = NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );

        let plan = build_dry_run_plan(&observation);

        assert!(plan.dry_run);
        assert!(!plan.mutates_host);
        assert!(plan.requires_root);
        assert!(plan.root_policy.is_some());
        assert_eq!(plan.steps.len(), 2);
        assert!(
            plan.steps
                .iter()
                .any(|step| step.action == NodeRuntimeAction::ActivateWireguardMesh)
        );
    }

    #[test]
    fn provider_fallback_preview_uses_injected_context() {
        let observation = NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: true,
                wireguard_active: true,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );
        let preview = command_preview_with_context(
            &observation,
            &NodeRuntimeAction::RestoreProviderFallbackRoute,
            &NetworkRepairRuntimeContext {
                route_command: "/sbin/ip".to_string(),
                network_actuator_bundle_dir: "/tmp/actuator".to_string(),
                provider_fallback_bundle_dir: "/tmp/provider".to_string(),
                dns_bundle_dir: "/tmp/dns".to_string(),
            },
        );

        assert_eq!(preview.len(), 1);
        match &preview[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/sbin/ip");
                assert!(argv.join(" ").contains("192.168.0.37"));
            }
            other => panic!(
                "unexpected invocation: {}",
                render_command(&NetworkCommandSurface {
                    role: "debug".to_string(),
                    invocation: other.clone(),
                })
            ),
        }
    }
}
