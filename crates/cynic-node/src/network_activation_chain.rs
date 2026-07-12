use chrono::Utc;
use cynic_host::bridge;
use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::runtime_context::observe_current_host_runtime_context;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;
use serde_json::Value;

use crate::dns_activation_status;
use crate::dns_plan;
use crate::network_actuator;
use crate::network_actuator_status;
use crate::network_evidence;
use crate::network_runtime_status;
use crate::observe;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActivationChainReport {
    schema: &'static str,
    generated_at: String,
    registry: String,
    pub(crate) status: &'static str,
    host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_resolution: Option<cynic_host::registry::HostProfileResolution>,
    stages: Vec<ActivationStage>,
    host_context: Value,
    bridge_status: Value,
    bridge_plan: Value,
    dns_status: Value,
    dns_plan: Value,
    actuator_status: Value,
    actuator_plan: Value,
    evidence: Value,
    runtime_status: Value,
}

#[derive(Debug, Clone, Serialize)]
struct ActivationStage {
    order: u8,
    id: &'static str,
    layer: &'static str,
    status: &'static str,
    objective: &'static str,
    canonical_surface: &'static str,
    commands: Vec<ActivationStageCommand>,
}

#[derive(Debug, Clone, Serialize)]
struct ActivationStageCommand {
    role: &'static str,
    command: String,
}

pub(crate) fn build_activation_chain_report(
    registry: &str,
    resolve_dns: bool,
    check_provider_connectivity: bool,
    provider_hosts_path: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
) -> Result<NetworkActivationChainReport, Box<dyn std::error::Error>> {
    let runtime_layout = current_runtime_layout();
    let host_context = observe_current_host_runtime_context();
    let bridge_status = bridge::bridge_status();
    let bridge_plan = bridge::bridge_activation_plan(&runtime_layout.host_exec_bundle_dir);
    let dns_status = dns_activation_status::build_dns_activation_status(
        registry,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
        resolve_dns,
    )?;
    let dns_plan = dns_plan::build_dns_plan();
    let observation = observe::observe_payload(Some(registry));
    let actuator_status = network_actuator_status::activation_status(&observation);
    let actuator_plan = network_actuator::activation_plan(
        &observation,
        &runtime_layout.network_actuator_bundle_dir,
    );
    let evidence = network_evidence::build_evidence(&observation);
    let runtime_status = network_runtime_status::build_network_runtime_status(
        registry,
        resolve_dns,
        check_provider_connectivity,
        provider_hosts_path,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
    )?;
    build_activation_chain_report_with_runtime(
        registry,
        host_context.host_profile,
        host_context.host_resolution.clone(),
        host_substrate_active(&host_context),
        host_context.host_bootstrap.status == SurfaceStatus::Active,
        bridge_status.status == "active",
        dns_status.status,
        actuator_status.status,
        runtime_status.status,
        runtime_layout.dns_bundle_dir,
        runtime_layout.network_actuator_bundle_dir,
        serde_json::to_value(&host_context)?,
        serde_json::to_value(&bridge_status)?,
        serde_json::to_value(&bridge_plan)?,
        serde_json::to_value(&dns_status)?,
        serde_json::to_value(&dns_plan)?,
        serde_json::to_value(&actuator_status)?,
        serde_json::to_value(&actuator_plan)?,
        serde_json::to_value(&evidence)?,
        serde_json::to_value(&runtime_status)?,
    )
}

