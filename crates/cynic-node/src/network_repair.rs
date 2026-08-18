use chrono::Utc;
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
struct NetworkRepairContext {
    route_command: String,
    sudo_command: String,
    network_actuator_helper_path: String,
    provider_fallback_bundle_dir: String,
    dns_bundle_dir: String,
}

impl NetworkRepairContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        Self {
            route_command: runtime_layout.capabilities().route_tool.command.clone(),
            sudo_command: runtime_layout.sudo_bin(),
            network_actuator_helper_path: runtime_layout.network_actuator_helper_path(),
            provider_fallback_bundle_dir: runtime_layout.provider_fallback_bundle_dir,
            dns_bundle_dir: runtime_layout.dns_bundle_dir,
        }
    }
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
    build_dry_run_plan_with_context(observation, &NetworkRepairContext::from_current_runtime())
}

fn build_dry_run_plan_with_context(
    observation: &NodeObservation,
    context: &NetworkRepairContext,
) -> NetworkRepairPlan {
    let steps = observation
        .remediation
        .iter()
        .map(|remediation| repair_step_with_context(observation, remediation, context))
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

fn repair_step_with_context(
    observation: &NodeObservation,
    remediation: &NodeRuntimeRemediation,
    context: &NetworkRepairContext,
) -> NetworkRepairStep {
    NetworkRepairStep {
        action: remediation.action.clone(),
        reason: remediation.reason,
        mutates_host: remediation.mutates_host,
        requires_root: remediation.requires_root,
        canonical_surface: remediation.canonical_surface,
        fallback_surface: remediation.fallback_surface.clone(),
        command_preview: command_preview_with_context(observation, &remediation.action, context),
    }
}

fn command_preview_with_context(
    observation: &NodeObservation,
    action: &NodeRuntimeAction,
    context: &NetworkRepairContext,
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
            expectation(
                "mesh_status_readback",
                "read back mesh status through the installed node observer before DNS recovery",
            ),
            expectation(
                "provider_fallback_dns_projection",
                format!(
                    "activate the installed provider fallback DNS projection from {} through its bounded adapter",
                    context.provider_fallback_bundle_dir
                ),
            ),
        ],
        NodeRuntimeAction::RestoreCanonicalDnsProjection => vec![
            expectation(
                "canonical_dns_status_readback",
                "read back canonical DNS through the installed node observer before projection recovery",
            ),
            expectation(
                "canonical_dns_projection",
                format!(
                    "activate the installed canonical DNS projection from {} through its bounded adapter",
                    context.dns_bundle_dir
                ),
            ),
        ],
        NodeRuntimeAction::RestoreCriticalTcp => vec![
            expectation(
                "fleet_reachability_readback",
                "read back fleet reachability through the node control transport",
            ),
            expectation(
                "failed_tcp_probe_followup",
                "ssh/curl probe failed hosts from network.observation.services.tcp",
            ),
        ],
        NodeRuntimeAction::ActivateSynapseRoute => vec![
            argv_command(
                "apply_synapse_route",
                vec![
                    context.sudo_command.clone(),
                    "-n".to_string(),
                    context.network_actuator_helper_path.clone(),
                    "apply".to_string(),
                    "--profile".to_string(),
                    "synapse-route".to_string(),
                    "--apply".to_string(),
                ],
            ),
            expectation(
                "post_actuation_observation",
                "read back synapse route and mesh health",
            ),
        ],
        NodeRuntimeAction::ActivateWireguardMesh => vec![
            argv_command(
                "apply_wireguard_mesh",
                vec![
                    context.sudo_command.clone(),
                    "-n".to_string(),
                    context.network_actuator_helper_path.clone(),
                    "apply".to_string(),
                    "--profile".to_string(),
                    "wireguard-mesh".to_string(),
                    "--apply".to_string(),
                ],
            ),
            expectation(
                "post_actuation_observation",
                "read back WireGuard mesh health",
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

    fn portable_context() -> NetworkRepairContext {
        NetworkRepairContext {
            route_command: "/portable/bin/ip".to_string(),
            sudo_command: "/portable/bin/sudo".to_string(),
            network_actuator_helper_path: "/portable/libexec/cynic-network-actuator".to_string(),
            provider_fallback_bundle_dir: "/portable/runtime/provider-fallback".to_string(),
            dns_bundle_dir: "/portable/runtime/dns".to_string(),
        }
    }

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

        let plan = build_dry_run_plan_with_context(&observation, &portable_context());

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
            &portable_context(),
        );

        assert_eq!(preview.len(), 1);
        match &preview[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/portable/bin/ip");
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

    #[test]
    fn actuator_preview_uses_injected_bundle_directories() {
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
        let preview = command_preview_with_context(
            &observation,
            &NodeRuntimeAction::ActivateSynapseRoute,
            &portable_context(),
        );

        assert!(preview.iter().map(render_command).any(|item| {
            item.contains("/portable/bin/sudo")
                && item.contains("/portable/libexec/cynic-network-actuator")
        }));
        assert!(preview.iter().all(|item| {
            !render_command(item).contains("cargo") && !render_command(item).contains("cynic ")
        }));
    }
}
