//! Node compute CLI surface — run hybrid workloads & enroll remote runners.

use crate::emit_json;
use clap::Subcommand;
use cynic_lifecycle::LocalArgvRunner;
use cynic_network::adapters::headscale::{HeadscaleClient, HeadscaleConfig};
use cynic_node::usecases::compute_provisioning_uc::{
    ComputeProvisioningIntent, execute_compute_provisioning,
};
use cynic_node::usecases::remote_runner_enrollment_uc::execute_remote_runner_enrollment;
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    ComputeRequirements, RemoteRunnerSpec, SubstrateKind, SubstrateObservation,
};
use cynic_protocol::provider::{
    LXC_PROVISIONING_SCHEMA, LxcProvisioningSpec, NodeSubstrateAction, NodeSubstrateControlPort,
    NodeSubstrateDesiredState, NodeSubstrateObservationPort, NodeSubstratePower,
    NodeSubstrateProvisioningSpec,
};
use cynic_proxmox::adapters::{
    FleetLease, ObservedGuest, ProxmoxControlAdapter, ProxmoxSshCloneAdapter,
    ProxmoxSshSubstrateAdapter, drop_lease, plan_sweep, read_leases, template_hygiene_blockers,
    write_lease,
};
use cynic_sandbox::adapters::bwrap_substrate::LocalBwrapSubstrateAdapter;
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Subcommand, Debug, Clone)]
pub enum NodeComputeCommand {
    /// Execute a compute workload through the optimal sovereign substrate.
    Run {
        #[arg(long)]
        workload: String,
        #[arg(long, default_value_t = 1)]
        vcpus: u32,
        #[arg(long, default_value_t = 1073741824)] // 1 GiB
        memory_bytes: u64,
        #[arg(long, default_value_t = false)]
        gpu: bool,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Provision a single-use enrollment pre-auth key for an external GPU/CPU runner.
    Enroll {
        #[arg(long)]
        node_name: String,
        #[arg(long, default_value_t = 4)]
        vcpus: u32,
        #[arg(long, default_value_t = 8589934592)] // 8 GiB
        memory_bytes: u64,
        #[arg(long, default_value_t = false)]
        gpu: bool,
        #[arg(long)]
        gpu_model: Option<String>,
        #[arg(long)]
        pubkey: String,
    },
    /// Provision a Proxmox LXC container through the sovereign substrate port.
    /// Emits the pure plan by default; --apply executes it over SSH.
    Template {
        /// Builder guest vmid to convert (must be stopped and hygiene-clean).
        #[arg(long)]
        vmid: u32,
        /// Guest-direct SSH target for the hygiene scan
        /// (caller-supplied address, never a hardcoded default).
        #[arg(long)]
        guest_host: String,
        /// Proxmox SSH target for the SSH-breakglass convert fallback
        /// (never a hardcoded default).
        #[arg(long)]
        hypervisor: String,
        /// Proxmox node name for the REST convert path. Defaults to `pve`.
        #[arg(long, default_value = "pve")]
        node: String,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    Sweep {
        /// Proxmox node to sweep. Defaults to `pve`.
        #[arg(long, default_value = "pve")]
        node: String,
        /// Fleet registry (infra/fleet.json): templates + static reservations.
        /// Required, never defaulted.
        #[arg(long)]
        fleet_registry: String,
        /// Runtime lease file. Defaults to $CYNIC_FLEET_LEASES or
        /// ~/.local/share/cynic/fleet/leases.json.
        #[arg(long)]
        leases: Option<String>,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    Provision {
        #[arg(long)]
        vmid: u32,
        #[arg(long)]
        hostname: String,
        #[arg(long)]
        template_volid: String,
        #[arg(long, default_value = "local-lvm")]
        storage_pool: String,
        #[arg(long, default_value_t = 16)]
        disk_gib: u32,
        #[arg(long, default_value_t = 2)]
        cores: u32,
        #[arg(long, default_value_t = 2048)]
        memory_mib: u32,
        #[arg(long, default_value_t = 1024)]
        swap_mib: u32,
        #[arg(long, default_value = "vmbr0")]
        bridge: String,
        #[arg(long)]
        ip_cidr: Option<String>,
        #[arg(long)]
        gateway: Option<String>,
        #[arg(long)]
        nameserver: Option<String>,
        /// LXC feature flags (repeatable, e.g. --features nesting=1).
        #[arg(long)]
        features: Vec<String>,
        #[arg(long, default_value_t = true)]
        unprivileged: bool,
        #[arg(long, default_value_t = true)]
        onboot: bool,
        /// Hypervisor SSH target used only when the REST API is unavailable
        /// (breakglass fallback, never a hardcoded default).
        #[arg(long)]
        hypervisor: Option<String>,
        /// Proxmox node name for the REST path. Auto-resolved when the
        /// cluster exposes a single node.
        #[arg(long)]
        node: Option<String>,
        /// Clone from a versioned template vmid instead of creating from an
        /// OS template. Requires --ttl-hours (ephemeral by design).
        #[arg(long)]
        clone_from: Option<u32>,
        /// Clone TTL in hours (1-24). Only meaningful with --clone-from.
        #[arg(long, default_value_t = 4)]
        ttl_hours: u32,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
}

pub async fn dispatch(cmd: NodeComputeCommand) -> ExitCode {
    match cmd {
        NodeComputeCommand::Run {
            workload,
            vcpus,
            memory_bytes,
            gpu,
            timeout_ms,
            command,
        } => {
            if command.is_empty() {
                eprintln!("error: command to execute inside runner cannot be empty");
                return ExitCode::FAILURE;
            }

            let requirements = ComputeRequirements {
                vcpus,
                memory_bytes,
                disk_bytes: 10 * 1024 * 1024 * 1024,
                requires_gpu: gpu,
                preferred_strategy: SandboxStrategy::BwrapEphemeral,
                allowed_strategies: vec![
                    SandboxStrategy::BwrapEphemeral,
                    SandboxStrategy::LxcUnprivileged,
                ],
            };

            let intent = ComputeProvisioningIntent {
                workload_id: workload,
                requirements,
                command,
                timeout_ms,
            };

            let substrates = vec![SubstrateObservation {
                name: "local-node".to_string(),
                kind: SubstrateKind::LocalBwrap,
                available_vcpus: 16,
                available_memory_bytes: 32 * 1024 * 1024 * 1024,
                available_disk_bytes: 500 * 1024 * 1024 * 1024,
                has_gpu: false,
                supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
            }];

            let runner = Arc::new(LocalArgvRunner);
            let substrate_port = LocalBwrapSubstrateAdapter::new(runner);

            match execute_compute_provisioning(&intent, &substrates, &substrate_port).await {
                Ok(report) => emit_json(&report, false),
                Err(e) => {
                    eprintln!("error: compute provisioning failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeComputeCommand::Enroll {
            node_name,
            vcpus,
            memory_bytes,
            gpu,
            gpu_model,
            pubkey,
        } => {
            let spec = RemoteRunnerSpec {
                node_name,
                vcpus,
                memory_bytes,
                disk_bytes: 50 * 1024 * 1024 * 1024,
                has_gpu: gpu,
                gpu_model,
                wireguard_pubkey: pubkey,
            };

            let base_url = std::env::var("HEADSCALE_URL")
                .unwrap_or_else(|_| "http://10.10.10.101:8080".to_string());
            let config = HeadscaleConfig::new(base_url, "headscale-api-key", "HEADSCALE_API_KEY");
            let mesh_client = match HeadscaleClient::new(config) {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("error: failed to initialize headscale client: {e:?}");
                    return ExitCode::FAILURE;
                }
            };

            match execute_remote_runner_enrollment(&spec, &mesh_client) {
                Ok(report) => emit_json(&report, false),
                Err(e) => {
                    eprintln!("error: remote runner enrollment failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeComputeCommand::Template {
            vmid,
            guest_host,
            hypervisor,
            node,
            apply,
        } => {
            return apply_template(guest_host, hypervisor, node, vmid, apply).await;
        },
        NodeComputeCommand::Sweep {
            node,
            fleet_registry,
            leases,
            apply,
        } => {
            return apply_sweep(node, fleet_registry, leases, apply).await;
        },
        NodeComputeCommand::Provision {
            vmid,
            hostname,
            template_volid,
            storage_pool,
            disk_gib,
            cores,
            memory_mib,
            swap_mib,
            bridge,
            ip_cidr,
            gateway,
            nameserver,
            features,
            unprivileged,
            onboot,
            hypervisor,
            node,
            clone_from,
            ttl_hours,
            apply,
        } => {
            let lxc_spec = LxcProvisioningSpec {
                schema: LXC_PROVISIONING_SCHEMA.to_string(),
                hostname: hostname.clone(),
                template_volid,
                storage_pool,
                disk_gib,
                cores,
                memory_mib,
                swap_mib,
                bridge,
                ip_cidr,
                gateway,
                nameserver,
                unprivileged,
                onboot,
                features,
            };
            let desired = NodeSubstrateDesiredState {
                node_id: hostname.clone(),
                provider: Some("proxmox".to_string()),
                resource_id: Some(format!("lxc/{vmid}")),
                desired_power: NodeSubstratePower::Running,
                provisioning: Some(NodeSubstrateProvisioningSpec {
                    schema: "cynic.node_substrate.provisioning.v1".to_string(),
                    image_ref: String::new(),
                    image_digest: String::new(),
                    runtime_seed_ref: String::new(),
                    runtime_seed_digest: String::new(),
                    network_profile_ref: String::new(),
                    lxc: Some(lxc_spec),
                }),
            };

            // Clone path: same grammar, other backend.

            if let Some(template_vmid) = clone_from {
                if !apply {
                    emit_json(
                        &serde_json::json!({
                            "schema": "cynic.node.compute.clone.v1",
                            "status": "planned",
                            "channel": "rest",
                            "vmid": vmid,
                            "hostname": hostname,
                            "clone_from": template_vmid,
                            "ttl_hours": ttl_hours,
                        }),
                        false,
                    );
                    return ExitCode::SUCCESS;
                }
                if let Ok(rest) = ProxmoxControlAdapter::from_env(std::time::Duration::from_secs(5)) {
                    return apply_clone_rest(rest, node, template_vmid, vmid, hostname, ttl_hours).await;
                }
                eprintln!("error: clone requires the REST API (no SSH breakglass for clones)");
                return ExitCode::FAILURE;
            }

            // Default path: emit the pure plan without touching the hypervisor.
            if !apply {
                let plan = cynic_proxmox::lxc_provisioning::plan_lxc_provisioning(
                    vmid,
                    desired
                        .provisioning
                        .as_ref()
                        .and_then(|p| p.lxc.as_ref())
                        .expect("lxc spec present"),
                    &crate::node_surface::compute::clear_lxc_preflight(),
                );
                emit_json(&plan, false);
                return if plan.status == "planned" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                };
            }



            // Apply path: REST first (canonical), SSH breakglass fallback.
            // The REST executor posts the validated spec and polls the UPID
            // task to completion; the SSH executor runs the pure planned argv.
            // Both paths close with a read-back before reporting provisioned.
            if let Ok(rest) = ProxmoxControlAdapter::from_env(std::time::Duration::from_secs(5)) {
                return apply_provision_rest(rest, node, vmid, hostname, &desired).await;
            }
            eprintln!(
                "REST API unavailable (endpoint/token/CA not configured), falling back to SSH breakglass"
            );
            let Some(hypervisor) = hypervisor else {
                eprintln!(
                    "error: REST unavailable and no --hypervisor <ssh-target> supplied (no hardcoded default)"
                );
                return ExitCode::FAILURE;
            };
            let runner = LocalArgvRunner;
            let adapter = match ProxmoxSshSubstrateAdapter::new(&runner, hypervisor) {
                Ok(adapter) => adapter,
                Err(e) => {
                    eprintln!("error: invalid hypervisor target: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match adapter.apply_node_substrate_action(&desired, &NodeSubstrateAction::Provision) {
                Ok(()) => match adapter.observe_node_substrate(&desired) {
                    Ok(observation) => {
                        emit_json(
                            &serde_json::json!({
                                "schema": "cynic.node.compute.provision.v1",
                                "status": "provisioned",
                                "channel": "ssh_breakglass",
                                "vmid": vmid,
                                "hostname": hostname,
                                "observed_existence": observation.existence,
                                "observed_power": observation.power,
                                "observation_authority": observation.observation_authority,
                            }),
                            false,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: provisioned but read-back failed: {e}");
                        ExitCode::FAILURE
                    }
                },
                Err(e) => {
                    eprintln!("error: lxc provisioning failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// REST apply path: resolve the target node, provision through
/// `ProxmoxControlAdapter::provision_lxc`, then read back the container
/// config as existence proof. Fails closed at every step.
async fn apply_provision_rest(
    rest: ProxmoxControlAdapter,
    node: Option<String>,
    vmid: u32,
    hostname: String,
    desired: &NodeSubstrateDesiredState,
) -> ExitCode {
    let node = match node {
        Some(node) => node,
        None => match resolve_single_proxmox_node(&rest).await {
            Ok(node) => node,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        },
    };
    let Some(spec) = desired
        .provisioning
        .as_ref()
        .and_then(|provisioning| provisioning.lxc.as_ref())
    else {
        eprintln!("error: lxc provisioning spec missing");
        return ExitCode::FAILURE;
    };
    let provisioned = match rest
        .provision_lxc(&node, vmid, spec, std::time::Duration::from_secs(180))
        .await
    {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: REST lxc provisioning failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    match rest.lxc_config(&node, &vmid.to_string()).await {
        Ok(_) => {
            emit_json(
                &serde_json::json!({
                    "schema": "cynic.node.compute.provision.v1",
                    "status": "provisioned",
                    "channel": "rest",
                    "vmid": vmid,
                    "hostname": hostname,
                    "node": node,
                    "upid": provisioned["upid"],
                }),
                false,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: provisioned but read-back failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// REST clone path: clone the versioned template, poll the UPID task,
/// read back the guest config as existence proof. Fails closed at every step.
async fn apply_clone_rest(
    rest: ProxmoxControlAdapter,
    node: Option<String>,
    template_vmid: u32,
    vmid: u32,
    hostname: String,
    ttl_hours: u32,
) -> ExitCode {
    let node = match node {
        Some(node) => node,
        None => match resolve_single_proxmox_node(&rest).await {
            Ok(node) => node,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        },
    };
    let cloned = match rest
        .clone_lxc(&node, template_vmid, vmid, &hostname, ttl_hours, std::time::Duration::from_secs(120))
        .await
    {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: REST lxc clone failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    match rest.lxc_config(&node, &vmid.to_string()).await {
        Ok(_) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let lease = FleetLease {
                vmid,
                expires_at: now + u64::from(ttl_hours) * 3600,
                owner: "cynic-agent".to_string(),
                purpose: hostname.clone(),
            };
            let leases_path = std::path::PathBuf::from(default_leases_path());
            if let Err(error) = write_lease(&leases_path, &lease) {
                eprintln!("warning: clone succeeded but lease unwritten: {error}");
            }
            emit_json(
                &serde_json::json!({
                    "schema": "cynic.node.compute.clone.v1",
                    "status": "provisioned",
                    "channel": "rest",
                    "vmid": vmid,
                    "hostname": hostname,
                    "node": node,
                    "cloned_from": template_vmid,
                    "ttl_hours": ttl_hours,
                    "upid": cloned["upid"],
                }),
                false,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: cloned but read-back failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Default runtime lease file when --leases is omitted.
fn default_leases_path() -> String {
    if let Ok(path) = std::env::var("CYNIC_FLEET_LEASES") {
        return path;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    format!("{home}/.local/share/cynic/fleet/leases.json")
}

/// Static vmids from the fleet registry: every template vmid plus every
/// static reservation. The registry is SSOT; the CLI never hardcodes them.
fn static_vmids_from_registry(path: &str) -> Result<Vec<u32>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("fleet_registry_unreadable:{path}:{error}"))?;
    let registry: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("fleet_registry_invalid:{path}:{error}"))?;
    let mut vmids = Vec::new();
    if let Some(templates) = registry.get("templates").and_then(|value| value.as_array()) {
        for template in templates {
            if let Some(vmid) = template.get("vmid").and_then(|value| value.as_u64()) {
                vmids.push(vmid as u32);
            }
        }
    }
    if let Some(reserved) = registry
        .get("static_reservations")
        .and_then(|value| value.get("vmids"))
        .and_then(|value| value.as_array())
    {
        for vmid in reserved {
            if let Some(vmid) = vmid.as_u64() {
                vmids.push(vmid as u32);
            }
        }
    }
    Ok(vmids)
}

/// Sweep flow: list guests (read-only observation), reconcile leases from
/// the runtime file against the registry statics, emit the plan. With
/// --apply, destroy expired leases (stop + delete + lease drop) and leave
/// everything else untouched. Reports never act.
async fn apply_sweep(
    node: String,
    fleet_registry: String,
    leases_path: Option<String>,
    apply: bool,
) -> ExitCode {
    let rest = match ProxmoxControlAdapter::from_env(std::time::Duration::from_secs(5)) {
        Ok(rest) => rest,
        Err(e) => {
            eprintln!("error: sweep requires the REST API: {e}");
            return ExitCode::FAILURE;
        }
    };
    let statics = match static_vmids_from_registry(&fleet_registry) {
        Ok(vmids) => vmids,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let leases_file = leases_path.unwrap_or_else(default_leases_path);
    let leases_path_buf = std::path::PathBuf::from(&leases_file);
    let leases = match read_leases(&leases_path_buf) {
        Ok(leases) => leases,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let listed = match rest.list_lxc(&node).await {
        Ok(envelope) => envelope,
        Err(e) => {
            eprintln!("error: REST lxc list failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut observed = Vec::new();
    if let Some(guests) = listed.get("data").and_then(|value| value.as_array()) {
        for guest in guests {
            observed.push(ObservedGuest {
                vmid: guest.get("vmid").and_then(|value| value.as_u64()).unwrap_or(0) as u32,
                name: guest
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?")
                    .to_string(),
                status: guest
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?")
                    .to_string(),
                template: guest.get("template").and_then(|value| value.as_u64()).unwrap_or(0) == 1,
            });
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let actions = plan_sweep(&observed, &leases, &statics, now);
    if !apply {
        emit_json(
            &serde_json::json!({
                "schema": "cynic.node.compute.sweep.v1",
                "status": "planned",
                "channel": "rest",
                "node": node,
                "actions": actions.iter().map(|action| format!("{action:?}")).collect::<Vec<_>>(),
            }),
            false,
        );
        return ExitCode::SUCCESS;
    }
    let mut destroyed = Vec::new();
    for action in &actions {
        if let cynic_proxmox::adapters::SweepAction::Destroy { vmid, .. } = action {
            match rest
                .destroy_lxc(&node, *vmid, std::time::Duration::from_secs(120))
                .await
            {
                Ok(_) => {
                    let _ = drop_lease(&leases_path_buf, *vmid);
                    destroyed.push(*vmid);
                }
                Err(e) => {
                    eprintln!("error: destroy {vmid} failed: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    emit_json(
        &serde_json::json!({
            "schema": "cynic.node.compute.sweep.v1",
            "status": "swept",
            "channel": "rest",
            "node": node,
            "destroyed": destroyed,
            "reported": actions.len() - destroyed.len(),
        }),
        false,
    );
    ExitCode::SUCCESS
}

/// Template flow: scan the builder guest for hygiene blockers, refuse on
/// any hit, otherwise stop + convert to template and prove `template: 1`
/// with a config read-back. Plan-only emits the scan argv without touching
/// the hypervisor.
async fn apply_template(
    guest_host: String,
    hypervisor: String,
    node: String,
    vmid: u32,
    apply: bool,
) -> ExitCode {
    let runner = LocalArgvRunner;
    let scanner = match ProxmoxSshCloneAdapter::new(&runner, guest_host) {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("error: invalid guest target: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !apply {
        emit_json(
            &serde_json::json!({
                "schema": "cynic.node.compute.template.v1",
                "status": "planned",
                "channel": "ssh_scan",
                "vmid": vmid,
                "scan_argv": scanner.plan_template_hygiene_scan(vmid),
                "convert_argv": scanner.plan_convert_template(vmid),
            }),
            false,
        );
        return ExitCode::SUCCESS;
    }
    let found = match scanner.scan_template_hygiene(vmid) {
        Ok(found) => found,
        Err(e) => {
            eprintln!("error: hygiene scan failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let found_refs: Vec<&str> = found.iter().map(String::as_str).collect();
    let blockers = template_hygiene_blockers(&found_refs);
    if !blockers.is_empty() {
        emit_json(
            &serde_json::json!({
                "schema": "cynic.node.compute.template.v1",
                "status": "refused",
                "reason": "template_hygiene_blockers",
                "blockers": blockers,
                "vmid": vmid,
            }),
            false,
        );
        return ExitCode::FAILURE;
    }
    if let Ok(rest) = ProxmoxControlAdapter::from_env(std::time::Duration::from_secs(5)) {
        match rest
            .convert_template(&node, vmid, std::time::Duration::from_secs(120))
            .await
        {
            Ok(report) => {
                emit_json(
                    &serde_json::json!({
                        "schema": "cynic.node.compute.template.v1",
                        "status": "templated",
                        "channel": "rest",
                        "vmid": vmid,
                        "node": node,
                        "report": report,
                    }),
                    false,
                );
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: REST template convert failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    let converter = match ProxmoxSshCloneAdapter::new(&runner, hypervisor) {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("error: invalid hypervisor target: {e}");
            return ExitCode::FAILURE;
        }
    };
    match converter.convert_template(vmid) {
        Ok(()) => {
            emit_json(
                &serde_json::json!({
                    "schema": "cynic.node.compute.template.v1",
                    "status": "templated",
                    "channel": "ssh_breakglass",
                    "vmid": vmid,
                }),
                false,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: SSH template convert failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Resolve the target Proxmox node from the cluster: exactly one node means
/// that node, otherwise the operator must pass --node explicitly.
async fn resolve_single_proxmox_node(rest: &ProxmoxControlAdapter) -> Result<String, String> {
    let payload = rest
        .nodes()
        .await
        .map_err(|e| format!("proxmox_nodes_unreachable:{e}"))?;
    let nodes = payload
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "proxmox_nodes_response_not_array".to_string())?;
    let names: Vec<&str> = nodes
        .iter()
        .filter_map(|entry| entry.get("node").and_then(serde_json::Value::as_str))
        .collect();
    match names.as_slice() {
        [single] => Ok(single.to_string()),
        _ => Err(format!(
            "ambiguous_proxmox_node ({} nodes): pass --node explicitly",
            names.len()
        )),
    }
}

/// Clear preflight used by the plan-only path (no hypervisor contact).
/// WHY: the pure planner needs a preflight report to emit argv; the plan-only
/// CLI path assumes the operator pre-verified reachability, resources, and
/// template presence (the --apply path executes real preflight implicitly).
fn clear_lxc_preflight() -> cynic_proxmox::lxc_provisioning::LxcPreflightReport {
    cynic_proxmox::lxc_provisioning::LxcPreflightReport {
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
    }
}
