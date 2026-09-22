use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use cynic_build::{
    RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA, RuntimeReleasePublicationManifest,
    SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA, SignedReleaseBasketReadBack,
    validate_signed_release_basket_read_back,
};
use cynic_protocol::{
    RuntimeArtifactSource, RuntimeArtifactTransportKind, canonical_json_bytes, digest_json,
};
use reqwest::{Body, Client, Url};
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;

use crate::ports::release_publication_port::{
    DetachedReleaseSignature, PackagedReleaseBasket, ReleaseArtifactPublicationPort,
    ReleasePublicationFuture, ReleasePublicationTargets,
};

#[derive(Debug, Clone)]
pub struct HttpReleaseArtifactPublication {
    base_url: Url,
    credential_path: PathBuf,
    max_bytes: u64,
    client: Client,
}

impl HttpReleaseArtifactPublication {
    pub fn new(
        artifact_base_url: &str,
        authentication_kind: &str,
        credential_path: PathBuf,
        max_bytes: u64,
        timeout: Duration,
    ) -> Result<Self, String> {
        let mut base_url = Url::parse(artifact_base_url)
            .map_err(|_| "release_publication_base_url_invalid".to_string())?;
        if base_url.scheme() != "https"
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || authentication_kind != "bearer_file"
            || !credential_path.is_absolute()
            || max_bytes == 0
            || timeout.is_zero()
        {
            return Err("release_publication_boundary_invalid".to_string());
        }
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let client = Client::builder()
            .https_only(true)
            .timeout(timeout)
            .build()
            .map_err(|error| format!("release_publication_client_create_failed:{error}"))?;
        Ok(Self {
            base_url,
            credential_path,
            max_bytes,
            client,
        })
    }

    fn credential(&self) -> Result<String, String> {
        let metadata = fs::symlink_metadata(&self.credential_path)
            .map_err(|error| format!("release_publication_credential_unobservable:{error}"))?;
        let mode = metadata.permissions().mode();
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || mode & 0o037 != 0
            || metadata.len() == 0
            || metadata.len() > 16 * 1024
        {
            return Err("release_publication_credential_unsafe".to_string());
        }
        let credential = fs::read_to_string(&self.credential_path)
            .map_err(|error| format!("release_publication_credential_read_failed:{error}"))?;
        let credential = credential.trim().to_string();
        if credential.is_empty() || credential.bytes().any(|byte| byte.is_ascii_control()) {
            return Err("release_publication_credential_invalid".to_string());
        }
        Ok(credential)
    }

    async fn put_file(&self, url: &str, path: &PathBuf, credential: &str) -> Result<(), String> {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| format!("release_publication_source_unobservable:{error}"))?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > self.max_bytes {
            return Err("release_publication_source_size_invalid".to_string());
        }
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|error| format!("release_publication_source_open_failed:{error}"))?;
        let response = self
            .client
            .put(url)
            .bearer_auth(credential)
            .header(reqwest::header::CONTENT_LENGTH, metadata.len().to_string())
            .body(Body::wrap_stream(ReaderStream::new(file)))
            .send()
            .await
            .map_err(|error| format!("release_publication_put_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "release_publication_put_status_invalid:{}",
                response.status()
            ));
        }
        Ok(())
    }

    async fn put_bytes(&self, url: &str, bytes: Vec<u8>, credential: &str) -> Result<(), String> {
        let response = self
            .client
            .put(url)
            .bearer_auth(credential)
            .header(reqwest::header::CONTENT_LENGTH, bytes.len().to_string())
            .body(bytes)
            .send()
            .await
            .map_err(|error| format!("release_publication_manifest_put_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "release_publication_manifest_put_status_invalid:{}",
                response.status()
            ));
        }
        Ok(())
    }

    async fn read_back(
        &self,
        url: &str,
        credential: &str,
        expected_digest: &str,
        expected_size: Option<u64>,
    ) -> Result<u64, String> {
        let mut response = self
            .client
            .get(url)
            .bearer_auth(credential)
            .send()
            .await
            .map_err(|error| format!("release_publication_read_back_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "release_publication_read_back_status_invalid:{}",
                response.status()
            ));
        }
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("release_publication_read_back_stream_failed:{error}"))?
        {
            size = size
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| "release_publication_read_back_size_overflow".to_string())?;
            if size > self.max_bytes {
                return Err("release_publication_read_back_size_limit_exceeded".to_string());
            }
            hasher.update(&chunk);
        }
        let digest = format!("sha256:{}", hex::encode(hasher.finalize()));
        if digest != expected_digest || expected_size.is_some_and(|expected| expected != size) {
            return Err("release_publication_read_back_mismatch".to_string());
        }
        Ok(size)
    }
}

