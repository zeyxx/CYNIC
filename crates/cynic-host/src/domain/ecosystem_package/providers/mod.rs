//! Concrete ecosystem package providers (adapters).
//!
//! Each tool owns its recipe. Dispatch lives here; domain types/port stay pure.

pub mod distro;
pub mod external;
pub mod llama_cpp;
pub mod ods;

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::types::{
    EcosystemPackageApplyResult, EcosystemPackagePlan, PrincipalInstallLayout,
};

use self::distro::{DOCKER, SYSTEMD};
use self::external::{CARGO, CLAUDE_CODE, CODEX_CLI, HUGGINGFACE_CLI, RUSTC};
use self::llama_cpp::LlamaCppProvider;
use self::ods::OdsProvider;

static LLAMA: LlamaCppProvider = LlamaCppProvider;
static ODS: OdsProvider = OdsProvider;

/// Look up a first-class provider by dep id.
pub fn provider(id: &str) -> Option<&'static dyn EcosystemPackageProvider> {
    match id {
        "llama.cpp" => Some(&LLAMA),
        "ods" => Some(&ODS),
        "huggingface-cli" => Some(&HUGGINGFACE_CLI),
        "codex-cli" => Some(&CODEX_CLI),
        "claude-code" => Some(&CLAUDE_CODE),
        "rustc" => Some(&RUSTC),
        "cargo" => Some(&CARGO),
        "docker" => Some(&DOCKER),
        "systemd" => Some(&SYSTEMD),
        _ => None,
    }
}

/// Build the first-class plan for a dep id (universal entry for deps plan/upgrade).
pub fn plan_for_dep(
    dep_id: &str,
    reason: &str,
    layout: &PrincipalInstallLayout,
) -> Option<EcosystemPackagePlan> {
    provider(dep_id).map(|p| p.plan(reason, layout))
}

/// Apply a known first-class provider by id.
pub fn apply_for_dep(
    dep_id: &str,
    reason: &str,
    layout: &PrincipalInstallLayout,
) -> Option<EcosystemPackageApplyResult> {
    let p = provider(dep_id)?;
    Some(p.apply(reason, layout))
}

/// Whether this id has a provider with an auto-applicable apply path.
pub fn has_auto_handler(dep_id: &str) -> bool {
    provider(dep_id)
        .map(|p| {
            matches!(
                p.channel(),
                crate::domain::ecosystem_package::types::ProviderChannel::Scripted
                    | crate::domain::ecosystem_package::types::ProviderChannel::External
            )
        })
        .unwrap_or(false)
}

// Re-export plan builders used by tests / callers that want the concrete fn.
pub use external::plan_external;
pub use llama_cpp::plan_llama_cpp;
pub use ods::plan_ods;
