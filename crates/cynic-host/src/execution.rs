pub mod admission;
pub mod bridge_admission;
pub mod engine;
pub mod principal_admission;
pub mod types;

#[cfg(test)]
mod admission_tests;
#[cfg(test)]
mod engine_security_tests;

pub use admission::{
    AdmittedHostOperation, AdmittedHostOperationPlan, HOST_OPERATION_PLAN_SCHEMA, HostOperationPlan,
};
pub(crate) use admission::{
    admit_host_operation_plan, build_service_operation_plan, digest_json,
    runtime_service_operation_policy_admission, service_operation_policy_admission,
    stderr_with_spawn_error,
};
pub use bridge_admission::privilege_bridge_preconditions;
pub(crate) use bridge_admission::{
    build_privilege_bridge_operation_plan, privilege_bridge_policy_admission,
};
pub use engine::{
    command_to_argv, execute_argv_commands, execute_execution_plan, execute_host_commands,
    invocation_to_argv,
};
pub(crate) use principal_admission::{
    build_principal_operation_plan, principal_operation_policy_admission,
};
pub use types::{
    HostArgvCommand, HostCommandExecution, HostExecutionPlan, HostExecutionPlanReport,
    HostExecutionReport, HostExecutionStep, HostExecutionStepExecution,
    host_execution_plan_from_commands,
};
