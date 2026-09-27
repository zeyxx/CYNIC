use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use cynic_node_protocol::MachineEnrollmentAuthority;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::decision::plan_machine_enrollment_authority;
use super::observe::observe_machine_enrollment_authority;
use super::types::{
    MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA, MachineEnrollmentAuthorityApplyReport,
    MachineEnrollmentAuthorityPlan, MachineEnrollmentAuthoritySpec,
};

pub fn apply_machine_enrollment_authority(
    spec: &MachineEnrollmentAuthoritySpec,
    explicit_apply: bool,
) -> MachineEnrollmentAuthorityApplyReport {
    let before = observe_machine_enrollment_authority(spec);
    let plan = plan_machine_enrollment_authority(spec, &before);
    if plan.status == "blocked" || plan.status == "converged" || !explicit_apply {
        let mut blockers = plan.blockers.clone();
        if plan.status == "planned" && !explicit_apply {
            blockers.push("explicit_apply_required".to_string());
        }
        return MachineEnrollmentAuthorityApplyReport {
            schema: MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA,
            status: if plan.status == "converged" {
                "converged"
            } else if plan.status == "blocked" {
                "blocked"
            } else {
                "planned"
            },
            mutates: false,
            requested_apply: explicit_apply,
            plan,
            before: before.clone(),
            after: before,
            authority: None,
            proofs_observed: Vec::new(),
            blockers,
            rollback_performed: false,
            secret_material_returned: false,
        };
    }

    let paths = affected_paths(spec);
    let snapshots = paths
        .iter()
        .map(|path| snapshot(path))
        .collect::<Result<Vec<_>, _>>();
    let snapshots = match snapshots {
        Ok(snapshots) => snapshots,
        Err(error) => return failed_without_mutation(plan, before, error),
    };
    let mutation = issue_authority(spec, &before);
    let (authority, mutation_error) = match mutation {
        Ok(authority) => (Some(authority), None),
        Err(error) => (None, Some(error)),
    };
    let after_mutation = observe_machine_enrollment_authority(spec);
    let read_back = plan_machine_enrollment_authority(spec, &after_mutation);
    let mut blockers = Vec::new();
    if let Some(error) = mutation_error {
        blockers.push(error);
    }
    if read_back.status != "converged" {
        blockers.push("machine_enrollment_authority_read_back_failed".to_string());
    }
    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        return MachineEnrollmentAuthorityApplyReport {
            schema: MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA,
            status: "converged",
            mutates: true,
            requested_apply: true,
            plan,
            before,
            after: after_mutation,
            authority,
            proofs_observed: vec![
                "enrollment_authority_read_back",
                "enrollment_signing_key_mode_read_back",
                "staged_authority_public_key_read_back",
                "staged_one_time_token_digest_read_back",
            ],
            blockers,
            rollback_performed: false,
            secret_material_returned: false,
        };
    }

    let rollback_performed = restore_snapshots(&snapshots).is_ok();
    if !rollback_performed {
        blockers.push("machine_enrollment_authority_rollback_failed".to_string());
    }
    let after = observe_machine_enrollment_authority(spec);
    MachineEnrollmentAuthorityApplyReport {
        schema: MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA,
        status: "blocked",
        mutates: true,
        requested_apply: true,
        plan,
        before,
        after,
        authority: None,
        proofs_observed: Vec::new(),
        blockers,
        rollback_performed,
        secret_material_returned: false,
    }
}

fn issue_authority(
    spec: &MachineEnrollmentAuthoritySpec,
    before: &super::types::MachineEnrollmentAuthorityObservation,
) -> Result<MachineEnrollmentAuthority, String> {
    ensure_directory(
        Path::new(&spec.config_root),
        spec.owner_id,
        spec.group_id,
        0o750,
    )?;
    ensure_directory(
        Path::new(&spec.staging_root),
        spec.owner_id,
        spec.group_id,
        0o750,
    )?;
    let signing_material = if before.signing_key_present {
        let raw = fs::read_to_string(spec.signing_key_path())
            .map_err(|error| format!("machine_enrollment_signing_key_read_failed:{error}"))?;
        EnrollmentSecret::from_str(raw.trim())?
    } else {
        EnrollmentSecret::generate()?
    };
    let signing_bytes: [u8; 32] = hex::decode(signing_material.expose_str())
        .map_err(|_| "machine_enrollment_signing_key_decode_failed".to_string())?
        .try_into()
        .map_err(|_| "machine_enrollment_signing_key_length_invalid".to_string())?;
    let signing_key = SigningKey::from_bytes(&signing_bytes);
    let token = EnrollmentSecret::generate()?;
    let token_digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(token.expose_str().as_bytes()))
    );
    let expires_at_unix_s = spec
        .now_unix_s
        .checked_add(spec.policy.token_ttl_s)
        .ok_or_else(|| "machine_enrollment_token_expiry_overflow".to_string())?;
    let renewal_authority_expires_at_unix_s = spec
        .now_unix_s
        .checked_add(spec.policy.renewal_authority_ttl_s)
        .ok_or_else(|| "machine_enrollment_renewal_expiry_overflow".to_string())?;
    let authority = MachineEnrollmentAuthority {
        authority_id: spec.policy.authority_id.clone(),
        seed_id: spec.seed_id.clone(),
        token_digest,
        expires_at_unix_s,
        renewal_authority_expires_at_unix_s,
        max_grant_ttl_s: spec.policy.max_grant_ttl_s,
        allowed_roles: vec![spec.role.clone()],
        allowed_capabilities: spec.requested_capabilities.clone(),
    };

    atomic_write(
        Path::new(&spec.signing_key_path()),
        format!("{}\n", signing_material.expose_str()).as_bytes(),
        0o640,
        spec.owner_id,
        spec.group_id,
    )?;
    atomic_write(
        Path::new(&spec.staged_public_key_path()),
        format!("{}\n", hex::encode(signing_key.verifying_key().as_bytes())).as_bytes(),
        0o640,
        spec.owner_id,
        spec.group_id,
    )?;
    atomic_write(
        Path::new(&spec.staged_token_path()),
        format!("{}\n", token.expose_str()).as_bytes(),
        0o600,
        spec.owner_id,
        spec.group_id,
    )?;
    let mut authority_bytes = serde_json::to_vec_pretty(&authority)
        .map_err(|error| format!("machine_enrollment_authority_encode_failed:{error}"))?;
    authority_bytes.push(b'\n');
    atomic_write(
        Path::new(&spec.authority_path()),
        &authority_bytes,
        0o640,
        spec.owner_id,
        spec.group_id,
    )?;
    Ok(authority)
}

