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

pub mod integration_apply;
pub mod integration_apply_types;
pub mod integration_catalog;
pub mod integration_connect;
pub mod integration_connect_types;
pub mod integration_decision;
pub mod integration_plan;
pub mod integration_plan_types;
pub mod integration_port;
pub mod integration_recipe;
pub mod integration_types;
pub mod layout;
pub mod port;
pub mod probe;
pub mod providers;
pub mod python_cli_policy;
pub mod python_tooling;
pub mod registry;
pub mod tooling_authority;
pub mod types;

// ── Stable public surface (CLI + tests) ─────────────────────────────────────

pub use integration_apply::apply_integration;
pub use integration_apply_types::IntegrationApplyReport;
pub use integration_catalog::integration_specs;
pub use integration_connect::connect_integration;
pub use integration_connect_types::IntegrationConnectReport;
pub use integration_decision::{
    IntegrationDecision, IntegrationDecisionSurface, decide_integration_action,
};
pub use integration_plan::{INTEGRATIONS_PLAN_SCHEMA, plan_integrations};
pub use integration_plan_types::{
    IntegrationExecutionPlan, IntegrationPlanAction, IntegrationPlanItem, IntegrationsPlanReport,
    IntegrationsPlanSummary,
};
pub use integration_port::{
    INTEGRATIONS_STATUS_SCHEMA, IntegrationObservationPort, observe_integrations,
};
pub use integration_recipe::{IntegrationRecipe, build_integration_recipe};
pub use integration_types::{
    ConnectionState, IntegrationCapability, IntegrationConnectionObservation,
    IntegrationDependencySpec, IntegrationDependencyView, IntegrationKind, IntegrationObservation,
    IntegrationObservationContext, IntegrationPrincipal, IntegrationSpec, IntegrationStatusSummary,
    IntegrationToolingAuthorityView, IntegrationsStatusReport, RawIntegrationObservation,
};
pub use layout::resolve_principal_install_layout;
pub use port::{EcosystemPackageProvider, apply_package_plan, materialize_argv};
pub use probe::{
    classify_python_install_provenance, observe_python_install_provenance, resolve_binary, which,
};
pub use providers::{
    apply_for_dep, has_auto_handler, plan_external, plan_for_dep, plan_llama_cpp, plan_ods,
};
pub use python_cli_policy::{PythonCliPolicy, python_cli_policy};
pub use python_tooling::observe_python_tooling;
pub use registry::{EcosystemProviderSpec, ecosystem_provider_specs, provider_spec};
pub use tooling_authority::{
    observe_python_install_authority, observe_tooling_authority, tooling_authority_status,
    tooling_provenance_from_python,
};
pub use types::{
    ApplyStep, EcosystemPackageApplyResult, EcosystemPackagePlan, InstallPrincipalKind,
    ObservedBinary, ObservedPythonTooling, PrincipalInstallLayout, ProviderChannel,
    PythonInstallAuthority, PythonInstallProvenance, RunAs, StepExecution, ToolingAuthority,
    ToolingAuthorityFamily, ToolingInstallProvenance, channel_auto_applicable,
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
