use std::error::Error;

/// Port: Interface to the physical hypervisor (Proxmox)
pub trait HypervisorPort {
    /// List all actual VM and LXC IDs currently existing on the host
    fn get_active_instances(&self) -> Result<Vec<u32>, Box<dyn Error>>;

    /// Terminate and destroy a specific Ghost Container (Waste)
    fn destroy_instance(&self, id: u32) -> Result<(), Box<dyn Error>>;
}
