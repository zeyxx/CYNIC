use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::package_provenance::{
    PackageRepositoryObservation, promotion_observation_for_repository,
};
use crate::domain::packages::PackageObservedState;
use crate::domain::principal::{PrincipalObservedState, PrincipalPolicy};
use crate::domain::privilege::{PrivilegeObservedState, PrivilegePolicy};
use crate::domain::services::{ServiceObservedState, ServiceObservedUnit, ServicePolicy};
use crate::ports::{
    HostHardwareObservationPort, HostPackageObservationPort, HostPrincipalObservationPort,
    HostPrivilegeBridgeObservationPort, HostPrivilegeObservationPort, HostServiceObservationPort,
    PreOsPlatformObservationPort,
};
use crate::process_observation::{command_exit_success, command_stdout};
use crate::runtime_layout::current_runtime_layout;
use crate::service_supervisor_observation::observe_service_supervisor;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHostObservationAdapter;

impl PreOsPlatformObservationPort for LocalHostObservationAdapter {
    fn observe_pre_os_platform(&self) -> cynic_node_protocol::PreOsPlatformObservation {
        crate::adapters::local_pre_os::LocalPreOsAdapter.observe_pre_os_platform()
    }
}

impl HostHardwareObservationPort for LocalHostObservationAdapter {
    fn observe_hardware(&self) -> crate::hardware_conformance::HardwareObservation {
        crate::hardware_conformance::HardwareObservation {
            inventory: observe_hardware_inventory(),
            baseline_digest: load_hardware_baseline_digest(),
            firmware_version: std::fs::read_to_string("/sys/class/dmi/id/bios_version")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            max_temperature_millicelsius: read_numeric_children("/sys/class/thermal", "temp")
                .into_iter()
                .max(),
            corrected_memory_errors: sum_numeric_children(
                "/sys/devices/system/edac/mc",
                "ce_count",
            ),
            uncorrected_memory_errors: sum_numeric_children(
                "/sys/devices/system/edac/mc",
                "ue_count",
            ),
            storage_device_count: std::fs::read_dir("/sys/class/block")
                .map(|entries| entries.filter_map(Result::ok).count())
                .unwrap_or(0),
            offline_memory_test: crate::hardware_conformance::EvidenceState::Unknown,
            memory_tuning: crate::hardware_conformance::EvidenceState::Unknown,
            runtime_tuning_overrides: observe_runtime_tuning_overrides(),
        }
    }
}

fn observe_runtime_tuning_overrides() -> Vec<String> {
    [
        (
            "amdgpu.ppfeaturemask",
            "/sys/module/amdgpu/parameters/ppfeaturemask",
        ),
        ("amdgpu.gttsize", "/sys/module/amdgpu/parameters/gttsize"),
        ("ttm.pages_limit", "/sys/module/ttm/parameters/pages_limit"),
        (
            "ttm.page_pool_size",
            "/sys/module/ttm/parameters/page_pool_size",
        ),
    ]
    .into_iter()
    .filter_map(|(name, path)| read_trimmed(path).map(|value| format!("{name}={value}")))
    .collect()
}

fn load_hardware_baseline_digest() -> Option<String> {
    let path = Path::new(&current_runtime_layout().config_dir).join("hardware-baseline.json");
    let payload = std::fs::read(path).ok()?;
    let baseline =
        serde_json::from_slice::<crate::hardware_conformance::HardwareBaseline>(&payload).ok()?;
    (baseline.schema == crate::hardware_conformance::HARDWARE_BASELINE_SCHEMA)
        .then_some(baseline.inventory_digest)
}

