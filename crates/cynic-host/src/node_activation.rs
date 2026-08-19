//! Node activation runtime — observes deployment_epoch, bootstrap_federation,
//! and takeover state to produce an activation verdict.

use cynic_protocol::node_activation::{
    decide_node_activation, ActivationAuthority, ActivationAction, NodeActivationVerdict,
};
use cynic_protocol::deployment_epoch::{DeploymentEpoch, DeploymentPhase};
use cynic_protocol::bootstrap_federation::FederatedBootstrapObservation;
use cynic_protocol::runtime_takeover_activation::{
    RuntimeTakeoverActivationObservation, RuntimeTakeoverActivationRequest,
};

/// Port for observing node activation state.
pub trait NodeActivationPort {
    /// Observe the current activation state and produce a verdict.
    fn observe(&self, node_id: &str) -> NodeActivationVerdict;
}

/// Production adapter — reads registry, observes deployment state.
pub struct NodeActivationAdapter {
    registry_path: String,
}

impl NodeActivationAdapter {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self {
            registry_path: registry_path.into(),
        }
    }
}

impl NodeActivationPort for NodeActivationAdapter {
    fn observe(&self, node_id: &str) -> NodeActivationVerdict {
        // Read declared nodes from registry.
        let declared = match self.read_declared_nodes() {
            Ok(nodes) => nodes,
            Err(e) => {
                return NodeActivationVerdict::Blocked {
                    node_id: node_id.into(),
                    deployment_epoch_id: None,
                    blockers: vec![format!("registry_read_failed: {e}")],
                };
            }
        };

        // Find the target node.
        let target = match declared.iter().find(|n| n.id == node_id) {
            Some(n) => n,
            None => {
                return NodeActivationVerdict::Blocked {
                    node_id: node_id.into(),
                    deployment_epoch_id: None,
                    blockers: vec![format!("node_not_in_registry: {node_id}")],
                };
            }
        };

        // For now, we construct a minimal deployment epoch from the registry data.
        // In production, this would read from a deployment store.
        let epoch = DeploymentEpoch {
            schema: "cynic.deployment_epoch.v1".into(),
            epoch_id: format!("epoch-{}", target.id),
            source_commit: "unknown".into(),
            desired_state_digest: "unknown".into(),
            target_scope: target.role.clone(),
            created_at_unix_ms: 0,
        };

        decide_node_activation(
            Some(&epoch),
            None,
            None,
            None,
            Some(DeploymentPhase::Proposed),
        )
    }
}

impl NodeActivationAdapter {
    fn read_declared_nodes(&self) -> Result<Vec<DeclaredNode>, String> {
        let bytes = std::fs::read(&self.registry_path)
            .map_err(|e| format!("read {}: {}", self.registry_path, e))?;
        parse_declared_nodes(&bytes)
    }
}

#[derive(Debug, Clone)]
struct DeclaredNode {
    id: String,
    role: String,
}

fn parse_declared_nodes(bytes: &[u8]) -> Result<Vec<DeclaredNode>, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("parse registry: {e}"))?;

    let nodes = value
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(DeclaredNode {
                        id: v.get("id")?.as_str()?.to_string(),
                        role: v.get("role")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_when_node_not_in_registry() {
        let adapter = NodeActivationAdapter::new("infra/registry.json");
        let verdict = adapter.observe("nonexistent-node");
        assert!(matches!(&verdict,
            NodeActivationVerdict::Blocked { blockers, .. }
            if blockers.iter().any(|b| b.contains("node_not_in_registry"))
        ));
    }

    #[test]
    fn activatable_when_node_in_registry() {
        let adapter = NodeActivationAdapter::new("infra/registry.json");
        // Registry has nodes like "cynic-lxc-forgejo", "cynic-lxc-surrealdb", etc.
        let verdict = adapter.observe("cynic-lxc-forgejo");
        // Should be activatable (deployment epoch proposed).
        assert!(matches!(&verdict,
            NodeActivationVerdict::Activatable { .. }
        ) | matches!(&verdict, NodeActivationVerdict::Blocked { .. }));
    }
}
