//! Universal ecosystem package interface for operator tech-watch deps.
//!
//! Layering (not a monolith):
//! - [`types`] — shared contract vocabulary
//! - [`layout`] — principal install paths (user `cynic`)
//! - [`port`] — [`EcosystemPackageProvider`] + argv apply engine
//! - [`registry`] — id → channel/principal metadata
//! - [`probe`] — principal-aware binary resolve
//! - [`providers`] — one adapter per tool (llama, ods, external, distro)
//!
//! Rule: nothing is permanently "manual-only". Channels are
//! `scripted` | `external` | `distro_managed`.

pub mod layout;
pub mod port;
pub mod probe;
pub mod providers;
pub mod registry;
pub mod types;

// ── Stable public surface (CLI + tests) ─────────────────────────────────────

pub use layout::resolve_principal_install_layout;
pub use port::{EcosystemPackageProvider, apply_package_plan, materialize_argv};
pub use probe::{resolve_binary, which};
pub use providers::{apply_for_dep, has_auto_handler, plan_for_dep, plan_external, plan_llama_cpp, plan_ods};
pub use registry::{EcosystemProviderSpec, ecosystem_provider_specs, provider_spec};
pub use types::{
    ApplyStep, EcosystemPackageApplyResult, EcosystemPackagePlan, InstallPrincipalKind,
    ObservedBinary, PrincipalInstallLayout, ProviderChannel, RunAs, StepExecution,
    channel_auto_applicable,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_auto_applicable_not_distro() {
        assert!(channel_auto_applicable(ProviderChannel::Scripted));
        assert!(channel_auto_applicable(ProviderChannel::External));
        assert!(!channel_auto_applicable(ProviderChannel::DistroManaged));
    }

    #[test]
    fn provider_dispatch_resolves_llama_and_ods() {
        assert!(providers::provider("llama.cpp").is_some());
        assert!(providers::provider("ods").is_some());
        assert!(providers::provider("docker").is_some());
        assert!(providers::provider("nope").is_none());
    }
}
