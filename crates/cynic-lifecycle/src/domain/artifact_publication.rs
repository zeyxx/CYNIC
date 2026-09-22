//! Source sync transport model — pure registry parsing.
//!
//! Decides how an [ArtifactPublicationRequest] publishes its artefact
//! based on the owning runtime's registry entry (provider_control
//! endpoint / release_publication protocol). No I/O: this only maps
//! registry data to a transport selector consumed by the port layer.
//!
//! Layering: cynic-lifecycle (domain) ← reads registry shapes.
//! Depends only on std + serde; never on adapter (http/ssh) crates.

use serde::{Deserialize, Serialize};

/// Physical transport for publishing an artefact to a runtime target.
///
/// Mirrors `release_publication.protocol` from [`infra/registry.json`].
/// Unknown strings default to [`TransportProtocol::HttpPut`] (legacy channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportProtocol {
    HttpPut,
    Ssh,
}

impl Default for TransportProtocol {
    #[inline]
    fn default() -> Self {
        TransportProtocol::HttpPut
    }
}

impl std::str::FromStr for TransportProtocol {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http_put" | "generic_http_put_get" | "http" => Ok(TransportProtocol::HttpPut),
            "ssh" => Ok(TransportProtocol::Ssh),
            "" => Ok(Self::default()),
            _ => Ok(Self::default()),
        }
    }
}

/// Target runtime descriptor, decoded from the node registry entry.
///
/// `node` is the registry host id (e.g. `synapse`); `target` is the
/// concrete endpoint/host used by a [`TransportProtocol`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationTarget {
    pub node: String,
    pub target: String,
    pub protocol: TransportProtocol,
}

/// A request to publish a built artefact to one target.
///
/// `proof_sha` closes the provenance chain with the build proof baked
/// into the binary ([`cynic-cli`] `version` surface / `.proof.a7f5293c`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPublicationRequest {
    pub proof_sha: String,
    pub target: PublicationTarget,
    pub artefact_name: String,
    pub local_path: String,
}

/// Result of a successful artefact publication (port adapter return value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedArtefact {
    pub proof_sha: String,
    pub target_node: String,
    pub transport: TransportProtocol,
    pub published_uri: String,
}

/// Error surfaced by the publication port adapter.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum PublishError {
    #[error("transport failure: {0}")]
    Transport(String),
    #[error("registry unreachable: {0}")]
    Registry(String),
    #[error("artefact not found: {0}")]
    NotFound(String),
}

/// Pure selection rule — given a registry entry shape, return the transport.
///
/// Encodes the `release_publication/protocol` policy from `infra/registry.json`
/// without touching network/filesystem. `protocol_str` is the raw value of
/// `release_publication.protocol` (e.g. `generic_http_put_get` or `ssh`).
pub fn select_transport(
    protocol_str: &str,
    http_target: &str,
    ssh_target: &str,
) -> PublicationTarget {
    let protocol: TransportProtocol = protocol_str
        .parse::<TransportProtocol>()
        .unwrap_or_default();
    let target = match protocol {
        TransportProtocol::Ssh => ssh_target,
        _ => http_target,
    };
    PublicationTarget {
        node: http_target.to_owned(),
        target: target.to_owned(),
        protocol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_put_is_default() {
        let t = select_transport("generic_http_put_get", "10.10.10.119", "cynic@10.10.10.119");
        assert_eq!(t.protocol, TransportProtocol::HttpPut);
        assert_eq!(t.target, "10.10.10.119");
    }

    #[test]
    fn ssh_explicit_protocol() {
        let t = select_transport("ssh", "10.10.10.119", "cynic@172.30.41.10");
        assert_eq!(t.protocol, TransportProtocol::Ssh);
        assert_eq!(t.target, "cynic@172.30.41.10");
    }

    #[test]
    fn unknown_protocol_falls_back_to_http() {
        let t = select_transport("ftp", "target.example", "cynic@host");
        assert_eq!(t.protocol, TransportProtocol::HttpPut);
        assert_eq!(t.target, "target.example");
    }

    #[test]
    fn request_serializes_provenance_chain() {
        let req = ArtifactPublicationRequest {
            proof_sha: "a7f5293c".into(),
            target: PublicationTarget {
                node: "synapse".into(),
                target: "10.10.10.119".into(),
                protocol: TransportProtocol::HttpPut,
            },
            artefact_name: "cynic".into(),
            local_path: "/tmp/cynic".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("a7f5293c"));
        assert!(json.contains("synapse"));
    }
}
