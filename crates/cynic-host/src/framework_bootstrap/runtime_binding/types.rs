use serde::Serialize;

pub const RUNTIME_BINDING_PLAN_SCHEMA: &str = "cynic.runtime_binding.plan.v1";
pub const RUNTIME_BINDING_REPORT_SCHEMA: &str = "cynic.runtime_binding.apply_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBindingSpec {
    pub authority_role: String,
    pub service_name: String,
    pub unit_name: String,
    pub scheme: String,
    pub endpoint_host: String,
    pub port: u16,
    pub network_plane: String,
    pub environment_path: String,
    pub service_unit_path: String,
    pub service_manager_argv: String,
    pub owner_id: u32,
    pub group_id: u32,
}

impl RuntimeBindingSpec {
    pub fn rest_addr(&self) -> String {
        format!("{}:{}", self.endpoint_host, self.port)
    }

    pub fn endpoint_url(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.endpoint_host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeBindingObservation {
    pub schema: &'static str,
    pub environment_present: bool,
    pub observed_rest_addr: Option<String>,
    pub environment_binding_matches: Option<bool>,
    pub service_unit_present: bool,
    pub service_unit_consumes_environment: Option<bool>,
    pub endpoint_reachable: Option<bool>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBindingAction {
    WriteEnvironmentProjection,
    RestartService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeBindingPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub authority_role: String,
    pub service_name: String,
    pub unit_name: String,
    pub endpoint_url: String,
    pub network_plane: String,
    pub actions: Vec<RuntimeBindingAction>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub required_proofs: Vec<&'static str>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeBindingCommandExecution {
    pub argv: Vec<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeBindingApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub plan: RuntimeBindingPlan,
    pub before: RuntimeBindingObservation,
    pub executions: Vec<RuntimeBindingCommandExecution>,
    pub after: RuntimeBindingObservation,
    pub proofs_observed: Vec<&'static str>,
    pub blockers: Vec<String>,
    pub rollback_performed: bool,
}
