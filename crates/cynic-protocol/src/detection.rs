use serde::{Deserialize, Serialize};

pub const DETECTION_REPORT_SCHEMA_V1: &str = "cynic.detection_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationMethod {
    NativeSyscall,
    DbusSocket,
    CgroupsV2,
    SysfsDirect,
    ProcFsDirect,
    PeHeaderInspect,
    ElfHeaderInspect,
    FileMetadata,
    AmbientFallback,
    SimulatedProbe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidualRisk {
    pub code: String,
    pub reason: String,
    pub consequence: String,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedCapability<T> {
    pub value: Option<T>,
    pub method: ObservationMethod,
    pub confidence_percent: u8,
    pub residual_risk: Option<ResidualRisk>,
}

impl<T> DetectedCapability<T> {
    pub fn resolved(value: T, method: ObservationMethod, confidence_percent: u8) -> Self {
        Self {
            value: Some(value),
            method,
            confidence_percent,
            residual_risk: None,
        }
    }

    pub fn unknown(method: ObservationMethod, risk: ResidualRisk) -> Self {
        Self {
            value: None,
            method,
            confidence_percent: 0,
            residual_risk: Some(risk),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorKind {
    SystemdUser,
    SystemdSystem,
    ProcessSupervisor,
    WindowsJobObject,
    IsolatedContainer,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallProvenance {
    StandaloneBinary,
    UvTool,
    UvVenv,
    GenericVirtualenv,
    VirtualEnv,
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub cpu_arch: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub ram_total_bytes: u64,
    pub vram_total_bytes: Option<u64>,
    pub gpu_accelerator_detected: bool,
    pub gpu_vendor: Option<String>,
    pub direct_compute_backend: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalOwnershipObservation {
    pub current_user: String,
    pub runtime_user_expected: String,
    pub matches_expected_principal: bool,
    pub is_privileged_admin: bool,
    pub config_dir_writable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detected_capability_resolved() {
        let cap = DetectedCapability::resolved(
            SupervisorKind::SystemdUser,
            ObservationMethod::DbusSocket,
            95,
        );
        assert!(cap.value.is_some());
        assert_eq!(cap.value.unwrap(), SupervisorKind::SystemdUser);
        assert_eq!(cap.method, ObservationMethod::DbusSocket);
        assert_eq!(cap.confidence_percent, 95);
        assert!(cap.residual_risk.is_none());
    }

    #[test]
    fn test_detected_capability_unknown() {
        let cap: DetectedCapability<String> = DetectedCapability::unknown(
            ObservationMethod::AmbientFallback,
            ResidualRisk {
                code: "ERR_UNREACHABLE".to_string(),
                reason: "Host unreachable".to_string(),
                consequence: "Fallback degraded mode".to_string(),
                mitigation: None,
            },
        );
        assert!(cap.value.is_none());
        assert_eq!(cap.method, ObservationMethod::AmbientFallback);
        assert_eq!(cap.confidence_percent, 0);
        assert!(cap.residual_risk.is_some());
    }

    #[test]
    fn test_component_install_authority_invariants() {
        let authority = ComponentInstallAuthority {
            component_name: "hermes-agent".to_string(),
            owning_principal: "cynic".to_string(),
            observed_provenance: InstallProvenance::UvTool,
            expected_provenance: InstallProvenance::UvTool,
            mutator_authority: MutatorAuthority::UvTool,
            is_canonical: true,
        };

        assert_eq!(authority.observed_provenance, authority.expected_provenance);
        assert!(authority.is_canonical);
        assert_eq!(authority.mutator_authority, MutatorAuthority::UvTool);
    }
}
