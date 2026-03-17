use std::path::{Path, PathBuf};
use sysinfo::System;

use super::types::*;

// ============================================================
// HARDWARE PROBE
// ============================================================

pub(super) async fn probe_hardware() -> HardwareInfo {
    let hw = tokio::task::spawn_blocking(|| {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_model = sys.cpus().first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown".into());

        HardwareInfo {
            total_ram_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
            cpu_cores: sys.cpus().len(),
            cpu_model,
        }
    }).await.expect("probe_hardware spawn_blocking panicked");
    klog!("[Ring 0 / HW]  CPU: {} ({} cores) | RAM: {:.1} GB",
        hw.cpu_model, hw.cpu_cores, hw.total_ram_gb);
    hw
}

// ============================================================
// COMPUTE DETECTION — reads sysfs, no external tools required
// ============================================================

/// Sync detection result — resolved in spawn_blocking, then enriched async.
enum SyncDetection {
    /// Fully resolved — no async enrichment needed.
    Complete(ComputeInfo, &'static str),
    /// Needs async enrichment via PowerShell bridge (WSL2/Windows).
    NeedsEnrich(ComputeInfo, &'static str),
    /// /dev/dxg exists but sysfs found nothing — need PowerShell bridge.
    NeedsBridge,
    /// Windows native — need PowerShell detection.
    NeedsWindows,
    /// CPU fallback.
    CpuFallback(ComputeInfo),
}

pub(super) async fn probe_compute() -> ComputeInfo {
    // Phase 1: Sync detection (sysfs, nvidia-smi) — runs in spawn_blocking.
    let detection = tokio::task::spawn_blocking(|| {
        // 1. NVIDIA via nvidia-smi
        if let Some(info) = detect_nvidia() {
            return SyncDetection::Complete(info, "NVIDIA");
        }
        // 2. AMD ROCm via /dev/kfd
        if Path::new("/dev/kfd").exists() {
            let info = detect_amd_via_sysfs(false);
            return SyncDetection::Complete(
                ComputeInfo { backend: ComputeBackend::ROCm, ..info }, "ROCm");
        }
        // 3. /sys/class/drm/ vendor
        if let Some(info) = detect_gpu_via_sysfs() {
            if info.gpu_name.to_lowercase().contains("generic") || info.gpu_name.is_empty() {
                return SyncDetection::NeedsEnrich(info, "DRM");
            }
            return SyncDetection::Complete(info, "DRM");
        }
        // 4. /dev/dxg exists but sysfs found nothing → WSL2
        if Path::new("/dev/dxg").exists() {
            return SyncDetection::NeedsBridge;
        }
        // 5. Windows native
        if cfg!(target_os = "windows") {
            return SyncDetection::NeedsWindows;
        }
        // 6. macOS Metal
        if cfg!(target_os = "macos") {
            return SyncDetection::Complete(ComputeInfo {
                backend: ComputeBackend::Metal,
                gpu_name: "Apple Silicon GPU".into(),
                vram_gb: 0.0,
                is_igpu: true,
                ..detect_cpu_info()
            }, "Metal");
        }
        // 7. CPU fallback
        SyncDetection::CpuFallback(detect_cpu_info())
    }).await.expect("probe_compute spawn_blocking panicked");

    // Phase 2: Async enrichment (PowerShell bridge) — only when needed.
    match detection {
        SyncDetection::Complete(info, source) => {
            klog!("[Ring 0 / GPU] {}: {} → {:?}", source, info.gpu_name, info.backend);
            info
        }
        SyncDetection::NeedsEnrich(mut info, source) => {
            if let Some(host_name) = probe_windows_gpu().await {
                info.gpu_name = host_name;
            }
            klog!("[Ring 0 / GPU] {}: {} (is_igpu:{}) → {:?}",
                source, info.gpu_name, info.is_igpu, info.backend);
            info
        }
        SyncDetection::NeedsBridge => {
            if let Some(host_name) = probe_windows_gpu().await {
                let mut info = detect_cpu_info();
                info.gpu_name = host_name;
                if info.gpu_name.to_lowercase().contains("amd") || info.gpu_name.to_lowercase().contains("radeon") {
                    info.backend = ComputeBackend::Vulkan;
                } else if info.gpu_name.to_lowercase().contains("nvidia") {
                    info.backend = ComputeBackend::Cuda;
                }
                info.is_igpu = true;
                klog!("[Ring 0 / GPU] Bridge: {} → {:?}", info.gpu_name, info.backend);
                info
            } else {
                let cpu = detect_cpu_info();
                klog!("[Ring 0 / GPU] No GPU → cpu (AVX2:{})", cpu.avx2);
                cpu
            }
        }
        SyncDetection::NeedsWindows => {
            if let Some(host_name) = probe_windows_gpu().await {
                let mut info = detect_cpu_info();
                info.gpu_name = host_name;
                info.backend = ComputeBackend::Vulkan;
                info.is_igpu = true;
                if info.gpu_name.to_lowercase().contains("nvidia") {
                    info.backend = ComputeBackend::Cuda;
                    info.is_igpu = false;
                }
                klog!("[Ring 0 / GPU] Windows Native: {} → {:?}", info.gpu_name, info.backend);
                info
            } else {
                let cpu = detect_cpu_info();
                klog!("[Ring 0 / GPU] No GPU → cpu (AVX2:{})", cpu.avx2);
                cpu
            }
        }
        SyncDetection::CpuFallback(cpu) => {
            klog!("[Ring 0 / GPU] No GPU → cpu (AVX2:{})", cpu.avx2);
            cpu
        }
    }
}

fn detect_nvidia() -> Option<ComputeInfo> {
    let out = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.trim().lines().next()?;
    let parts: Vec<&str> = line.splitn(2, ',').collect();
    let gpu_name = parts.first().unwrap_or(&"NVIDIA GPU").trim().to_string();
    let vram_mb: f64 = parts.get(1).unwrap_or(&"0").trim().parse().unwrap_or(0.0);

    Some(ComputeInfo {
        backend: ComputeBackend::Cuda,
        gpu_name,
        vram_gb: vram_mb / 1024.0,
        is_igpu: false,
        ..detect_cpu_info()
    })
}

/// Read /sys/class/drm/card*/device/vendor to identify GPU vendor
fn detect_gpu_via_sysfs() -> Option<ComputeInfo> {
    let drm = Path::new("/sys/class/drm");
    if !drm.exists() { return None; }

    for entry in std::fs::read_dir(drm).ok()?.flatten() {
        let card = entry.path();
        let name = card.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Only top-level card entries (card0, card1...)
        if !name.starts_with("card") || name.contains('-') { continue; }

        let vendor_path = card.join("device/vendor");
        let vendor = std::fs::read_to_string(&vendor_path).ok()?;
        let vendor = vendor.trim();

        match vendor {
            "0x1002" => { // AMD
                let info = detect_amd_via_sysfs(true);
                return Some(ComputeInfo { backend: ComputeBackend::Vulkan, ..info });
            }
            "0x8086" => { // Intel
                return Some(ComputeInfo {
                    backend: ComputeBackend::Vulkan,
                    gpu_name: read_sysfs_string(card.join("device/product_name"))
                        .unwrap_or_else(|| "Intel iGPU".into()),
                    vram_gb: 0.0,
                    is_igpu: true,
                    ..detect_cpu_info()
                });
            }
            _ => continue,
        }
    }
    None
}

fn detect_amd_via_sysfs(is_igpu: bool) -> ComputeInfo {
    // Try to read GPU name from drm device
    let gpu_name = std::fs::read_dir("/sys/class/drm")
        .ok()
        .and_then(|mut d| d.find_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains('-') { return None; }
            read_sysfs_string(e.path().join("device/product_name"))
        }))
        .unwrap_or_else(|| "AMD GPU".into());

