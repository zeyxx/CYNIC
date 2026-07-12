//! LXC Exec Bridge Adapter
//!
//! Cet adaptateur remplace le prototype Python `organ-fleet`.
//! Il interagit avec le helper sécurisé `cynic-lxc-exec-bridge` sur l'hyperviseur Proxmox
//! via `sudo -n`, en passant la requête au format JSON par stdin.
//! C'est ce composant qui permet au Kernel de restart `llama-server` ou de lire `nvidia-smi`
//! sans jamais ouvrir de connexion SSH.

use cynic_host::process_observation::capture_command_with_stdin;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::{Deserialize, Serialize};

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
        let runtime_layout = current_runtime_layout();
        let sudo_bin = runtime_layout.sudo_bin();
        let lxc_exec_bridge_bin = runtime_layout.lxc_exec_bridge_bin();

        let output = capture_command_with_stdin(&sudo_bin, &["-n", &lxc_exec_bridge_bin], &payload)
            .await
            .map_err(|e| format!("Failed to spawn sudo {lxc_exec_bridge_bin}: {e}"))?;

        if output.success {
            // Le helper Python d'origine retourne normalement du JSON structuré.
            match serde_json::from_str::<LxcExecResponse>(&output.stdout) {
                Ok(resp) => Ok(resp),
                Err(_) => {
                    // Fallback si le helper crache directement le raw output
                    Ok(LxcExecResponse {
                        exit_code: 0,
                        stdout: output.stdout,
                        stderr: output.stderr,
                    })
                }
            }
        } else {
            Err(format!(
                "LXC Bridge execution failed with code {:?}. Stderr: {}",
                output.exit_code, output.stderr
            ))
        }
    }
}
