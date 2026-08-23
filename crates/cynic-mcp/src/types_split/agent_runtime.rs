use schemars::JsonSchema;
use serde::Deserialize;

/// Read-only observation of declared external agent runtimes.
/// Kernel-optional: probes binaries directly via the injected ArgvRunner.
/// Mirrors the `cynic runtime agents --json` CLI surface.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentRuntimeObserveParams {}
