//! Discovery — typed capability declaration and node observation.
//!
//! When multiple surfaces (contracts, protocol, CLI, MCP, skills, runtime)
//! need to know what the fleet *can* do (not what it *may* do — that's
//! `federation_authz`), this module provides the types to declare, observe,
//! and aggregate capabilities.
//!
//! Phase A: types only. Phase B: adapters in `cynic-discovery-host`.
//! Phase C: `/capabilities` endpoint in `cynic-kernel`.

pub mod event;
pub mod node;
pub mod port;
pub mod types;

pub use event::DiscoveryEvent;
pub use node::{FleetCapabilities, NodeCapabilities};
pub use port::{DiscoveryObservationPort, NodeDiscoveryObservationPort};
pub use types::{
    Capability, DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_EVENT_SCHEMA, DISCOVERY_NODE_SCHEMA,
    NodeRole,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_role_roundtrip() {
        for role in [
            NodeRole::Orchestrator,
            NodeRole::Builder,
            NodeRole::DevStation,
            NodeRole::InferenceProxy,
            NodeRole::Storage,
            NodeRole::Forge,
            NodeRole::Network,
            NodeRole::Unknown,
        ] {
            let s = role.as_str();
            let parsed: NodeRole = serde_json::from_str(&format!("\"{s}\"")).unwrap();
            assert_eq!(role, parsed);
        }
    }

    #[test]
    fn capability_requires_endpoint() {
        assert!(Capability::BuildExecute.requires_endpoint());
        assert!(Capability::InferenceServe.requires_endpoint());
        assert!(!Capability::FleetObserve.requires_endpoint());
        assert!(!Capability::HostSupervision.requires_endpoint());
    }

    #[test]
    fn node_capabilities_has() {
        let node = NodeCapabilities::new(
            "test-node",
            NodeRole::Builder,
            "10.0.0.1:8080",
            vec![Capability::BuildExecute, Capability::FleetObserve],
        );
        assert!(node.has_capability(&Capability::BuildExecute));
        assert!(node.has_capability(&Capability::FleetObserve));
        assert!(!node.has_capability(&Capability::InferenceServe));
    }

    #[test]
    fn fleet_capabilities_find() {
        let fleet = FleetCapabilities::new(vec![
            NodeCapabilities::new(
                "builder-1",
                NodeRole::Builder,
                "10.0.0.1:8080",
                vec![Capability::BuildExecute],
            ),
            NodeCapabilities::new(
                "orch-1",
                NodeRole::Orchestrator,
                "10.0.0.2:8080",
                vec![Capability::FleetObserve],
            ),
        ]);
        let builders = fleet.find_by_role(NodeRole::Builder);
        assert_eq!(builders.len(), 1);
        assert_eq!(builders[0].node_id, "builder-1");
        assert!(fleet.has_builder());
    }

    #[test]
    fn discovery_event_serializes() {
        let event = DiscoveryEvent::NodeJoined {
            node_id: "node-1".to_string(),
            role: NodeRole::Builder,
            capabilities: vec![Capability::BuildExecute],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"node_joined\""));
        assert!(json.contains("\"node_id\":\"node-1\""));
    }

    #[test]
    fn schema_constants_unique() {
        assert_ne!(DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_NODE_SCHEMA);
        assert_ne!(DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_EVENT_SCHEMA);
    }
}
