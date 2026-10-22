//! Discovery host — adapters and bridges for capability observation.
//!
//! Phase A: facade + bridge. Re-exports the build-discovery adapters from
//! `cynic-host` and adds conversion to `cynic-discovery` types.
//!
//! Phase B: extract adapters physically from `cynic-host` monolith here.

// ── Re-exports from cynic-build (domain types) ───────────────────────────────

pub use cynic_build::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryPort, BuildDiscoveryReport, DiscoveredCrate,
    HostTriple, Toolchain,
};
pub use cynic_build::build_discovery_recipe::{
    BUILD_DISCOVERY_RECIPE_SCHEMA, BuildDiscoveryRecipe,
};

// ── Re-exports from cynic-host (adapters) ────────────────────────────────────

pub use cynic_host::adapters::local_build_discovery::LocalBuildDiscoveryAdapter;
pub use cynic_host::adapters::remote_build_discovery::RemoteBuildDiscoveryAdapter;
pub use cynic_host::fleet_discovery::FleetDiscoveryAdapter;

// ── Re-exports from cynic-discovery (pure types) ─────────────────────────────

pub use cynic_discovery::{
    Capability, DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_EVENT_SCHEMA, DISCOVERY_NODE_SCHEMA,
    DiscoveryEvent, DiscoveryObservationPort, FleetCapabilities, NodeCapabilities, NodeRole,
};

// ── Bridge: BuildDiscoveryReport → Discovery types ───────────────────────────

use std::time::{SystemTime, UNIX_EPOCH};

/// Map a `BuildDiscoveryReport` (host-specific) to a `NodeCapabilities`
/// (discovery domain). Pure conversion, no I/O.
pub fn build_report_to_capabilities(
    node_id: &str,
    endpoint: &str,
    report: &BuildDiscoveryReport,
) -> NodeCapabilities {
    let role = classify_role(report);
    let capabilities = build_capabilities(report);
    NodeCapabilities {
        node_id: node_id.to_string(),
        role,
        endpoint: endpoint.to_string(),
        capabilities,
        observed_at_unix_s: report.observed_at_unix_s,
    }
}

/// Classify a node's role based on its build discovery report.
fn classify_role(report: &BuildDiscoveryReport) -> NodeRole {
    if report
        .crates
        .iter()
        .any(|c| c.bins.iter().any(|b| b == "cynic"))
    {
        // Has the cynic binary → could be orchestrator or builder
        if report.crates.len() > 10 {
            NodeRole::Orchestrator
        } else {
            NodeRole::Builder
        }
    } else {
        NodeRole::Unknown
    }
}

/// Derive discovery capabilities from a build report.
fn build_capabilities(report: &BuildDiscoveryReport) -> Vec<Capability> {
    let mut caps = Vec::new();
    if !report.crates.is_empty() {
        caps.push(Capability::BuildExecute);
    }
    // A node with cynic-cli bin is also a dev station
    if report
        .crates
        .iter()
        .flat_map(|c| &c.bins)
        .any(|b| b == "cynic")
    {
        caps.push(Capability::HostSupervision);
    }
    caps
}

/// Current unix timestamp (helper).
pub fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Discovery event from build changes ───────────────────────────────────────

/// Convert a capability delta into a `DiscoveryEvent::CapabilityChanged`.
pub fn capability_delta_event(
    node_id: &str,
    before: &[Capability],
    after: &[Capability],
) -> DiscoveryEvent {
    let added: Vec<Capability> = after
        .iter()
        .filter(|c| !before.contains(c))
        .cloned()
        .collect();
    let removed: Vec<Capability> = before
        .iter()
        .filter(|c| !after.contains(c))
        .cloned()
        .collect();
    DiscoveryEvent::CapabilityChanged {
        node_id: node_id.to_string(),
        added,
        removed,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> BuildDiscoveryReport {
        BuildDiscoveryReport {
            schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
            toolchain: Toolchain("1.82.0".to_string()),
            host_triple: HostTriple("x86_64-unknown-linux-gnu".to_string()),
            targets: vec!["x86_64-unknown-linux-gnu".to_string()],
            crates: vec![
                DiscoveredCrate {
                    name: "cynic".to_string(),
                    edition: "2024".to_string(),
                    bins: vec!["cynic".to_string()],
                    libs: vec![],
                    tests: vec![],
                    custom_build: false,
                },
                DiscoveredCrate {
                    name: "cynic-host".to_string(),
                    edition: "2024".to_string(),
                    bins: vec![],
                    libs: vec!["cynic-host".to_string()],
                    tests: vec![],
                    custom_build: false,
                },
            ],
            observed_at_unix_s: 1700000000,
            residual_unknowns: vec![],
        }
    }

    #[test]
    fn bridge_build_report_to_capabilities() {
        let report = sample_report();
        let caps = build_report_to_capabilities("builder-1", "10.10.10.119:8080", &report);
        assert_eq!(caps.node_id, "builder-1");
        assert_eq!(caps.role, NodeRole::Builder);
        assert!(caps.has_capability(&Capability::BuildExecute));
        assert!(caps.has_capability(&Capability::HostSupervision));
        assert_eq!(caps.observed_at_unix_s, 1700000000);
    }

    #[test]
    fn bridge_role_orchestrator_many_crates() {
        let mut report = sample_report();
        // Add many crates to trigger Orchestrator classification
        for i in 0..15 {
            report.crates.push(DiscoveredCrate {
                name: format!("extra-crate-{i}"),
                edition: "2024".to_string(),
                bins: vec![],
                libs: vec![format!("extra-crate-{i}")],
                tests: vec![],
                custom_build: false,
            });
        }
        let caps = build_report_to_capabilities("orch-1", "10.10.10.1:8080", &report);
        assert_eq!(caps.role, NodeRole::Orchestrator);
    }

    #[test]
    fn bridge_role_unknown_no_crates() {
        let report = BuildDiscoveryReport {
            crates: vec![],
            ..sample_report()
        };
        let caps = build_report_to_capabilities("empty", "10.0.0.1", &report);
        assert_eq!(caps.role, NodeRole::Unknown);
        assert!(!caps.has_capability(&Capability::BuildExecute));
    }

    #[test]
    fn capability_delta_event_detects_changes() {
        let before = vec![Capability::BuildExecute];
        let after = vec![Capability::BuildExecute, Capability::InferenceServe];
        let event = capability_delta_event("node-1", &before, &after);
        match event {
            DiscoveryEvent::CapabilityChanged { added, removed, .. } => {
                assert_eq!(added, vec![Capability::InferenceServe]);
                assert!(removed.is_empty());
            }
            _ => panic!("expected CapabilityChanged"),
        }
    }

    #[test]
    fn schema_constants_pass_through() {
        assert_eq!(
            DISCOVERY_CAPABILITIES_SCHEMA,
            "cynic.discovery.capabilities.v1"
        );
        assert_eq!(DISCOVERY_NODE_SCHEMA, "cynic.discovery.node.v1");
        assert_eq!(DISCOVERY_EVENT_SCHEMA, "cynic.discovery.event.v1");
    }
}
