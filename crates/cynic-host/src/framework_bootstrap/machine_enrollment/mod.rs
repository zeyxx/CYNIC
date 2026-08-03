mod apply;
mod decision;
mod observe;
mod registry;
mod types;

pub use apply::apply_machine_enrollment_authority;
pub use decision::plan_machine_enrollment_authority;
pub use observe::observe_machine_enrollment_authority;
pub use registry::{
    resolve_local_machine_enrollment_authority_spec, resolve_machine_enrollment_authority_spec,
};
pub use types::{
    MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA, MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA,
    MachineEnrollmentAuthorityAction, MachineEnrollmentAuthorityApplyReport,
    MachineEnrollmentAuthorityObservation, MachineEnrollmentAuthorityPlan,
    MachineEnrollmentAuthoritySpec,
};

#[cfg(test)]
mod tests;