fn build_activation_chain_report_with_runtime(
    registry: &str,
    host_profile: &'static str,
    host_resolution: Option<cynic_host::registry::HostProfileResolution>,
    host_substrate_active: bool,
    host_bootstrap_active: bool,
    bridge_active: bool,
    dns_status_value: &'static str,
    actuator_status_value: &'static str,
    runtime_status_value: &'static str,
    dns_bundle_dir: String,
    network_actuator_bundle_dir: String,
    host_context: Value,
    bridge_status: Value,
    bridge_plan: Value,
    dns_status: Value,
    dns_plan: Value,
    actuator_status: Value,
    actuator_plan: Value,
    evidence: Value,
    runtime_status: Value,
) -> Result<NetworkActivationChainReport, Box<dyn std::error::Error>> {
    let stages = vec![
        ActivationStage {
            order: 1,
            id: "host_substrate",
            layer: "os_baseline",
            status: if host_substrate_active {
                "active"
            } else {
                "blocked"
            },
            objective: "establish principal, privilege, packages and services baseline",
            canonical_surface: "cynic bootstrap host plan",
            commands: vec![
                activation_command(
                    "observe",
                    format!("cynic bootstrap host observe --registry {registry}"),
                ),
                activation_command(
                    "plan",
                    format!("cynic bootstrap host plan --registry {registry}"),
                ),
            ],
        },
        ActivationStage {
            order: 2,
            id: "host_bridge",
            layer: "privilege_bridge",
            status: if bridge_active { "active" } else { "blocked" },
            objective: "install the bounded host-exec sudo bridge once",
            canonical_surface: "cynic bootstrap bridge plan",
            commands: vec![
                activation_command("plan", "cynic bootstrap bridge plan"),
                activation_command("status", "cynic bootstrap bridge status"),
            ],
        },
        ActivationStage {
            order: 3,
            id: "host_bootstrap",
            layer: "bootstrap_proof",
            status: if host_bootstrap_active {
                "active"
            } else {
                "blocked"
            },
            objective: "prove host baseline and privilege bridge are both live",
            canonical_surface: "cynic bootstrap chain",
            commands: vec![activation_command(
                "prove",
                format!("cynic bootstrap chain --registry {registry}"),
            )],
        },
        ActivationStage {
            order: 4,
            id: "dns_local",
            layer: "dns_plane",
            status: dns_status_value,
            objective: "install canonical local Synapse DNS projection with CoreDNS",
            canonical_surface: "cynic network dns plan",
            commands: vec![
                activation_command(
                    "bundle",
                    format!(
                        "cynic network dns bundle --output-dir {} --registry {registry}",
                        dns_bundle_dir
                    ),
                ),
                activation_command(
                    "status",
                    format!("cynic network dns status --registry {registry} --resolve-dns"),
                ),
            ],
        },
        ActivationStage {
            order: 5,
            id: "network_actuator",
            layer: "root_network_mutation",
            status: actuator_status_value,
            objective: "install and prove the bounded root network actuator",
            canonical_surface: "cynic network actuator plan",
            commands: vec![
                activation_command(
                    "build",
                    "cargo build -q -p cynic-node --bin cynic-network-actuator",
                ),
                activation_command(
                    "bundle",
                    format!(
                        "cynic network actuator bundle --output-dir {}",
                        network_actuator_bundle_dir
                    ),
                ),
                activation_command("status", "cynic network actuator status"),
            ],
        },
        ActivationStage {
            order: 6,
            id: "pre_post_evidence",
            layer: "evidence",
            status: "required",
            objective: "capture consistent before/after proof around every bounded mutation",
            canonical_surface: "cynic network evidence",
            commands: vec![
                activation_command(
                    "evidence",
                    format!("cynic network evidence --registry {registry}"),
                ),
                activation_command(
                    "runtime",
                    format!("cynic network runtime --registry {registry} --resolve-dns"),
                ),
            ],
        },
        ActivationStage {
            order: 7,
            id: "runtime_readiness",
            layer: "runtime_outcome",
            status: runtime_status_value,
            objective: "prove the node reaches runtime readiness without hidden drift",
            canonical_surface: "cynic network runtime",
            commands: vec![activation_command(
                "runtime",
                format!("cynic network runtime --registry {registry} --resolve-dns"),
            )],
        },
    ];

    let status = if runtime_status_value == "mesh_active" {
        "active"
    } else if stages.iter().any(|stage| stage.status == "blocked") {
        "blocked"
    } else {
        "planned"
    };

    Ok(NetworkActivationChainReport {
        schema: "cynic.node_network_activation_chain.v1",
        generated_at: Utc::now().to_rfc3339(),
        registry: registry.to_string(),
        status,
        host_profile,
        host_resolution,
        stages,
        host_context,
        bridge_status,
        bridge_plan,
        dns_status,
        dns_plan,
        actuator_status,
        actuator_plan,
        evidence,
        runtime_status,
    })
}

fn activation_command(role: &'static str, command: impl Into<String>) -> ActivationStageCommand {
    ActivationStageCommand {
        role,
        command: command.into(),
    }
}

fn host_substrate_active(context: &cynic_host::runtime_context::HostRuntimeContext) -> bool {
    let reports = &context.host_substrate;
    reports.principal.status == SurfaceStatus::Active
        && reports.privilege.status == SurfaceStatus::Active
        && reports.packages.status == SurfaceStatus::Active
        && reports.services.status == SurfaceStatus::Active
}

#[cfg(test)]
mod tests {
    use super::build_activation_chain_report_with_runtime;
    use cynic_host::registry::DEFAULT_REGISTRY_PATH;

    #[test]
    fn activation_chain_report_has_stable_schema_and_stage_order() {
        let report = build_activation_chain_report_with_runtime(
            DEFAULT_REGISTRY_PATH,
            "test-host",
            None,
            false,
            false,
            false,
            "blocked",
            "missing_activation",
            "provider_fallback_active",
            "/tmp/test-dns-bundle".to_string(),
            "/tmp/test-network-actuator-bundle".to_string(),
            serde_json::json!({}),
            serde_json::json!({"status": "blocked"}),
            serde_json::json!({}),
            serde_json::json!({"status": "blocked"}),
            serde_json::json!({}),
            serde_json::json!({"status": "missing_activation"}),
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({"status": "provider_fallback_active"}),
        )
        .expect("activation chain report");

        assert_eq!(report.schema, "cynic.node_network_activation_chain.v1");
        assert_eq!(report.stages.len(), 7);
        assert_eq!(
            report.stages.first().map(|stage| stage.id),
            Some("host_substrate")
        );
        assert_eq!(
            report.stages.last().map(|stage| stage.id),
            Some("runtime_readiness")
        );
    }
}
