use std::process::ExitCode;

use cynic_node::NetworkCommands;
use serde_json::json;

use crate::network_surface::{NetworkCommand, NetworkMeshCommand};
use crate::runtime_paths::{self, RuntimePaths};
use crate::{emit_json, fail, registry_path};

pub(crate) fn try_execute_mesh(
    command: NetworkCommand,
    runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Mesh { command } = command else {
        return Err(command);
    };

    let exit = match command {
        NetworkMeshCommand::Plan { registry, provider } => {
            cynic_node::execute_network_command(&NetworkCommands::MeshPlan {
                json: true,
                registry: registry_path(registry),
                provider,
            })
        }
        NetworkMeshCommand::Bundle {
            registry,
            provider,
            output_dir,
        } => cynic_node::execute_network_command(&NetworkCommands::MeshBundle {
            json: true,
            registry: registry_path(registry),
            provider,
            output_dir,
        }),
        NetworkMeshCommand::ApplyPlan { bundle_dir } => {
            cynic_node::execute_network_command(&NetworkCommands::MeshApplyPlan {
                json: true,
                bundle_dir,
            })
        }
        NetworkMeshCommand::TargetAttestation {
            harness_kind,
            target_id,
            transport,
            transport_target,
            expected_network_boundary,
            virsh_bin,
            ip_bin,
        } => cynic_node::execute_network_command(&NetworkCommands::MeshTargetAttestation {
            json: true,
            harness_kind,
            target_id,
            transport,
            transport_target,
            expected_network_boundary,
            virsh_bin,
            ip_bin: ip_bin.unwrap_or_else(|| {
                cynic_host::runtime_layout::current_runtime_layout()
                    .capabilities()
                    .route_tool
                    .command
            }),
        }),
        NetworkMeshCommand::ApplyProfile {
            bundle_dir,
            profile,
            apply,
            closed_environment,
        } => cynic_node::execute_network_command(&NetworkCommands::MeshApplyProfile {
            json: true,
            bundle_dir,
            profile,
            apply,
            closed_environment,
        }),
        NetworkMeshCommand::ApplyTarget {
            bundle_dir,
            profile,
            apply,
            harness_kind,
            target_attestation,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        } => cynic_node::execute_network_command(&NetworkCommands::MeshApplyTarget {
            json: true,
            bundle_dir,
            profile,
            apply,
            harness_kind,
            target_attestation,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        }),
        NetworkMeshCommand::Status {
            registry,
            resolve_dns,
            include_records,
            observe_runtime,
        } => {
            let registry = registry_path(registry);
            emit_mesh_status(
                runtime,
                &registry,
                resolve_dns,
                include_records,
                observe_runtime,
            )
        }
    };

    Ok(exit)
}

fn emit_mesh_status(
    runtime: &runtime_paths::RuntimePaths,
    registry: &str,
    resolve_dns: bool,
    _include_records: bool,
    observe_runtime: bool,
) -> ExitCode {
    let status = match cynic_node::build_network_runtime_status_value(
        registry,
        resolve_dns,
        false,
        &runtime.provider_hosts_path.to_string_lossy(),
        &runtime.dns_hosts_path.to_string_lossy(),
        &runtime.dns_corefile_path.to_string_lossy(),
        &runtime.dns_service_path.to_string_lossy(),
        &runtime.resolved_dropin_path.to_string_lossy(),
    ) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };

    let observation = status
        .get("observation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_observation = if observe_runtime {
        observation
            .get("runtime_observation")
            .cloned()
            .unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };
    let runtime_status = observation
        .get("network_mesh_status")
        .and_then(|mesh| mesh.get("runtime_status"))
        .and_then(|value| value.as_str())
        .unwrap_or("not_observed");
    let dns_status = if resolve_dns {
        status
            .get("dns_activation")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            .map(|value| if value == "active" { "ok" } else { "violation" })
            .unwrap_or("violation")
    } else {
        observation
            .get("network_mesh_status")
            .and_then(|mesh| mesh.get("dns_status"))
            .and_then(|value| value.as_str())
            .unwrap_or("not_checked")
    };
    let topology = observation
        .get("topology")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
        "zone": topology.get("dns_zone").cloned().unwrap_or_else(|| json!("unknown")),
        "registry_ref": registry,
        "canonical_service_plane": topology
            .get("canonical_service_plane")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "synapse_cidr": topology.get("synapse_cidr").cloned().unwrap_or(serde_json::Value::Null),
        "provider_fallback_planes": [],
        "controller_status": serde_json::Value::Null,
        "policy_status": observation
            .get("network_mesh_status")
            .and_then(|mesh| mesh.get("policy_status"))
            .cloned()
            .unwrap_or(json!("ok")),
        "runtime_status": runtime_status,
        "resolve_dns": resolve_dns,
        "peers": 0,
        "dns_status": dns_status,
        "records": json!([]),
        "dns_checks": status
            .get("dns_activation")
            .and_then(|value| value.get("checks"))
            .cloned()
            .map(|checks| json!([checks]))
            .unwrap_or_else(|| json!([])),
        "runtime_observation": runtime_observation,
        "drift": runtime_observation.get("drift").cloned().unwrap_or_else(|| json!([])),
        "blocking": runtime_observation
            .get("blocking")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "missing": status
            .get("dns_activation")
            .and_then(|value| value.get("missing"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "offline": [],
    });
    let should_fail = matches!(runtime_status, "blocked" | "provider_fallback_active")
        || dns_status == "violation";
    emit_json(&payload, should_fail)
}
