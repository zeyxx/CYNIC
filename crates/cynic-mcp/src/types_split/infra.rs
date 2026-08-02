#[derive(Debug, Deserialize, JsonSchema)]
pub struct InfraRegistryReadParams {
    /// Optional dot-path selector, e.g. network_policy or lifecycle.kernel_release.
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProxmoxActionParams {
    pub node: String,
    pub target_type: String, // "qemu" or "lxc"
    pub vmid: u64,
    pub action: String, // "start", "stop", "reboot", "suspend", "resume"
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProxmoxProvisionParams {
    pub node: String,
    pub vmid: u64,
    pub name: String,
    pub archive: String,
    pub storage: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProxmoxExecParams {
    pub node: String,
    pub vmid: u64,
    /// Exact guest-agent argv. It is never parsed as a command line or shell input.
    pub argv: Vec<String>,
}

#[derive(Debug, serde::Serialize, Deserialize, JsonSchema)]
pub struct InfraProvisionParams {
    /// The target node or service to provision (e.g. kernel_primary, storage_primary)
    pub target: String,
}
