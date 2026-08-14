use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::context::lab_context;
use super::image::image_preflight;
use super::registry::read_registry;
use super::types::{
    LabBundleReport, LabGuestBootstrapPreflight, LabImagePreflight, LabIsolationPlan, LabVmPlan,
};
use super::{guest_bootstrap_preflight, isolation_plan, vm_plan};

pub fn write_lab_bundle(registry_path: &str, output_dir: &str) -> Result<LabBundleReport, String> {
    let registry = read_registry(registry_path)?;
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes must be an array".to_string())?;
    let isolation = isolation_plan(registry.get("network_policy"));
    let image = image_preflight(&registry);
    let bootstrap = guest_bootstrap_preflight(&registry);
    let vms = nodes.iter().map(vm_plan).collect::<Result<Vec<_>, _>>()?;
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir.join("domains"))
        .map_err(|error| format!("failed to create domains dir: {error}"))?;
    fs::create_dir_all(output_dir.join("cloud-init"))
        .map_err(|error| format!("failed to create cloud-init dir: {error}"))?;

    let mut files = Vec::new();
    write_file(
        output_dir.join("network.xml"),
        libvirt_network_xml(&isolation),
        output_dir,
        &mut files,
    )?;
    for vm in &vms {
        write_file(
            output_dir.join("domains").join(format!("{}.xml", vm.name)),
            libvirt_domain_xml(vm, &image, &isolation),
            output_dir,
            &mut files,
        )?;
        write_file(
            output_dir
                .join("cloud-init")
                .join(format!("{}-user-data.yaml", vm.name)),
            cloud_init_user_data(vm, &bootstrap),
            output_dir,
            &mut files,
        )?;
        write_file(
            output_dir
                .join("cloud-init")
                .join(format!("{}-meta-data.yaml", vm.name)),
            cloud_init_meta_data(vm),
            output_dir,
            &mut files,
        )?;
        write_file(
            output_dir
                .join("cloud-init")
                .join(format!("{}-network-config.yaml", vm.name)),
            cloud_init_network_config(vm),
            output_dir,
            &mut files,
        )?;
    }

    let manifest = lab_bundle_manifest(registry_path, &isolation, &image, &bootstrap, &vms, &files);
    write_file(
        output_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("failed to serialize manifest: {error}"))?,
        output_dir,
        &mut files,
    )?;

    Ok(LabBundleReport {
        schema: "cynic.lab.bundle.v1",
        context: lab_context(),
        status: "written",
        mutates: false,
        registry_path: registry_path.to_string(),
        output_dir: output_dir.display().to_string(),
        files,
        network_name: "cynic-lab-isolated",
        vm_count: vms.len(),
        safety_rules: vec![
            "bundle generation writes files only under output_dir",
            "bundle must be reviewed before virsh or qemu-img execution",
            "no host routes, firewall rules, or DNS config are changed by bundle generation",
        ],
    })
}

fn write_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(output_dir)
        .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
    files.push(relative.display().to_string());
    Ok(())
}

fn lab_bundle_manifest(
    registry_path: &str,
    isolation: &LabIsolationPlan,
    image: &LabImagePreflight,
    bootstrap: &LabGuestBootstrapPreflight,
    vms: &[LabVmPlan],
    files: &[String],
) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        (
            "schema",
            Value::String("cynic.lab.bundle_manifest.v1".to_string()),
        ),
        ("mutates", Value::Bool(false)),
        ("registry_path", Value::String(registry_path.to_string())),
        (
            "network_name",
            Value::String(isolation.network_name.to_string()),
        ),
        (
            "network_mode",
            Value::String(isolation.network_mode.to_string()),
        ),
        (
            "base_image",
            serde_json::json!({
                "path": image.path.clone(),
                "source_url": image.source_url.clone(),
                "sha256_url": image.sha256_url.clone(),
                "sha256": image.sha256.clone(),
                "sha256_declared": image.checksum_declared,
                "sha256_valid_shape": image.checksum_valid_shape,
                "exists_on_host": image.exists_on_host
            }),
        ),
        (
            "guest_bootstrap",
            serde_json::json!({
                "transport": bootstrap.transport.clone(),
                "ssh_user": bootstrap.ssh_user.clone(),
                "declared": bootstrap.declared
            }),
        ),
        ("vm_count", serde_json::json!(vms.len())),
        ("vms", serde_json::json!(vms)),
        ("files", serde_json::json!(files)),
    ])
}

