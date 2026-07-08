use chrono::Utc;
use serde::Serialize;

use crate::domain::node_observation::{
    NodeRuntimeAction, NodeRuntimeBlocker, NodeRuntimeRemediation, NodeRuntimeSeverity,
    NodeRuntimeStatus,
};
use crate::network_root_policy::{NetworkRootPolicy, build_root_policy};
use crate::observe::NodeObservation;

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
    fallback_surface: &'static str,
    command_preview: Vec<String>,
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
        fallback_surface: remediation.fallback_surface,
        command_preview: command_preview(observation, &remediation.action),
    }
}

fn command_preview(observation: &NodeObservation, action: &NodeRuntimeAction) -> Vec<String> {
    match action {
        NodeRuntimeAction::RestoreProviderFallbackRoute => observation
            .topology
            .provider_fallback_gateway
            .as_ref()
            .map(|gateway| {
                vec![format!(
                    "ip route replace {} via {}",
                    observation.topology.provider_fallback_cidr, gateway
                )]
            })
            .unwrap_or_else(|| {
                vec![format!(
                    "ip route replace {} via <registry.hypervisor.network.local>",
                    observation.topology.provider_fallback_cidr
                )]
            }),
        NodeRuntimeAction::RestoreCriticalDns => vec![
            "cynic mesh status --json".to_string(),
            "cargo run -p cynic-node -- network provider-fallback-hosts --json --registry infra/registry.json --output-dir /tmp/cynic-provider-fallback-hosts".to_string(),
        ],
        NodeRuntimeAction::RestoreCanonicalDnsProjection => vec![
            "cargo run -p cynic-node -- network dns-activation-status --json --registry infra/registry.json --resolve-dns".to_string(),
            "cargo run -p cynic-node -- network dns-bundle --json --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns".to_string(),
            "cargo run -p cynic-node -- observe --json".to_string(),
        ],
        NodeRuntimeAction::RestoreCriticalTcp => vec![
            "cynic fleet status --json".to_string(),
            "ssh/curl probe failed hosts from network.observation.services.tcp".to_string(),
        ],
        NodeRuntimeAction::ActivateSynapseRoute => vec![
            "cynic-node observe --json --fail-on-warning".to_string(),
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator".to_string(),
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator".to_string(),
        ],
        NodeRuntimeAction::ActivateWireguardMesh => vec![
            "cynic-node observe --json --fail-on-warning".to_string(),
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator".to_string(),
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::node_observation::NodeRuntimeCapabilities;

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
}
