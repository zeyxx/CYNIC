//! LifecyclePort — formal port for lifecycle validation, workflow state, and transition contracts.
//!
//! In accordance with foundation §481 (one_boundary_one_reason) and framework §472/§476,
//! this port decouples higher-layer consumers (kernel, CLI, MCP) from direct static
//! function calls to cynic-host lifecycle implementation modules.

use serde_json::{Map, Value};

/// Formal port interface for host lifecycle governance, workflow validation, and diagnostics.
pub trait LifecyclePort: Send + Sync {
    /// Validates layered lifecycle constraints for an orchestrator organ.
    fn validate_workflow_layers(&self, workflow: &Value, orchestrator_organ: &str) -> Vec<String>;

    /// Validates transition contracts between lifecycle states.
    fn validate_transition_contract(&self, workflow: &Value) -> Vec<String>;

    /// Validates forge task lifecycle contracts against declared registry.
    fn validate_forge_lifecycle_contract(
        &self,
        content: &Value,
        registry: Option<&Value>,
    ) -> Result<(), String>;

    /// Identifies active lifecycle blockers in a registry snapshot.
    fn lifecycle_blockers(
        &self,
        registry: &Value,
        fleet_probe: Option<&Value>,
        fleet_inventory_probe: Option<&Value>,
        env: Option<&Map<String, Value>>,
    ) -> Result<Value, String>;

    /// Identifies activation evidence gaps in a registry snapshot.
    fn activation_evidence_gaps(
        &self,
        registry: &Value,
        env: Option<&Map<String, Value>>,
    ) -> Result<Value, String>;
}

/// Local/canonical implementation of `LifecyclePort` delegating to cynic-host domain modules.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalLifecycleAdapter;

impl LifecyclePort for LocalLifecycleAdapter {
    fn validate_workflow_layers(&self, workflow: &Value, orchestrator_organ: &str) -> Vec<String> {
        crate::lifecycle::workflow_state::validate_workflow_layers(workflow, orchestrator_organ)
    }

    fn validate_transition_contract(&self, workflow: &Value) -> Vec<String> {
        crate::lifecycle::workflow_state::validate_transition_contract(workflow)
    }

    fn validate_forge_lifecycle_contract(
        &self,
        content: &Value,
        registry: Option<&Value>,
    ) -> Result<(), String> {
        crate::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(content, registry)
    }

    fn lifecycle_blockers(
        &self,
        registry: &Value,
        fleet_probe: Option<&Value>,
        fleet_inventory_probe: Option<&Value>,
        env: Option<&Map<String, Value>>,
    ) -> Result<Value, String> {
        crate::lifecycle::diagnostics::lifecycle_blockers(
            registry,
            fleet_probe,
            fleet_inventory_probe,
            env,
        )
    }

    fn activation_evidence_gaps(
        &self,
        registry: &Value,
        env: Option<&Map<String, Value>>,
    ) -> Result<Value, String> {
        crate::lifecycle::diagnostics::activation_evidence_gaps(registry, env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn local_adapter_implements_lifecycle_port() {
        let adapter = LocalLifecycleAdapter;
        let port: &dyn LifecyclePort = &adapter;

        // Test transition contract validation on minimal workflow
        let workflow = json!({
            "states": {}
        });
        let transitions = port.validate_transition_contract(&workflow);
        assert!(!transitions.is_empty());
    }
}
