//! Bare-Metal / Host Exec Adapter
//! 
//! Cet adaptateur permet à `cynic-node` d'exécuter des actions natives sur un nœud 
//! Bare-Metal, AWS EC2, ou un conteneur standard (non-LXC Proxmox).
//! Typiquement utilisé pour interagir avec `systemd` (ex: restart llama-server) 
//! ou lire des métriques GPU (`nvidia-smi`) directement sur l'hôte.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct HostExecRequest {
    pub action: String, // ex: "systemd_restart", "shell"
    pub command: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct HostExecAdapter;

impl HostExecAdapter {
    pub async fn execute(request: HostExecRequest) -> Result<HostExecResponse, String> {
        if request.command.is_empty() {
            return Err("Command vector cannot be empty".to_string());
        }

        let root_cmd = &request.command[0];
        
        // Par sécurité, on peut whitelister certaines commandes comme systemctl ou nvidia-smi
        if root_cmd != "systemctl" && root_cmd != "nvidia-smi" && root_cmd != "echo" {
            return Err(format!("Command '{}' is not whitelisted for HostExecAdapter", root_cmd));
        }

        let args = &request.command[1..];
        
        let output = Command::new(root_cmd)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to spawn {}: {}", root_cmd, e))?;

        Ok(HostExecResponse {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
