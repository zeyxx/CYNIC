pub mod bundle;
pub mod cli_install;
pub mod epoch;
pub mod lifecycle;
pub mod machine_enrollment;
mod placement;
pub mod propagation;
pub mod provider_enrollment;
pub mod runtime_binding;
pub mod scripts;
pub mod source_snapshot;
mod types;

pub use bundle::{write_framework_bootstrap_bundle, write_framework_bootstrap_bundle_for_role};
pub use cli_install::install_standalone_cli;
pub use epoch::{
    FrameworkEpochPhase, FrameworkEpochPlan, advance_framework_epoch, plan_framework_epoch,
};
pub use lifecycle::{
    FrameworkLifecycleAudit, FrameworkLifecycleDimension, audit_framework_lifecycle,
};
pub use machine_enrollment::{
    MachineEnrollmentAuthorityApplyReport, MachineEnrollmentAuthorityPlan,
    apply_machine_enrollment_authority, resolve_local_machine_enrollment_authority_spec,
    resolve_machine_enrollment_authority_spec,
};
pub use propagation::{
    FrameworkPropagationExecution, FrameworkPropagationOperation, FrameworkPropagationPlan,
    FrameworkPropagationReport, apply_framework_propagation, plan_framework_propagation,
};
pub use provider_enrollment::{
    ProviderControlEnrollmentPlan, ProviderControlEnrollmentReport,
    apply_provider_control_enrollment, observe_provider_control_enrollment,
    plan_provider_control_enrollment, resolve_local_provider_control_enrollment_spec,
    resolve_local_provider_control_enrollment_spec_from_registry,
};
pub use runtime_binding::{
    RuntimeBindingApplyReport, RuntimeBindingPlan, apply_runtime_binding,
    resolve_local_runtime_binding_spec,
};
pub use source_snapshot::{
    SourceSnapshotPlan, SourceSnapshotReport, create_source_snapshot, plan_source_snapshot,
};
pub use types::{FrameworkBootstrapBundleReport, StandaloneCliInstallReport};
