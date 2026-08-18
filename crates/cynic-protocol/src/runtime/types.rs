use serde::Serialize;

pub const RUNTIME_EXECUTION_SUITE_SCHEMA: &str = "cynic.runtime_execution_suite.v1";
pub const RUNTIME_EXECUTION_PLAN_SCHEMA: &str = "cynic.runtime_execution_plan.v1";
pub const RUNTIME_PROOF_SCHEMA: &str = "cynic.runtime_proof.v1";
pub const RUNTIME_PROOF_BUNDLE_SCHEMA: &str = "cynic.runtime_proof_bundle.v1";
pub const RUNTIME_PROOF_PERSISTENCE_SCHEMA: &str = "cynic.runtime_proof_persistence.v1";
pub const INFERENCE_STATUS_PROOF_SCHEMA: &str = "cynic.inference.status.v1";

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeExecutionSuite {
    pub schema: &'static str,
    pub status: &'static str,
    pub registry_path: String,
    pub profile: String,
    pub rule: &'static str,
    pub source_prisms: Vec<&'static str>,
    pub cloud_characteristics: Vec<&'static str>,
    pub deployment_models: Vec<&'static str>,
    pub service_models: Vec<&'static str>,
    pub registry_summary: RuntimeRegistrySummary,
    pub layers: Vec<RuntimeExecutionLayer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeExecutionPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub registry_path: String,
    pub profile: String,
    pub suite_schema: &'static str,
    pub rule: &'static str,
    pub proof_model: &'static str,
    pub registry_summary: RuntimeRegistrySummary,
    pub observed_gates: Vec<String>,
    pub layers: Vec<RuntimePlannedExecutionLayer>,
    pub executable_commands: Vec<RuntimeExecutionCommand>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeRegistrySummary {
    pub node_count: usize,
    pub roles: Vec<String>,
    pub service_count: usize,
    pub dns_zone: Option<String>,
    pub provider_cidr: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeExecutionLayer {
    pub id: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub cloud_model: &'static str,
    pub service_model: &'static str,
    pub deployment_model: &'static str,
    pub shared_responsibility: RuntimeSharedResponsibility,
    pub mutability: &'static str,
    pub cloud_characteristics: Vec<&'static str>,
    pub risk_controls: Vec<&'static str>,
    pub entry_gate: Vec<&'static str>,
    pub read_only_probe: Vec<String>,
    pub dry_run: Option<Vec<String>>,
    pub apply: Option<Vec<String>>,
    pub exit_gate: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimePlannedExecutionLayer {
    pub id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub reason: String,
    pub entry_gate: Vec<&'static str>,
    pub missing_entry_gate: Vec<&'static str>,
    pub read_only_probe: RuntimeExecutionCommand,
    pub dry_run: Option<RuntimeExecutionCommand>,
    pub apply: Option<RuntimeExecutionCommand>,
    pub exit_gate: Vec<&'static str>,
    pub can_execute_read_only: bool,
    pub can_dry_run: bool,
    pub can_apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeExecutionCommand {
    pub kind: &'static str,
    pub argv: Vec<String>,
    pub mutates: bool,
    pub implemented: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSharedResponsibility {
    pub operator: Vec<&'static str>,
    pub provider_or_substrate: Vec<&'static str>,
}

pub fn runtime_shared_responsibility(
    operator: &[&'static str],
    provider_or_substrate: &[&'static str],
) -> RuntimeSharedResponsibility {
    RuntimeSharedResponsibility {
        operator: operator.to_vec(),
        provider_or_substrate: provider_or_substrate.to_vec(),
    }
}
