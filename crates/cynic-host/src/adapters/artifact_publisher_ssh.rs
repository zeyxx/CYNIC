//! SSH fallback adapter for [`ArtifactPublisher`].
//!
//! Last-resort transport: used only when the spec declares
//! `protocol = "ssh"`. HTTP PUT stays the primary channel.
//! Copies via `scp` (BatchMode, strict host keys, 10s timeout) into
//! `.cynic-releases/<proof>/<artefact>` under the remote home, then
//! reports the `ssh://` URI. Fail-closed on any transport error.
//!
//! Layering: cynic-host (adapter) -> cynic-lifecycle (port trait).
//! The runner is injected so tests use [`MapArgvRunner`] without network.

use std::path::Path;

use cynic_lifecycle::domain::artifact_publication::{
    ArtifactPublicationRequest, PublishError, PublishedArtefact, TransportProtocol,
};
use cynic_lifecycle::ports::artifact_publisher_port::ArtifactPublisher;
use cynic_lifecycle::{ArgvRunner, MapArgvRunner};

/// SSH publisher targeting `request.target.target` (e.g. `cynic@host`).
pub struct SshArtifactPublisher<R: ArgvRunner = MapArgvRunner> {
    runner: R,
}

impl SshArtifactPublisher<MapArgvRunner> {
    pub fn with_runner<R: ArgvRunner>(runner: R) -> SshArtifactPublisher<R> {
        SshArtifactPublisher { runner }
    }
}

impl<R: ArgvRunner> SshArtifactPublisher<R> {
    fn scp_argv(&self, source: &str, target: &str) -> Result<Vec<String>, PublishError> {
        validate_ssh_target(target)?;
        if source.is_empty() || source.starts_with('-') || source.contains(char::is_whitespace) {
            return Err(PublishError::Transport(
                "invalid local artefact path".to_string(),
            ));
        }
        Ok(vec![
            "scp".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            "ConnectTimeout=10".to_string(),
            source.to_string(),
            target.to_string(),
        ])
    }
}

fn validate_ssh_target(target: &str) -> Result<(), PublishError> {
    if target.trim().is_empty()
        || target.starts_with('-')
        || target.contains(char::is_whitespace)
        || target.chars().any(|c| ";|&$()<>".contains(c))
    {
        return Err(PublishError::Transport("invalid ssh target".to_string()));
    }
    Ok(())
}

impl<R: ArgvRunner> ArtifactPublisher for SshArtifactPublisher<R> {
    fn publish(
        &self,
        request: &ArtifactPublicationRequest,
    ) -> Result<PublishedArtefact, PublishError> {
        if request.target.protocol != TransportProtocol::Ssh {
            return Err(PublishError::Transport(format!(
                "publisher mismatch: target protocol {:?}, expected Ssh",
                request.target.protocol
            )));
        }
        if !Path::new(&request.local_path).is_file() {
            return Err(PublishError::NotFound(request.local_path.clone()));
        }
        let remote_path = format!(
            ".cynic-releases/{}/{}",
            request.proof_sha, request.artefact_name
        );
        let destination = format!("{}:{remote_path}", request.target.target);
        let argv = self.scp_argv(&request.local_path, &destination)?;
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let output = self.runner.run(&refs);
        if !output.success() {
            return Err(PublishError::Transport(format!(
                "scp failed: {}",
                output.spawn_error.unwrap_or(output.stderr)
            )));
        }
        Ok(PublishedArtefact {
            proof_sha: request.proof_sha.clone(),
            target_node: request.target.node.clone(),
            transport: TransportProtocol::Ssh,
            published_uri: format!("ssh://{destination}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::domain::artifact_publication::PublicationTarget;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    fn request() -> ArtifactPublicationRequest {
        ArtifactPublicationRequest {
            proof_sha: "abc123".to_string(),
            target: PublicationTarget {
                node: "synapse".to_string(),
                target: "cynic@10.10.10.119".to_string(),
                protocol: TransportProtocol::Ssh,
            },
            artefact_name: "cynic".to_string(),
            local_path: "/bin/sh".to_string(),
        }
    }

    #[test]
    fn rejects_http_target() {
        let mut req = request();
        req.target.protocol = TransportProtocol::HttpPut;
        let publisher = SshArtifactPublisher::with_runner(MapArgvRunner::default());
        assert!(matches!(
            publisher.publish(&req),
            Err(PublishError::Transport(_))
        ));
    }

    #[test]
    fn rejects_malicious_target() {
        let mut req = request();
        req.target.target = "cynic@host; rm -rf /".to_string();
        let publisher = SshArtifactPublisher::with_runner(MapArgvRunner::default());
        assert!(matches!(
            publisher.publish(&req),
            Err(PublishError::Transport(_))
        ));
    }

    #[test]
    fn scp_argv_is_bounded() {
        let publisher = SshArtifactPublisher::with_runner(MapArgvRunner::default());
        let argv = publisher
            .scp_argv("/tmp/a", "cynic@h:.cynic-releases/p/a")
            .expect("argv");
        assert_eq!(argv[0], "scp");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(argv.contains(&"StrictHostKeyChecking=yes".to_string()));
        assert_eq!(argv[argv.len() - 2], "/tmp/a");
    }

    #[test]
    fn runner_failure_is_transport_error() {
        let runner = MapArgvRunner::default();
        let publisher = SshArtifactPublisher::with_runner(runner);
        let err = publisher.publish(&request()).unwrap_err();
        assert!(matches!(err, PublishError::Transport(_)));
    }

    #[test]
    fn successful_copy_reports_ssh_uri() {
        let mut runner = MapArgvRunner::default();
        runner.responses.push((
            vec![
                "scp".to_string(),
                "-o".to_string(),
                "BatchMode=yes".to_string(),
                "-o".to_string(),
                "StrictHostKeyChecking=yes".to_string(),
                "-o".to_string(),
                "ConnectTimeout=10".to_string(),
                "/bin/sh".to_string(),
                "cynic@10.10.10.119:.cynic-releases/abc123/cynic".to_string(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        let publisher = SshArtifactPublisher::with_runner(runner);
        let done = publisher.publish(&request()).expect("published");
        assert_eq!(done.transport, TransportProtocol::Ssh);
        assert_eq!(
            done.published_uri,
            "ssh://cynic@10.10.10.119:.cynic-releases/abc123/cynic"
        );
    }
}