fn libvirt_network_xml(isolation: &LabIsolationPlan) -> String {
    format!(
        r#"<network>
  <name>{}</name>
  <bridge name='virbr-cynic-lab' stp='on' delay='0'/>
  <domain name='labmesh.local'/>
  <!-- isolated network: no forward element, no host default route mutation -->
  <ip address='172.30.41.1' netmask='255.255.255.0'/>
</network>
"#,
        isolation.network_name
    )
}

fn libvirt_domain_xml(
    vm: &LabVmPlan,
    image: &LabImagePreflight,
    isolation: &LabIsolationPlan,
) -> String {
    let disk_path = format!("/var/lib/cynic/lab/disks/{}.qcow2", vm.name);
    let seed_path = format!("/var/lib/cynic/lab/seeds/{}-seed.iso", vm.name);
    let base = image.path.as_deref().unwrap_or("<declare-base-image>");
    format!(
        r#"<domain type='kvm'>
  <name>{}</name>
  <memory unit='MiB'>1024</memory>
  <vcpu placement='static'>1</vcpu>
  <metadata>
    <cynic:lab xmlns:cynic='https://cynic.local/lab'>
      <role>{}</role>
      <baseImage>{}</baseImage>
      <meshIp>{}</meshIp>
    </cynic:lab>
  </metadata>
  <os>
    <type arch='x86_64' machine='pc'>hvm</type>
    <boot dev='hd'/>
  </os>
  <devices>
    <disk type='file' device='disk'>
      <driver name='qemu' type='qcow2'/>
      <source file='{}'/>
      <target dev='vda' bus='virtio'/>
    </disk>
    <disk type='file' device='cdrom'>
      <driver name='qemu' type='raw'/>
      <source file='{}'/>
      <target dev='sda' bus='sata'/>
      <readonly/>
    </disk>
    <interface type='network'>
      <source network='{}'/>
      <model type='virtio'/>
    </interface>
    <console type='pty'/>
  </devices>
</domain>
"#,
        vm.name,
        vm.role,
        base,
        vm.mesh_ip.as_deref().unwrap_or("<mesh-ip>"),
        disk_path,
        seed_path,
        isolation.network_name
    )
}

fn cloud_init_user_data(vm: &LabVmPlan, bootstrap: &LabGuestBootstrapPreflight) -> String {
    let user = bootstrap.ssh_user.as_deref().unwrap_or("cynic");
    format!(
        r#"#cloud-config
hostname: {}
manage_etc_hosts: true
users:
  - name: {}
    groups: [sudo]
    shell: /bin/bash
    sudo: ['ALL=(ALL) NOPASSWD:ALL']
    ssh_authorized_keys: []
write_files:
  - path: /etc/cynic/lab-node.json
    permissions: '0644'
    content: |
      {{
        "id": "{}",
        "role": "{}",
        "mesh_ip": "{}"
      }}
runcmd:
  - ['mkdir', '-p', '/etc/cynic', '/var/lib/cynic']
"#,
        vm.name,
        user,
        vm.id,
        vm.role,
        vm.mesh_ip.as_deref().unwrap_or("")
    )
}

fn cloud_init_meta_data(vm: &LabVmPlan) -> String {
    format!("instance-id: {}\nlocal-hostname: {}\n", vm.id, vm.name)
}

fn cloud_init_network_config(vm: &LabVmPlan) -> String {
    let address = vm.local_ip.as_deref().unwrap_or("172.30.41.254");
    format!(
        r#"version: 2
ethernets:
  enp1s0:
    dhcp4: false
    addresses:
      - {}/24
    routes: []
"#,
        address
    )
}