fn observe_hardware_inventory() -> crate::hardware_conformance::HardwareInventory {
    let mut devices = observe_pci_devices()
        .into_iter()
        .chain(observe_storage_devices())
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| (&left.bus, &left.address).cmp(&(&right.bus, &right.address)));
    crate::hardware_conformance::HardwareInventory {
        system_vendor: read_trimmed("/sys/class/dmi/id/sys_vendor"),
        system_product: read_trimmed("/sys/class/dmi/id/product_name"),
        board_vendor: read_trimmed("/sys/class/dmi/id/board_vendor"),
        board_name: read_trimmed("/sys/class/dmi/id/board_name"),
        board_version: read_trimmed("/sys/class/dmi/id/board_version"),
        firmware_vendor: read_trimmed("/sys/class/dmi/id/bios_vendor"),
        firmware_date: read_trimmed("/sys/class/dmi/id/bios_date"),
        cpu_model: cpu_model(),
        cpu_microcode: read_trimmed("/sys/devices/system/cpu/cpu0/microcode/version"),
        logical_cpu_count: std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(0),
        kernel_release: read_trimmed("/proc/sys/kernel/osrelease"),
        devices,
    }
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn cpu_model() -> Option<String> {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("model name\t:").map(str::trim))
        .map(str::to_string)
}

fn observe_pci_devices() -> Vec<crate::hardware_conformance::HardwareDevice> {
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            crate::hardware_conformance::HardwareDevice {
                bus: "pci".to_string(),
                address: entry.file_name().to_string_lossy().into_owned(),
                class: read_trimmed(path.join("class")),
                vendor_id: read_trimmed(path.join("vendor")),
                device_id: read_trimmed(path.join("device")),
                model: None,
                firmware: None,
                driver: std::fs::read_link(path.join("driver"))
                    .ok()
                    .and_then(|driver| {
                        driver
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    }),
            }
        })
        .collect()
}

fn observe_storage_devices() -> Vec<crate::hardware_conformance::HardwareDevice> {
    let Ok(entries) = std::fs::read_dir("/sys/class/block") else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| !entry.path().join("partition").exists())
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with("loop"))
        .map(|entry| {
            let path = entry.path();
            crate::hardware_conformance::HardwareDevice {
                bus: "block".to_string(),
                address: entry.file_name().to_string_lossy().into_owned(),
                class: None,
                vendor_id: read_trimmed(path.join("device/vendor")),
                device_id: None,
                model: read_trimmed(path.join("device/model")),
                firmware: read_trimmed(path.join("device/firmware_rev"))
                    .or_else(|| read_trimmed(path.join("device/rev"))),
                driver: std::fs::read_link(path.join("device/driver"))
                    .ok()
                    .and_then(|driver| {
                        driver
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    }),
            }
        })
        .collect()
}

fn read_numeric_children(root: &str, file_name: &str) -> Vec<i64> {
    let mut values = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return values;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path().join(file_name);
        if let Ok(value) = std::fs::read_to_string(path)
            && let Ok(value) = value.trim().parse()
        {
            values.push(value);
        }
    }
    values
}

fn sum_numeric_children(root: &str, file_name: &str) -> Option<u64> {
    let values = read_numeric_children(root, file_name);
    (!values.is_empty()).then(|| {
        values
            .into_iter()
            .filter_map(|value| u64::try_from(value).ok())
            .sum()
    })
}

impl HostPrivilegeBridgeObservationPort for LocalHostObservationAdapter {
    fn observe_privilege_bridge(
        &self,
        spec: &crate::bridge::PrivilegeBridgeInstallSpec,
    ) -> crate::bridge::PrivilegeBridgeObservedState {
        super::privilege_bridge::observe(spec)
    }
}

impl HostPrincipalObservationPort for LocalHostObservationAdapter {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        let capabilities = current_runtime_layout().capabilities();
        let resolver_command = capabilities.name_service.resolver_command;
        let passwd = command_stdout(&resolver_command, &["passwd", policy.runtime_user.as_str()]);
        let group = command_stdout(&resolver_command, &["group", policy.runtime_group.as_str()]);
        let observed_home_dir = passwd
            .as_deref()
            .and_then(|line| line.split(':').nth(5))
            .map(str::to_string);
        let expected_uid = passwd
            .as_deref()
            .and_then(|line| line.split(':').nth(2))
            .and_then(|value| value.parse::<u32>().ok());
        let expected_gid = group
            .as_deref()
            .and_then(|line| line.split(':').nth(2))
            .and_then(|value| value.parse::<u32>().ok());

