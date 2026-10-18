//! Runtime artifact publication manifests and verified pulls.
//!
//! Publish is local-only (digest the bytes, record the manifest). Pull is
//! pull-only HTTP(S) with a fail-closed digest check: a mismatch or an
//! unreachable sums source refuses, never falls back.

use sha2::{Digest, Sha256};

pub const ARTIFACT_PUBLICATION_MANIFEST_SCHEMA: &str = "cynic.runtime_artifact_publication.v1";

/// Runtime artifact arms: publish a manifest or pull with verification.
#[derive(clap::Subcommand, Debug)]
pub(crate) enum BuildArtifactCommand {
    /// Digest a built artifact and write its publication manifest (local only).
    Publish {
        /// Path to the built artifact file.
        #[arg(long)]
        artifact: String,
        /// Remote path under the store releases root (e.g. runtime-releases/v1/cynic).
        #[arg(long)]
        remote_path: String,
        /// Exact 40-character source commit the artifact was built from.
        #[arg(long)]
        source_commit: String,
        /// Publisher identity (usually the builder node id).
        #[arg(long, default_value = "cynic-builder")]
        publisher: String,
        /// Where to write the publication manifest JSON.
        #[arg(long)]
        output: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Pull an artifact over HTTP(S) and refuse it unless the digest matches.
    Pull {
        /// Path to the publication manifest JSON.
        #[arg(long)]
        manifest: String,
        /// Store base URL (e.g. https://10.10.10.103:8443).
        #[arg(long)]
        base_url: String,
        /// Where to write the verified artifact.
        #[arg(long)]
        output: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ArtifactPublicationManifest {
    pub schema: String,
    pub artifact_remote_path: String,
    pub sha256_hex: String,
    pub artifact_bytes: u64,
    pub source_commit: String,
    pub published_at_ms: u64,
    pub publisher: String,
}

/// SHA-256 of raw bytes, lowercase hex.
pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Build a publication manifest for already-built bytes. Pure: the caller
/// supplies the clock.
pub fn publication_manifest(
    artifact_remote_path: &str,
    artifact_bytes: &[u8],
    source_commit: &str,
    published_at_ms: u64,
    publisher: &str,
) -> ArtifactPublicationManifest {
    ArtifactPublicationManifest {
        schema: ARTIFACT_PUBLICATION_MANIFEST_SCHEMA.to_string(),
        artifact_remote_path: artifact_remote_path.to_string(),
        sha256_hex: digest_bytes(artifact_bytes),
        artifact_bytes: artifact_bytes.len() as u64,
        source_commit: source_commit.to_string(),
        published_at_ms,
        publisher: publisher.to_string(),
    }
}

/// Fail-closed verification: exact digest match only.
pub fn verify_pulled_bytes(manifest: &ArtifactPublicationManifest, bytes: &[u8]) -> Result<(), String> {
    let actual = digest_bytes(bytes);
    if actual != manifest.sha256_hex {
        return Err(format!(
            "artifact_digest_mismatch: expected {} got {actual}",
            manifest.sha256_hex
        ));
    }
    if bytes.len() as u64 != manifest.artifact_bytes {
        return Err(format!(
            "artifact_size_mismatch: expected {} got {}",
            manifest.artifact_bytes,
            bytes.len()
        ));
    }
    Ok(())
}

pub fn publish_artifact(
    artifact_path: &str,
    remote_path: &str,
    source_commit: &str,
    publisher: &str,
    output: &str,
    json: bool,
) -> Result<(), String> {
    let bytes =
        std::fs::read(artifact_path).map_err(|e| format!("artifact_unreadable: {e}"))?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let manifest = publication_manifest(remote_path, &bytes, source_commit, now_ms, publisher);
    let rendered = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("manifest_unrenderable: {e}"))?;
    std::fs::write(output, format!("{rendered}\n"))
        .map_err(|e| format!("manifest_unwritable: {e}"))?;
    if json {
        println!("{rendered}");
    } else {
        println!("published {output} sha256={} bytes={}", manifest.sha256_hex, manifest.artifact_bytes);
    }
    Ok(())
}

pub fn pull_artifact(
    manifest_path: &str,
    base_url: &str,
    output: &str,
    json: bool,
) -> Result<(), String> {
    let raw =
        std::fs::read_to_string(manifest_path).map_err(|e| format!("manifest_unreadable: {e}"))?;
    let manifest: ArtifactPublicationManifest =
        serde_json::from_str(&raw).map_err(|e| format!("manifest_unparsable: {e}"))?;
    if manifest.schema != ARTIFACT_PUBLICATION_MANIFEST_SCHEMA {
        return Err(format!("manifest_schema_unknown: {}", manifest.schema));
    }
    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        manifest.artifact_remote_path.trim_start_matches('/')
    );
    // reqwest::blocking builds its own runtime: keep it off the caller's
    // async runtime (dropping it inside one panics) via a plain OS thread.
    let bytes = std::thread::spawn(move || {
        let response =
            reqwest::blocking::get(&url).map_err(|e| format!("pull_unreachable: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("pull_refused: http {}", response.status()));
        }
        response.bytes().map_err(|e| format!("pull_unreadable: {e}"))
    })
    .join()
    .map_err(|_| "pull_worker_panicked".to_string())??;
    verify_pulled_bytes(&manifest, &bytes)?;
    std::fs::write(output, &bytes).map_err(|e| format!("artifact_unwritable: {e}"))?;
    if json {
        println!("{}", serde_json::to_string(&manifest).unwrap_or_default());
    } else {
        println!("pulled {output} sha256={} bytes={}", manifest.sha256_hex, bytes.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_verifies() {
        let bytes = b"fake-binary-bytes";
        let manifest = publication_manifest(
            "runtime-releases/v1/cynic",
            bytes,
            "abc123",
            1700000000000,
            "cynic-builder",
        );
        assert!(verify_pulled_bytes(&manifest, bytes).is_ok());
        let rendered = serde_json::to_string(&manifest).unwrap();
        let back: ArtifactPublicationManifest = serde_json::from_str(&rendered).unwrap();
        assert_eq!(back, manifest);
    }

    #[test]
    fn tampered_bytes_refuse() {
        let manifest = publication_manifest("r/cynic", b"real", "abc", 1, "b");
        let err = verify_pulled_bytes(&manifest, b"fake").unwrap_err();
        assert!(err.starts_with("artifact_digest_mismatch"));
    }

    #[test]
    fn truncated_bytes_refuse() {
        let bytes = b"0123456789";
        let mut manifest = publication_manifest("r/cynic", bytes, "abc", 1, "b");
        manifest.sha256_hex = digest_bytes(b"01234");
        let err = verify_pulled_bytes(&manifest, b"01234").unwrap_err();
        assert!(err.starts_with("artifact_size_mismatch"));
    }
}
