use serde::Serialize;

use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Observed,
    Declared,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProbeObservation {
    pub status: ProbeStatus,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ProbeObservation {
    pub fn observed(source: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            status: ProbeStatus::Observed,
            source: source.into(),
            value: Some(value.into()),
            error: None,
        }
    }

    pub fn declared(source: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            status: ProbeStatus::Declared,
            source: source.into(),
            value: Some(value.into()),
            error: None,
        }
    }

    pub fn unavailable(source: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            status: ProbeStatus::Unavailable,
            source: source.into(),
            value: None,
            error: Some(error.into()),
        }
    }

    pub fn is_available(&self) -> bool {
        self.status != ProbeStatus::Unavailable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidence {
    Proven,
    DeclaredByVersion,
    TransportBlocked,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageCapability {
    pub id: &'static str,
    pub evidence: CapabilityEvidence,
    pub useful_for: &'static str,
    pub proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageCapabilityObservations {
    pub cli_version: ProbeObservation,
    pub server_version: ProbeObservation,
    pub server_health: ProbeObservation,
    pub engine: ProbeObservation,
    pub transport: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageRisk {
    pub id: &'static str,
    pub severity: &'static str,
    pub finding: String,
    pub required_proof: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageCapabilitiesReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub endpoint: String,
    pub observations: StorageCapabilityObservations,
    pub capabilities: Vec<StorageCapability>,
    pub risks: Vec<StorageRisk>,
    pub residual_unknowns: Vec<&'static str>,
    pub next_actions: Vec<String>,
}
