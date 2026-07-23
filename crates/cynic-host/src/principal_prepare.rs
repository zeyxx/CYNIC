mod admitted;

use serde::Serialize;

use crate::domain::principal::PrincipalPolicy;
use crate::execution::{HostOperationPlan, build_principal_operation_plan};
use crate::registry::HostProfileResolution;

pub use admitted::{
    PRINCIPAL_PREPARE_EXECUTION_SCHEMA, PrincipalPrepareExecutionReport,
    PrincipalPrepareOperationReport, PrincipalPrepareOperationResult, principal_prepare_apply,
    principal_prepare_preconditions,
};

pub const PRINCIPAL_PREPARE_PLAN_SCHEMA: &str = "cynic.host.principal_prepare_plan.v1";

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPreparePlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub policy: PrincipalPreparePolicyView,
    pub operation_plan: HostOperationPlan,
    pub blockers: Vec<String>,
    pub safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPreparePolicyView {
    pub runtime_user: String,
    pub runtime_group: String,
    pub home_dir: String,
    pub config_dir: String,
    pub state_dirs: Vec<String>,
}

pub fn principal_prepare_plan(policy: &PrincipalPolicy) -> PrincipalPreparePlanReport {
    principal_prepare_plan_with_resolution(policy, None)
}

pub fn principal_prepare_plan_with_resolution(
    policy: &PrincipalPolicy,
    resolution: Option<&HostProfileResolution>,
) -> PrincipalPreparePlanReport {
    PrincipalPreparePlanReport {
        schema: PRINCIPAL_PREPARE_PLAN_SCHEMA,
        status: "planned",
        mutates: false,
        policy: policy_view(policy),
        operation_plan: operation_plan(policy, resolution),
        blockers: Vec::new(),
        safety_rules: vec![
            "principal preparation requires explicit human-admin authority",
            "domain plans contain typed operations, never provider commands",
            "execution stops on first failure",
            "completion requires postflight identity, path, and ownership proofs",
        ],
    }
}

pub(super) fn operation_plan(
    policy: &PrincipalPolicy,
    resolution: Option<&HostProfileResolution>,
) -> HostOperationPlan {
    build_principal_operation_plan(policy, execution_target(resolution))
}

pub(super) fn execution_target(resolution: Option<&HostProfileResolution>) -> String {
    resolution
        .and_then(|resolution| resolution.matched_node.clone())
        .or_else(|| resolution.map(|resolution| resolution.hostname_observed.clone()))
        .unwrap_or_else(|| "local-host".to_string())
}

pub(super) fn policy_view(policy: &PrincipalPolicy) -> PrincipalPreparePolicyView {
    PrincipalPreparePolicyView {
        runtime_user: policy.runtime_user.clone(),
        runtime_group: policy.runtime_group.clone(),
        home_dir: policy.home_dir.clone(),
        config_dir: policy.config_dir.clone(),
        state_dirs: policy.state_dirs.clone(),
    }
}

#[cfg(test)]
mod tests;
