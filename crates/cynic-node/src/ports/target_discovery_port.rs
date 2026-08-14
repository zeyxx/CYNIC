use cynic_protocol::TargetCandidate;

/// Provider adapter for read-only target discovery. Implementations may query
/// local OS state, libvirt, Proxmox, or another provider, but never mutate.
pub trait TargetDiscoveryPort {
    fn provider(&self) -> &str;
    fn discover(&self) -> Result<Vec<TargetCandidate>, String>;
}
