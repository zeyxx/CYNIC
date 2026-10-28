use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DnsActivationFileSpec {
    pub source: String,
    pub target: String,
    pub sha256: String,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DnsActivationSpec {
    pub schema: &'static str,
    pub zone: String,
    pub bundle_dir: String,
    pub provider: String,
    pub provider_package: String,
    pub provider_provenance_proved: bool,
    pub service_unit: String,
    pub resolver_unit: String,
    pub service_manager_kind: String,
    pub service_manager_bin: String,
    pub resolver_bin: String,
    pub probe_fqdn: String,
    pub files: Vec<DnsActivationFileSpec>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DnsActivationFileObservation {
    pub target: String,
    pub present: bool,
    pub regular: bool,
    pub sha256: Option<String>,
    pub mode: Option<u32>,
    pub owner_uid: Option<u32>,
    pub owner_gid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DnsActivationObservation {
    pub schema: &'static str,
    pub files: Vec<DnsActivationFileObservation>,
    pub service_active: Option<bool>,
    pub service_enabled: Option<bool>,
    pub probe_resolves: Option<bool>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DnsActivationAction {
    InstallFile { source: String, target: String },
    ExecuteArgv { argv: Vec<String> },
    ObserveReadBack,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DnsActivationPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub zone: String,
    pub provider: String,
    pub actions: Vec<DnsActivationAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DnsActivationApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub plan: DnsActivationPlan,
    pub before: DnsActivationObservation,
    pub after: Option<DnsActivationObservation>,
    pub rollback_after: Option<DnsActivationObservation>,
    pub executed_argv: Vec<Vec<String>>,
    pub rolled_back: bool,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}
