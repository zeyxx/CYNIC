use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

use cynic_protocol::{RuntimeArtifactSource, RuntimeArtifactTransportKind};
use sha2::{Digest, Sha256};

use crate::ports::runtime_artifact_transport_port::{
    RuntimeArtifactTransferReceipt, RuntimeArtifactTransportPort,
};

#[derive(Debug, Clone)]
pub struct HttpRuntimeArtifactTransport {
    allowed_root: PathBuf,
    max_bytes: u64,
    client: reqwest::Client,
}

impl HttpRuntimeArtifactTransport {
    pub fn new(allowed_root: PathBuf, max_bytes: u64, timeout: Duration) -> Result<Self, String> {
        if !allowed_root.is_absolute() || max_bytes == 0 {
            return Err("runtime artifact transport boundary is invalid".to_string());
        }
        let client = reqwest::Client::builder()
            .https_only(true)
            .timeout(timeout)
            .build()
            .map_err(|error| format!("runtime artifact HTTP client creation failed:{error}"))?;
        Ok(Self {
            allowed_root,
            max_bytes,
            client,
        })
    }

    fn prepare_destination(&self, destination: &Path) -> Result<PathBuf, String> {
        let relative = destination
            .strip_prefix(&self.allowed_root)
            .map_err(|_| "runtime_artifact_destination_outside_inbox".to_string())?;
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("runtime_artifact_destination_unsafe".to_string());
        }
        ensure_safe_directory_tree(&self.allowed_root)?;
        let parent = destination
            .parent()
            .ok_or_else(|| "runtime_artifact_destination_parent_missing".to_string())?;
        let relative_parent = parent
            .strip_prefix(&self.allowed_root)
            .map_err(|_| "runtime_artifact_destination_outside_inbox".to_string())?;
        let mut current = self.allowed_root.clone();
        for component in relative_parent.components() {
            let Component::Normal(name) = component else {
                return Err("runtime_artifact_destination_parent_unsafe".to_string());
            };
            current.push(name);
            ensure_safe_directory(&current)?;
        }
        match fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err("runtime_artifact_destination_unsafe".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("runtime_artifact_destination_unobservable:{error}"));
            }
        }
        let filename = destination
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "runtime_artifact_destination_filename_invalid".to_string())?;
        Ok(parent.join(format!(
            ".{filename}.partial-{}-{}",
            std::process::id(),
            now_unix_ms()
        )))
    }

    fn finish(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        temporary: &Path,
        expected_digest: Option<&str>,
        digest: String,
        bytes_written: u64,
    ) -> Result<RuntimeArtifactTransferReceipt, String> {
        if expected_digest.is_some_and(|expected| expected != digest) {
            cleanup_temporary(temporary);
            return Err("runtime_artifact_digest_read_back_mismatch".to_string());
        }
        fs::rename(temporary, destination).map_err(|error| {
            cleanup_temporary(temporary);
            format!("runtime_artifact_atomic_activation_failed:{error}")
        })?;
        let directory_sync = File::open(
            destination
                .parent()
                .expect("prepared destination always has a parent"),
        )
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("runtime_artifact_directory_sync_failed:{error}"));
        let mut receipt = receipt(
            source,
            destination,
            expected_digest,
            digest,
            bytes_written,
            true,
        );
        if let Err(error) = directory_sync {
            receipt.status = "unknown";
            receipt.blockers.push(error);
        }
        Ok(receipt)
    }

    fn stage_local(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        expected_digest: Option<&str>,
    ) -> Result<RuntimeArtifactTransferReceipt, String> {
        let source_path = Path::new(&source.locator);
        let metadata = fs::symlink_metadata(source_path)
            .map_err(|error| format!("runtime_artifact_source_unobservable:{error}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("runtime_artifact_source_not_regular_file".to_string());
        }
        if source_path == destination {
            let (digest, bytes) = digest_file(source_path, self.max_bytes)?;
            if expected_digest.is_some_and(|expected| expected != digest) {
                return Err("runtime_artifact_digest_read_back_mismatch".to_string());
            }
            return Ok(receipt(
                source,
                destination,
                expected_digest,
                digest,
                bytes,
                false,
            ));
        }
        let temporary = self.prepare_destination(destination)?;
        let mut input = File::open(source_path)
            .map_err(|error| format!("runtime_artifact_source_open_failed:{error}"))?;
        let mut output = create_temporary(&temporary)?;
        let mut hasher = Sha256::new();
        let mut bytes_written = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer).map_err(|error| {
                cleanup_temporary(&temporary);
                format!("runtime_artifact_source_read_failed:{error}")
            })?;
            if read == 0 {
                break;
            }
            bytes_written = checked_size(bytes_written, read, self.max_bytes, &temporary)?;
            hasher.update(&buffer[..read]);
            output.write_all(&buffer[..read]).map_err(|error| {
                cleanup_temporary(&temporary);
                format!("runtime_artifact_write_failed:{error}")
            })?;
        }
        output.sync_all().map_err(|error| {
            cleanup_temporary(&temporary);
            format!("runtime_artifact_sync_failed:{error}")
        })?;
        self.finish(
            source,
            destination,
            &temporary,
            expected_digest,
            format!("sha256:{}", hex::encode(hasher.finalize())),
            bytes_written,
        )
    }
}

