mod apply;
mod plan;
mod types;

pub use apply::apply_framework_propagation;
pub use plan::plan_framework_propagation;
pub use types::{
    FrameworkPropagationExecution, FrameworkPropagationOperation, FrameworkPropagationPlan,
    FrameworkPropagationReport,
};

#[cfg(test)]
mod tests;
