//! Canonical fleet-truth domain: observation reconcile + build_runner
//! selection. Reconciled from declared registry nodes, observed resources,
//! and organ alignment into an effective fleet state plus a read-back report.

pub mod build_runner_selection;
pub mod model;
pub mod rules;
#[cfg(test)]
mod tests;

pub use build_runner_selection::{SelectedBuildRunner, select_build_runner};
pub use model::*;
use rules::{
    build_effective_node, energy_summary, match_observed, push_declared_node_anomalies,
    push_observed_resource_anomalies, push_organ_alignment_anomalies, summary,
};

use std::collections::{BTreeMap, BTreeSet};

/// Reconcile declared nodes + observed resources into an effective fleet
/// report. Pure: no IO, no mutation.
pub fn build_report(
    declared_nodes: &[Node],
    observed_resources: &[ObservedResource],
    registry_organs: &BTreeSet<String>,
    organ_dirs: &BTreeSet<String>,
) -> FleetReport {
    let mut observed_by_identity: BTreeMap<String, ObservedResource> = BTreeMap::new();
    for resource in observed_resources {
        for identity in resource.identities() {
            observed_by_identity.insert(identity, resource.clone());
        }
    }

    let mut anomalies = Vec::new();
    let mut effective = Vec::new();

    for node in declared_nodes {
        let observed = match_observed(node, &observed_by_identity);
        push_declared_node_anomalies(&mut anomalies, node, observed);
        effective.push(build_effective_node(node, observed));
    }

    push_observed_resource_anomalies(&mut anomalies, observed_resources, declared_nodes);
    push_organ_alignment_anomalies(&mut anomalies, registry_organs, organ_dirs);

    let energy = energy_summary(declared_nodes);
    let summary = summary(declared_nodes, observed_resources, &anomalies);

    FleetReport {
        summary,
        desired: declared_nodes.to_vec(),
        declared: declared_nodes.to_vec(),
        observed: observed_resources.to_vec(),
        effective,
        anomalies,
        energy,
    }
}