fn ensure_directory(path: &Path, owner_id: u32, group_id: u32, mode: u32) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("machine_enrollment_directory_create_failed:{error}"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("machine_enrollment_directory_mode_failed:{error}"))?;
    std::os::unix::fs::chown(path, Some(owner_id), Some(group_id))
        .map_err(|error| format!("machine_enrollment_directory_owner_failed:{error}"))
}

fn atomic_write(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    owner_id: u32,
    group_id: u32,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "machine_enrollment_target_parent_missing".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "machine_enrollment_target_name_invalid".to_string())?;
    let temporary = parent.join(format!(".{file_name}.next-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        std::os::unix::fs::chown(&temporary, Some(owner_id), Some(group_id))?;
        fs::rename(&temporary, path)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("machine_enrollment_atomic_write_failed:{error}"))
}

#[derive(Debug)]
struct FileSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    mode: Option<u32>,
    owner_id: Option<u32>,
    group_id: Option<u32>,
}

fn snapshot(path: &Path) -> Result<FileSnapshot, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let metadata = fs::metadata(path)
                .map_err(|error| format!("machine_enrollment_snapshot_failed:{error}"))?;
            Ok(FileSnapshot {
                path: path.to_path_buf(),
                bytes: Some(bytes),
                mode: Some(metadata.permissions().mode() & 0o777),
                owner_id: Some(metadata.uid()),
                group_id: Some(metadata.gid()),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileSnapshot {
            path: path.to_path_buf(),
            bytes: None,
            mode: None,
            owner_id: None,
            group_id: None,
        }),
        Err(error) => Err(format!("machine_enrollment_snapshot_failed:{error}")),
    }
}

fn restore_snapshots(snapshots: &[FileSnapshot]) -> Result<(), String> {
    for snapshot in snapshots.iter().rev() {
        if let Some(bytes) = &snapshot.bytes {
            atomic_write(
                &snapshot.path,
                bytes,
                snapshot.mode.unwrap_or(0o600),
                snapshot.owner_id.unwrap_or(0),
                snapshot.group_id.unwrap_or(0),
            )?;
        } else if snapshot.path.is_file() {
            fs::remove_file(&snapshot.path)
                .map_err(|error| format!("machine_enrollment_rollback_remove_failed:{error}"))?;
        }
    }
    Ok(())
}

fn affected_paths(spec: &MachineEnrollmentAuthoritySpec) -> [PathBuf; 4] {
    [
        PathBuf::from(spec.signing_key_path()),
        PathBuf::from(spec.staged_public_key_path()),
        PathBuf::from(spec.staged_token_path()),
        PathBuf::from(spec.authority_path()),
    ]
}

fn failed_without_mutation(
    plan: MachineEnrollmentAuthorityPlan,
    before: super::types::MachineEnrollmentAuthorityObservation,
    blocker: String,
) -> MachineEnrollmentAuthorityApplyReport {
    MachineEnrollmentAuthorityApplyReport {
        schema: MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA,
        status: "blocked",
        mutates: false,
        requested_apply: true,
        plan,
        before: before.clone(),
        after: before,
        authority: None,
        proofs_observed: Vec::new(),
        blockers: vec![blocker],
        rollback_performed: false,
        secret_material_returned: false,
    }
}

struct EnrollmentSecret(Vec<u8>);

impl EnrollmentSecret {
    fn generate() -> Result<Self, String> {
        let mut random = [0u8; 32];
        let result = std::fs::File::open("/dev/urandom")
            .and_then(|mut source| std::io::Read::read_exact(&mut source, &mut random));
        if let Err(error) = result {
            random.zeroize();
            return Err(format!("machine_enrollment_entropy_unavailable:{error}"));
        }
        let encoded = hex::encode(random);
        random.zeroize();
        Ok(Self(encoded.into_bytes()))
    }

    fn from_str(value: &str) -> Result<Self, String> {
        if value.is_empty() || !value.is_ascii() {
            return Err("machine_enrollment_secret_material_invalid".to_string());
        }
        Ok(Self(value.as_bytes().to_vec()))
    }

    fn expose_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl std::fmt::Debug for EnrollmentSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("EnrollmentSecret([REDACTED])")
    }
}

impl Drop for EnrollmentSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
