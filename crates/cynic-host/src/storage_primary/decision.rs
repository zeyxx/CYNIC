use std::fs;
use std::path::Path;

use super::types::{
    StoragePrimaryDesiredState, StoragePrimaryPlanReport, StoragePrimaryStatusReport,
};

pub fn build_plan(registry_path: &Path, schema_path: &Path) -> StoragePrimaryPlanReport {
    match build_desired_state(registry_path, schema_path) {
        Ok((desired, unknowns)) => StoragePrimaryPlanReport {
            schema: "cynic.storage_primary.plan.v1",
            status: "planned",
            authority: "infra/registry.json",
            mutates: false,
            desired: Some(desired),
            blockers: Vec::new(),
            unknowns,
            next_actions: vec![
                "observe_storage_primary_status".to_string(),
                "build_activation_bundle_only_when_bootstrap_is_required".to_string(),
            ],
        },
        Err(blockers) => StoragePrimaryPlanReport {
            schema: "cynic.storage_primary.plan.v1",
            status: "blocked",
            authority: "infra/registry.json",
            mutates: false,
            desired: None,
            blockers,
            unknowns: Vec::new(),
            next_actions: vec!["repair_storage_primary_registry_contract".to_string()],
        },
    }
}

pub fn evaluate_status(
    plan: &StoragePrimaryPlanReport,
    evidence: serde_json::Value,
) -> StoragePrimaryStatusReport {
    let health_ok = evidence
        .pointer("/health/healthy")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let dns_ok = plan.desired.as_ref().is_some_and(|desired| {
        evidence
            .pointer("/mesh_status/dns_checks")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|check| {
                check.get("node").and_then(serde_json::Value::as_str)
                    == Some(desired.node_id.as_str())
                    && check.get("status").and_then(serde_json::Value::as_str) == Some("ok")
            })
    });
    let provider_running = evidence
        .pointer("/fleet_report/effective/0/observed_status")
        .and_then(serde_json::Value::as_str)
        .map(|status| status == "running");

    let mut blockers = plan.blockers.clone();
    if !health_ok {
        blockers.push("storage_service_health_not_proven".to_string());
    }
    if !dns_ok {
        blockers.push("storage_synapse_dns_not_proven".to_string());
    }
    let mut unknowns = plan.unknowns.clone();
    if provider_running.is_none() {
        unknowns.push("provider_power_state_not_observed".to_string());
    } else if provider_running == Some(false) {
        blockers.push("provider_reports_storage_not_running".to_string());
    }
    let ready = plan.status == "planned" && blockers.is_empty();
    let next_actions = blockers
        .iter()
        .map(|blocker| match blocker.as_str() {
            "storage_service_health_not_proven" => "observe_or_reconcile_surrealdb_service",
            "storage_synapse_dns_not_proven" => "repair_storage_primary_dns_record",
            "provider_reports_storage_not_running" => "plan_provider_power_reconciliation",
            _ => "repair_storage_primary_desired_state",
        })
        .map(str::to_string)
        .collect();
    StoragePrimaryStatusReport {
        schema: "cynic.storage_primary.status.v2",
        status: if ready { "active" } else { "incomplete" },
        ready,
        authority: "infra/registry.json",
        desired: plan.desired.clone(),
        checks: serde_json::json!({
            "service_health": health_ok,
            "storage_dns": dns_ok,
            "provider_running": provider_running,
            "bootstrap_bundle_required_for_steady_state": false,
            "hypervisor_exec_bridge_required_for_steady_state": false
        }),
        blockers,
        unknowns,
        next_actions,
    }
}

