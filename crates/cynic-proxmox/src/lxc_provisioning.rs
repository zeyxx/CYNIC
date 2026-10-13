//! Pure Proxmox LXC container provisioning plan.
//!
//! WHY: the provider substrate port already admits a `Provision` action and
//! the L0 predicate already validates concrete container parameters, but no
//! pure planner translates a validated spec into the exact `pct` argv the
//! hypervisor executes. Planning in the adapter would hide policy inside I/O.
//!
//! HOW: this module is zero-I/O. It takes the validated desired state and the
//! observed preflight and emits an ordered argv plan. Every mutation requires
//! an explicit `apply` flag; a blocked plan carries zero actions.
//!
//! NEGATIVE BOUNDARY: never touches the network, never spawns processes,
//! never reads the registry — the adapter owns execution, this module owns
//! the decision.

use cynic_protocol::provider::{LxcProvisioningSpec, validate_lxc_provisioning_spec};

pub const LXC_PROVISIONING_PLAN_SCHEMA: &str = "cynic.proxmox.lxc_provisioning_plan.v1";

/// Hypervisor preflight observation consumed by the pure planner.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcPreflightReport {
    pub schema: String,
    pub hypervisor_reachable: bool,
    pub available_memory_bytes: u64,
    pub available_disk_bytes: u64,
    pub vmid_available: bool,
    pub vmid_existing_status: Option<String>,
    pub template_present: bool,
    pub storage_pool_available: bool,
    pub network_bridge_present: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
}

/// One ordered argv step of the provisioning plan.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcProvisioningAction {
    pub step: u32,
    pub kind: String,
    pub purpose: String,
    pub argv: Vec<String>,
    pub privileged: bool,
}

/// Deterministic provisioning plan: pure projection of desired state and
/// preflight into an ordered argv sequence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcProvisioningPlan {
    pub schema: String,
    pub status: String,
    pub desired_vmid: u32,
    pub desired_hostname: String,
    pub actions: Vec<LxcProvisioningAction>,
    pub rollback_intent: String,
    pub mutates: bool,
    pub blockers: Vec<String>,
}

/// Parses "<kind>/<id>" provider resource refs (e.g. "lxc/106").
pub fn parse_proxmox_resource(resource_id: &str) -> Result<(String, u32), String> {
    let (kind, id) = resource_id
        .split_once('/')
        .ok_or_else(|| "invalid_proxmox_resource_ref".to_string())?;
    if !matches!(kind, "lxc" | "qemu") {
        return Err("invalid_proxmox_resource_ref".to_string());
    }
    let vmid: u32 = id
        .parse()
        .map_err(|_| "invalid_proxmox_resource_ref".to_string())?;
    if vmid == 0 {
        return Err("invalid_proxmox_resource_ref".to_string());
    }
    Ok((kind.to_string(), vmid))
}

