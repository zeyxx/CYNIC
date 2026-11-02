use std::path::Path;

use cynic_protocol::RuntimeArtifactSource;

pub trait RuntimeArtifactTransportPort {
    async fn stage(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        expected_digest: Option<&str>,
    ) -> Result<cynic_protocol::artifact_transport::ArtifactTransferReceipt, String>;
}
