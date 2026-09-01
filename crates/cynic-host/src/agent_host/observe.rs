//! Proxmox preflight observation adapter for Agent Host VM.

use cynic_protocol::AgentHostDesired;
use serde_json::Value;

use crate::update_lifecycle::ArgvRunner;

use super::types::{AGENT_HOST_PREFLIGHT_REPORT_SCHEMA, AgentHostPreflightReport};

/// Port for observing hypervisor preflight conditions.
pub trait AgentHostObservationPort {
    fn observe_agent_host_preflight(
        &self,
        desired: &AgentHostDesired,
    ) -> Result<AgentHostPreflightReport, String>;
}

/// Proxmox hypervisor observer via SSH runner executing pvesh.
pub struct ProxmoxSshAgentHostObserver<'a, R: ArgvRunner> {
    runner: &'a R,
    ssh_target: String,
}

impl<'a, R: ArgvRunner> ProxmoxSshAgentHostObserver<'a, R> {
    pub fn new(runner: &'a R, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_proxmox_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn pvesh(&self, args: &[String]) -> Result<crate::update_lifecycle::ArgvOutput, String> {
        let mut argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pvesh".to_string(),
        ];
        argv.extend(args.iter().cloned());
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if output.success() {
            Ok(output)
        } else {
            Err(format!(
                "proxmox_pvesh_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ))
        }
    }
}

impl<R: ArgvRunner> AgentHostObservationPort for ProxmoxSshAgentHostObserver<'_, R> {
    fn observe_agent_host_preflight(
        &self,
        desired: &AgentHostDesired,
    ) -> Result<AgentHostPreflightReport, String> {
        let mut blockers = Vec::new();

        // 1. Check hypervisor node status
        let status_res = self.pvesh(&[
            "get".to_string(),
            "/nodes/pve/status".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);

        let (
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
        ) = match status_res {
            Ok(output) => {
                let parsed: Value = serde_json::from_str(output.stdout.trim())
                    .map_err(|e| format!("invalid_status_json:{e}"))?;
                let version = parsed["pveversion"].as_str().map(ToString::to_string);
                let mem_total = parsed["memory"]["total"].as_u64().unwrap_or(0);
                let mem_used = parsed["memory"]["used"].as_u64().unwrap_or(0);
                let mem_avail = mem_total.saturating_sub(mem_used);
                let disk_avail = parsed["rootfs"]["avail"].as_u64().unwrap_or(0);
                (true, version, mem_avail, disk_avail)
            }
            Err(e) => {
                blockers.push(format!("hypervisor_status_unreachable:{e}"));
                (false, None, 0, 0)
            }
        };

        // 2. Check cluster resources to see if storage pool and VMID are free
        let resources_res = self.pvesh(&[
            "get".to_string(),
            "/cluster/resources".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);

        let mut vmid_available = true;
        let mut vmid_existing_status = None;
        let mut storage_pool_available = false;

        if let Ok(output) = resources_res {
            if let Ok(resources) = serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                for item in resources {
                    if let Some(vmid) = item.get("vmid").and_then(Value::as_u64) {
                        if vmid == desired.vmid as u64 {
                            vmid_available = false;
                            vmid_existing_status = item
                                .get("status")
                                .and_then(Value::as_str)
                                .map(ToString::to_string);
                        }
                    }
                    if item.get("type").and_then(Value::as_str) == Some("storage")
                        && item.get("storage").and_then(Value::as_str)
                            == Some(&desired.storage_pool)
                    {
                        storage_pool_available = true;
                    }
                }
            }
        }

        // 3. Check storage/local/content to verify noble cloud-init image is present
        let content_res = self.pvesh(&[
            "get".to_string(),
            "/nodes/pve/storage/local/content".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);

        let mut template_present = false;
        let mut template_volid = None;

        if let Ok(output) = content_res {
            if let Ok(contents) = serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                for item in contents {
                    if let Some(volid) = item.get("volid").and_then(Value::as_str) {
                        if volid.contains("cloudimg") || volid.contains("noble") {
                            template_present = true;
                            template_volid = Some(volid.to_string());
                            break;
                        }
                    }
                }
            }
        }

        // 4. Check network interfaces for vmbr0
        let network_res = self.pvesh(&[
            "get".to_string(),
            "/nodes/pve/network".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);

        let mut network_bridge_present = false;
        if let Ok(output) = network_res {
            if let Ok(ifaces) = serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                for item in ifaces {
                    if item.get("iface").and_then(Value::as_str) == Some(&desired.network_bridge) {
                        network_bridge_present = true;
                        break;
                    }
                }
            }
        }

        Ok(AgentHostPreflightReport {
            schema: AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
            vmid_available,
            vmid_existing_status,
            template_present,
            template_volid,
            storage_pool_available,
            network_bridge_present,
            blockers,
        })
    }
}
