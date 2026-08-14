use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use cynic_protocol::{
    DetectedCapability, HardwareCapabilities, ObservationMethod, ResidualRisk,
};

pub fn detect_hardware_capabilities() -> DetectedCapability<HardwareCapabilities> {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_memory();

    let physical_cores = sysinfo::System::physical_core_count().unwrap_or(1);
    let logical_cores = sysinfo::System::cpus(&sys).len();
    let ram_total_bytes = sys.total_memory();

    let cpu_arch = std::env::consts::ARCH.to_string();

    #[cfg(target_os = "linux")]
    let gpu_detected = std::path::Path::new("/proc/driver/nvidia/version").exists()
        || std::path::Path::new("/dev/dri").exists();

    #[cfg(target_os = "windows")]
    let gpu_detected = true;

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let gpu_detected = false;

    let capabilities = HardwareCapabilities {
        cpu_arch,
        physical_cores,
        logical_cores,
        ram_total_bytes,
        vram_total_bytes: None,
        gpu_accelerator_detected: gpu_detected,
        gpu_vendor: if gpu_detected {
            Some("Generic/Detected".to_string())
        } else {
            None
        },
        direct_compute_backend: if gpu_detected {
            Some("Vulkan/DirectX".to_string())
        } else {
            None
        },
    };

    if gpu_detected {
        DetectedCapability::resolved(
            capabilities,
            ObservationMethod::NativeSyscall,
            90,
        )
    } else {
        DetectedCapability::resolved(
            capabilities,
            ObservationMethod::ProcFsDirect,
            75,
        )
    }
}