impl RuntimeArtifactTransportPort for HttpRuntimeArtifactTransport {
    async fn stage(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        expected_digest: Option<&str>,
    ) -> Result<RuntimeArtifactTransferReceipt, String> {
        if source.transport == RuntimeArtifactTransportKind::LocalFile {
            return self.stage_local(source, destination, expected_digest);
        }
        let temporary = self.prepare_destination(destination)?;
        let mut response = self
            .client
            .get(&source.locator)
            .send()
            .await
            .map_err(|error| format!("runtime_artifact_fetch_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "runtime_artifact_fetch_status_invalid:{}",
                response.status()
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_bytes)
        {
            return Err("runtime_artifact_size_limit_exceeded".to_string());
        }
        let mut output = create_temporary(&temporary)?;
        let mut hasher = Sha256::new();
        let mut bytes_written = 0_u64;
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(error) => {
                    cleanup_temporary(&temporary);
                    return Err(format!("runtime_artifact_stream_failed:{error}"));
                }
            };
            bytes_written = checked_size(bytes_written, chunk.len(), self.max_bytes, &temporary)?;
            hasher.update(&chunk);
            output.write_all(&chunk).map_err(|error| {
                cleanup_temporary(&temporary);
                format!("runtime_artifact_write_failed:{error}")
            })?;
        }
        output.sync_all().map_err(|error| {
            cleanup_temporary(&temporary);
            format!("runtime_artifact_sync_failed:{error}")
        })?;
        self.finish(
            source,
            destination,
            &temporary,
            expected_digest,
            format!("sha256:{}", hex::encode(hasher.finalize())),
            bytes_written,
        )
    }
}

fn ensure_safe_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err("runtime_artifact_inbox_directory_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(path)
            .map_err(|error| format!("runtime_artifact_inbox_create_failed:{error}")),
        Err(error) => Err(format!("runtime_artifact_inbox_unobservable:{error}")),
    }
}

fn ensure_safe_directory_tree(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("runtime_artifact_inbox_root_not_absolute".to_string());
    }
    let mut current = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => current.push(name),
            _ => return Err("runtime_artifact_inbox_root_unsafe".to_string()),
        }
        ensure_safe_directory(&current)?;
    }
    Ok(())
}

fn create_temporary(path: &Path) -> Result<File, String> {
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("runtime_artifact_temporary_create_failed:{error}"))
}

fn checked_size(current: u64, added: usize, maximum: u64, temporary: &Path) -> Result<u64, String> {
    let next = current
        .checked_add(added as u64)
        .ok_or_else(|| "runtime_artifact_size_overflow".to_string())?;
    if next > maximum {
        cleanup_temporary(temporary);
        return Err("runtime_artifact_size_limit_exceeded".to_string());
    }
    Ok(next)
}

fn digest_file(path: &Path, maximum: u64) -> Result<(String, u64), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("runtime_artifact_read_back_open_failed:{error}"))?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("runtime_artifact_read_back_failed:{error}"))?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .ok_or_else(|| "runtime_artifact_size_overflow".to_string())?;
        if bytes > maximum {
            return Err("runtime_artifact_size_limit_exceeded".to_string());
        }
        hasher.update(&buffer[..read]);
    }
    Ok((format!("sha256:{}", hex::encode(hasher.finalize())), bytes))
}

fn receipt(
    source: &RuntimeArtifactSource,
    destination: &Path,
    expected_digest: Option<&str>,
    observed_digest: String,
    bytes_written: u64,
    mutates: bool,
) -> RuntimeArtifactTransferReceipt {
    RuntimeArtifactTransferReceipt {
        schema: "cynic.runtime_artifact.transfer_receipt.v1",
        status: "proved",
        source: source.clone(),
        destination: destination.display().to_string(),
        expected_digest: expected_digest.map(str::to_string),
        evidence_ref: observed_digest.clone(),
        observed_digest,
        bytes_written,
        mutates,
        blockers: Vec::new(),
    }
}

fn cleanup_temporary(path: &Path) {
    let _ = fs::remove_file(path);
}

fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_transport_stages_atomically_and_proves_digest() {
        let root = tempfile::tempdir().expect("inbox root");
        let source_root = tempfile::tempdir().expect("source root");
        let source_path = source_root.path().join("release.tar");
        fs::write(&source_path, b"signed release basket").expect("source artifact");
        let digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(b"signed release basket"))
        );
        let transport = HttpRuntimeArtifactTransport::new(
            root.path().to_path_buf(),
            1024,
            Duration::from_secs(1),
        )
        .expect("transport");
        let destination = root.path().join("release.tar");
        let receipt = transport
            .stage(
                &RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::LocalFile,
                    locator: source_path.display().to_string(),
                },
                &destination,
                Some(&digest),
            )
            .await
            .expect("stage artifact");

        assert_eq!(receipt.status, "proved");
        assert_eq!(receipt.observed_digest, digest);
        assert_eq!(
            fs::read(destination).expect("read back"),
            b"signed release basket"
        );
    }

    #[tokio::test]
    async fn destination_outside_inbox_is_rejected_without_mutation() {
        let root = tempfile::tempdir().expect("inbox root");
        let source = root.path().join("source");
        fs::write(&source, b"artifact").expect("source artifact");
        let outside = tempfile::tempdir().expect("outside root");
        let transport = HttpRuntimeArtifactTransport::new(
            root.path().to_path_buf(),
            1024,
            Duration::from_secs(1),
        )
        .expect("transport");

        let error = transport
            .stage(
                &RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::LocalFile,
                    locator: source.display().to_string(),
                },
                &outside.path().join("release.tar"),
                None,
            )
            .await
            .expect_err("outside destination must fail");

        assert_eq!(error, "runtime_artifact_destination_outside_inbox");
    }
}
