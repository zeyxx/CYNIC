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
    PythonInstallProvenance,
};
use crate::domain::ecosystem_package::{apply_package_plan, provider_spec, python_cli_policy};

use self::distro::{DOCKER, SYSTEMD};
use self::external::{CARGO, CLAUDE_CODE, CODEX_CLI, HERMES_AGENT, HUGGINGFACE_CLI, RUSTC, UV};
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
        "uv" => Some(&UV),
        "hermes-agent" => Some(&HERMES_AGENT),
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
    if let Some(policy) = python_cli_policy(dep_id) {
        let mut argv = policy
            .canonical_update_argv
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        if matches!(
            policy.expected_python_provenance,
            PythonInstallProvenance::UvTool
        ) {
            argv = prepend_uv_home_env(argv, layout);
        }
        let as_principal = provider_spec(dep_id)
            .map(|spec| {
                matches!(
                    spec.principal,
                    crate::domain::ecosystem_package::types::InstallPrincipalKind::RuntimeUser
                )
            })
            .unwrap_or(false);
        return Some(external::plan_external(
            dep_id,
            reason,
            argv,
            layout,
            as_principal,
        ));
    }
    provider(dep_id).map(|p| p.plan(reason, layout))
}

fn prepend_uv_home_env(argv: Vec<String>, layout: &PrincipalInstallLayout) -> Vec<String> {
    let mut wrapped = vec!["env".into(), format!("HOME={}", layout.home_dir.display())];

    wrapped.push(format!(
        "UV_CACHE_DIR={}",
        layout.home_dir.join(".cache/uv").display()
    ));
    wrapped.push(format!(
        "UV_TOOL_DIR={}",
        layout.home_dir.join(".local/share/uv/tools").display()
    ));
    wrapped.push(format!("PATH={}", layout.bin_dir.display()));
    wrapped.extend(argv);
    wrapped
}

/// Apply a known first-class provider by id.
pub fn apply_for_dep(
    dep_id: &str,
    reason: &str,
    layout: &PrincipalInstallLayout,
) -> Option<EcosystemPackageApplyResult> {
    if let Some(plan) = plan_for_dep(dep_id, reason, layout) {
        return Some(apply_package_plan(&plan));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ecosystem_package::types::{InstallPrincipalKind, RunAs};
    use std::path::PathBuf;

    fn principal_layout() -> PrincipalInstallLayout {
        PrincipalInstallLayout {
            principal: InstallPrincipalKind::RuntimeUser,
            runtime_user: "cynic".into(),
            home_dir: PathBuf::from("/home/test-principal"),
            bin_dir: PathBuf::from("/home/test-principal/.local/bin"),
            src_dir: PathBuf::from("/home/test-principal/.local/share/cynic/src"),
            share_dir: PathBuf::from("/home/test-principal/.local/share/cynic"),
            run_as: RunAs::Current,
            writable: true,
            notes: vec![],
        }
    }

    #[test]
    fn hermes_plan_injects_uv_runtime_home() {
        let layout = principal_layout();
        let plan = plan_for_dep("hermes-agent", "test", &layout)
            .expect("hermes-agent must have a policy-backed plan");
        assert_eq!(plan.principal, InstallPrincipalKind::RuntimeUser);
        assert!(plan.steps.iter().all(|step| step.as_principal));
        let first = &plan.steps[0].argv;
        assert_eq!(first.first(), Some(&"env".to_string()));
        assert!(first.iter().any(|arg| arg == "HOME=/home/test-principal"));
        assert!(
            first
                .iter()
                .any(|arg| arg == "UV_CACHE_DIR=/home/test-principal/.cache/uv")
        );
        assert!(
            first
                .iter()
                .any(|arg| arg == "UV_TOOL_DIR=/home/test-principal/.local/share/uv/tools")
        );
        assert!(
            first
                .iter()
                .any(|arg| arg == "PATH=/home/test-principal/.local/bin")
        );
        assert!(first.iter().any(|arg| arg == "hermes-agent"));
        assert_eq!(
            first
                .iter()
                .position(|arg| arg == "uv")
                .expect("uv command should be present"),
            5
        );
        assert_eq!(
            first
                .iter()
                .position(|arg| arg == "tool")
                .expect("uv tool arg should be present"),
            6
        );
        assert_eq!(
            first
                .iter()
                .position(|arg| arg == "install")
                .expect("uv install arg should be present"),
            7
        );
    }
}
