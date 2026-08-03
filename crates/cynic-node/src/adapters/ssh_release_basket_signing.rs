use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

use crate::{
    adapters::local_release_basket_packaging::digest_file,
    ports::release_publication_port::{
        DetachedReleaseSignature, PackagedReleaseBasket, ReleaseBasketSigningPort,
        ReleasePublicationFuture,
    },
};

#[derive(Debug, Clone)]
pub struct SshReleaseBasketSigning {
    signing_key_path: PathBuf,
    signer_identity: String,
    namespace: String,
    max_bytes: u64,
}

impl SshReleaseBasketSigning {
    pub fn new(
        signing_key_path: PathBuf,
        signer_identity: String,
        namespace: String,
        max_bytes: u64,
    ) -> Result<Self, String> {
        if !signing_key_path.is_absolute()
            || signer_identity.trim().is_empty()
            || namespace != cynic_protocol::RUNTIME_RELEASE_SIGNATURE_NAMESPACE
            || max_bytes == 0
        {
            return Err("release_signing_boundary_invalid".to_string());
        }
        Ok(Self {
            signing_key_path,
            signer_identity,
            namespace,
            max_bytes,
        })
    }
}

impl ReleaseBasketSigningPort for SshReleaseBasketSigning {
    fn sign<'a>(
        &'a self,
        basket: &'a PackagedReleaseBasket,
    ) -> ReleasePublicationFuture<'a, DetachedReleaseSignature> {
        let signing_key_path = self.signing_key_path.clone();
        let signer_identity = self.signer_identity.clone();
        let namespace = self.namespace.clone();
        let basket_path = basket.path.clone();
        let max_bytes = self.max_bytes;
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let metadata = fs::symlink_metadata(&signing_key_path)
                    .map_err(|error| format!("release_signing_key_unobservable:{error}"))?;
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || metadata.permissions().mode() & 0o037 != 0
                {
                    return Err("release_signing_key_unsafe".to_string());
                }
                let probe = Command::new("ssh-keygen")
                    .arg("-?")
                    .output()
                    .map_err(|error| format!("release_signing_capability_probe_failed:{error}"))?;
                let help = format!(
                    "{}{}",
                    String::from_utf8_lossy(&probe.stdout),
                    String::from_utf8_lossy(&probe.stderr)
                );
                if !help.contains("-Y sign") {
                    return Err("release_signing_capability_unknown".to_string());
                }
                let signature_path = PathBuf::from(format!("{}.sig", basket_path.display()));
                if fs::symlink_metadata(&signature_path).is_ok() {
                    return Err("release_signature_destination_exists".to_string());
                }
                let output = Command::new("ssh-keygen")
                    .args(["-Y", "sign", "-f"])
                    .arg(&signing_key_path)
                    .arg("-n")
                    .arg(&namespace)
                    .arg(&basket_path)
                    .output()
                    .map_err(|error| format!("release_signing_spawn_failed:{error}"))?;
                if !output.status.success() {
                    return Err(format!(
                        "release_signing_failed:{}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                let (digest, size_bytes) = digest_file(&signature_path, max_bytes)?;
                Ok(DetachedReleaseSignature {
                    path: signature_path,
                    digest,
                    size_bytes,
                    signer_identity,
                    namespace,
                })
            })
            .await
            .map_err(|error| format!("release_signing_task_failed:{error}"))?
        })
    }
}
