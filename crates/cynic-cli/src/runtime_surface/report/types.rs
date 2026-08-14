use cynic_host::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) lifecycle_status: LifecycleStatus,
    pub(crate) lifecycle_disposition: LifecycleDisposition,
    pub(crate) next_boundary_kind: BoundaryKind,
    pub(crate) next_boundary_reason: String,
    pub(crate) next_command: Option<String>,
    pub(crate) generated_at_unix_ms: u128,
    pub(crate) purpose: &'static str,
    pub(crate) phases: Vec<RuntimeReportPhase>,
    pub(crate) inference: InferenceReportSummary,
    pub(crate) sovereign_git: SovereignGitSummary,
    pub(crate) persistence: RuntimeReportPersistence,
    pub(crate) reports: Vec<RuntimeReportSurface>,
    pub(crate) rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeReportPersistence {
    pub(crate) default_path: &'static str,
    pub(crate) written_path: Option<String>,
    pub(crate) mutates_filesystem: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeReportPhase {
    pub(crate) phase: &'static str,
    pub(crate) environment: &'static str,
    pub(crate) authority: &'static str,
    pub(crate) isolation_boundary: &'static str,
    pub(crate) target_scope: &'static str,
    pub(crate) mutates_host_network: bool,
    pub(crate) required_reports: Vec<&'static str>,
    pub(crate) promotion_gate: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct InferenceReportSummary {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) config_path: Option<String>,
    pub(crate) backend_count: usize,
    pub(crate) sovereign_count: usize,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SovereignGitSummary {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) forgejo_remote_present: bool,
    pub(crate) forgejo_service_health: Option<String>,
    pub(crate) forgejo_repo_auth_readiness: Option<String>,
    pub(crate) forgejo_host_resolution_status: Option<String>,
    pub(crate) forgejo_tcp_reachability_status: Option<String>,
    pub(crate) forgejo_url: Option<String>,
    pub(crate) next_boundary_kind: BoundaryKind,
    pub(crate) next_boundary_reason: String,
    pub(crate) next_command: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeReportSurface {
    pub(crate) id: &'static str,
    pub(crate) command: &'static str,
    pub(crate) phase: &'static str,
    pub(crate) mutates: bool,
    pub(crate) purpose: &'static str,
}
