//! Bare-Metal / Host Exec Adapter
//!
//! Cet adaptateur permet à `cynic-node` d'exécuter des actions natives sur un nœud
//! Bare-Metal, AWS EC2, ou un conteneur standard (non-LXC Proxmox).
//! Typiquement utilisé pour interagir avec `systemd` (ex: restart llama-server)
//! ou lire des métriques GPU (`nvidia-smi`) directement sur l'hôte.

use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::{Deserialize, Serialize};

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
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        let nvidia_smi_bin = runtime_layout.nvidia_smi_bin();
        let allowed_commands = [
            capabilities.service_manager.command.as_str(),
            nvidia_smi_bin.as_str(),
            "echo",
        ];

        if !allowed_commands.contains(&root_cmd.as_str()) {
            return Err(format!(
                "Command '{}' is not whitelisted for HostExecAdapter",
                root_cmd
            ));
        }

        let args = &request.command[1..];
        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();

        let output = capture_command(root_cmd, &args_ref)
            .map_err(|e| format!("Failed to spawn {}: {}", root_cmd, e))?;

        Ok(HostExecResponse {
            exit_code: output.exit_code.unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
