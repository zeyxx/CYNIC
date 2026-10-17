use cynic_host::domain::ecosystem_package::{
    InstallPrincipalKind, ObservedPythonTooling, ProviderChannel, PythonInstallProvenance,
    ToolingAuthorityFamily, ToolingInstallProvenance,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct PinsFile {
    pub(crate) pins: Vec<PinEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PinEntry {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) min_version: Option<String>,
    #[serde(default)]
    pub(crate) min_build: Option<u64>,
    #[serde(default)]
    pub(crate) desired_build: Option<u64>,
    #[serde(default)]
    pub(crate) min_capability: Option<String>,
    #[serde(default)]
    pub(crate) required: Option<bool>,
    #[serde(default)]
    pub(crate) notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepWatch {
    pub(crate) upstream: Option<&'static str>,
    pub(crate) check_hint: &'static str,
    pub(crate) update_hint: &'static str,
    pub(crate) update_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepEntry {
    pub(crate) id: &'static str,
    pub(crate) role: &'static str,
    pub(crate) category: &'static str,
    pub(crate) tier: &'static str,
    pub(crate) path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) status: String,
    pub(crate) upgrade_mode: UpgradeMode,
    pub(crate) channel: ProviderChannel,
    pub(crate) install_principal: InstallPrincipalKind,
    pub(crate) under_principal: bool,
    pub(crate) tooling_authority_family: Option<ToolingAuthorityFamily>,
    pub(crate) tooling_install_provenance: Option<ToolingInstallProvenance>,
    pub(crate) expected_tooling_install_provenance: Option<ToolingInstallProvenance>,
    pub(crate) python_install_provenance: Option<PythonInstallProvenance>,
    pub(crate) expected_python_install_provenance: Option<PythonInstallProvenance>,
    pub(crate) install_authority_status: String,
    pub(crate) canonical_update_authority: Option<PythonCliUpdateAuthorityView>,
    pub(crate) pin: Option<PinView>,
    pub(crate) watch: DepWatch,
}

#[derive(Debug, Serialize)]
pub(crate) struct PythonCliUpdateAuthorityView {
    pub(crate) observed_payload_provenance: Option<String>,
    pub(crate) expected_payload_provenance: Option<String>,
    pub(crate) canonical_update_command: String,
    pub(crate) update_authority_note: String,
    pub(crate) normalizable_now: bool,
    pub(crate) requires_uv: bool,
    pub(crate) requires_runtime_principal: bool,
    pub(crate) can_auto_apply: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct PinView {
    pub(crate) min_version: Option<String>,
    pub(crate) min_build: Option<u64>,
    pub(crate) desired_build: Option<u64>,
    pub(crate) min_capability: Option<String>,
    pub(crate) notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepsReport {
    pub(crate) schema: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) pins_path: String,
    pub(crate) pins_loaded: bool,
    pub(crate) host_path_note: String,
    pub(crate) runtime_layout: PrincipalLayoutView,
    pub(crate) python_tooling: PythonToolingView,
    pub(crate) tooling_authorities: Vec<ToolingAuthorityView>,
    pub(crate) deps: Vec<DepEntry>,
    pub(crate) summary: DepsSummary,
    pub(crate) next: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PrincipalLayoutView {
    pub(crate) runtime_user: String,
    pub(crate) home_dir: String,
    pub(crate) bin_dir: String,
    pub(crate) src_dir: String,
    pub(crate) writable: bool,
    pub(crate) run_as: String,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PythonToolingView {
    pub(crate) uv_path: Option<String>,
    pub(crate) uv_under_principal: bool,
    pub(crate) python3_path: Option<String>,
    pub(crate) python3_under_principal: bool,
    pub(crate) uv_tools_dir: String,
    pub(crate) principal_local_bin_dir: String,
    pub(crate) notes: Vec<String>,
}

impl From<ObservedPythonTooling> for PythonToolingView {
    fn from(value: ObservedPythonTooling) -> Self {
        Self {
            uv_path: value.uv.path,
            uv_under_principal: value.uv.under_principal,
            python3_path: value.python3.path,
            python3_under_principal: value.python3.under_principal,
            uv_tools_dir: value.uv_tools_dir.display().to_string(),
            principal_local_bin_dir: value.principal_local_bin_dir.display().to_string(),
            notes: value.notes,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ToolingAuthorityView {
    pub(crate) dep_id: String,
    pub(crate) family: String,
    pub(crate) observed_provenance: Option<String>,
    pub(crate) expected_provenance: Option<String>,
    pub(crate) status: String,
    pub(crate) path: Option<String>,
    pub(crate) under_principal: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepsSummary {
    pub(crate) observed: usize,
    pub(crate) missing: usize,
    pub(crate) stale: usize,
    pub(crate) desired_upgrade: usize,
    pub(crate) noncanonical: usize,
    pub(crate) ok: usize,
    pub(crate) required_blocked: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct DepsPlanReport {
    pub(crate) schema: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) pins_path: String,
    pub(crate) actions: Vec<PlanAction>,
    pub(crate) copy_paste: Vec<String>,
    pub(crate) frictionless: FrictionlessHints,
}

#[derive(Debug, Serialize)]
pub(crate) struct PlanAction {
    pub(crate) priority: u32,
    pub(crate) dep_id: String,
    pub(crate) reason: String,
    pub(crate) commands: Vec<String>,
    pub(crate) mutates: bool,
    pub(crate) upgrade_mode: UpgradeMode,
    pub(crate) channel: ProviderChannel,
    pub(crate) install_principal: InstallPrincipalKind,
    pub(crate) auto_applicable: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct FrictionlessHints {
    pub(crate) after_cli_change: &'static str,
    pub(crate) daily_check: &'static str,
    pub(crate) model_start: &'static str,
    pub(crate) pins_edit: &'static str,
}

pub(crate) struct DepSpec {
    pub(crate) id: &'static str,
    pub(crate) role: &'static str,
    pub(crate) category: &'static str,
    pub(crate) tier: &'static str,
    pub(crate) bins: &'static [&'static str],
    pub(crate) version_args: &'static [&'static str],
    pub(crate) version_from: VersionFrom,
    pub(crate) upstream: Option<&'static str>,
    pub(crate) tooling_authority_family: Option<ToolingAuthorityFamily>,
    pub(crate) python_distribution: Option<&'static str>,
    pub(crate) expected_tooling_provenance: Option<ToolingInstallProvenance>,
    pub(crate) expected_python_provenance: Option<PythonInstallProvenance>,
    pub(crate) check_hint: &'static str,
    pub(crate) update_hint: &'static str,
    pub(crate) update_commands: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpgradeMode {
    Scripted,
    External,
    DistroManaged,
}

pub(crate) enum VersionFrom {
    StdoutFirstLine,
    JsonField(&'static str),
    HelpPresence,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpgradeReport {
    pub(crate) schema: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) confirm_token: String,
    pub(crate) confirm_required: bool,
    pub(crate) human_validation: HumanValidation,
    pub(crate) candidates: Vec<UpgradeCandidate>,
    pub(crate) results: Vec<UpgradeResult>,
    pub(crate) after: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HumanValidation {
    pub(crate) steps: Vec<&'static str>,
    pub(crate) apply_command_template: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct UpgradeCandidate {
    pub(crate) dep_id: String,
    pub(crate) status: String,
    pub(crate) upgrade_mode: UpgradeMode,
    pub(crate) auto_applicable: bool,
    pub(crate) reason: String,
    pub(crate) commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpgradeResult {
    pub(crate) dep_id: String,
    pub(crate) attempted: bool,
    pub(crate) ok: bool,
    pub(crate) argv: Vec<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout_tail: String,
    pub(crate) stderr_tail: String,
    pub(crate) message: String,
}
