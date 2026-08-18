use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretStorageState {
    PlainText,
    EncryptedSystemdCreds,
    Unknown,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretHardeningObservation {
    pub path: PathBuf,
    pub state: SecretStorageState,
    pub identity: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretHardeningPlan {
    pub status: String,
    pub actions: Vec<SecretHardeningAction>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretHardeningAction {
    EncryptWithSystemdCreds,
    RotateEncryptedSecret,
}

pub fn observe_secret_hardening(path: &Path, identity: &str) -> SecretHardeningObservation {
    if !path.exists() {
        return SecretHardeningObservation {
            path: path.to_path_buf(),
            state: SecretStorageState::Missing,
            identity: identity.to_string(),
            evidence_refs: vec![],
        };
    }

    // Check if it's a systemd-cred by trying to decrypt the first few bytes or using a tool
    // For now, we check if the file is plain text or if systemd-creds recognizes it
    let output = Command::new("systemd-creds")
        .args(["decrypt", path.to_str().unwrap_or(""), "--dry-run"])
        .output();

    let state = match output {
        Ok(out) if out.status.success() => SecretStorageState::EncryptedSystemdCreds,
        _ => SecretStorageState::PlainText,
    };

    SecretHardeningObservation {
        path: path.to_path_buf(),
        state,
        identity: identity.to_string(),
        evidence_refs: vec![format!("secret-observe:{}", path.display())],
    }
}

pub fn reconcile_secret_hardening(
    observation: &SecretHardeningObservation,
) -> SecretHardeningPlan {
    match observation.state {
        SecretStorageState::PlainText => SecretHardeningPlan {
            status: "planned".to_string(),
            actions: vec![SecretHardeningAction::EncryptWithSystemdCreds],
            blockers: vec![],
        },
        SecretStorageState::EncryptedSystemdCreds => SecretHardeningPlan {
            status: "converged".to_string(),
            actions: vec![],
            blockers: vec![],
        },
        _ => SecretHardeningPlan {
            status: "blocked".to_string(),
            actions: vec![],
            blockers: vec!["secret_not_found".to_string()],
        },
    }
}

pub fn apply_secret_hardening(
    observation: &SecretHardeningObservation,
    plan: &SecretHardeningPlan,
) -> Result<(), String> {
    for action in &plan.actions {
        match action {
            SecretHardeningAction::EncryptWithSystemdCreds => {
                let temp_path = observation.path.with_extension("tmp");
                
                // 1. Read plain text
                let content = std::fs::read(&observation.path)
                    .map_err(|e| format!("read_plain_secret_failed:{e}"))?;
                
                // 2. Create temporary file for encryption
                std::fs::write(&temp_path, &content)
                    .map_err(|e| format!("write_temp_secret_failed:{e}"))?;
                
                // 3. Encrypt using systemd-creds
                // systemd-creds encrypt --name=<identity> <src> <dst>
                let status = Command::new("sudo")
                    .args([
                        "systemd-creds",
                        "encrypt",
                        "--name",
                        &observation.identity,
                        temp_path.to_str().unwrap(),
                        observation.path.to_str().unwrap(),
                    ])
                    .status()
                    .map_err(|e| format!("systemd_creds_spawn_failed:{e}"))?;
                
                if !status.success() {
                    return Err("systemd_creds_encryption_failed".to_string());
                }
                
                // 4. Cleanup
                let _ = std::fs::remove_file(&temp_path);
            }
            SecretHardeningAction::RotateEncryptedSecret => {
                return Err("rotate_not_implemented".to_string());
            }
        }
    }
    Ok(())
}
