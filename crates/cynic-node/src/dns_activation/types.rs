use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DnsActivationFileSpec {
    pub source: String,
    pub target: String,
    pub sha256: String,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DnsActivationSpec {
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
pub(crate) struct DnsActivationFileObservation {
    pub target: String,
    pub present: bool,
    pub regular: bool,
    pub sha256: Option<String>,
    pub mode: Option<u32>,
    pub owner_uid: Option<u32>,
    pub owner_gid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DnsActivationObservation {
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
pub(crate) enum DnsActivationAction {
    InstallFile { source: String, target: String },
    ExecuteArgv { argv: Vec<String> },
    ObserveReadBack,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DnsActivationPlan {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) zone: String,
    pub(crate) provider: String,
    pub(crate) actions: Vec<DnsActivationAction>,
    pub(crate) blockers: Vec<String>,
    pub(crate) unknowns: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsActivationApplyReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) plan: DnsActivationPlan,
    pub(crate) before: DnsActivationObservation,
    pub(crate) after: Option<DnsActivationObservation>,
    pub(crate) rollback_after: Option<DnsActivationObservation>,
    pub(crate) executed_argv: Vec<Vec<String>>,
    pub(crate) rolled_back: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) rollback_intent: &'static str,
}
