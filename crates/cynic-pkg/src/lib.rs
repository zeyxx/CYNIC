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
pub mod pins;
pub mod python_cli_policy;
pub mod python_tooling;
pub mod registry;
pub mod specs;
pub mod tooling_authority;
pub mod types;
pub mod upstream_probe;

pub mod evaluate;
pub mod plan;

pub use evaluate::{channel_for_dep, cmp_loose_version, extract_build_number, format_python_provenance, mode_from_channel, parse_version_triple, principal_for_dep, python_cli_can_auto_apply, python_cli_normalizable_now, python_cli_requires_runtime_principal, python_cli_requires_uv, evaluate_status};
pub use plan::{action_priority, needs_upgrade, reason_for_dep};

pub use pins::{PinEntry, PinsFile, load_pins, resolve_pins_path};
pub use specs::{DepSpec, UpgradeMode, VersionFrom, specs};

pub use authority::{LocalPrincipalAuthority, PrincipalAuthorityPort};
pub use execution::{
    EcosystemPackageExecutionPort, LocalEcosystemPackageExecution,
};
pub use integration::*;
pub use layout::{
    RuntimePrincipalLayout, default_runtime_principal_layout, resolve_principal_install_layout,
    resolve_principal_install_layout_with_authority,
};
pub use port::{
    EcosystemPackageProvider, apply_package_plan, apply_package_plan_with_executor,
    materialize_argv,
};
pub use probe::{
    classify_python_install_provenance, observe_python_install_provenance, resolve_binary, which,
};
pub use providers::{
    apply_for_dep, has_auto_handler, plan_external, plan_for_dep, plan_llama_cpp, plan_ods,
};
pub use python_cli_policy::{PythonCliPolicy, python_cli_policy};
pub use python_tooling::observe_python_tooling;
pub use registry::{ecosystem_provider_specs, provider_spec};
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
pub use upstream_probe::{
    HttpUpstreamProbe, OfflineUpstreamProbe, UpstreamProbePort, UpstreamSourceKind,
    UpstreamVersionInfo, probe_github_latest_release, probe_npm_latest_version,
    probe_pypi_latest_version, probe_upstream_version, probe_upstream_version_with_port,
};
