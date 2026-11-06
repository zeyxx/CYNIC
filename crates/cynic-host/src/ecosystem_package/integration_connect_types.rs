use serde::Serialize;

use crate::execution::{HostExecutionPlan, HostExecutionPlanReport};

use super::{IntegrationObservation, IntegrationPlanAction};

#[derive(Debug, Clone, Serialize)]
pub struct IntegrationConnectReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub integration_id: String,
    pub action: IntegrationPlanAction,
    pub before: IntegrationObservation,
    pub execution_plan: HostExecutionPlan,
    pub execution_report: HostExecutionPlanReport,
    pub after: IntegrationObservation,
    pub blockers: Vec<String>,
    pub rules: Vec<&'static str>,
}
