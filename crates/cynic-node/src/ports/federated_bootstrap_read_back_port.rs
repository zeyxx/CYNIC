use cynic_protocol::{FederatedBootstrapBinding, FederatedBootstrapReadBack};

/// Read-only boundary for a node-owned bootstrap proof.
///
/// The adapter may use a local socket, authenticated HTTP, or a future mesh
/// transport. The use case only accepts a proof bound to the requested source
/// commit and deployment epoch.
pub trait FederatedBootstrapReadBackPort: Send + Sync {
    async fn read_federated_bootstrap(
        &self,
        node_id: &str,
        expected_binding: &FederatedBootstrapBinding,
    ) -> Result<FederatedBootstrapReadBack, String>;
}
