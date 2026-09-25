use serde::Serialize;

use crate::command_surface::HostCommandSurface;

use super::integration_types::{
    IntegrationDependencyView, IntegrationPrincipal, IntegrationToolingAuthorityView,
};

<<<<<<< HEAD
pub use cynic_pkg::integration::decision::IntegrationPlanAction;
=======
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationPlanAction {
    None,
    RepairServiceReachability,
    ConnectOperatorCli,
    ConnectRuntimeCli,
    AdoptRuntimeCli,
}
>>>>>>> github/main

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationExecutionPlan {
    pub preflight: Vec<HostCommandSurface>,
    pub apply: Vec<HostCommandSurface>,
    pub observe: Vec<HostCommandSurface>,
    pub proofs_required: Vec<String>,
    pub rollback_intent: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationPlanItem {
    pub id: String,
    pub current_status: String,
    pub action: IntegrationPlanAction,
    pub requires_apply: bool,
    pub privilege_required: bool,
    pub dependency: Option<IntegrationDependencyView>,
    pub desired_principal: IntegrationPrincipal,
    pub observed_principal: Option<IntegrationPrincipal>,
    pub execution: IntegrationExecutionPlan,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationsPlanSummary {
    pub total: usize,
    pub no_action: usize,
    pub planned_actions: usize,
    pub privilege_required: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationsPlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub runtime_user: String,
    pub operator_user: String,
    pub integrations: Vec<IntegrationPlanItem>,
    pub dependency_tooling_authorities: Vec<IntegrationToolingAuthorityView>,
    pub summary: IntegrationsPlanSummary,
    pub rules: Vec<&'static str>,
}
