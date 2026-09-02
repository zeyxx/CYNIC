use clap::Subcommand;

use crate::cli_json::emit_pretty_json;
use cynic_protocol::proxmox::*;
use cynic_node::adapters::proxmox_control::ProxmoxControlAdapter;

#[derive(Subcommand)]
pub(crate) enum HypervisorCommand {
    #[command(name = "status")]
    Status {
        #[arg(long)]
        json: bool,
    },
    #[command(name = "list")]
    List {
        #[arg(long)]
        json: bool,
    },
    #[command(name = "extend-disk")]
    ExtendDisk {
        #[arg(long)]
        vmid: u32,
        #[arg(long)]
        size_gb: u64,
        #[arg(long)]
        yes: bool,
    },
    #[command(name = "snapshot")]
    Snapshot {
        #[arg(long)]
        vmid: u32,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    #[command(name = "rollback")]
    Rollback {
        #[arg(long)]
        vmid: u32,
        #[arg(long)]
        name: String,
    },
}

pub(crate) async fn execute_hypervisor(command: HypervisorCommand) -> Result<(), String> {
    let adapter = ProxmoxControlAdapter::from_env(std::time::Duration::from_secs(30))?;

    match command {
        HypervisorCommand::Status { json } => {
            let status = ProxmoxControlAdapter::observe_from_env(std::time::Duration::from_secs(5)).await;
            if json {
                emit_pretty_json(&status);
            } else {
                println!("Hypervisor: {}", status.status);
                println!("Reachable: {:?}", status.api_reachable);
            }
            Ok(())
        }
        HypervisorCommand::List { json } => {
            let status = ProxmoxControlAdapter::observe_from_env(std::time::Duration::from_secs(5)).await;
            if json {
                emit_pretty_json(&status);
            } else {
                println!("Hypervisor: {} VMs={:?}", status.status, status.resource_count);
            }
            Ok(())
        }
        HypervisorCommand::ExtendDisk { vmid, size_gb, yes } => {
            let ext = DiskExtension::new(vmid, size_gb);
            let blockers = ext.validate();
            if !blockers.is_empty() {
                return Err(format!("Invalid extension: {}", blockers.join(", ")));
            }
            let resources = adapter.cluster_resources().await
                .map_err(|e| format!("Failed to query resources: {e}"))?;
            let node = find_vm_node(&resources, vmid)
                .ok_or_else(|| format!("VM {vmid} not found"))?;
            let disk = match ext.controller {
                DiskController::RootFS => "rootfs",
                DiskController::SCSI0 => "scsi0",
                DiskController::SCSI1 => "scsi1",
                DiskController::IDE0 => "ide0",
                DiskController::VirtIO0 => "virtio0",
            };
            if !yes {
                println!("Will extend VM {vmid} disk '{disk}' by {size_gb}G on node '{node}'");
                println!("Use --yes to execute.");
                return Ok(());
            }
            println!("Extending VM {vmid} disk '{disk}' by {size_gb}G...");
            match adapter.lxc_resize(&node, &vmid.to_string(), disk, size_gb).await {
                Ok(result) => {
                    println!("Success: disk extended");
                    emit_pretty_json(&result);
                    Ok(())
                }
                Err(e) => Err(format!("Extension failed: {e}")),
            }
        }
        HypervisorCommand::Snapshot { vmid, name, description } => {
            let resources = adapter.cluster_resources().await
                .map_err(|e| format!("Failed to query resources: {e}"))?;
            let node = find_vm_node(&resources, vmid)
                .ok_or_else(|| format!("VM {vmid} not found"))?;
            let vm_type = if vm_lxc(&adapter, &node, vmid).await { "lxc" } else { "qemu" };
            let result = adapter.create_snapshot(&node, vm_type, vmid, &name, description.as_deref()).await
                .map_err(|e| format!("Snapshot failed: {e}"))?;
            println!("Snapshot '{name}' created for VM {vmid}");
            emit_pretty_json(&result);
            Ok(())
        }
        HypervisorCommand::Rollback { vmid, name } => {
            let resources = adapter.cluster_resources().await
                .map_err(|e| format!("Failed to query resources: {e}"))?;
            let node = find_vm_node(&resources, vmid)
                .ok_or_else(|| format!("VM {vmid} not found"))?;
            let vm_type = if vm_lxc(&adapter, &node, vmid).await { "lxc" } else { "qemu" };
            let result = adapter.rollback_snapshot(&node, vm_type, vmid, &name).await
                .map_err(|e| format!("Rollback failed: {e}"))?;
            println!("VM {vmid} rolled back to '{name}'");
            emit_pretty_json(&result);
            Ok(())
        }
    }
}

fn find_vm_node(resources: &serde_json::Value, vmid: u32) -> Option<String> {
    let arr = resources.as_array()?;
    for item in arr {
        if item.get("vmid").and_then(serde_json::Value::as_u64) == Some(vmid as u64) {
            return item.get("node").and_then(serde_json::Value::as_str).map(String::from);
        }
    }
    None
}

async fn vm_lxc(adapter: &ProxmoxControlAdapter, node: &str, vmid: u32) -> bool {
    adapter.get_json(&format!("/api2/json/nodes/{node}/lxc/{vmid}/status/current")).await.is_ok()
}
