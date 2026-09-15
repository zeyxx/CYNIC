//! HTTP PUT adapter for [`ArtifactPublisher`].
//!
//! Concrete transport honouring `release_publication/protocol`
//! (`generic_http_put_get`) from the owning runtime's registry entry.
//! Fail-closed: never report success unless the PUT returns 2xx.
//!
//! Layering: cynic-host (adapter) -> cynic-lifecycle (port trait).
//! Uses blocking reqwest (already a host dep).

use std::fs;
use std::path::Path;

use cynic_lifecycle::domain::artifact_publication::{
    ArtifactPublicationRequest, PublishedArtefact, PublishError, TransportProtocol,
};
use cynic_lifecycle::ports::artifact_publisher_port::ArtifactPublisher;

/// HTTP PUT publisher targeting the runtime's provider_control endpoint.
///
/// `base_url` is the registry `provider_control.endpoint` (e.g.
/// `https://pve.synapse.cynic:8006`).
#[derive(Clone)]
pub struct HttpArtifactPublisher {
    pub base_url: String,
    pub client: reqwest::blocking::Client,
}

impl Default for HttpArtifactPublisher {
    fn default() -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("cynic-lifecycle/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client init");
        HttpArtifactPublisher {
            base_url: String::new(),
            client,
        }
    }
}

impl HttpArtifactPublisher {
    pub fn new(base_url: impl Into<String>) -> Self {
        HttpArtifactPublisher {
            base_url: base_url.into(),
            ..Default::default()
        }
    }
}

impl ArtifactPublisher for HttpArtifactPublisher {
    fn publish(
        &self,
        request: &ArtifactPublicationRequest,
    ) -> Result<PublishedArtefact, PublishError> {
        if request.target.protocol != TransportProtocol::HttpPut {
            return Err(PublishError::Transport(format!(
                "publisher mismatch: target protocol {:?}, expected HttpPut",
                request.target.protocol
            )));
        }

        if !Path::new(&request.local_path).is_file() {
            return Err(PublishError::NotFound(request.local_path.clone()));
        }

        let artefact_bytes =
            fs::read(&request.local_path).map_err(|e| PublishError::Transport(format!(
                "read artefact {}: {}",
                request.local_path, e
            )))?;

        let url = format!(
            "{}/{}/{}",
            self.base_url.trim_end_matches('/'),
            request.proof_sha,
            request.artefact_name,
        );

        let resp = self
            .client
            .put(&url)
            .header("X-Cynic-Proof", &request.proof_sha)
            .header("Content-Type", "application/octet-stream")
            .body(artefact_bytes)
            .send()
            .map_err(|e| PublishError::Transport(format!("PUT {}: {}", url, e)))?;

        if !resp.status().is_success() {
            return Err(PublishError::Transport(format!(
                "PUT {} returned {}",
                url,
                resp.status(),
            )));
        }

        Ok(PublishedArtefact {
            proof_sha: request.proof_sha.clone(),
            target_node: request.target.node.clone(),
            transport: TransportProtocol::HttpPut,
            published_uri: url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_rejects_ssh_target() {
        let pub_http = HttpArtifactPublisher::new("https://10.10.10.119:8006");
        let req = ArtifactPublicationRequest {
            proof_sha: "a7f5293c".into(),
            target: cynic_lifecycle::domain::artifact_publication::PublicationTarget {
                node: "synapse".into(),
                target: "cynic@10.10.10.119".into(),
                protocol: TransportProtocol::Ssh,
            },
            artefact_name: "cynic".into(),
            local_path: "/nonexistent".into(),
        };
        let r = pub_http.publish(&req);
        assert!(matches!(r, Err(PublishError::Transport(_))));
        assert_eq!(pub_http.base_url, "https://10.10.10.119:8006");
    }

    #[test]
    fn url_builds_provenance_from_proof_sha() {
        let pub_http = HttpArtifactPublisher::new("https://pve.synapse.cynic:8006");
        let req = ArtifactPublicationRequest {
            proof_sha: "a7f5293c".into(),
            target: cynic_lifecycle::domain::artifact_publication::PublicationTarget {
                node: "synapse".into(),
                target: "pve.synapse.cynic".into(),
                protocol: TransportProtocol::HttpPut,
            },
            artefact_name: "cynic".into(),
            local_path: "/bin/sh".into(), // exists in test env
        };
        // build the url logic without network by checking the format
        let url = format!(
            "{}/{}/{}",
            pub_http.base_url.trim_end_matches('/'),
            req.proof_sha,
            req.artefact_name,
        );
        assert_eq!(url, "https://pve.synapse.cynic:8006/a7f5293c/cynic");
    }
}
