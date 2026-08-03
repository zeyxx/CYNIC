use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::ports::credential_material_sink_port::CredentialMaterialSinkPort;

/// Local runtime sink for a secret already provisioned with a secure parent.
/// It never creates a missing parent and never follows a symlink.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalCredentialMaterialSink;

impl CredentialMaterialSinkPort for LocalCredentialMaterialSink {
    fn store(&self, secret_ref: &str, material: &[u8]) -> Result<String, String> {
        let path = PathBuf::from(secret_ref);
        validate_path(&path)?;
        let parent = path
            .parent()
            .ok_or_else(|| "credential_material_parent_missing".to_string())?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|error| format!("credential_material_parent_unreadable:{error}"))?;
        if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
            return Err("credential_material_parent_rejected".to_string());
        }
        if fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err("credential_material_target_symlink".to_string());
        }
        let name = path
            .file_name()
            .ok_or_else(|| "credential_material_filename_missing".to_string())?
            .to_string_lossy();
        let temp = parent.join(format!(".{name}.{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o640);
        }
        let mut file = options
            .open(&temp)
            .map_err(|error| format!("credential_material_temp_create_failed:{error}"))?;
        file.write_all(material)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("credential_material_temp_sync_failed:{error}"))?;
        fs::rename(&temp, &path)
            .map_err(|error| format!("credential_material_publish_failed:{error}"))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("credential_material_parent_sync_failed:{error}"))?;
        let read_back = fs::read(&path)
            .map_err(|error| format!("credential_material_read_back_failed:{error}"))?;
        if read_back != material {
            return Err("credential_material_read_back_mismatch".to_string());
        }
        Ok(format!(
            "sha256:{}",
            hex::encode(Sha256::digest(&read_back))
        ))
    }
}

fn validate_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("credential_material_path_invalid".to_string());
    }
    Ok(())
}