        let mut owned_paths = Vec::new();
        let mut ownership_unknown_paths = Vec::new();
        for path in policy.managed_paths() {
            match path_ownership(&path, expected_uid, expected_gid) {
                PathOwnership::Owned => owned_paths.push(path),
                PathOwnership::Unknown => ownership_unknown_paths.push(path),
                PathOwnership::Mismatch => {}
            }
        }

        PrincipalObservedState {
            user_present: passwd.is_some(),
            group_present: group.is_some(),
            observed_home_dir,
            present_state_dirs: policy
                .state_dirs
                .iter()
                .filter(|path| Path::new(path.as_str()).is_dir())
                .cloned()
                .collect(),
            owned_paths,
            ownership_unknown_paths,
        }
    }
}

enum PathOwnership {
    Owned,
    Mismatch,
    Unknown,
}

fn path_ownership(
    path: &str,
    expected_uid: Option<u32>,
    expected_gid: Option<u32>,
) -> PathOwnership {
    let (Some(expected_uid), Some(expected_gid)) = (expected_uid, expected_gid) else {
        return PathOwnership::Unknown;
    };
    if identity_is_mapped("/proc/self/uid_map", expected_uid) != Some(true)
        || identity_is_mapped("/proc/self/gid_map", expected_gid) != Some(true)
    {
        return PathOwnership::Unknown;
    }
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.uid() == expected_uid && metadata.gid() == expected_gid => {
            PathOwnership::Owned
        }
        Ok(_) | Err(_) => PathOwnership::Mismatch,
    }
}

fn identity_is_mapped(map_path: &str, identity: u32) -> Option<bool> {
    let content = std::fs::read_to_string(map_path).ok()?;
    Some(identity_in_map(&content, identity))
}

fn identity_in_map(content: &str, identity: u32) -> bool {
    content.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let inside = fields.next().and_then(|value| value.parse::<u64>().ok());
        let _outside = fields.next().and_then(|value| value.parse::<u64>().ok());
        let length = fields.next().and_then(|value| value.parse::<u64>().ok());
        match (inside, length) {
            (Some(inside), Some(length)) => {
                let identity = u64::from(identity);
                identity >= inside && identity < inside.saturating_add(length)
            }
            _ => false,
        }
    })
}

impl HostPrivilegeObservationPort for LocalHostObservationAdapter {
    fn observe_privilege(&self, policy: &PrivilegePolicy) -> PrivilegeObservedState {
        let helper_dir_present = Path::new(&policy.helper_dir).is_dir();
        let sudoers_projection_present = Path::new(&policy.sudoers_dir).is_dir();
        let sudo = current_runtime_layout().sudo_bin();
        // Denial proof is about the *runtime principal*, not the operator:
        // `sudo -u <subject> sudo -n /bin/sh -c true` must fail (no passwordless
        // unrestricted shell for cynic). Operator NOPASSWD:ALL must not mask this.
        let subject = policy.sudoers_subject.as_str();
        let denial_proof_ready = command_exit_success(
            &sudo,
            &["-u", subject, &sudo, "-n", "/bin/sh", "-c", "true"],
        )
        .is_some_and(|ok| !ok);

        PrivilegeObservedState {
            helper_dir_present,
            sudoers_projection_present,
            denial_proof_ready,
        }
    }
}

impl HostPackageObservationPort for LocalHostObservationAdapter {
    fn observe_packages(
        &self,
        policy: &crate::domain::packages::PackagePolicy,
    ) -> PackageObservedState {
        PackageObservedState {
            installed_packages: policy
                .bootstrap_packages
                .iter()
                .filter(|pkg| package_present(pkg))
                .cloned()
                .collect(),
            promotion: observe_package_promotion(policy),
            promotion_assessed_at_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0),
        }
    }
}

fn observe_package_promotion(
    policy: &crate::domain::packages::PackagePolicy,
) -> crate::domain::package_provenance::PackagePromotionObservation {
    let Ok(raw) = std::fs::read_to_string(&policy.source_projection_path) else {
        return Default::default();
    };
    let Ok(receipt) = serde_json::from_str::<PackageRepositoryObservation>(&raw) else {
        return Default::default();
    };
    promotion_observation_for_repository(&policy.provenance, &receipt).unwrap_or_default()
}

