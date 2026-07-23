use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use cynic_protocol::digest_json;

use crate::hardware_baseline_apply::{
    HardwareBaselineInstallPort, HardwareBaselineInstallProof, HardwareBaselineInstallSpec,
};
use crate::hardware_conformance::HardwareBaseline;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHardwareBaselineInstaller;

impl HardwareBaselineInstallPort for LocalHardwareBaselineInstaller {
    fn install_absent_only(
        &self,
        spec: &HardwareBaselineInstallSpec,
    ) -> Result<HardwareBaselineInstallProof, String> {
        let target = Path::new(&spec.target_path);
        let parent = target
            .parent()
            .ok_or_else(|| "hardware_baseline_target_parent_missing".to_string())?;
        if !parent.is_dir() {
            return Err("hardware_baseline_target_parent_absent".to_string());
        }
        if target.exists() {
            return Err("hardware_baseline_target_already_exists".to_string());
        }
        let baseline = serde_json::from_slice::<HardwareBaseline>(&spec.payload)
            .map_err(|error| format!("hardware_baseline_payload_invalid:{error}"))?;
        let payload_digest = digest_json(&baseline).map_err(|error| error.to_string())?;
        let temporary = temporary_path(target, &payload_digest)?;
        let result = install_via_atomic_link(&temporary, target, spec);
        let _ = fs::remove_file(&temporary);
        result?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("hardware_baseline_parent_sync_failed:{error}"))?;
        Ok(HardwareBaselineInstallProof {
            target_path: spec.target_path.clone(),
            payload_digest,
            created: true,
        })
    }
}

fn temporary_path(target: &Path, digest: &str) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or_else(|| "hardware_baseline_target_name_missing".to_string())?
        .to_string_lossy();
    let suffix = digest.strip_prefix("sha256:").unwrap_or(digest);
    Ok(target.with_file_name(format!(".{name}.{suffix}.tmp")))
}

fn install_via_atomic_link(
    temporary: &Path,
    target: &Path,
    spec: &HardwareBaselineInstallSpec,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .map_err(|error| format!("hardware_baseline_temp_create_failed:{error}"))?;
    file.set_permissions(fs::Permissions::from_mode(spec.mode))
        .and_then(|_| file.write_all(&spec.payload))
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("hardware_baseline_temp_write_failed:{error}"))?;
    fs::hard_link(temporary, target).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            "hardware_baseline_target_already_exists".to_string()
        } else {
            format!("hardware_baseline_atomic_link_failed:{error}")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_conformance::HARDWARE_BASELINE_SCHEMA;

    #[test]
    fn installer_creates_once_and_refuses_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("hardware-baseline.json");
        let baseline = HardwareBaseline {
            schema: HARDWARE_BASELINE_SCHEMA.to_string(),
            subject: "fixture".to_string(),
            inventory_digest: format!("sha256:{}", "a".repeat(64)),
        };
        let mut payload = serde_json::to_vec_pretty(&baseline).expect("payload");
        payload.push(b'\n');
        let spec = HardwareBaselineInstallSpec {
            target_path: target.display().to_string(),
            payload,
            mode: 0o644,
        };
        let installer = LocalHardwareBaselineInstaller;
        assert!(installer.install_absent_only(&spec).is_ok());
        assert_eq!(
            installer.install_absent_only(&spec),
            Err("hardware_baseline_target_already_exists".to_string())
        );
    }
}
