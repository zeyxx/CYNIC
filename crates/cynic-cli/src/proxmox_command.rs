use clap::Subcommand;
use std::process::ExitCode;

use crate::proxmox;

#[derive(Subcommand)]
pub enum ProxmoxCommand {
    /// Observe all Proxmox cluster resources
    Observe {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info for a specific resource
    Info {
        /// Resource ID (lxc/100, qemu/105, node/pve)
        resource_id: String,
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Destroy a stopped resource
    Destroy {
        /// Resource ID to destroy
        vmid: u32,
        /// Dry-run: verify only
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// List available VM/CT templates
    Templates,
}

pub(crate) fn execute_proxmox(command: ProxmoxCommand) -> ExitCode {
    match command {
        ProxmoxCommand::Observe { json } => {
            match proxmox::observe_resources() {
                Ok(output) => {
                    if json {
                        println!("{}", output);
                    } else {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
                            if let Some(resources) = json.as_array() {
                                println!("{:<12} {:<25} {:<10} {:<6} {:<6} {:<6}",
                                    "ID", "NAME", "STATUS", "CPU", "RAM", "DISK");
                                for r in resources {
                                    let id = r["id"].as_str().unwrap_or("?");
                                    let name = r["name"].as_str().unwrap_or("?");
                                    let status = r["status"].as_str().unwrap_or("?");
                                    let cpu = r["maxcpu"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                                    let ram = r["maxmem"].as_u64().map(|v| format!("{}G", v / 1024 / 1024)).unwrap_or_else(|| "-".to_string());
                                    let disk = r["maxdisk"].as_u64().map(|v| format!("{}G", v / 1024 / 1024 / 1024)).unwrap_or_else(|| "-".to_string());
                                    println!("{:<12} {:<25} {:<10} {:<6} {:<6} {:<6}", id, name, status, cpu, ram, disk);
                                }
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("proxmox error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        ProxmoxCommand::Info { resource_id, json } => {
            match proxmox::info_resource(&resource_id) {
                Ok(output) => {
                    println!("{}", output);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("proxmox error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        ProxmoxCommand::Destroy { vmid, dry_run, yes } => {
            if dry_run {
                println!("dry run: would destroy {} (no changes made)", vmid);
                return ExitCode::SUCCESS;
            }
            if !yes {
                eprintln!("use --yes to confirm destruction of {}", vmid);
                return ExitCode::FAILURE;
            }
            match proxmox::destroy_resource(vmid) {
                Ok(output) => {
                    println!("{}", output);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("proxmox error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        ProxmoxCommand::Templates => {
            match proxmox::list_templates() {
                Ok(output) => {
                    println!("{}", output);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("proxmox error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
    }
}
