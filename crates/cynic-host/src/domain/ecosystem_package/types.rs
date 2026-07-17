//! Universal contract types for ecosystem package providers.
//!
//! No provider recipes here — only the shared vocabulary for observe/plan/apply/proof.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How CYNIC is allowed to apply an upgrade for this package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderChannel {
    /// First-class CYNIC argv plan under human consent (`--yes` / confirm token).
    Scripted,
    /// Vendor/self-update CLI that CYNIC may invoke (e.g. `hf update`).
    External,
    /// OS/distro ownership — plan documents path; apply stays with package manager.
    DistroManaged,
}

/// Which principal owns files and process identity for the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPrincipalKind {
    /// Runtime user declared in runtime authority (`cynic`).
    RuntimeUser,
    /// Current process owner user-local tree (`$HOME/.local`).
    OperatorLocal,
    /// Root / system paths (`/usr`, `/opt` under distro control).
    SystemManaged,
}

/// How to execute argv for the principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunAs {
    /// Current euid already is (or can write as) the principal.
    Current,
    /// `sudo -u <user> -- <argv…>` (passwordless preferred).
    SudoUser(String),
}

/// Resolved install layout under the chosen principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalInstallLayout {
    pub principal: InstallPrincipalKind,
    pub runtime_user: String,
    pub home_dir: PathBuf,
    /// `$HOME/.local/bin` (or operator equivalent).
    pub bin_dir: PathBuf,
    /// `$HOME/.local/share/cynic/src` — git checkouts / build trees.
    pub src_dir: PathBuf,
    /// `$HOME/.local/share/cynic` — shared state for ecosystem tooling.
    pub share_dir: PathBuf,
    pub run_as: RunAs,
    /// True when apply can write to principal paths (directly or via sudo -u).
    pub writable: bool,
    pub notes: Vec<String>,
}

/// One argv step in a plan/apply chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyStep {
    pub role: String,
    pub argv: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub mutates: bool,
    /// True when the step needs elevated host privilege beyond principal sudo -u.
    pub privilege_required: bool,
    /// When true, wrap argv with principal `run_as` (sudo -u cynic).
    pub as_principal: bool,
}

/// Desired mutation plan for one ecosystem package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcosystemPackagePlan {
    pub dep_id: String,
    pub channel: ProviderChannel,
    pub principal: InstallPrincipalKind,
    pub layout: PrincipalInstallLayout,
    pub reason: String,
    pub steps: Vec<ApplyStep>,
    /// Human-consent apply may run these steps.
    pub auto_applicable: bool,
    pub proofs: Vec<String>,
    /// Human-readable summary lines (also used in `cynic deps plan`).
    pub summary_commands: Vec<String>,
}

/// Result of applying one package plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcosystemPackageApplyResult {
    pub dep_id: String,
    pub attempted: bool,
    pub ok: bool,
    pub channel: ProviderChannel,
    pub principal: InstallPrincipalKind,
    pub steps_run: Vec<StepExecution>,
    pub message: String,
    pub proofs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepExecution {
    pub role: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub ok: bool,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

/// Observation of a single binary under the universal probe model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedBinary {
    pub path: Option<String>,
    pub version: Option<String>,
    /// true when path is under principal layout (preferred).
    pub under_principal: bool,
}

/// True when `cynic deps upgrade --yes` may run a first-class apply path.
pub fn channel_auto_applicable(channel: ProviderChannel) -> bool {
    matches!(
        channel,
        ProviderChannel::Scripted | ProviderChannel::External
    )
}