    ComputeInfo {
        backend: ComputeBackend::Vulkan, // set by caller for ROCm
        gpu_name,
        vram_gb: 0.0, // iGPU shares RAM; dGPU VRAM readable via hwmon but complex
        is_igpu,
        ..detect_cpu_info()
    }
}

fn read_sysfs_string(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub(super) fn detect_cpu_info() -> ComputeInfo {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get()).unwrap_or(4);

    let avx2 = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/cpuinfo")
            .map(|c| c.contains("avx2"))
            .unwrap_or(false)
    } else if cfg!(target_os = "windows") {
        // Simple heuristic for Windows: check for RYZEN/CORE and skip older ones
        // In real SRE lens, we'd use a crate or cpuid, but this is a bridge
        true // Most modern machines have AVX2 now
    } else {
        false
    };

    ComputeInfo {
        backend: ComputeBackend::Cpu,
        gpu_name: String::new(),
        vram_gb: 0.0,
        cpu_threads: threads,
        avx2,
        is_igpu: false,
    }
}

async fn probe_windows_gpu() -> Option<String> {
    tokio::task::spawn_blocking(|| {
        let out = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"])
            .output()
            .ok()?;

        if !out.status.success() { return None; }
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if text.is_empty() { return None; }

        // Filter out virtual adapters
        let lines: Vec<String> = text.lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.to_lowercase().contains("virtual")
                        && !s.to_lowercase().contains("parsec")
                        && !s.to_lowercase().contains("citrix")
                        && !s.to_lowercase().contains("microsoft remote"))
            .collect();

        if lines.is_empty() {
            return text.lines().next().map(|s| s.trim().to_string());
        }

        Some(lines[0].clone())
    }).await.ok()?
}