/// Pure planner: reconciles a validated LXC spec with the observed preflight.
/// Zero I/O. Emits argv actions only when no blocker remains.
pub fn plan_lxc_provisioning(
    vmid: u32,
    spec: &LxcProvisioningSpec,
    preflight: &LxcPreflightReport,
) -> LxcProvisioningPlan {
    let mut blockers = validate_lxc_provisioning_spec(spec);

    if !preflight.hypervisor_reachable {
        blockers.push("hypervisor_unreachable".to_string());
    }
    let memory_bytes = u64::from(spec.memory_mib) * 1024 * 1024;
    let swap_bytes = u64::from(spec.swap_mib) * 1024 * 1024;
    let disk_bytes = u64::from(spec.disk_gib) * 1024 * 1024 * 1024;
    if preflight.available_memory_bytes < memory_bytes + swap_bytes {
        blockers.push(format!(
            "insufficient_hypervisor_memory: required={}, available={}",
            memory_bytes + swap_bytes,
            preflight.available_memory_bytes
        ));
    }
    if preflight.available_disk_bytes < disk_bytes {
        blockers.push(format!(
            "insufficient_hypervisor_disk: required={}, available={}",
            disk_bytes, preflight.available_disk_bytes
        ));
    }
    if !preflight.storage_pool_available {
        blockers.push(format!("storage_pool_unavailable:{}", spec.storage_pool));
    }
    if !preflight.network_bridge_present {
        blockers.push(format!("network_bridge_missing:{}", spec.bridge));
    }
    if !preflight.template_present {
        blockers.push(format!("template_missing:{}", spec.template_volid));
    }
    let existing_running =
        !preflight.vmid_available && preflight.vmid_existing_status.as_deref() == Some("running");
    if !preflight.vmid_available && !existing_running {
        blockers.push(format!("vmid_already_in_use:{vmid}"));
    }

    blockers.extend(preflight.blockers.iter().cloned());
    blockers.sort();
    blockers.dedup();

    if existing_running && blockers.is_empty() {
        return LxcProvisioningPlan {
            schema: LXC_PROVISIONING_PLAN_SCHEMA.to_string(),
            status: "converged".to_string(),
            desired_vmid: vmid,
            desired_hostname: spec.hostname.clone(),
            actions: vec![],
            rollback_intent: "none_existing_container_not_modified".to_string(),
            mutates: false,
            blockers,
        };
    }

    let actions = if blockers.is_empty() {
        vec![
            LxcProvisioningAction {
                step: 1,
                kind: "create_lxc_container".to_string(),
                purpose: format!(
                    "create container {vmid} ({}) from template with {} cores, {} MiB RAM, {} GiB disk",
                    spec.hostname, spec.cores, spec.memory_mib, spec.disk_gib
                ),
                argv: build_pct_create_argv(vmid, spec),
                privileged: true,
            },
            LxcProvisioningAction {
                step: 2,
                kind: "start_lxc_container".to_string(),
                purpose: format!("start container {vmid}"),
                argv: vec!["pct".to_string(), "start".to_string(), vmid.to_string()],
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
    let mutates = !actions.is_empty();

    LxcProvisioningPlan {
        schema: LXC_PROVISIONING_PLAN_SCHEMA.to_string(),
        status,
        desired_vmid: vmid,
        desired_hostname: spec.hostname.clone(),
        actions,
        rollback_intent: format!("pct stop {vmid} ; pct destroy {vmid}"),
        mutates,
        blockers,
    }
}

/// Builds the exact `pct create` argv for a validated spec. Pure string
/// assembly: network, rootfs, and feature flags in canonical `pct` order.
fn build_pct_create_argv(vmid: u32, spec: &LxcProvisioningSpec) -> Vec<String> {
    let mut net0 = format!("name=eth0,bridge={}", spec.bridge);
    if let Some(ip_cidr) = spec.ip_cidr.as_deref() {
        net0.push_str(&format!(",ip={ip_cidr}"));
        if let Some(gateway) = spec.gateway.as_deref() {
            net0.push_str(&format!(",gw={gateway}"));
        }
    } else {
        net0.push_str(",ip=dhcp");
    }

    let mut argv = vec![
        "pct".to_string(),
        "create".to_string(),
        vmid.to_string(),
        spec.template_volid.clone(),
        "--hostname".to_string(),
        spec.hostname.clone(),
        "--cores".to_string(),
        spec.cores.to_string(),
        "--memory".to_string(),
        spec.memory_mib.to_string(),
        "--swap".to_string(),
        spec.swap_mib.to_string(),
        "--rootfs".to_string(),
        format!("{}:{}", spec.storage_pool, spec.disk_gib),
        "--net0".to_string(),
        net0,
        "--onboot".to_string(),
        if spec.onboot { "1" } else { "0" }.to_string(),
        "--unprivileged".to_string(),
        if spec.unprivileged { "1" } else { "0" }.to_string(),
    ];
    if let Some(nameserver) = spec.nameserver.as_deref() {
        argv.push("--nameserver".to_string());
        argv.push(nameserver.to_string());
    }
    for feature in &spec.features {
        argv.push("--features".to_string());
        argv.push(feature.clone());
    }
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> LxcProvisioningSpec {
        LxcProvisioningSpec {
            schema: cynic_protocol::provider::LXC_PROVISIONING_SCHEMA.to_string(),
            hostname: "cynic-lxc-embedding".to_string(),
            template_volid: "local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst".to_string(),
            storage_pool: "local-lvm".to_string(),
            disk_gib: 16,
            cores: 2,
            memory_mib: 2048,
            swap_mib: 1024,
            bridge: "vmbr0".to_string(),
            ip_cidr: Some("10.10.10.106/24".to_string()),
            gateway: Some("10.10.10.1".to_string()),
            nameserver: Some("10.10.10.1".to_string()),
            unprivileged: true,
            onboot: true,
            features: vec!["nesting=1".to_string()],
        }
    }

    fn preflight() -> LxcPreflightReport {
        LxcPreflightReport {
            schema: "cynic.proxmox.lxc_preflight.v1".to_string(),
            hypervisor_reachable: true,
            available_memory_bytes: 13 * 1024 * 1024 * 1024,
            available_disk_bytes: 750 * 1024 * 1024 * 1024,
            vmid_available: true,
            vmid_existing_status: None,
            template_present: true,
            storage_pool_available: true,
            network_bridge_present: true,
            blockers: vec![],
        }
    }

    #[test]
    fn clear_preflight_produces_planned_pct_create_argv() {
        let plan = plan_lxc_provisioning(106, &spec(), &preflight());

        assert_eq!(plan.status, "planned");
        assert!(plan.mutates);
        assert!(plan.blockers.is_empty());
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.actions[0].argv[0], "pct");
        assert_eq!(plan.actions[0].argv[1], "create");
        assert_eq!(plan.actions[0].argv[2], "106");
        assert!(
            plan.actions[0]
                .argv
                .contains(&"cynic-lxc-embedding".to_string())
        );
        assert!(
            plan.actions[0]
                .argv
                .contains(&"name=eth0,bridge=vmbr0,ip=10.10.10.106/24,gw=10.10.10.1".to_string())
        );
        assert_eq!(plan.actions[1].argv[1], "start");
    }

    #[test]
    fn insufficient_memory_blocks_plan() {
        let mut pf = preflight();
        pf.available_memory_bytes = 1024 * 1024 * 1024;
        let plan = plan_lxc_provisioning(106, &spec(), &pf);

        assert_eq!(plan.status, "blocked");
        assert!(!plan.mutates);
        assert!(plan.actions.is_empty());
        assert!(
            plan.blockers
                .iter()
                .any(|blocker| blocker.starts_with("insufficient_hypervisor_memory"))
        );
    }

    #[test]
    fn vmid_conflict_blocks_plan() {
        let mut pf = preflight();
        pf.vmid_available = false;
        pf.vmid_existing_status = Some("stopped".to_string());
        let plan = plan_lxc_provisioning(106, &spec(), &pf);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"vmid_already_in_use:106".to_string())
        );
    }

    #[test]
    fn observed_running_container_is_converged_without_mutation() {
        let mut pf = preflight();
        pf.vmid_available = false;
        pf.vmid_existing_status = Some("running".to_string());
        let plan = plan_lxc_provisioning(106, &spec(), &pf);

        assert_eq!(plan.status, "converged");
        assert!(!plan.mutates);
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn missing_template_blocks_plan() {
        let mut pf = preflight();
        pf.template_present = false;
        let plan = plan_lxc_provisioning(106, &spec(), &pf);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .iter()
                .any(|blocker| blocker.starts_with("template_missing"))
        );
    }

    #[test]
    fn parses_lxc_resource_refs() {
        assert_eq!(
            parse_proxmox_resource("lxc/106"),
            Ok(("lxc".to_string(), 106))
        );
        assert!(parse_proxmox_resource("106").is_err());
        assert!(parse_proxmox_resource("vm/106").is_err());
        assert!(parse_proxmox_resource("lxc/0").is_err());
    }
}
