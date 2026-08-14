use std::env;
use std::path::Path;
use std::process::ExitCode;

use cynic_host::host_bootstrap_evidence;
use cynic_host::registry::{resolve_host_profile, resolve_host_profile_resolution};
use serde::Serialize;

use crate::{emit_json, fail, registry_path};

#[derive(Debug, Serialize)]
struct ConstellationBootstrapReport {
    schema: &'static str,
    status: &'static str,
    registry_path: String,
    rule: &'static str,
    lifecycle_status: String,
    next_boundary_kind: String,
    next_boundary_reason: String,
    next_command: Option<String>,
    nodes: Vec<ConstellationNodeBootstrap>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConstellationNodeBootstrap {
    node: String,
    role: String,
    maturity: &'static str,
    lock_state: &'static str,
    current_host_match: bool,
    required_proof: Vec<String>,
    observed_proof: Vec<String>,
    missing_proof: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lifecycle_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_boundary_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_boundary_reason: Option<String>,
    next_command: String,
}

pub(crate) fn execute_constellation(json: bool, registry: Option<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path(registry);
    match build_constellation_bootstrap_report(&registry) {
        Ok(report) => emit_json(&report, report.status != "ok"),
        Err(error) => fail(error),
    }
}

fn build_constellation_bootstrap_report(
    registry_path: &str,
) -> Result<ConstellationBootstrapReport, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let nodes = registry
        .get("nodes")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "registry has no nodes array".to_string())?;
    let current_hostname = current_hostname();
    let mut reports = Vec::new();
    let mut next_actions = Vec::new();

    for node in nodes {
        let name = node_name(node)?;
        let role = node
            .get("role")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        let current_host_match = node_matches_hostname(node, &current_hostname);
        let required_proof = required_proof_for_node(&registry, node, &role);
        let mut observed_proof = Vec::new();
        let mut missing_proof = Vec::new();
        let mut lifecycle_status = None;
        let mut next_boundary_kind = None;
        let mut next_boundary_reason = None;
        let mut canonical_next_command = None;

        if current_host_match {
            observed_proof.push("host_identity_observed".to_string());
            if let Ok(resolution) = resolve_host_profile_resolution(&name, Some(registry_path)) {
                if let Ok(profile) = resolve_host_profile(&name, Some(registry_path)) {
                    if let Ok(evidence) = host_bootstrap_evidence::observe_host_bootstrap_evidence(
                        profile,
                        Some(resolution),
                    ) {
                        observed_proof
                            .push(format!("host_bootstrap_reached={}", evidence.reached_stage));
                        lifecycle_status = Some(evidence.lifecycle_status.as_str().to_string());
                        next_boundary_kind = Some(evidence.next_boundary_kind.as_str().to_string());
                        next_boundary_reason = Some(evidence.next_boundary_reason.clone());
                        canonical_next_command = evidence.next_command.clone();
                        if evidence.target_satisfied {
                            observed_proof.push("host_bootstrap_target_satisfied".to_string());
                        }
                    }
                }
            }
        }

        for proof in &required_proof {
            if !observed_proof.iter().any(|item| item.starts_with(proof)) {
                missing_proof.push(proof.clone());
            }
        }

        let lock_state = if missing_proof.is_empty() {
            "lockable"
        } else if current_host_match {
            "observed_incomplete"
        } else {
            "candidate"
        };
        let maturity = match lock_state {
            "lockable" => "observed",
            "observed_incomplete" => "candidate_observed",
            _ => "candidate",
        };
        let next_command = if let Some(command) = canonical_next_command {
            command
        } else if current_host_match {
            format!("cynic bootstrap host evidence --node {name} --registry {registry_path}")
        } else {
            format!(
                "run on target host: cynic bootstrap host evidence --node {name} --registry {registry_path}"
            )
        };
        if lock_state != "lockable" {
            next_actions.push(next_command.clone());
        }
        reports.push(ConstellationNodeBootstrap {
            node: name,
            role,
            maturity,
            lock_state,
            current_host_match,
            required_proof,
            observed_proof,
            missing_proof,
            lifecycle_status,
            next_boundary_kind,
            next_boundary_reason,
            next_command,
        });
    }

    let status = if reports.iter().all(|node| node.lock_state == "lockable") {
        "ok"
    } else {
        "bootstrap_required"
    };
    let first_blocked = reports.iter().find(|node| node.lock_state != "lockable");
    let (lifecycle_status, next_boundary_kind, next_boundary_reason, next_command) =
        if status == "ok" {
            (
                "ready".to_string(),
                "observe".to_string(),
                "all declared nodes have satisfied their local bootstrap proof requirements"
                    .to_string(),
                None,
            )
        } else if let Some(node) = first_blocked {
            match (
                node.lifecycle_status.as_ref(),
                node.next_boundary_kind.as_ref(),
                node.next_boundary_reason.as_ref(),
            ) {
                (Some(status), Some(kind), Some(reason)) => (
                    status.clone(),
                    kind.clone(),
                    reason.clone(),
                    Some(node.next_command.clone()),
                ),
                _ => (
                    "connection_required".to_string(),
                    "connect".to_string(),
                    format!(
                        "bootstrap evidence is not yet observable from this host for node {}",
                        node.node
                    ),
                    Some(node.next_command.clone()),
                ),
            }
        } else {
            (
                "missing".to_string(),
                "observe".to_string(),
                "constellation bootstrap status could not derive a blocked node".to_string(),
                None,
            )
        };
    Ok(ConstellationBootstrapReport {
        schema: "cynic.constellation_bootstrap.v1",
        status,
        registry_path: Path::new(registry_path).display().to_string(),
        rule: "registry nodes are targets until each role has local runtime proof; lock only observed roles",
        lifecycle_status,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
        nodes: reports,
        next_actions,
    })
}

fn node_name(node: &serde_json::Value) -> Result<String, String> {
    node.get("name")
        .or_else(|| node.get("id"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "registry node missing name/id".to_string())
}

fn node_matches_hostname(node: &serde_json::Value, hostname: &str) -> bool {
    if node
        .get("name")
        .or_else(|| node.get("id"))
        .and_then(|value| value.as_str())
        == Some(hostname)
    {
        return true;
    }
    node.get("aliases")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .any(|alias| alias.as_str() == Some(hostname))
}

fn current_hostname() -> String {
    env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn required_proof_for_node(
    registry: &serde_json::Value,
    node: &serde_json::Value,
    role: &str,
) -> Vec<String> {
    let mut proofs = registry
        .pointer(&format!(
            "/lifecycle/kernel_release/host_bootstrap/role_profiles/{role}/required_proof"
        ))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if proofs.is_empty() {
        proofs.push("host_identity_observed".to_string());
        proofs.push("host_bootstrap_target_satisfied".to_string());
        for service in node
            .get("services")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let Some(name) = service
                .get("name")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let Some(port) = service.get("port").and_then(|value| value.as_u64()) else {
                continue;
            };
            proofs.push(format!("service:{name}:{port}:observed"));
        }
    }

    proofs.sort();
    proofs.dedup();
    proofs
}
