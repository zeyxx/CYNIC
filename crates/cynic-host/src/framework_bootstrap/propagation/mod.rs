mod apply;
mod decision;
mod observe;
mod plan;
mod types;

pub use apply::apply_framework_propagation;
pub use plan::{plan_framework_propagation, plan_framework_propagation_with_transport};
pub use types::{
    FrameworkPropagationDecision, FrameworkPropagationExecution, FrameworkPropagationObservation,
    FrameworkPropagationOperation, FrameworkPropagationPlan, FrameworkPropagationProbe,
    FrameworkPropagationReport,
};

#[cfg(test)]
mod tests;
