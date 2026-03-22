use std::path::{Path, PathBuf};

use super::types::*;

// ============================================================
// ENVIRONMENT PROBE
// ============================================================

pub(super) async fn probe_environment() -> Result<EnvInfo, tokio::task::JoinError> {
    // All filesystem probes run in spawn_blocking — never stall the tokio runtime.
    let (os, is_wsl2, is_docker, is_proxmox_lxc, wsl2_windows_host, models_dir) =
        tokio::task::spawn_blocking(|| {
            let os = if cfg!(target_os = "linux") { "linux" }
                      else if cfg!(target_os = "windows") { "windows" }
                      else { "macos" }.to_string();

            let is_wsl2 = std::fs::read_to_string("/proc/version")
                .map(|v| v.to_lowercase().contains("microsoft"))
                .unwrap_or(false);

            let is_docker = Path::new("/.dockerenv").exists();
            let is_proxmox_lxc = std::fs::read_to_string("/proc/1/environ")
                .map(|v| v.contains("container=lxc"))
                .unwrap_or(false);

            let wsl2_windows_host = if is_wsl2 {
                std::fs::read_to_string("/etc/resolv.conf").ok()
                    .and_then(|c| c.lines()
                        .find(|l| l.starts_with("nameserver"))
                        .and_then(|l| l.split_whitespace().nth(1))
                        .map(|s| s.to_string()))
            } else { None };

            let models_dir = ensure_models_dir();
            (os, is_wsl2, is_docker, is_proxmox_lxc, wsl2_windows_host, models_dir)
        }).await?;

    klog!("[Ring 0 / Env] OS: {} | WSL2: {} | Docker: {} | LXC: {} | WinHost: {:?}",
        os, is_wsl2, is_docker, is_proxmox_lxc, wsl2_windows_host);

    let io_benchmark_mbps = benchmark_io(&models_dir).await;

    Ok(EnvInfo {
        os,
        is_wsl2,
        is_docker,
        is_proxmox_lxc,
        wsl2_windows_host,
        io_benchmark_mbps,
    })
}

pub(super) async fn benchmark_io(path: &Path) -> f64 {
    let path_display = path.display().to_string();
    let path = path.to_path_buf();
    let mbps = tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let test_file = path.join(".cynic_io_test");
        let data = vec![0u8; 100 * 1024 * 1024]; // 100MB test
        let start = std::time::Instant::now();

        let mut f = match std::fs::File::create(&test_file) {
            Ok(f) => f,
            Err(_) => return 0.0,
        };

        if f.write_all(&data).is_err() { return 0.0; }
        if f.sync_all().is_err() { return 0.0; }

        let duration = start.elapsed().as_secs_f64();
        let _ = std::fs::remove_file(&test_file);

        100.0 / duration
    }).await.unwrap_or(0.0);
    klog!("[Ring 0 / Env] IO Benchmark: {:.1} MB/s on {}", mbps, path_display);
    mbps
}

/// Convert a Windows path like "D:\Models" to WSL2 path "/mnt/d/Models"
pub(super) fn windows_to_wsl_path(win_path: &str) -> PathBuf {
    let normalized = win_path.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//") {
        // \\server\share → skip for now
        return PathBuf::from(rest);
    }
    // D:/Models → /mnt/d/Models
    if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
        let drive = (normalized.as_bytes()[0].to_ascii_lowercase() as char).to_string();
        let rest = &normalized[2..]; // strip "D:"
        return PathBuf::from(format!("/mnt/{}{}", drive, rest));
    }
    PathBuf::from(normalized)
}

pub(super) fn ensure_models_dir() -> PathBuf {
    // Priority: 1. CYNIC_MODELS_DIR env var, 2. ~/.cynic/models
    let dir = if let Ok(env_path) = std::env::var("CYNIC_MODELS_DIR") {
        PathBuf::from(env_path)
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(".cynic").join("models")
    };

    if !dir.exists() {
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("README.txt"),
            "CYNIC Industrial Model Directory\n\
             ===============================\n\
             Note: Redirection active via $CYNIC_MODELS_DIR\n"
        ).ok();
        klog!("[Ring 0 / LLM] Created models directory: {}", dir.display());
    }
    dir
}