impl HostServiceObservationPort for LocalHostObservationAdapter {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState {
        let supervisor_observation =
            observe_service_supervisor("local-host-substrate".to_string(), policy);
        ServiceObservedState {
            units: supervisor_observation
                .services
                .iter()
                .map(service_observation_to_substrate_unit)
                .collect(),
        }
    }
}

fn package_present(package: &str) -> bool {
    let layout = current_runtime_layout();
    let capabilities = layout.capabilities();
    let argv = capabilities
        .package_manager
        .kind
        .query_argv(package, &layout);
    command_exit_success(&argv[0], &string_args(&argv[1..])).unwrap_or(false)
        || command_exit_success(&layout.which_bin(), &[package]).unwrap_or(false)
}

fn string_args(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

fn service_observation_to_substrate_unit(
    unit: &cynic_protocol::ServiceSupervisorUnitObservation,
) -> ServiceObservedUnit {
    ServiceObservedUnit {
        unit_name: unit.unit_name.clone(),
        present: unit_present(unit),
        enabled: unit_enabled(unit),
        active: unit.status == "active",
    }
}

fn unit_present(unit: &cynic_protocol::ServiceSupervisorUnitObservation) -> bool {
    match unit.selected_provider.as_str() {
        "systemd" => unit.load_state == "loaded",
        "openrc" | "process" | "process_port" => unit.probe_status == "ok",
        _ => unit.status == "active" || unit.status == "degraded",
    }
}

fn unit_enabled(unit: &cynic_protocol::ServiceSupervisorUnitObservation) -> bool {
    match unit.selected_provider.as_str() {
        "systemd" => matches!(
            unit.unit_file_state.as_str(),
            "enabled" | "enabled-runtime" | "static" | "generated"
        ),
        "openrc" | "process" | "process_port" => unit.status == "active",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{identity_in_map, service_observation_to_substrate_unit};
    use cynic_protocol::ServiceSupervisorUnitObservation;

    fn unit(
        selected_provider: &str,
        load_state: &str,
        unit_file_state: &str,
        status: &'static str,
        probe_status: &'static str,
    ) -> ServiceSupervisorUnitObservation {
        ServiceSupervisorUnitObservation {
            service_id: "svc".to_string(),
            unit_name: "svc.service".to_string(),
            health_port: None,
            health_kind: None,
            desired_provider: None,
            selected_provider: selected_provider.to_string(),
            selected_scope: "system".to_string(),
            target_source: "test".to_string(),
            desired_state: "enabled_and_active".to_string(),
            restart_policy: "on-failure".to_string(),
            probe_status,
            probe_detail: "test".to_string(),
            provider_attempts: Vec::new(),
            load_state: load_state.to_string(),
            unit_file_state: unit_file_state.to_string(),
            active_state: status.to_string(),
            sub_state: "unknown".to_string(),
            restart_count: None,
            active_enter_timestamp_monotonic: None,
            status,
        }
    }

    #[test]
    fn systemd_supervisor_observation_maps_to_substrate_unit() {
        let observed = service_observation_to_substrate_unit(&unit(
            "systemd", "loaded", "enabled", "active", "ok",
        ));

        assert!(observed.present);
        assert!(observed.enabled);
        assert!(observed.active);
    }

    #[test]
    fn openrc_supervisor_observation_does_not_require_systemctl() {
        let observed = service_observation_to_substrate_unit(&unit(
            "openrc", "unknown", "unknown", "active", "ok",
        ));

        assert!(observed.present);
        assert!(observed.enabled);
        assert!(observed.active);
    }

    #[test]
    fn unknown_supervisor_observation_remains_blocked() {
        let observed = service_observation_to_substrate_unit(&unit(
            "openrc", "unknown", "unknown", "unknown", "blocked",
        ));

        assert!(!observed.present);
        assert!(!observed.enabled);
        assert!(!observed.active);
    }

    #[test]
    fn identity_mapping_rejects_ids_outside_the_visible_namespace() {
        let map = "1000 0 1\n";

        assert!(identity_in_map(map, 1000));
        assert!(!identity_in_map(map, 0));
        assert!(!identity_in_map(map, 1001));
        assert!(identity_in_map("0 0 4294967295\n", 1001));
    }
}
