use std::path::Path;

use cynic_protocol::{DetectedCapability, InstallProvenance, ObservationMethod};

pub fn detect_install_provenance(binary_path: &Path) -> DetectedCapability<InstallProvenance> {
    let normalized = binary_path.to_string_lossy().replace('\\', "/");

    if normalized.contains("/.local/share/uv/tools/") {
        return DetectedCapability::resolved(
            InstallProvenance::UvTool,
            ObservationMethod::FileMetadata,
            95,
        );
    }

    if normalized.contains("/.venv/") || normalized.contains("/venv/") {
        return DetectedCapability::resolved(
            InstallProvenance::UvVenv,
            ObservationMethod::FileMetadata,
            90,
        );
    }

    if normalized.contains("/.local/pipx/venvs/") {
        return DetectedCapability::resolved(
            InstallProvenance::Pipx,
            ObservationMethod::FileMetadata,
            90,
        );
    }

    if normalized.starts_with("/usr/bin/")
        || normalized.starts_with("/bin/")
        || normalized.starts_with("/usr/local/bin/")
    {
        return DetectedCapability::resolved(
            InstallProvenance::DistroManagedPackage,
            ObservationMethod::FileMetadata,
            85,
        );
    }

    if normalized.contains("/.local/bin/") {
        return DetectedCapability::resolved(
            InstallProvenance::ScriptedLocal,
            ObservationMethod::FileMetadata,
            80,
        );
    }

    DetectedCapability::resolved(
        InstallProvenance::StandaloneBinary,
        ObservationMethod::FileMetadata,
        70,
    )
}
