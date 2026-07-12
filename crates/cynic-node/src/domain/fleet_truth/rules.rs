use std::collections::{BTreeMap, BTreeSet};

use super::{
    Anomaly, ConstellationSummary, EffectiveNode, EnergySummary, FleetSummary, Node,
    ObservedResource,
};

pub(super) fn match_observed<'a>(
    node: &Node,
    observed_by_identity: &'a BTreeMap<String, ObservedResource>,
) -> Option<&'a ObservedResource> {
    let mut ordered_identities = vec![node.name.clone()];
    ordered_identities.extend(node.aliases.iter().cloned());
    if let Some(proxmox_id) = &node.proxmox_id {
        ordered_identities.push(proxmox_id.clone());
    }
    ordered_identities
        .iter()
        .find_map(|identity| observed_by_identity.get(identity))
}

pub(super) fn effective_status(node: &Node, observed: Option<&ObservedResource>) -> String {
    if matches!(
        node.status.as_str(),
        "planned" | "planned_obsolescence" | "archived"
    ) {
        return node.status.clone();
    }
    if observed.is_some_and(|item| item.status == "running") {
        return "active".to_string();
    }
    if node.status == "active" {
        return "stale".to_string();
    }
    node.status.clone()
}

pub(super) fn push_declared_node_anomalies(
    anomalies: &mut Vec<Anomaly>,
    node: &Node,
    observed: Option<&ObservedResource>,
) {
    if node.status == "active" && observed.is_none() {
        anomalies.push(Anomaly {
            severity: "critical".to_string(),
            kind: "active_without_runtime_proof".to_string(),
            target: node.name.clone(),
            message: "Node is declared active but no provider observation matched its name, aliases, or Proxmox id.".to_string(),
        });
    }
    if node.energy_profile.is_empty() {
        anomalies.push(Anomaly {
            severity: "warning".to_string(),
            kind: "missing_energy_profile".to_string(),
            target: node.name.clone(),
            message: "Node has no energy_profile; intentional sleep cannot be distinguished from failure.".to_string(),
        });
    }
}

pub(super) fn push_observed_resource_anomalies(
    anomalies: &mut Vec<Anomaly>,
    observed_resources: &[ObservedResource],
    declared_nodes: &[Node],
) {
    let mut declared_identities = BTreeSet::new();
    for node in declared_nodes {
        declared_identities.extend(node.identities());
    }

    for resource in observed_resources {
        if resource.identities().is_disjoint(&declared_identities) {
            anomalies.push(Anomaly {
                severity: "warning".to_string(),
                kind: "observed_unregistered_resource".to_string(),
                target: resource.name.clone(),
                message: format!(
                    "{} observed resource is not declared in registry.",
                    resource.provider
                ),
            });
        }
    }
}

pub(super) fn push_organ_alignment_anomalies(
    anomalies: &mut Vec<Anomaly>,
    registry_organs: &BTreeSet<String>,
    organ_dirs: &BTreeSet<String>,
) {
    for directory in organ_dirs {
        let canonical = if directory.starts_with("organ-") {
            directory.clone()
        } else {
            format!("organ-{directory}")
        };
        if !registry_organs.contains(directory) && !registry_organs.contains(&canonical) {
            anomalies.push(Anomaly {
                severity: "warning".to_string(),
                kind: "organ_dir_without_registry_state".to_string(),
                target: directory.clone(),
                message:
                    "Organ directory exists without active/planned/absorbed/archived registry state."
                        .to_string(),
            });
        }
    }

    for organ in registry_organs {
        let mut candidates = BTreeSet::from([organ.clone()]);
        candidates.insert(organ.trim_start_matches("organ-").to_string());
        if organ != "organ-fleet"
            && !organ_dirs.contains(organ)
            && candidates.is_disjoint(organ_dirs)
        {
            anomalies.push(Anomaly {
                severity: "warning".to_string(),
                kind: "registry_organ_without_directory".to_string(),
                target: organ.clone(),
                message:
                    "Registry declares a legacy organ entry that has no matching runtime body."
                        .to_string(),
            });
        }
    }
}

pub(super) fn build_effective_node(
    node: &Node,
    observed: Option<&ObservedResource>,
) -> EffectiveNode {
    EffectiveNode {
        name: node.name.clone(),
        role: node.role.clone(),
        constellation: node.constellation.clone(),
        declared_status: node.status.clone(),
        observed_status: observed
            .map(|item| item.status.clone())
            .unwrap_or_else(|| "unobserved".to_string()),
        effective_status: effective_status(node, observed),
        provider: observed.map(|item| item.provider.clone()),
        observed_constellation: observed.and_then(|item| item.constellation.clone()),
    }
}

pub(super) fn energy_summary(nodes: &[Node]) -> EnergySummary {
    let mut by_class = BTreeMap::new();
    let mut missing = Vec::new();
    for node in nodes {
        match node
            .energy_profile
            .get("class")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
        {
            Some(energy_class) => {
                *by_class.entry(energy_class.to_string()).or_insert(0) += 1;
            }
            None => missing.push(node.name.clone()),
        }
    }
    EnergySummary {
        classes: by_class,
        missing_profiles: missing,
    }
}

pub(super) fn summary(
    nodes: &[Node],
    observed_resources: &[ObservedResource],
    anomalies: &[Anomaly],
) -> FleetSummary {
    let critical: Vec<&Anomaly> = anomalies
        .iter()
        .filter(|anomaly| anomaly.severity == "critical")
        .collect();
    FleetSummary {
        status: if critical.is_empty() {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        declared_nodes: nodes.len(),
        observed_resources: observed_resources.len(),
        constellations: constellation_summary(nodes, observed_resources),
        anomaly_count: anomalies.len(),
        critical_anomaly_count: critical.len(),
        critical_targets: critical
            .iter()
            .map(|anomaly| anomaly.target.clone())
            .collect(),
    }
}

fn constellation_summary(
    nodes: &[Node],
    observed_resources: &[ObservedResource],
) -> ConstellationSummary {
    let mut declared = BTreeMap::new();
    let mut observed = BTreeMap::new();
    for node in nodes {
        if let Some(constellation) = &node.constellation {
            *declared.entry(constellation.clone()).or_insert(0) += 1;
        }
    }
    for resource in observed_resources {
        if let Some(constellation) = &resource.constellation {
            *observed.entry(constellation.clone()).or_insert(0) += 1;
        }
    }
    ConstellationSummary { declared, observed }
}