fn build_desired_state(
    registry_path: &Path,
    schema_path: &Path,
) -> Result<(StoragePrimaryDesiredState, Vec<String>), Vec<String>> {
    let bytes =
        fs::read(registry_path).map_err(|error| vec![format!("registry_unavailable:{error}")])?;
    let registry: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| vec![format!("registry_invalid_json:{error}")])?;
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| vec!["registry_nodes_missing".to_string()])?;
    let storage_nodes = nodes
        .iter()
        .filter(|node| {
            node.get("role").and_then(serde_json::Value::as_str) == Some("storage_primary")
        })
        .collect::<Vec<_>>();
    if storage_nodes.len() != 1 {
        return Err(vec![format!(
            "storage_primary_cardinality_invalid:{}",
            storage_nodes.len()
        )]);
    }
    let node = storage_nodes[0];
    let node_id = required_string(node, "id", "storage_primary_node_id_missing")?;
    let synapse = node
        .pointer("/network/synapse")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| vec!["storage_primary_synapse_ip_missing".to_string()])?;
    let service = node
        .get("services")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|service| {
            service.get("name").and_then(serde_json::Value::as_str) == Some("surrealdb")
        })
        .ok_or_else(|| vec!["storage_primary_surrealdb_service_missing".to_string()])?;
    let port = service
        .get("port")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| vec!["storage_primary_surrealdb_port_missing".to_string()])?;
    let scheme = service
        .get("scheme")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("http");
    let unit = required_string(service, "unit", "storage_primary_unit_missing")?;
    let reservation = registry
        .pointer("/network_policy/reservations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|item| item.get("name").and_then(serde_json::Value::as_str) == Some(node_id.as_str()))
        .ok_or_else(|| vec!["storage_primary_ipam_reservation_missing".to_string()])?;
    if reservation
        .get("synapse")
        .and_then(serde_json::Value::as_str)
        != Some(synapse)
    {
        return Err(vec![
            "storage_primary_ipam_reservation_mismatch".to_string(),
        ]);
    }
    let host = reservation
        .get("dns")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .find(|name| name.ends_with(".synapse.cynic"))
        .unwrap_or(synapse);
    let target = registry
        .pointer("/fleet_desired/targets")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|item| {
            item.get("role").and_then(serde_json::Value::as_str) == Some("storage_primary")
        })
        .ok_or_else(|| vec!["storage_primary_fleet_target_missing".to_string()])?;
    let provider = required_string(target, "provider", "storage_primary_provider_missing")?;
    let resource_id =
        required_string(target, "resource_id", "storage_primary_resource_id_missing")?;
    let mut unknowns = Vec::new();
    if !schema_path.is_file() {
        unknowns.push("storage_schema_source_not_observed".to_string());
    }
    Ok((
        StoragePrimaryDesiredState {
            node_id,
            role: "storage_primary".to_string(),
            provider,
            resource_id,
            endpoint: format!("{scheme}://{host}:{port}"),
            unit,
            schema_path: schema_path.display().to_string(),
            runtime_identity: "cynic-runtime".to_string(),
            secret_class: "RuntimeMachineIdentity",
        },
        unknowns,
    ))
}

fn required_string(
    value: &serde_json::Value,
    key: &str,
    blocker: &str,
) -> Result<String, Vec<String>> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| vec![blocker.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_does_not_require_breakglass_bridge_or_bundle() {
        let plan = StoragePrimaryPlanReport {
            schema: "cynic.storage_primary.plan.v1",
            status: "planned",
            authority: "infra/registry.json",
            mutates: false,
            desired: Some(StoragePrimaryDesiredState {
                node_id: "storage".to_string(),
                role: "storage_primary".to_string(),
                provider: "proxmox".to_string(),
                resource_id: "lxc/103".to_string(),
                endpoint: "http://storage.synapse.cynic:8000".to_string(),
                unit: "surrealdb.service".to_string(),
                schema_path: "schema.surql".to_string(),
                runtime_identity: "cynic-runtime".to_string(),
                secret_class: "RuntimeMachineIdentity",
            }),
            blockers: Vec::new(),
            unknowns: Vec::new(),
            next_actions: Vec::new(),
        };
        let status = evaluate_status(
            &plan,
            serde_json::json!({
                "health": {"healthy": true},
                "mesh_status": {"dns_checks": [{"node": "storage", "status": "ok"}]},
                "fleet_report": {"effective": [{"observed_status": null}]}
            }),
        );
        assert!(status.ready);
        assert_eq!(status.status, "active");
        assert_eq!(status.unknowns, vec!["provider_power_state_not_observed"]);
    }
}
