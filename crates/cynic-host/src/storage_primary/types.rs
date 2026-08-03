use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoragePrimaryDesiredState {
    pub node_id: String,
    pub role: String,
    pub provider: String,
    pub resource_id: String,
    pub endpoint: String,
    pub unit: String,
    pub artifact_endpoint: String,
    pub artifact_unit: String,
    pub artifact_base_path: String,
    pub schema_path: String,
    pub runtime_identity: String,
    pub secret_class: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoragePrimaryPlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub authority: &'static str,
    pub mutates: bool,
    pub desired: Option<StoragePrimaryDesiredState>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoragePrimaryBundleReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates_runtime: bool,
    pub output_dir: String,
    pub files: Vec<String>,
    pub digests: Vec<String>,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoragePrimaryStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub ready: bool,
    pub authority: &'static str,
    pub desired: Option<StoragePrimaryDesiredState>,
    pub checks: serde_json::Value,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_actions: Vec<String>,
}
