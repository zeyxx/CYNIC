//! Inference domain types — backend status and initialization errors.
//!
//! No backend implementation belongs here. This module carries only domain data.

/// 5-state health lifecycle — φ-bounded: UNKNOWN until proven.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendStatus {
    Unknown,
    Healthy,
    Degraded { latency_ms: f64 },
    Critical,
    Recovering,
}

impl BackendStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded { .. })
    }

    /// Map onto the generic supervisor lifecycle (G10, G3 rule: degraded and
    /// critical block like failed; recovering is transitional toward active
    /// like starting).
    pub fn supervisor_state(&self) -> cynic_protocol::supervision::SupervisorState {
        use cynic_protocol::supervision::SupervisorState as S;
        match self {
            Self::Healthy => S::Active,
            Self::Degraded { .. } => S::Failed,
            Self::Critical => S::Failed,
            Self::Recovering => S::Starting,
            Self::Unknown => S::Unknown,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Backend initialization failed: {0}")]
pub struct BackendInitError(String);

impl BackendInitError {
    pub fn from_http(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::supervision::SupervisorState as S;

    #[test]
    fn backend_status_maps_onto_supervisor_lifecycle() {
        assert_eq!(BackendStatus::Healthy.supervisor_state(), S::Active);
        assert_eq!(
            BackendStatus::Degraded { latency_ms: 1.0 }.supervisor_state(),
            S::Failed
        );
        assert_eq!(BackendStatus::Critical.supervisor_state(), S::Failed);
        assert_eq!(BackendStatus::Recovering.supervisor_state(), S::Starting);
        assert_eq!(BackendStatus::Unknown.supervisor_state(), S::Unknown);
    }
}
