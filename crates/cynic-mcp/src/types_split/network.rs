#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkIpamCheckParams {
    /// Registry node name to check.
    pub node: String,
    /// Service plane to validate. Defaults to synapse.
    pub plane: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshDnsRenderParams {
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
    /// Include short aliases alongside FQDN records. Defaults to true.
    pub include_short_names: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshStatusParams {
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
    /// Resolve Synapse DNS names from the current host. Defaults to false.
    pub resolve_dns: Option<bool>,
    /// Include rendered DNS records in the response. Defaults to true.
    pub include_records: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshDnsActivationPlanParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshDnsActivationBundleParams {
    /// Output directory for the non-mutating Synapse DNS activation bundle. Defaults to /tmp/cynic-synapse-dns.
    pub output_dir: Option<String>,
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshDnsActivationApplyParams {
    /// Existing digest-bound bundle directory under /tmp.
    pub bundle_dir: Option<String>,
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
    /// Must be true before file installation or service activation is attempted.
    #[serde(default)]
    pub apply: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshProviderFallbackHostsParams {
    /// Output directory for the non-mutating provider fallback hosts bundle. Defaults to /tmp/cynic-provider-fallback.
    pub output_dir: Option<String>,
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshProviderFallbackStatusParams {
    /// Hosts file to inspect. Defaults to /etc/hosts.
    pub hosts_path: Option<String>,
    /// Registry reference. Only infra/registry.json is supported in v1.
    pub registry_ref: Option<String>,
    /// Check declared provider fallback service ports. Defaults to false.
    pub check_connectivity: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkMeshDnsActivationStatusParams {
    /// Optional explicit hosts file path. Defaults to /etc/coredns/synapse.hosts.
    pub hosts_path: Option<String>,
    /// Optional explicit CoreDNS Corefile path. Defaults to /etc/coredns/Corefile.
    pub corefile_path: Option<String>,
    /// Optional explicit systemd unit path. Defaults to /etc/systemd/system/coredns-synapse.service.
    pub service_path: Option<String>,
    /// Optional explicit systemd-resolved drop-in path.
    pub resolved_dropin_path: Option<String>,
    /// Resolve Synapse DNS from the current host while checking activation. Defaults to true.
    pub resolve_dns: Option<bool>,
}
