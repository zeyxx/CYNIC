//! Pure reconciliation decisions for the Agent Host VM.

use cynic_protocol::{AgentHostDesired, validate_agent_host_desired};

use super::types::{
    AGENT_HOST_VM_PLAN_SCHEMA, AgentHostPreflightReport, AgentHostVmAction, AgentHostVmPlan,
};

/// Pure decision function reconciling desired state with observed hypervisor preflight.
pub fn plan_agent_host_vm(
    desired: &AgentHostDesired,
    preflight: &AgentHostPreflightReport,
) -> AgentHostVmPlan {
    let mut blockers = Vec::new();

    // 1. Validate desired state schema & invariants
    blockers.extend(validate_agent_host_desired(desired));

    // 2. Validate preflight observation
    if !preflight.hypervisor_reachable {
        blockers.push("hypervisor_unreachable".to_string());
    }
    if preflight.available_memory_bytes < desired.memory_bytes {
        blockers.push(format!(
            "insufficient_hypervisor_memory: required={}, available={}",
            desired.memory_bytes, preflight.available_memory_bytes
        ));
    }
    if preflight.available_disk_bytes < desired.disk_bytes {
        blockers.push(format!(
            "insufficient_hypervisor_disk: required={}, available={}",
            desired.disk_bytes, preflight.available_disk_bytes
        ));
    }
    if !preflight.storage_pool_available {
        blockers.push(format!("storage_pool_unavailable:{}", desired.storage_pool));
    }
    if !preflight.network_bridge_present {
        blockers.push(format!("network_bridge_missing:{}", desired.network_bridge));
    }
    if !preflight.template_present {
        blockers.push("cloudinit_base_image_missing".to_string());
    }
    let existing_vm_running =
        !preflight.vmid_available && preflight.vmid_existing_status.as_deref() == Some("running");
    if !preflight.vmid_available && !existing_vm_running {
        blockers.push(format!("vmid_already_in_use:{}", desired.vmid));
    }

    blockers.extend(preflight.blockers.iter().cloned());
    blockers.sort();
    blockers.dedup();

    if existing_vm_running {
        return AgentHostVmPlan {
            schema: AGENT_HOST_VM_PLAN_SCHEMA.to_string(),
            status: if blockers.is_empty() {
                "converged".to_string()
            } else {
                "blocked".to_string()
            },
            desired: desired.clone(),
            preflight: preflight.clone(),
            actions: vec![],
            rollback_intent: "none_existing_vm_not_modified".to_string(),
            mutates: false,
            blockers,
        };
    }

    let vmid = desired.vmid;
    let template_volid = preflight
        .template_volid
        .as_deref()
        .unwrap_or("local:iso/noble-server-cloudimg-amd64.img");
    let disk_gib = desired.disk_bytes / (1024 * 1024 * 1024);
    let ram_mib = desired.memory_bytes / (1024 * 1024);
    let ip_config = desired
        .network_ip
        .as_deref()
        .unwrap_or("10.10.10.102/24,gw=10.10.10.1");

    let actions = if blockers.is_empty() {
        vec![
            AgentHostVmAction {
                step: 1,
                kind: "create_qemu_vm".to_string(),
                purpose: format!(
                    "create base QEMU VM {vmid} with {} cores, {} MiB RAM",
                    desired.vcpus, ram_mib
                ),
                argv: vec![
                    "qm".to_string(),
                    "create".to_string(),
                    vmid.to_string(),
                    "--name".to_string(),
                    desired.host_node_id.clone(),
                    "--cores".to_string(),
                    desired.vcpus.to_string(),
                    "--memory".to_string(),
                    ram_mib.to_string(),
                    "--net0".to_string(),
                    format!("virtio,bridge={}", desired.network_bridge),
                    "--scsihw".to_string(),
                    "virtio-scsi-pci".to_string(),
                ],
                privileged: true,
            },
            AgentHostVmAction {
                step: 2,
                kind: "import_cloudinit_disk".to_string(),
                purpose: format!(
                    "import cloud image disk onto storage pool {}",
                    desired.storage_pool
                ),
                argv: vec![
                    "qm".to_string(),
                    "set".to_string(),
                    vmid.to_string(),
                    "--scsi0".to_string(),
                    format!(
                        "{}:{},import-from={}",
                        desired.storage_pool, disk_gib, template_volid
                    ),
                ],
                privileged: true,
            },
            AgentHostVmAction {
                step: 3,
                kind: "attach_cloudinit_drive".to_string(),
                purpose: "attach cloud-init metadata drive".to_string(),
                argv: vec![
                    "qm".to_string(),
                    "set".to_string(),
                    vmid.to_string(),
                    "--ide2".to_string(),
                    format!("{}:cloudinit", desired.storage_pool),
                ],
                privileged: true,
            },
            AgentHostVmAction {
                step: 4,
                kind: "configure_boot_and_display".to_string(),
                purpose: "set boot order and serial console".to_string(),
                argv: vec![
                    "qm".to_string(),
                    "set".to_string(),
                    vmid.to_string(),
                    "--boot".to_string(),
                    "order=scsi0".to_string(),
                    "--serial0".to_string(),
                    "socket".to_string(),
                    "--vga".to_string(),
                    "serial0".to_string(),
                ],
                privileged: true,
            },
            AgentHostVmAction {
                step: 5,
                kind: "configure_network_and_user".to_string(),
                purpose: format!("configure IP {} and user {}", ip_config, desired.principal),
                argv: vec![
                    "qm".to_string(),
                    "set".to_string(),
                    vmid.to_string(),
                    "--ipconfig0".to_string(),
                    format!("ip={}", ip_config),
                    "--nameserver".to_string(),
                    "10.10.10.1".to_string(),
                    "--searchdomain".to_string(),
                    "synapse.cynic".to_string(),
                    "--ciuser".to_string(),
                    desired.principal.clone(),
                ],
                privileged: true,
            },
            AgentHostVmAction {
                step: 6,
                kind: "start_vm".to_string(),
                purpose: format!("start cynic-agent-host VM {vmid}"),
                argv: vec!["qm".to_string(), "start".to_string(), vmid.to_string()],
                privileged: true,
            },
        ]
    } else {
        vec![]
    };

    let status = if blockers.is_empty() {
        "planned".to_string()
    } else {
        "blocked".to_string()
    };

    let rollback_intent =
        format!("qm stop {vmid} ; qm destroy {vmid} --purge 1 --destroy-unreferenced-disks 1");
    let mutates = !actions.is_empty();

    AgentHostVmPlan {
        schema: AGENT_HOST_VM_PLAN_SCHEMA.to_string(),
        status,
        desired: desired.clone(),
        preflight: preflight.clone(),
        actions,
        rollback_intent,
        mutates,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_preflight() -> AgentHostPreflightReport {
        AgentHostPreflightReport {
            schema: super::super::types::AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable: true,
            hypervisor_version: Some("pve-manager/8.4.0".to_string()),
            available_memory_bytes: 14 * 1024 * 1024 * 1024,
            available_disk_bytes: 750 * 1024 * 1024 * 1024,
            vmid_available: true,
            vmid_existing_status: None,
            template_present: true,
            template_volid: Some("local:iso/noble-server-cloudimg-amd64.img".to_string()),
            storage_pool_available: true,
            network_bridge_present: true,
            blockers: vec![],
        }
    }

    #[test]
    fn clear_preflight_produces_planned_actions() {
        let desired = AgentHostDesired::default();
        let preflight = clear_preflight();
        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "planned");
        assert!(plan.mutates);
        assert!(plan.blockers.is_empty());
        assert_eq!(plan.actions.len(), 6);
        assert_eq!(plan.actions[0].argv[0], "qm");
        assert_eq!(plan.actions[0].argv[1], "create");
        assert_eq!(plan.actions[0].argv[2], "102");
    }

    #[test]
    fn insufficient_memory_blocks_plan() {
        let desired = AgentHostDesired::default();
        let mut preflight = clear_preflight();
        preflight.available_memory_bytes = 1024 * 1024 * 1024; // only 1 GiB available, need 4 GiB
        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .iter()
                .any(|b| b.starts_with("insufficient_hypervisor_memory"))
        );
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn missing_template_blocks_plan() {
        let desired = AgentHostDesired::default();
        let mut preflight = clear_preflight();
        preflight.template_present = false;
        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"cloudinit_base_image_missing".to_string())
        );
    }

    #[test]
    fn running_vm_with_unobserved_preflight_is_blocked_without_mutation() {
        let desired = AgentHostDesired::default();
        let mut preflight = clear_preflight();
        preflight.hypervisor_reachable = false;
        preflight.vmid_available = false;
        preflight.vmid_existing_status = Some("running".to_string());
        preflight.blockers = vec!["unobserved_preflight".to_string()];

        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "blocked");
        assert!(!plan.mutates);
        assert!(plan.actions.is_empty());
        assert_eq!(plan.rollback_intent, "none_existing_vm_not_modified");
        assert!(
            plan.blockers
                .contains(&"hypervisor_unreachable".to_string())
        );
        assert!(plan.blockers.contains(&"unobserved_preflight".to_string()));
    }

    #[test]
    fn observed_running_vm_is_converged_without_mutation() {
        let desired = AgentHostDesired::default();
        let mut preflight = clear_preflight();
        preflight.vmid_available = false;
        preflight.vmid_existing_status = Some("running".to_string());

        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "converged");
        assert!(!plan.mutates);
        assert!(plan.actions.is_empty());
        assert!(plan.blockers.is_empty());
    }

    #[test]
    fn vmid_conflict_blocks_plan() {
        let desired = AgentHostDesired::default();
        let mut preflight = clear_preflight();
        preflight.vmid_available = false;
        preflight.vmid_existing_status = Some("stopped".to_string());
        let plan = plan_agent_host_vm(&desired, &preflight);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"vmid_already_in_use:102".to_string())
        );
    }
}
