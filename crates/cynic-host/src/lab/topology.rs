use std::path::Path;

use serde_json::Value;

use super::types::{
    LabExecutionStep, LabGuestBootstrapPreflight, LabImagePreflight, LabIsolationPlan,
    LabProviderPlan, LabVmPlan,
};

pub(crate) fn guest_bootstrap_preflight(registry: &Value) -> LabGuestBootstrapPreflight {
    let bootstrap = registry
        .get("lab")
        .and_then(|lab| lab.get("guest_bootstrap"));
    LabGuestBootstrapPreflight {
        declared: bootstrap.is_some(),
        transport: bootstrap
            .and_then(|bootstrap| bootstrap.get("transport"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ssh_user: bootstrap
            .and_then(|bootstrap| bootstrap.get("ssh_user"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

pub(crate) fn provision_blockers(
    provider: &LabProviderPlan,
    image: &LabImagePreflight,
    guest_bootstrap: &LabGuestBootstrapPreflight,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !provider.qemu_available {
        blockers.push("qemu-system-x86_64 is not available");
    }
    if !provider.virsh_available {
        blockers.push("virsh is not available");
    }
    if !provider.kvm_available {
        blockers.push("/dev/kvm is not available");
    }
    if !image.declared {
        blockers.push("registry lab.base_image is missing");
    }
    if !image.checksum_declared {
        blockers.push("registry lab.base_image.sha256 is missing");
    }
    if image.checksum_declared && !image.checksum_valid_shape {
        blockers.push("registry lab.base_image.sha256 must be a 64-character lowercase hex digest");
    }
    if image.declared && !image.exists_on_host {
        blockers.push("registry lab.base_image.path does not exist on host");
    }
    if !guest_bootstrap.declared {
        blockers.push("registry lab.guest_bootstrap is missing");
    }
    if guest_bootstrap.transport.as_deref() != Some("ssh") {
        blockers.push("registry lab.guest_bootstrap.transport must be ssh");
    }
    blockers
}

pub(crate) fn provider_plan() -> LabProviderPlan {
    let qemu_available = command_available("qemu-system-x86_64");
    let virsh_available = command_available("virsh");
    let kvm_available = Path::new("/dev/kvm").exists();
    LabProviderPlan {
        selected: "qemu-libvirt",
        qemu_available,
        virsh_available,
        kvm_available,
        rationale: "qemu/libvirt can run isolated local VMs without mutating host routing for mesh tests",
    }
}

pub(crate) fn isolation_plan(network_policy: Option<&Value>) -> LabIsolationPlan {
    let planes = network_policy
        .and_then(|value| value.get("planes"))
        .and_then(Value::as_object);
    let local_cidr = planes
        .and_then(|planes| planes.get("proxmox_guest"))
        .and_then(|plane| plane.get("cidr"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let canonical_plane = planes.and_then(|planes| {
        planes.values().find(|plane| {
            plane
                .get("canonical")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
    });
    LabIsolationPlan {
        network_name: "cynic-lab-isolated",
        network_mode: "libvirt-isolated",
        host_routing_mutation: false,
        cidr: local_cidr,
        canonical_mesh_cidr: canonical_plane
            .and_then(|plane| plane.get("cidr"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        dns_zone: canonical_plane
            .and_then(|plane| plane.get("dns_zone"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

pub(crate) fn vm_plan(node: &Value) -> Result<LabVmPlan, String> {
    let id = required_str(node, "id")?;
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();
    let role = required_str(node, "role")?.to_string();
    let status = node
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("candidate")
        .to_string();
    let network = node.get("network");
    let services = node
        .get("services")
        .and_then(Value::as_array)
        .map(|services| {
            services
                .iter()
                .filter_map(|service| service.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(LabVmPlan {
        id: id.to_string(),
        name,
        role,
        status,
        local_ip: network
            .and_then(|value| value.get("local"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        mesh_ip: network
            .and_then(|value| value.get("synapse"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        services,
        image_policy: "fresh_ephemeral_guest",
        provision_state: "planned_not_created",
    })
}

pub(crate) fn execution_suite(registry_path: &str) -> Vec<LabExecutionStep> {
    vec![
        LabExecutionStep {
            order: 1,
            name: "plan",
            mutates: false,
            status: "implemented",
            command: format!("cynic lab plan --json --registry {registry_path}"),
        },
        LabExecutionStep {
            order: 2,
            name: "provision",
            mutates: true,
            status: "blocked",
            command: format!("cynic lab provision --registry {registry_path} --apply"),
        },
        LabExecutionStep {
            order: 3,
            name: "guest_mesh_apply",
            mutates: true,
            status: "blocked",
            command: "cynic network mesh apply-profile --bundle-dir <guest-bundle> --profile mesh-route-install --apply --closed-environment".to_string(),
        },
        LabExecutionStep {
            order: 4,
            name: "runtime_proof",
            mutates: false,
            status: "blocked",
            command: format!("cynic lab e2e --registry {registry_path}"),
        },
    ]
}

pub(crate) fn provision_steps(registry_path: &str) -> Vec<LabExecutionStep> {
    vec![
        LabExecutionStep {
            order: 1,
            name: "create_isolated_network",
            mutates: true,
            status: "blocked",
            command: "virsh net-define <generated-cynic-lab-isolated.xml>".to_string(),
        },
        LabExecutionStep {
            order: 2,
            name: "clone_base_images",
            mutates: true,
            status: "blocked",
            command: "qemu-img create -f qcow2 -b <base-image> <vm-disk>".to_string(),
        },
        LabExecutionStep {
            order: 3,
            name: "define_domains",
            mutates: true,
            status: "blocked",
            command: "virsh define <generated-domain.xml>".to_string(),
        },
        LabExecutionStep {
            order: 4,
            name: "start_domains",
            mutates: true,
            status: "blocked",
            command: "virsh start <lab-vm>".to_string(),
        },
        LabExecutionStep {
            order: 5,
            name: "guest_bootstrap",
            mutates: true,
            status: "blocked",
            command: format!("cynic bootstrap host plan --registry {registry_path} --node <guest>"),
        },
    ]
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("registry node missing string field {key}"))
}

fn command_available(command: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|directory| directory.join(command).is_file()))
        .unwrap_or(false)
}
