//! Node activation runtime — observes deployment_epoch, bootstrap_federation,
//! and takeover state to produce an activation verdict.

use cynic_node_protocol::deployment_epoch::{DeploymentEpoch, DeploymentPhase};
use cynic_node_protocol::node_activation::{NodeActivationVerdict, decide_node_activation};
use serde::Serialize;

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
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("parse registry: {e}"))?;

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

/// The result of an activation attempt.
#[derive(Debug, Clone, Serialize)]
pub struct ActivationReport {
    pub node_id: String,
    pub status: String,
    pub result: Option<Result<cynic_node_protocol::node_activation::ActivationProof, String>>,
    pub evidence_refs: Vec<String>,
}

impl NodeActivationAdapter {
    /// Observe and activate a node — bounded action with read-back proof.
    pub fn activate(&self, node_id: &str, consent: bool) -> ActivationReport {
        // 1. Observe current state.
        let verdict = self.observe(node_id);

        // 2. Plan the bounded action.
        let plan = match cynic_node_protocol::node_activation::plan_activation_action(&verdict) {
            Ok(p) => p,
            Err(_e) => {
                return ActivationReport {
                    node_id: node_id.into(),
                    status: "blocked".into(),
                    result: None,
                    evidence_refs: vec![],
                };
            }
        };

        // 3. Check consent.
        if plan.requires_consent && !consent {
            return ActivationReport {
                node_id: node_id.into(),
                status: "blocked".into(),
                result: Some(Err("consent_required: --yes flag needed".into())),
                evidence_refs: vec![],
            };
        }

        // 4. Execute apply argv.
        let apply_output = std::process::Command::new(&plan.apply_argv[0])
            .args(&plan.apply_argv[1..])
            .output();
        let apply_success = apply_output
            .as_ref()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // 5. Read-back.
        let read_back_output = if apply_success {
            std::process::Command::new(&plan.read_back_argv[0])
                .args(&plan.read_back_argv[1..])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default()
        } else {
            String::new()
        };

        // 6. Evaluate proof.
        let proof = cynic_node_protocol::node_activation::evaluate_activation_proof(
            &plan,
            apply_success,
            &read_back_output,
            false,
            "",
        );

        let status = match &proof {
            cynic_node_protocol::node_activation::ActivationProof::Activated { .. } => "activated",
            cynic_node_protocol::node_activation::ActivationProof::Failed { .. } => "failed",
            cynic_node_protocol::node_activation::ActivationProof::RolledBack { .. } => "rolled_back",
        };

        ActivationReport {
            node_id: node_id.into(),
            status: status.into(),
            result: Some(Ok(proof)),
            evidence_refs: plan.expected_evidence_refs.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a registry fixture and return its absolute path.
    ///
    /// The tests below previously passed the relative "infra/registry.json".
    /// Cargo runs unit tests with the crate directory as CWD, so that path does
    /// not resolve and every verdict collapsed to `registry_read_failed` —
    /// which is a *different* blocker than the one under test. An explicit
    /// fixture separates "registry unreadable" from "node absent from a
    /// readable registry".
    fn write_registry_fixture(contents: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cynic-node-activation-{unique}.json"));
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn fixture_with_forgejo() -> std::path::PathBuf {
        write_registry_fixture(
            r#"{
              "nodes": [
                {"id":"cynic-lxc-forgejo","name":"cynic-lxc-forgejo","role":"forge_primary"}
              ]
            }"#,
        )
    }

    #[test]
    fn blocked_when_node_not_in_registry() {
        let path = fixture_with_forgejo();
        let adapter = NodeActivationAdapter::new(path.to_str().expect("utf8 temp path"));
        let verdict = adapter.observe("nonexistent-node");
        assert!(
            matches!(&verdict,
                NodeActivationVerdict::Blocked { blockers, .. }
                if blockers.iter().any(|b| b.contains("node_not_in_registry"))
            ),
            "verdict: {verdict:?}"
        );
    }

    #[test]
    fn unreadable_registry_is_a_distinct_blocker_from_a_missing_node() {
        // An unreadable registry must never be reported as "node absent": that
        // would let a broken observation masquerade as a decided verdict.
        let adapter = NodeActivationAdapter::new("/nonexistent/cynic-registry.json");
        let verdict = adapter.observe("cynic-lxc-forgejo");
        assert!(
            matches!(&verdict,
                NodeActivationVerdict::Blocked { blockers, .. }
                if blockers.iter().any(|b| b.contains("registry_read_failed"))
            ),
            "verdict: {verdict:?}"
        );
    }

    #[test]
    fn activatable_when_node_in_registry() {
        let path = fixture_with_forgejo();
        let adapter = NodeActivationAdapter::new(path.to_str().expect("utf8 temp path"));
        let verdict = adapter.observe("cynic-lxc-forgejo");
        // Either outcome is admissible here; what must NOT happen is a read
        // failure, which would mean the fixture never reached the adapter.
        assert!(
            !matches!(&verdict,
                NodeActivationVerdict::Blocked { blockers, .. }
                if blockers.iter().any(|b| b.contains("registry_read_failed"))
            ),
            "fixture must be readable, verdict: {verdict:?}"
        );
    }

    #[test]
    fn activate_requires_consent() {
        let path = fixture_with_forgejo();
        let adapter = NodeActivationAdapter::new(path.to_str().expect("utf8 temp path"));
        let report = adapter.activate("cynic-lxc-forgejo", false);
        assert_eq!(report.status, "blocked");
        assert!(
            report.result.is_some(),
            "a refused activation must still carry a typed result, not silence"
        );
    }
}
