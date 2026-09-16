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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::artifact_publication::TransportProtocol;
    use crate::ports::artifact_publisher_port::ArtifactPublisher;
    use cynic_protocol::artifact_release::{
        ArtifactPublicationTarget, RuntimeArtifactReleaseSpec, RuntimeArtifactSpec,
    };

    struct StubPublisher {
        result: Result<PublishedArtefact, PublishError>,
    }

    impl ArtifactPublisher for StubPublisher {
        fn publish(
            &self,
            request: &ArtifactPublicationRequest,
        ) -> Result<PublishedArtefact, PublishError> {
            match &self.result {
                Ok(done) => Ok(PublishedArtefact {
                    proof_sha: request.proof_sha.clone(),
                    target_node: request.target.node.clone(),
                    transport: request.target.protocol,
                    published_uri: done.published_uri.clone(),
                }),
                Err(error) => Err(error.clone()),
            }
        }
    }

    fn spec_with(publication: Option<ArtifactPublicationTarget>) -> KernelReleaseSpec {
        KernelReleaseSpec {
            schema: "cynic.runtime_artifact_release.spec.v1".to_string(),
            node_id: "test-node".to_string(),
            phase: "conformance".to_string(),
            service_id: "cynic-node".to_string(),
            identity_schema: "cynic.service_identity_evidence.v1".to_string(),
            artifact: RuntimeArtifactSpec {
                id: "cynic-node-test".to_string(),
                path: "/tmp/cynic-node".to_string(),
                executable_name: "cynic-node".to_string(),
                sha256: "a".repeat(64),
                version: "0.2.0".to_string(),
                git_sha: "deadbeef".to_string(),
            },
            install_root: "/var/lib/cynic".to_string(),
            identity_url: "http://127.0.0.1:9090/identity".to_string(),
            restart_argv: vec!["systemctl".to_string(), "restart".to_string()],
            requires_privilege: false,
            publication,
        }
    }

    #[test]
    fn no_target_means_no_request() {
        let spec = spec_with(None);
        assert!(publication_request_for(&spec, "proof", "http", "ssh").is_none());
        let stub = StubPublisher {
            result: Err(PublishError::Transport("must not be called".to_string())),
        };
        assert!(
            publish_released_artifact(&spec, "proof", "http", "ssh", &stub)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn request_carries_provenance_and_staged_path() {
        let spec = spec_with(Some(ArtifactPublicationTarget {
            node: "synapse".to_string(),
            endpoint: String::new(),
            protocol: "http_put".to_string(),
        }));
        let req =
            publication_request_for(&spec, "proof123", "https://pve:8006", "ssh").expect("request");
        assert_eq!(req.proof_sha, "proof123");
        assert_eq!(req.target.protocol, TransportProtocol::HttpPut);
        assert_eq!(req.target.target, "https://pve:8006");
        assert_eq!(req.artefact_name, "cynic-node");
        assert!(
            req.local_path
                .ends_with(&format!("releases/{}/cynic-node", "a".repeat(64)))
        );
    }

    #[test]
    fn spec_endpoint_overrides_transport_target() {
        let spec = spec_with(Some(ArtifactPublicationTarget {
            node: "synapse".to_string(),
            endpoint: "cynic@10.10.10.119".to_string(),
            protocol: "ssh".to_string(),
        }));
        let req = publication_request_for(&spec, "proof", "https://pve:8006", "fallback")
            .expect("request");
        assert_eq!(req.target.protocol, TransportProtocol::Ssh);
        assert_eq!(req.target.target, "cynic@10.10.10.119");
        assert_eq!(req.target.node, "synapse");
    }

    #[test]
    fn transport_failure_propagates() {
        let spec = spec_with(Some(ArtifactPublicationTarget {
            node: "synapse".to_string(),
            endpoint: String::new(),
            protocol: "http_put".to_string(),
        }));
        let stub = StubPublisher {
            result: Err(PublishError::Transport("boom".to_string())),
        };
        let err = publish_released_artifact(&spec, "proof", "https://pve:8006", "ssh", &stub)
            .unwrap_err();
        assert!(matches!(err, PublishError::Transport(_)));
    }
}
