//! LXC Exec Bridge Adapter
//!
//! Cet adaptateur remplace le prototype Python `organ-fleet`.
//! Il interagit avec le helper sécurisé `cynic-lxc-exec-bridge` sur l'hyperviseur Proxmox
//! via `sudo -n`, en passant la requête au format JSON par stdin.
//! C'est ce composant qui permet au Kernel de restart `llama-server` ou de lire `nvidia-smi`
//! sans jamais ouvrir de connexion SSH.

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Requête formatée pour le helper `cynic-lxc-exec-bridge`
#[derive(Debug, Deserialize, Serialize)]
pub struct LxcExecRequest {
    pub action: String, // ex: "exec", "status"
    pub vmid: u32,
    pub command: Vec<String>,
}

/// Réponse retournée par l'exécution dans le conteneur LXC
#[derive(Debug, Deserialize, Serialize)]
pub struct LxcExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct LxcExecBridge;

impl LxcExecBridge {
    /// Exécute une commande de manière isolée dans le conteneur LXC
    pub async fn execute(request: LxcExecRequest) -> Result<LxcExecResponse, String> {
        let payload =
            serde_json::to_string(&request).map_err(|e| format!("Serialization error: {}", e))?;

        // `sudo -n` garantit qu'on ne bloque pas en demandant un mot de passe.
        // Seul l'utilisateur cynic est autorisé via /etc/sudoers.d/cynic-lxc-exec-bridge
        let mut child = Command::new("sudo")
            .arg("-n")
            .arg("cynic-lxc-exec-bridge")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sudo cynic-lxc-exec-bridge: {}", e))?;

        // On injecte le JSON dans stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| format!("Stdin write error: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| format!("Wait output error: {}", e))?;

        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            // Le helper Python d'origine retourne normalement du JSON structuré.
            match serde_json::from_str::<LxcExecResponse>(&stdout_str) {
                Ok(resp) => Ok(resp),
                Err(_) => {
                    // Fallback si le helper crache directement le raw output
                    Ok(LxcExecResponse {
                        exit_code: 0,
                        stdout: stdout_str,
                        stderr: stderr_str,
                    })
                }
            }
        } else {
            Err(format!(
                "LXC Bridge execution failed with code {:?}. Stderr: {}",
                output.status.code(),
                stderr_str
            ))
        }
    }
}
