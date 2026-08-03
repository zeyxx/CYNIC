use std::path::Path;

use cynic_protocol::{NodeEnrollmentDesiredState, validate_node_enrollment_desired_state};
use serde_json::Value;

use super::types::RuntimeBindingSpec;

pub fn resolve_local_runtime_binding_spec(
    registry_path: &Path,
) -> Result<RuntimeBindingSpec, String> {
    let bytes = std::fs::read(registry_path).map_err(|error| {
        format!(
            "failed to read runtime registry {}: {error}",
            registry_path.display()
        )
    })?;
    let root: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "failed to decode runtime registry {}: {error}",
            registry_path.display()
        )
    })?;
    let desired = root
        .get("node_enrollment")
        .cloned()
        .ok_or_else(|| "node_enrollment_desired_state_missing".to_string())?;
    let desired: NodeEnrollmentDesiredState = serde_json::from_value(desired)
        .map_err(|_| "node_enrollment_desired_state_invalid".to_string())?;
    let blockers = validate_node_enrollment_desired_state(&desired);
    if !blockers.is_empty() {
        return Err(blockers.join(","));
    }
    let nodes = root
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "runtime_registry_nodes_missing".to_string())?;
    let authority_node = super::super::placement::unique_role_node(
        nodes,
        &desired.authority_role,
        "runtime_binding_authority",
    )?;
    super::super::placement::ensure_local_placement(
        nodes,
        authority_node,
        &crate::runtime_layout::observed_hostname(),
        "runtime_binding_authority",
    )?;
    let services = authority_node
        .get("services")
        .and_then(Value::as_array)
        .ok_or_else(|| "runtime_binding_services_missing".to_string())?;
    let matches = services
        .iter()
        .filter(|service| {
            service.get("name").and_then(Value::as_str) == Some(desired.endpoint_service.as_str())
        })
        .collect::<Vec<_>>();
    let [service] = matches.as_slice() else {
        return Err("runtime_binding_endpoint_service_not_unique".to_string());
    };
    let scheme = service
        .get("scheme")
        .and_then(Value::as_str)
        .filter(|scheme| matches!(*scheme, "http" | "https"))
        .ok_or_else(|| "runtime_binding_endpoint_scheme_invalid".to_string())?;
    let endpoint_host = service
        .get("endpoint_host")
        .and_then(Value::as_str)
        .filter(|host| !host.trim().is_empty())
        .ok_or_else(|| "runtime_binding_endpoint_host_missing".to_string())?;
    let port = service
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
        .ok_or_else(|| "runtime_binding_endpoint_port_invalid".to_string())?;
    let network_plane = service
        .get("network_plane")
        .and_then(Value::as_str)
        .filter(|plane| !plane.trim().is_empty())
        .ok_or_else(|| "runtime_binding_network_plane_missing".to_string())?;
    let unit_name = service
        .get("unit")
        .and_then(Value::as_str)
        .filter(|unit| !unit.trim().is_empty() && !unit.contains('/'))
        .ok_or_else(|| "runtime_binding_service_unit_invalid".to_string())?;
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    let owner_id = crate::identity_database::resolve_identity_id("/etc/passwd", "root")
        .ok_or_else(|| "runtime_binding_root_identity_unresolved".to_string())?;
    let group_id = crate::identity_database::resolve_identity_id(
        "/etc/group",
        &authority.principal.runtime_group,
    )
    .ok_or_else(|| "runtime_binding_runtime_group_unresolved".to_string())?;
    let layout = crate::runtime_layout::current_runtime_layout();

    Ok(RuntimeBindingSpec {
        authority_role: desired.authority_role,
        service_name: desired.endpoint_service,
        unit_name: unit_name.to_string(),
        scheme: scheme.to_string(),
        endpoint_host: endpoint_host.to_string(),
        port,
        network_plane: network_plane.to_string(),
        environment_path: authority.config_root.join("env").display().to_string(),
        service_unit_path: Path::new(&layout.service_unit_dir)
            .join(unit_name)
            .display()
            .to_string(),
        service_manager_argv: layout.systemctl_bin(),
        owner_id,
        group_id,
    })
}
