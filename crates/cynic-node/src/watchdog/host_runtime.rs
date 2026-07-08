use sysinfo::Disks;

pub(super) fn disk_totals_gb(disks: &Disks) -> (f64, f64) {
    let mut total = 0.0;
    let mut available = 0.0;
    for disk in disks {
        total += disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        available += disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
    }
    (total, available)
}

pub(super) fn bytes_to_gb(value: u64) -> f64 {
    value as f64 / 1024.0 / 1024.0 / 1024.0
}

pub(super) fn hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn local_ips() -> Vec<String> {
    std::process::Command::new("hostname")
        .arg("-I")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn systemd_service_status(service: &str) -> String {
    std::process::Command::new("systemctl")
        .args(["is-active", service])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|status| !status.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
