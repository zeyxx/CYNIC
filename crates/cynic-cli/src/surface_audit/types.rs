use serde::Serialize;

use crate::agent_exposure_audit::AgentExposureAuditReport;

#[derive(Serialize)]
pub(crate) struct SurfaceAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) surface_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) surfaces: Vec<SurfaceAuditEntry>,
    pub(crate) external_projection_findings: Vec<ExternalProjectionFinding>,
    pub(crate) agent_exposure: AgentExposureAuditReport,
}

#[derive(Serialize)]
pub(crate) struct SurfaceAuditEntry {
    pub(crate) id: String,
    pub(crate) status: &'static str,
    pub(crate) owner_crate: String,
    pub(crate) domain_owner: String,
    pub(crate) adapter_module: String,
    pub(crate) adapter_path: String,
    pub(crate) owner_module_paths: Vec<String>,
    pub(crate) mutation_policy: String,
    pub(crate) checks: Vec<SurfaceAuditCheck>,
}

#[derive(Serialize)]
pub(crate) struct SurfaceAuditCheck {
    pub(crate) name: &'static str,
    pub(crate) status: &'static str,
    pub(crate) detail: String,
}

#[derive(Serialize)]
pub(crate) struct ExternalProjectionFinding {
    pub(crate) id: &'static str,
    pub(crate) status: &'static str,
    pub(crate) projection_class: &'static str,
    pub(crate) file_path: String,
    pub(crate) evidence: Vec<&'static str>,
    pub(crate) detail: String,
}