impl ReleaseArtifactPublicationPort for HttpReleaseArtifactPublication {
    fn targets(&self, release_id: &str) -> Result<ReleasePublicationTargets, String> {
        if !cynic_build::valid_release_publication_id(release_id) {
            return Err("release_publication_release_id_invalid".to_string());
        }
        let source = |suffix: &str| -> Result<RuntimeArtifactSource, String> {
            let url = self
                .base_url
                .join(&format!("{release_id}/{release_id}{suffix}"))
                .map_err(|_| "release_publication_target_invalid".to_string())?;
            Ok(RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: url.to_string(),
            })
        };
        Ok(ReleasePublicationTargets {
            basket: source(".tar")?,
            detached_signature: source(".tar.sig")?,
            manifest: source(".manifest.json")?,
        })
    }

    fn publish<'a>(
        &'a self,
        node_id: &'a str,
        workload_id: &'a str,
        basket: &'a PackagedReleaseBasket,
        signature: &'a DetachedReleaseSignature,
        manifest: &'a RuntimeReleasePublicationManifest,
        targets: &'a ReleasePublicationTargets,
        observed_at_ms: u64,
    ) -> ReleasePublicationFuture<'a, SignedReleaseBasketReadBack> {
        Box::pin(async move {
            if manifest.schema != RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA
                || manifest.basket_source != targets.basket
                || manifest.detached_signature_source != targets.detached_signature
                || manifest.basket_digest != basket.digest
                || manifest.detached_signature_digest != signature.digest
                || manifest.signer_identity != signature.signer_identity
                || manifest.signature_namespace != signature.namespace
            {
                return Err("release_publication_manifest_binding_mismatch".to_string());
            }
            let credential = self.credential()?;
            let manifest_bytes = canonical_json_bytes(manifest).map_err(|error| {
                format!("release_publication_manifest_serialization_failed:{error}")
            })?;
            let manifest_digest = digest_json(manifest)
                .map_err(|error| format!("release_publication_manifest_digest_failed:{error}"))?;
            self.put_file(&targets.basket.locator, &basket.path, &credential)
                .await?;
            self.read_back(
                &targets.basket.locator,
                &credential,
                &basket.digest,
                Some(basket.size_bytes),
            )
            .await?;
            self.put_file(
                &targets.detached_signature.locator,
                &signature.path,
                &credential,
            )
            .await?;
            self.read_back(
                &targets.detached_signature.locator,
                &credential,
                &signature.digest,
                Some(signature.size_bytes),
            )
            .await?;
            self.put_bytes(&targets.manifest.locator, manifest_bytes, &credential)
                .await?;
            self.read_back(
                &targets.manifest.locator,
                &credential,
                &manifest_digest,
                None,
            )
            .await?;
            let read_back = SignedReleaseBasketReadBack {
                schema: SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA.to_string(),
                node_id: node_id.to_string(),
                workload_id: workload_id.to_string(),
                manifest: manifest.clone(),
                manifest_digest,
                manifest_source: targets.manifest.clone(),
                basket_size_bytes: basket.size_bytes,
                detached_signature_size_bytes: signature.size_bytes,
                observed_at_ms,
                residual_unknowns: vec![],
            };
            let blockers = validate_signed_release_basket_read_back(&read_back);
            if blockers.is_empty() {
                Ok(read_back)
            } else {
                Err(format!(
                    "release_publication_read_back_invalid:{}",
                    blockers.join(",")
                ))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_targets_are_secret_free_https_and_release_scoped() {
        let root = tempfile::tempdir().unwrap();
        let adapter = HttpReleaseArtifactPublication::new(
            "https://artifacts.cynic/releases",
            "bearer_file",
            root.path().join("machine-token"),
            1024,
            Duration::from_secs(1),
        )
        .unwrap();

        let targets = adapter.targets("release-a").unwrap();

        assert_eq!(
            targets.basket.locator,
            "https://artifacts.cynic/releases/release-a/release-a.tar"
        );
        assert_eq!(
            targets.detached_signature.locator,
            "https://artifacts.cynic/releases/release-a/release-a.tar.sig"
        );
        assert!(!targets.manifest.locator.contains('@'));
        assert!(!targets.manifest.locator.contains('?'));
    }

    #[test]
    fn publication_boundary_rejects_inline_credentials() {
        let result = HttpReleaseArtifactPublication::new(
            "https://operator:secret@artifacts.cynic/releases",
            "bearer_file",
            PathBuf::from("/var/lib/cynic/artifact-token"),
            1024,
            Duration::from_secs(1),
        );

        assert!(result.is_err());
    }
}
