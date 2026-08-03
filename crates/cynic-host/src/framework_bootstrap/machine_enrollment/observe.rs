use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use cynic_protocol::MachineEnrollmentAuthority;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

use super::types::{
    MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA, MachineEnrollmentAuthorityObservation,
    MachineEnrollmentAuthoritySpec,
};

pub fn observe_machine_enrollment_authority(
    spec: &MachineEnrollmentAuthoritySpec,
) -> MachineEnrollmentAuthorityObservation {
    let authority_path = Path::new(&spec.config_root).join("machine-enrollment-authority.json");
    let signing_key_path = Path::new(&spec.config_root).join("machine-enrollment-signing-key");
    let staged_public_key_path = Path::new(&spec.staging_root).join("enrollment-authority.pub");
    let staged_token_path = Path::new(&spec.staging_root).join("enrollment-token");
    let mut unknowns = Vec::new();

    let authority = read_authority(&authority_path, &mut unknowns);
    let signing_key = read_signing_key(&signing_key_path, &mut unknowns);
    let signing_key_metadata = std::fs::metadata(&signing_key_path).ok();
    let signing_key_mode = signing_key_metadata
        .as_ref()
        .map(|metadata| metadata.permissions().mode() & 0o777);
    let authority_key_id = signing_key.as_ref().map(|key| {
        format!(
            "sha256:{}",
            hex::encode(Sha256::digest(key.verifying_key().as_bytes()))
        )
    });
    let staged_public_key_matches = signing_key.as_ref().map(|key| {
        std::fs::read_to_string(&staged_public_key_path)
            .ok()
            .is_some_and(|value| value.trim() == hex::encode(key.verifying_key().as_bytes()))
    });
    let staged_token_matches = authority.as_ref().map(|authority| {
        std::fs::read_to_string(&staged_token_path)
            .ok()
            .map(|token| format!("sha256:{}", hex::encode(Sha256::digest(token.trim()))))
            .as_deref()
            == Some(authority.token_digest.as_str())
    });

    MachineEnrollmentAuthorityObservation {
        schema: MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA,
        authority_present: authority_path.is_file(),
        authority_valid: authority.as_ref().map(authority_valid),
        authority_id: authority
            .as_ref()
            .map(|authority| authority.authority_id.clone()),
        seed_id: authority
            .as_ref()
            .map(|authority| authority.seed_id.clone()),
        token_digest: authority.map(|authority| authority.token_digest),
        signing_key_present: signing_key_path.is_file(),
        signing_key_valid: signing_key.as_ref().map(|_| true),
        signing_key_mode,
        signing_key_owner_id: signing_key_metadata.as_ref().map(MetadataExt::uid),
        signing_key_group_id: signing_key_metadata.as_ref().map(MetadataExt::gid),
        authority_key_id,
        staged_public_key_matches,
        staged_token_matches,
        unknowns,
        evidence_refs: vec![
            format!(
                "machine_enrollment_authority_file:{}",
                authority_path.display()
            ),
            format!(
                "machine_enrollment_signing_key:{}",
                signing_key_path.display()
            ),
            format!(
                "machine_enrollment_public_key_stage:{}",
                staged_public_key_path.display()
            ),
            format!(
                "machine_enrollment_token_stage:{}",
                staged_token_path.display()
            ),
        ],
    }
}

fn read_authority(path: &Path, unknowns: &mut Vec<String>) -> Option<MachineEnrollmentAuthority> {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(authority) => Some(authority),
            Err(error) => {
                unknowns.push(format!("machine_enrollment_authority_invalid:{error}"));
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("machine_enrollment_authority_unreadable:{error}"));
            None
        }
    }
}

fn read_signing_key(path: &Path, unknowns: &mut Vec<String>) -> Option<SigningKey> {
    match std::fs::read_to_string(path) {
        Ok(raw) => match hex::decode(raw.trim())
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        {
            Some(bytes) => Some(SigningKey::from_bytes(&bytes)),
            None => {
                unknowns.push("machine_enrollment_signing_key_invalid".to_string());
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("machine_enrollment_signing_key_unreadable:{error}"));
            None
        }
    }
}

fn authority_valid(authority: &MachineEnrollmentAuthority) -> bool {
    !authority.authority_id.trim().is_empty()
        && valid_sha256(&authority.seed_id)
        && valid_sha256(&authority.token_digest)
        && authority.expires_at_unix_s > 0
        && authority.renewal_authority_expires_at_unix_s >= authority.expires_at_unix_s
        && authority.max_grant_ttl_s > 0
        && !authority.allowed_roles.is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
