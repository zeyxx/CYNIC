use crate::runtime_harness::{RuntimeHarnessContext, closed_local_lab_context};

pub(crate) fn lab_context() -> RuntimeHarnessContext {
    closed_local_lab_context()
}
