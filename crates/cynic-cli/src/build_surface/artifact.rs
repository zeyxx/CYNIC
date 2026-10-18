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
    /// Pull an artifact from an explicit URL and refuse it unless the
    /// digest matches the manifest.
    Pull {
        /// Path to the publication manifest JSON.
        #[arg(long)]
        manifest: String,
        /// Exact download URL (release attachment, store path, ...).
        #[arg(long)]
        url: String,
        /// Where to write the verified artifact.
        #[arg(long)]
        output: String,
        /// File holding the bearer token (the store is private).
        #[arg(long, default_value = "/etc/cynic/secrets/forgejo-token")]
        token_file: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Publish a built artifact as a Forgejo release attachment (the store).
    /// Creates the release for the tag when absent, uploads the artifact
    /// and its manifest, prints the download URLs.
    Release {
        /// Path to the built artifact file.
        #[arg(long)]
        artifact: String,
        /// Path to the publication manifest JSON (from publish).
        #[arg(long)]
        manifest: String,
        /// Repository as owner/name.
        #[arg(long)]
        repo: String,
        /// Release tag (e.g. runtime-v1 or the source commit).
        #[arg(long)]
        tag: String,
        /// Forgejo API base (no trailing slash).
        #[arg(long, default_value = "http://forgejo.synapse.cynic:3000/api/v1")]
        api_base: String,
        /// File holding the bearer token (never argv).
        #[arg(long, default_value = "/etc/cynic/secrets/forgejo-token")]
        token_file: String,
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
    url: &str,
    output: &str,
    token_file: &str,
    json: bool,
) -> Result<(), String> {
    let raw =
        std::fs::read_to_string(manifest_path).map_err(|e| format!("manifest_unreadable: {e}"))?;
    let manifest: ArtifactPublicationManifest =
        serde_json::from_str(&raw).map_err(|e| format!("manifest_unparsable: {e}"))?;
    if manifest.schema != ARTIFACT_PUBLICATION_MANIFEST_SCHEMA {
        return Err(format!("manifest_schema_unknown: {}", manifest.schema));
    }
    let url = url.to_string();
    let token = std::fs::read_to_string(token_file)
        .map_err(|e| format!("token_unreadable: {e}"))?
        .trim()
        .to_string();
    // reqwest::blocking builds its own runtime: keep it off the caller's
    // async runtime (dropping it inside one panics) via a plain OS thread.
    let bytes = std::thread::spawn(move || {
        let response =
            reqwest::blocking::Client::new()
                .get(&url)
                .header("Authorization", format!("token {token}"))
                .send()
                .map_err(|e| format!("pull_unreachable: {e}"))?;
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

/// Publish a built artifact as Forgejo release attachments. The local
/// artifact is re-verified against its manifest before upload; the release
/// is created for the tag when absent. All network I/O stays on a plain OS
/// thread (see pull_artifact).
pub fn release_artifact(
    artifact_path: &str,
    manifest_path: &str,
    repo: &str,
    tag: &str,
    api_base: &str,
    token_file: &str,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("manifest_unreadable: {e}"))?;
    let manifest: ArtifactPublicationManifest =
        serde_json::from_str(&raw).map_err(|e| format!("manifest_unparsable: {e}"))?;
    let local = std::fs::read(artifact_path).map_err(|e| format!("artifact_unreadable: {e}"))?;
    verify_pulled_bytes(&manifest, &local)?;
    let token = std::fs::read_to_string(token_file)
        .map_err(|e| format!("token_unreadable: {e}"))?
        .trim()
        .to_string();
    let api = api_base.trim_end_matches('/').to_string();
    let repo = repo.to_string();
    let tag = tag.to_string();
    let artifact_name = std::path::Path::new(artifact_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("artifact")
        .to_string();

    let urls = std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let release_id: i64 = match client
            .get(format!("{api}/repos/{repo}/releases/tags/{tag}"))
            .header("Authorization", format!("token {token}"))
            .send()
            .map_err(|e| format!("release_lookup_unreachable: {e}"))?
        {
            found if found.status().is_success() => found
                .json::<serde_json::Value>()
                .map_err(|e| format!("release_lookup_unparsable: {e}"))?
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "release_lookup_missing_id".to_string())?,
            _ => client
                .post(format!("{api}/repos/{repo}/releases"))
                .header("Authorization", format!("token {token}"))
                .json(&serde_json::json!({
                    "tag_name": tag,
                    "name": tag,
                    "body": format!("Runtime artifact release {tag} (source {})", manifest.source_commit),
                }))
                .send()
                .map_err(|e| format!("release_create_unreachable: {e}"))?
                .error_for_status()
                .map_err(|e| format!("release_create_refused: {e}"))?
                .json::<serde_json::Value>()
                .map_err(|e| format!("release_create_unparsable: {e}"))?
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "release_create_missing_id".to_string())?,
        };
        let mut uploaded = Vec::new();
        for (filename, bytes) in [
            (artifact_name, local),
            ("publication-manifest.json".to_string(), raw.into_bytes()),
        ] {
            let part = reqwest::blocking::multipart::Part::bytes(bytes).file_name(filename.clone());
            let form = reqwest::blocking::multipart::Form::new().part("attachment", part);
            let resp = client
                .post(format!("{api}/repos/{repo}/releases/{release_id}/assets?name={filename}"))
                .header("Authorization", format!("token {token}"))
                .multipart(form)
                .send()
                .map_err(|e| format!("asset_upload_unreachable: {filename}: {e}"))?
                .error_for_status()
                .map_err(|e| format!("asset_upload_refused: {filename}: {e}"))?;
            let url = resp
                .json::<serde_json::Value>()
                .map_err(|e| format!("asset_upload_unparsable: {filename}: {e}"))?
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            uploaded.push(url);
        }
        Ok::<Vec<String>, String>(uploaded)
    })
    .join()
    .map_err(|_| "release_worker_panicked".to_string())??;

    if json {
        println!("{}", serde_json::to_string(&urls).unwrap_or_default());
    } else {
        for url in &urls {
            println!("released {url}");
        }
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
