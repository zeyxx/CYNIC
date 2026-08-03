use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use cynic_protocol::{
    NODE_IDENTITY_OBSERVATION_SCHEMA, NodeIdentityDesiredState, NodeIdentityObservation,
    SecurityEvidenceState,
};

use crate::ports::node_identity_port::NodeIdentityPort;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalNodeIdentity;

impl NodeIdentityPort for LocalNodeIdentity {
    fn observe(&self, desired: &NodeIdentityDesiredState) -> NodeIdentityObservation {
        observe_path(desired)
    }

    fn generate(&self, desired: &NodeIdentityDesiredState) -> Result<(), String> {
        generate_key(desired)
    }
}

fn observe_path(desired: &NodeIdentityDesiredState) -> NodeIdentityObservation {
    let path = Path::new(&desired.signing_key_path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return observation(
                desired,
                SecurityEvidenceState::Absent,
                None,
                None,
                None,
                vec![],
            );
        }
        Err(error) => {
            return observation(
                desired,
                SecurityEvidenceState::Unknown,
                None,
                None,
                None,
                vec![format!("node_identity_metadata_unavailable:{error}")],
            );
        }
    };
    let mode = metadata.permissions().mode() & 0o777;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return observation(
            desired,
            SecurityEvidenceState::Refuted,
            Some(mode),
            None,
            None,
            vec![],
        );
    }
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            return observation(
                desired,
                SecurityEvidenceState::Unknown,
                Some(mode),
                None,
                None,
                vec![format!("node_identity_read_unavailable:{error}")],
            );
        }
    };
    let secret: [u8; 32] = match hex::decode(raw.trim())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
    {
        Some(secret) => secret,
        None => {
            return observation(
                desired,
                SecurityEvidenceState::Refuted,
                Some(mode),
                None,
                None,
                vec![],
            );
        }
    };
    let public = SigningKey::from_bytes(&secret).verifying_key().to_bytes();
    let public_hex = hex::encode(public);
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(public)));
    observation(
        desired,
        SecurityEvidenceState::Proven,
        Some(mode),
        Some(digest),
        Some(public_hex),
        vec![],
    )
}

fn generate_key(desired: &NodeIdentityDesiredState) -> Result<(), String> {
    let path = Path::new(&desired.signing_key_path);
    let parent = path
        .parent()
        .ok_or_else(|| "node_identity_parent_missing".to_string())?;
    reject_symlink_ancestors(parent)?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("node_identity_parent_create_failed:{error}"))?;
    reject_symlink_ancestors(parent)?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|error| format!("node_identity_parent_observation_failed:{error}"))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err("node_identity_parent_unsafe".to_string());
    }
    if fs::symlink_metadata(path).is_ok() {
        return Err("node_identity_target_already_exists".to_string());
    }

    let mut secret = Zeroizing::new([0_u8; 32]);
    fs::File::open("/dev/urandom")
        .and_then(|mut entropy| entropy.read_exact(&mut *secret))
        .map_err(|error| format!("node_identity_os_entropy_unavailable:{error}"))?;
    let public = SigningKey::from_bytes(&*secret).verifying_key().to_bytes();
    let nonce = hex::encode(Sha256::digest(public));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "node_identity_file_name_invalid".to_string())?;
    let temporary = parent.join(format!(".{file_name}.next.{}", &nonce[..16]));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("node_identity_temporary_create_failed:{error}"))?;
    let encoded = Zeroizing::new(hex::encode(&*secret));
    let write_result = (|| {
        writeln!(file, "{}", encoded.as_str())
            .map_err(|error| format!("node_identity_temporary_write_failed:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("node_identity_temporary_sync_failed:{error}"))?;
        fs::hard_link(&temporary, path)
            .map_err(|error| format!("node_identity_atomic_publish_failed:{error}"))?;
        fs::remove_file(&temporary)
            .map_err(|error| format!("node_identity_temporary_cleanup_failed:{error}"))?;
        FileSync::sync_directory(parent)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn reject_symlink_ancestors(path: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("node_identity_path_contains_symlink".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("node_identity_path_component_unobservable:{error}"));
            }
        }
    }
    Ok(())
}

struct FileSync;

impl FileSync {
    fn sync_directory(path: &Path) -> Result<(), String> {
        fs::File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("node_identity_parent_sync_failed:{error}"))
    }
}

fn observation(
    desired: &NodeIdentityDesiredState,
    key_state: SecurityEvidenceState,
    private_key_mode: Option<u32>,
    machine_identity_digest: Option<String>,
    machine_public_key_hex: Option<String>,
    unknowns: Vec<String>,
) -> NodeIdentityObservation {
    let evidence_refs = machine_identity_digest.iter().cloned().collect();
    NodeIdentityObservation {
        schema: NODE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
        node_id: desired.node_id.clone(),
        key_state,
        private_key_mode,
        machine_identity_digest,
        machine_public_key_hex,
        evidence_refs,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        NODE_IDENTITY_DESIRED_STATE_SCHEMA, NodeIdentityDecision, reconcile_node_identity,
    };

    #[test]
    fn generation_is_create_once_and_read_back_proven() {
        let root = tempfile::tempdir().expect("tempdir");
        let desired = NodeIdentityDesiredState {
            schema: NODE_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "builder-a".to_string(),
            signing_key_path: root
                .path()
                .join("identity/node-signing.key")
                .display()
                .to_string(),
        };
        let adapter = LocalNodeIdentity;

        assert_eq!(
            reconcile_node_identity(&desired, &adapter.observe(&desired)).decision,
            NodeIdentityDecision::Generate
        );
        adapter.generate(&desired).expect("generate identity");
        let observed = adapter.observe(&desired);

        assert_eq!(observed.key_state, SecurityEvidenceState::Proven);
        assert_eq!(observed.private_key_mode, Some(0o600));
        assert_eq!(
            reconcile_node_identity(&desired, &observed).decision,
            NodeIdentityDecision::AlreadyPresent
        );
        assert_eq!(
            adapter.generate(&desired).expect_err("must not overwrite"),
            "node_identity_target_already_exists"
        );
    }
}
