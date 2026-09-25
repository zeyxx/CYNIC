#[derive(Debug, Deserialize, JsonSchema)]
pub struct OperatorTransportStatusParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SshTargetInspectParams {
    /// DNS name or IP literal. The adapter rejects shell syntax and never persists trust.
    pub host: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FleetNodeStatusParams {
    /// Registry node name, alias, or Proxmox resource id.
    pub node: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FleetInventoryParams {
    /// Optional registry path. Defaults to the canonical infra/registry.json under the MCP project root.
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FleetReconcilePlanParams {
    /// Identifier declared under infra/registry.json fleet_desired.targets[].id.
    pub target_id: String,
    /// Optional registry path. Defaults to the canonical infra/registry.json under the MCP project root.
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FleetReconcileApplyParams {
    pub target_id: String,
    pub actor: String,
    /// Must be true before any provider mutation is attempted.
    pub apply: bool,
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FleetNodeExecParams {
    /// Registry node name, alias, or Proxmox resource id.
    pub node: String,
    /// Command to run through the canonical provider adapter.
    pub command: String,
    /// Human/machine purpose used for audit and policy review.
    pub purpose: String,
    /// Maximum command runtime. Defaults to 30 seconds.
    pub timeout_seconds: Option<u64>,
    /// Optional caller id for bridge audit payloads.
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LxcExecBridgePlanParams {
    /// Optional helper source path relative to the project root.
    pub helper_source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LxcExecBridgeBundleParams {
    /// Output directory for the non-mutating activation bundle. Defaults to /tmp/cynic-lxc-bridge.
    pub output_dir: Option<String>,
    /// Optional helper source path relative to the project root.
    pub helper_source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LxcExecBridgeStatusParams {
    /// Optional explicit local helper path. Defaults to the policy helper_path.
    pub helper_path: Option<String>,
    /// Optional explicit installed policy path.
    pub installed_policy_path: Option<String>,
    /// Optional explicit sudoers path.
    pub sudoers_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ServiceStatusParams {
    /// Registry node name or alias.
    pub node: String,
    /// Service declared under registry nodes[].services[].
    pub service: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ServiceHealthParams {
    /// Registry node name or alias.
    pub node: String,
    /// Service declared under registry nodes[].services[].
    pub service: String,
    /// Optional explicit endpoint. Defaults to the service's Synapse health endpoint when a port is declared.
    pub endpoint: Option<String>,
    /// HTTP timeout. Defaults to 5 seconds.
    pub timeout_seconds: Option<u64>,
}
