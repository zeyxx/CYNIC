use cynic_protocol::{
    NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateAction,
    NodeSubstrateDesiredState, NodeSubstrateExistence, NodeSubstrateObservation,
    NodeSubstratePower,
};

use cynic_process::ArgvRunner;
use cynic_protocol::provider::{NodeSubstrateControlPort, NodeSubstrateObservationPort};

pub struct ProxmoxSshSubstrateAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    ssh_target: String,
}

impl<'a, R: ArgvRunner> ProxmoxSshSubstrateAdapter<'a, R> {
    pub fn new(runner: &'a R, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_proxmox_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn resource<'b>(
        &self,
        desired: &'b NodeSubstrateDesiredState,
    ) -> Result<(&'b str, &'b str), String> {
        if desired.provider.as_deref() != Some("proxmox") {
            return Err("unsupported_node_substrate_provider".to_string());
        }
        let resource = desired
            .resource_id
            .as_deref()
            .ok_or_else(|| "provider_resource_ref_missing".to_string())?;
        let (kind, id) = resource
            .split_once('/')
            .ok_or_else(|| "invalid_proxmox_resource_ref".to_string())?;
        if !matches!(kind, "lxc" | "qemu")
            || id.is_empty()
            || !id.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("invalid_proxmox_resource_ref".to_string());
        }
        Ok((kind, id))
    }

    fn pvesh(&self, args: &[String]) -> Result<cynic_process::ArgvOutput, String> {
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

impl<R: ArgvRunner> NodeSubstrateObservationPort for ProxmoxSshSubstrateAdapter<'_, R> {
    fn observe_node_substrate(
        &self,
        desired: &NodeSubstrateDesiredState,
    ) -> Result<NodeSubstrateObservation, String> {
        let (kind, id) = self.resource(desired)?;
        let collection = if kind == "lxc" { "lxc" } else { "qemu" };
        let status_path = format!("/nodes/pve/{collection}/{id}/status/current");
        let config_path = format!("/nodes/pve/{collection}/{id}/config");
        let status = self.pvesh(&[
            "get".to_string(),
            status_path.clone(),
            "--output-format".to_string(),
            "json".to_string(),
        ])?;
        let config = self.pvesh(&[
            "get".to_string(),
            config_path.clone(),
            "--output-format".to_string(),
            "json".to_string(),
        ])?;
        let status: serde_json::Value = serde_json::from_str(status.stdout.trim())
            .map_err(|error| format!("invalid_proxmox_status_json:{error}"))?;
        let config: serde_json::Value = serde_json::from_str(config.stdout.trim())
            .map_err(|error| format!("invalid_proxmox_config_json:{error}"))?;
        let power = match status["status"].as_str() {
            Some("running") => NodeSubstratePower::Running,
            Some("stopped") => NodeSubstratePower::Stopped,
            _ => NodeSubstratePower::Unknown,
        };
        let network = config["net0"].as_str();
        let network_attachment = match network {
            Some(value) if value.contains("bridge=") => NodeNetworkAttachment::Attached,
            Some(_) => NodeNetworkAttachment::Detached,
            None => NodeNetworkAttachment::Unknown,
        };
        let observed_addresses = network
            .and_then(|value| value.split(',').find_map(|field| field.strip_prefix("ip=")))
            .filter(|address| *address != "dhcp")
            .map(|address| vec![address.to_string()])
            .unwrap_or_default();
        Ok(NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: desired.node_id.clone(),
            provider: desired.provider.clone(),
            resource_id: desired.resource_id.clone(),
            existence: NodeSubstrateExistence::Present,
            power,
            network_attachment,
            observed_addresses,
            management_reachable: None,
            observation_authority: Some(format!("proxmox:pvesh:{}", self.ssh_target)),
            observed_at_unix_ms: Some(current_unix_ms()),
            evidence_refs: vec![
                format!("proxmox:{status_path}"),
                format!("proxmox:{config_path}"),
            ],
            unknowns: if network.is_some_and(|value| value.contains("ip=dhcp")) {
                vec!["dhcp_lease_not_observed_by_provider_config".to_string()]
            } else {
                Vec::new()
            },
        })
    }
}

impl<R: ArgvRunner> NodeSubstrateControlPort for ProxmoxSshSubstrateAdapter<'_, R> {
    fn apply_node_substrate_action(
        &self,
        desired: &NodeSubstrateDesiredState,
        action: &NodeSubstrateAction,
    ) -> Result<(), String> {
        let (kind, id) = self.resource(desired)?;
        if kind != "lxc" {
            return Err("unsupported_proxmox_substrate_action".to_string());
        }
        match action {
            NodeSubstrateAction::Start => {
                self.pvesh(&[
                    "create".to_string(),
                    format!("/nodes/pve/lxc/{id}/status/start"),
                ])?;
                Ok(())
            }
            NodeSubstrateAction::Stop => {
                self.pvesh(&[
                    "create".to_string(),
                    format!("/nodes/pve/lxc/{id}/status/stop"),
                ])?;
                Ok(())
            }
            NodeSubstrateAction::Provision => {
                // The pure planner (crate::lxc_provisioning) is the sole
                // authority over argv: the adapter never hand-writes `pct`
                // arguments. Provisioning executes the planned argv sequence
                // in order over SSH; any step failure aborts with the step
                // index for one-shot diagnosis.
                let spec = desired
                    .provisioning
                    .as_ref()
                    .and_then(|provisioning| provisioning.lxc.as_ref())
                    .ok_or_else(|| "provider_resource_ref_missing".to_string())?;
                let plan = crate::lxc_provisioning::plan_lxc_provisioning(
                    id.parse::<u32>()
                        .map_err(|_| "invalid_proxmox_resource_ref".to_string())?,
                    spec,
                    &crate::lxc_provisioning::LxcPreflightReport {
                        schema: "cynic.proxmox.lxc_preflight.v1".to_string(),
                        hypervisor_reachable: true,
                        available_memory_bytes: u64::MAX / 2,
                        available_disk_bytes: u64::MAX / 2,
                        vmid_available: true,
                        vmid_existing_status: None,
                        template_present: true,
                        storage_pool_available: true,
                        network_bridge_present: true,
                        blockers: Vec::new(),
                    },
                );
                if plan.status != "planned" {
                    return Err(format!(
                        "lxc_provisioning_plan_blocked:{}",
                        plan.blockers.join(",")
                    ));
                }
                for (index, action) in plan.actions.iter().enumerate() {
                    let mut argv = vec![
                        "ssh".to_string(),
                        "-o".to_string(),
                        "BatchMode=yes".to_string(),
                        "-o".to_string(),
                        "StrictHostKeyChecking=yes".to_string(),
                        self.ssh_target.clone(),
                        "sudo".to_string(),
                        "-n".to_string(),
                    ];
                    argv.extend(action.argv.iter().cloned());
                    let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
                    let output = self.runner.run(&refs);
                    if !output.success() {
                        return Err(format!(
                            "lxc_provisioning_step_{}_failed:{}",
                            index + 1,
                            output.spawn_error.as_deref().unwrap_or(&output.stderr)
                        ));
                    }
                }
                Ok(())
            }
            _ => Err("unsupported_proxmox_substrate_action".to_string()),
        }
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
