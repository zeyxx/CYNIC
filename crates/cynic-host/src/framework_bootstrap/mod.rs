pub mod bundle;
pub mod cli_install;
pub mod lifecycle;
pub mod machine_enrollment;
mod placement;
pub mod propagation;
pub mod runtime_binding;
pub mod scripts;
mod types;

pub use bundle::{write_framework_bootstrap_bundle, write_framework_bootstrap_bundle_for_role};
pub use cli_install::install_standalone_cli;
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
pub use runtime_binding::{
    RuntimeBindingApplyReport, RuntimeBindingPlan, apply_runtime_binding,
    resolve_local_runtime_binding_spec,
};
pub use types::{FrameworkBootstrapBundleReport, StandaloneCliInstallReport};
