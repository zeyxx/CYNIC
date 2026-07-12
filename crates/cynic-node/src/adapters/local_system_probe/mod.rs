use serde_json::json;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::domain::system_observation::{
    PlatformObservation, SystemMetricsSample, critical_proxmox_vmids_to_restart, parse_zpool_health,
};
use crate::ports::system_probe_port::{SystemMetricsPort, SystemProbePlatformPort};

pub struct SysinfoMetricsSampler {
    sys: System,
    disks: Disks,
}

impl SysinfoMetricsSampler {
    pub fn new() -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            ),
            disks: Disks::new_with_refreshed_list(),
        }
    }
}

impl SystemMetricsPort for SysinfoMetricsSampler {
    fn sample_metrics(&mut self) -> SystemMetricsSample {
        self.sys.refresh_all();
        self.disks.refresh(true);

        let mut disk_total = 0.0;
        let mut disk_available = 0.0;
        for disk in &self.disks {
            disk_total += disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
            disk_available += disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        }

        SystemMetricsSample {
            cpu_usage_percent: self.sys.global_cpu_usage(),
            memory_used_gb: self.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
            memory_total_gb: self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
            disk_total_gb: disk_total,
            disk_available_gb: disk_available,
            load_average_1m: System::load_average().one,
            uptime_seconds: System::uptime(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPlatformProbe;

impl SystemProbePlatformPort for LinuxPlatformProbe {
    fn observe_platform(&self) -> PlatformObservation {
        #[cfg(target_os = "linux")]
        {
            use cynic_host::process_observation::capture_command;

            let mut zfs_health = serde_json::Value::Null;
            let mut proxmox_resources = serde_json::Value::Null;
            let mut critical_vmids_to_restart = Vec::new();

            if let Ok(output) = capture_command("zpool", &["list", "-H", "-o", "name,health"])
                && output.success
            {
                zfs_health = parse_zpool_health(output.stdout.as_bytes());
            }

            if let Ok(output) = capture_command(
                "sudo",
                &[
                    "-n",
                    "pvesh",
                    "get",
                    "/cluster/resources",
                    "--output-format",
                    "json",
                ],
            ) && output.success
                && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&output.stdout)
            {
                critical_vmids_to_restart = critical_proxmox_vmids_to_restart(&json_val);
                proxmox_resources = json!({ "resources": &json_val });
            }

            PlatformObservation {
                zfs_health,
                proxmox_resources,
                critical_vmids_to_restart,
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            PlatformObservation {
                zfs_health: serde_json::Value::Null,
                proxmox_resources: serde_json::Value::Null,
                critical_vmids_to_restart: Vec::new(),
            }
        }
    }

    fn restart_critical_vmid(&self, vmid: &str) {
        #[cfg(target_os = "linux")]
        {
            use cynic_host::process_observation::capture_command;

            let _ = capture_command("sudo", &["-n", "qm", "start", vmid]);
        }

        #[cfg(not(target_os = "linux"))]
        let _ = vmid;
    }
}

#[cfg(test)]
mod tests {
    use super::LinuxPlatformProbe;

    #[test]
    fn linux_platform_probe_is_constructible() {
        let _probe = LinuxPlatformProbe;
    }
}
