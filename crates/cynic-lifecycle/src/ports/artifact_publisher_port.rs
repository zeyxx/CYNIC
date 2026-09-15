//! Artifact publisher port — abstract transport for releasing a built
//! artefact to a runtime target.
//!
//! Layering: `cynic-lifecycle` (port) ← adapter (cynic-host or http/ssh
//! crate) ← usecase (`kernel_release::reconcile_runtime_artifact_release`).
//!
//! The domain [`ArtifactPublicationRequest`](artifact_publication::ArtifactPublicationRequest)
//! is transport-agnostic; this port is the seam that lets a concrete
//! adapter (HTTP PUT / SSH) honour it.

use crate::domain::artifact_publication::{
    ArtifactPublicationRequest, PublishError, PublishedArtefact,
};

/// Publish `request.artefact` (local path) to `request.target`.
///
/// Implementors translate `request.target.protocol` into the matching
/// transport (HTTP PUT or SSH copy) against `request.target.target`.
/// Must be fail-closed: never report success on a transport failure.
pub trait ArtifactPublisher {
    fn publish(&self, request: &ArtifactPublicationRequest) -> Result<PublishedArtefact, PublishError>;
}

/// Convenience adaptor — injects an [ArtifactPublisher] into a closure
/// usecase so tests can stub transport without touching network/SSH.
pub trait PublishArtefact {
    fn run(&self, publisher: &dyn ArtifactPublisher) -> Result<PublishedArtefact, PublishError>;
}
