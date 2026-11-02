use std::path::Path;

use cynic_protocol::RuntimeArtifactSource;

pub use cynic_protocol::artifact_transport::ArtifactTransferReceipt as RuntimeArtifactTransferReceipt;

pub trait RuntimeArtifactTransportPort {
    async fn stage(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        expected_digest: Option<&str>,
    ) -> Result<RuntimeArtifactTransferReceipt, String>;
}
