use std::path::Path;

use cynic_protocol::{DetectedCapability, ObservationMethod, ResidualRisk, SupervisorKind};

pub fn detect_supervisor() -> DetectedCapability<SupervisorKind> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let systemd_socket = Path::new(&runtime_dir).join("systemd/private");
            if systemd_socket.exists() {
                return DetectedCapability::resolved(
                    SupervisorKind::SystemdUser,
                    ObservationMethod::DbusSocket,
                    95,
                );
            }
        }

        if Path::new("/run/systemd/system").exists() {
            return DetectedCapability::resolved(
                SupervisorKind::SystemdSystem,
                ObservationMethod::FileMetadata,
                90,
            );
        }

        DetectedCapability::resolved(
            SupervisorKind::ProcessSupervisor,
            ObservationMethod::ProcFsDirect,
            80,
        )
    }

    #[cfg(target_os = "windows")]
    {
        DetectedCapability::resolved(
            SupervisorKind::WindowsJobObject,
            ObservationMethod::NativeSyscall,
            85,
        )
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        DetectedCapability::unknown(
            ObservationMethod::AmbientFallback,
            ResidualRisk {
                code: "SUPERVISOR_UNKNOWN_OS".to_string(),
                reason: "Platform does not support standard systemd or JobObjects.".to_string(),
                consequence: "Process supervision falls back to generic unmonitored spawn.".to_string(),
                mitigation: Some("Run inside a Linux LXC/VM container for full supervision.".to_string()),
            },
        )
    }
}
