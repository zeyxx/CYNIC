use crate::domain::mesh::MeshTopology;
use crate::ports::mesh_port::{MeshAdapter, MeshStateProvider};
use std::error::Error;
use tracing::info;

/// Use Case: Establish and synchronize the sovereign WireGuard mesh
#[derive(Debug)]
pub struct MeshSyncUseCase<A: MeshAdapter, S: MeshStateProvider> {
    adapter: A,
    state_provider: S,
    local_ip: String,
    private_key: String,
}

impl<A: MeshAdapter, S: MeshStateProvider> MeshSyncUseCase<A, S> {
    pub fn new(adapter: A, state_provider: S, local_ip: String, private_key: String) -> Self {
        Self {
            adapter,
            state_provider,
            local_ip,
            private_key,
        }
    }

    /// Run a full synchronization cycle for the Mesh network
    pub async fn execute(&self) -> Result<(), Box<dyn Error>> {
        info!("Initiating Mesh Synchronization (Homeostasis L2)...");

        // 1. Ensure the underlying interface exists (e.g., synapse0)
        self.adapter
            .initialize_interface(&self.local_ip, &self.private_key)?;

        // 2. Fetch the true state of the organism from SSoT
        let peers = self.state_provider.fetch_known_peers().await?;

        // 3. Build the Domain Entity
        let mut topology = MeshTopology::new("10.20.0.0/16".to_string());
        for peer in peers {
            topology.add_peer(peer);
        }

        // 4. Apply the true state to the physical reality (WireGuard kernel)
        self.adapter.sync_topology(&topology)?;

        info!("Mesh Synchronization complete. Topology applied.");
        Ok(())
    }
}
