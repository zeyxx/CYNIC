#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoragePrimaryPlanParams {
    /// Optional registry path relative to the project root.
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoragePrimaryBundleParams {
    /// Output directory for the non-mutating storage activation bundle. Defaults to /tmp/cynic-storage-primary.
    pub output_dir: Option<String>,
    /// Optional registry path relative to the project root.
    pub registry: Option<String>,
    /// Optional schema path relative to the project root.
    pub schema: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoragePrimaryReadinessParams {
    /// Resolve Synapse DNS while composing mesh evidence. Defaults to true.
    pub resolve_mesh_dns: Option<bool>,
    /// HTTP timeout for service health. Defaults to 3 seconds.
    pub service_health_timeout_seconds: Option<u64>,
    /// Optional default bundle path. Must be absolute under /tmp.
    pub bundle_default_dir: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoragePrimaryStatusParams {
    /// Optional activation bundle directory produced by organ-keep.
    pub bundle_dir: Option<String>,
    /// When bundle_dir is absent, check the default local activation bundle path. Defaults to true.
    pub auto_bundle_dir: Option<bool>,
    /// Optional default bundle path for auto_bundle_dir. Must be absolute under /tmp.
    pub bundle_default_dir: Option<String>,
    /// Optional JSON file produced by lxc-exec-bridge activation-status.
    pub bridge_status_json: Option<String>,
    /// Compose LXC exec bridge evidence inside the MCP proxy and pass it to organ-keep without a temp file.
    pub auto_bridge_status: Option<bool>,
    /// Optional JSON file with fleet node evidence.
    pub fleet_report_json: Option<String>,
    /// Optional JSON file produced by network_mesh_status with resolve_dns=true.
    pub mesh_status_json: Option<String>,
    /// Compose mesh evidence inside the MCP proxy and pass it to organ-keep without a temp file.
    pub auto_mesh_status: Option<bool>,
    /// Resolve Synapse DNS while composing auto mesh evidence. Implies auto_mesh_status.
    pub resolve_mesh_dns: Option<bool>,
    /// Compose fleet node evidence inside the MCP proxy and pass it to organ-keep without a temp file.
    pub auto_fleet_status: Option<bool>,
    /// Optional JSON file with service health evidence.
    pub health_json: Option<String>,
    /// Probe the declared Synapse service health endpoint and pass evidence to organ-keep without a temp file.
    pub auto_service_health: Option<bool>,
    /// HTTP timeout for auto_service_health. Defaults to 5 seconds.
    pub service_health_timeout_seconds: Option<u64>,
}

