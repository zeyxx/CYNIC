#![forbid(unsafe_code)]

//! `cynic-pkg` — Universal sovereign package ecosystem and tooling authority.
//!
//! Provides package definitions, layouts, providers, probes, and execution ports
//! for sovereign dependencies and agent runtime tools.

pub mod authority;
pub mod execution;
pub mod integration;
pub mod layout;
pub mod port;
pub mod probe;
pub mod providers;
pub mod python_cli_policy;
pub mod python_tooling;
pub mod registry;
pub mod tooling_authority;
pub mod types;
pub mod upstream_probe;

pub use authority::{LocalPrincipalAuthority, PrincipalAuthorityPort};
pub use execution::{
    EcosystemPackageExecutionPort, LocalEcosystemPackageExecution, PackageCommandOutput,
};
pub use layout::{
    default_runtime_principal_layout, resolve_principal_install_layout,
    resolve_principal_install_layout_with_authority, RuntimePrincipalLayout,
};
pub use port::{
    apply_package_plan, apply_package_plan_with_executor, materialize_argv,
    EcosystemPackageProvider,
};
pub use probe::{
    classify_python_install_provenance, observe_python_install_provenance, resolve_binary, which,
};
pub use providers::{
    apply_for_dep, has_auto_handler, plan_external, plan_for_dep, plan_llama_cpp, plan_ods,
};
pub use python_cli_policy::{python_cli_policy, PythonCliPolicy};
pub use python_tooling::observe_python_tooling;
pub use registry::{ecosystem_provider_specs, provider_spec, EcosystemProviderSpec};
pub use tooling_authority::{
    observe_python_install_authority, observe_tooling_authority, tooling_authority_status,
    tooling_provenance_from_python,
};
pub use types::{
    channel_auto_applicable, ApplyStep, EcosystemPackageApplyResult, EcosystemPackagePlan,
    InstallPrincipalKind, ObservedBinary, ObservedPythonTooling, PrincipalInstallLayout,
    ProviderChannel, PythonInstallAuthority, PythonInstallProvenance, RunAs, StepExecution,
    ToolingAuthority, ToolingAuthorityFamily, ToolingInstallProvenance,
};
pub use upstream_probe::{
    probe_github_latest_release, probe_npm_latest_version, probe_pypi_latest_version,
    probe_upstream_version, probe_upstream_version_with_port, HttpUpstreamProbe,
    OfflineUpstreamProbe, UpstreamProbePort, UpstreamSourceKind, UpstreamVersionInfo,
};
pub use integration::*;

