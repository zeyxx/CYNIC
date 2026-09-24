use serde::{Deserialize, Serialize};

use cynic_build::HarnessAuthority;

pub const RUNTIME_HARNESS_CONTEXT_SCHEMA: &str = "cynic.runtime_harness.context.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHarnessContext {
    pub schema: &'static str,
    pub kind: RuntimeHarnessKind,
    pub role: &'static str,
    pub authority: HarnessAuthority,
    pub isolation_boundary: &'static str,
    pub target_scope: &'static str,
    pub mutates_host_network: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeHarnessKind {
    BareMetalDirect,
    RootlessContainer,
    IsolatedNamespace,
    SimulatedHarness,
}
