use crate::domain::mesh::{MeshTopology, PeerIdentity};
use std::error::Error;

/// Port: Primary Interface for applying Mesh configuration to the underlying OS/Kernel.
/// This trait isolates our Domain from the concrete transport implementation
/// (canonical WireGuard today) and from any system-level CLI.
pub trait MeshAdapter {
    /// Initialize the mesh interface (e.g., creating the `synapse0` device)
    fn initialize_interface(&self, local_ip: &str, private_key: &str)
    -> Result<(), Box<dyn Error>>;

    /// Synchronize the OS routing and peers with the true Domain state.
    fn sync_topology(&self, topology: &MeshTopology) -> Result<(), Box<dyn Error>>;
}

/// Port: Primary Interface for fetching the canonical Mesh truth from the SSoT.
/// Provider coordination adapters such as Headscale must not redefine this truth.
pub trait MeshStateProvider {
    /// Fetch all known peers from the central nervous system.
    async fn fetch_known_peers(&self) -> Result<Vec<PeerIdentity>, Box<dyn Error>>;
}
