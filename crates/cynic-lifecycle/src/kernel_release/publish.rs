//! Post-activation artifact publication.
//!
//! After [`reconcile_kernel_release`](super::reconcile::reconcile_kernel_release)
//! reports success, the CLI may publish the staged artifact to the remote
//! target declared in `spec.publication`. This module builds the
//! transport-agnostic request and drives the injected
//! [`ArtifactPublisher`](crate::ports::artifact_publisher_port::ArtifactPublisher)
//! port — it owns no transport (HTTP/SSH live in adapters).

use std::path::Path;

use super::types::KernelReleaseSpec;
use crate::domain::artifact_publication::{
    ArtifactPublicationRequest, PublishError, PublishedArtefact, select_transport,
};
use crate::ports::artifact_publisher_port::ArtifactPublisher;

/// Build the publication request for a successfully released spec.
///
/// Returns `None` when the spec declares no `publication` target
/// (local-only release). `proof_sha` closes the provenance chain
/// (build proof baked into the binary).
pub fn publication_request_for(
    spec: &KernelReleaseSpec,
    proof_sha: &str,
    http_target: &str,
    ssh_target: &str,
) -> Option<ArtifactPublicationRequest> {
    let target = spec.publication.as_ref()?;
    let protocol_str = target.protocol.as_str();
    let mut selected = select_transport(protocol_str, http_target, ssh_target);
    selected.node = target.node.clone();
    if !target.endpoint.trim().is_empty() {
        selected.target = target.endpoint.clone();
    }
    Some(ArtifactPublicationRequest {
        proof_sha: proof_sha.to_string(),
        target: selected,
        artefact_name: spec.artifact.executable_name.clone(),
        local_path: Path::new(&spec.install_root)
            .join("releases")
            .join(&spec.artifact.sha256)
            .join(&spec.artifact.executable_name)
            .display()
            .to_string(),
    })
}

/// Publish a released artifact through the injected publisher.
///
/// No-op (`Ok(None)`) when the spec declares no publication target.
/// Fail-closed: transport errors propagate, never masked as success.
pub fn publish_released_artifact(
    spec: &KernelReleaseSpec,
    proof_sha: &str,
    http_target: &str,
    ssh_target: &str,
    publisher: &dyn ArtifactPublisher,
) -> Result<Option<PublishedArtefact>, PublishError> {
    let Some(request) = publication_request_for(spec, proof_sha, http_target, ssh_target) else {
        return Ok(None);
    };
    publisher.publish(&request).map(Some)
}

/// Re-export for port consumers resolving through `kernel_release`.
pub use crate::domain::artifact_publication::PublicationTarget as ResolvedPublicationTarget;
pub use crate::domain::artifact_publication::select_transport as resolve_publication_transport;
