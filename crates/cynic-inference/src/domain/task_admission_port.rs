//! Admission port for the DomainTaskQueue (plan §10 step 3).
//!
//! The queue never probes capacity itself: it consumes an injected
//! [`TaskAdmissionPort`] that answers "may this payload enter the queue
//! now?" from node-declared envelope + freshness policy + latest capacity.
//! Tests inject fakes; production binds the host LocalCapacityAdapter.

use cynic_protocol::task_admission::TaskAdmission;
use cynic_protocol::task_pipeline::EnergyBudget;

/// Read-only admission authority. Implemented host-side; the queue only
/// knows this trait, keeping the dependency arrow inference → protocol.
pub trait TaskAdmissionPort: Send + Sync {
    /// Budgets of windows currently open on the host (journal read-back).
    fn open_reservations(&self) -> Vec<EnergyBudget>;
    /// Node-declared concurrent ceilings (None dim = unbounded).
    fn envelope(&self) -> EnergyBudget;
    /// Whether the latest CapacityObservation is fresh under node policy.
    fn capacity_fresh(&self) -> bool;

    /// Convenience: full decision for one payload.
    fn admit(&self, payload: &serde_json::Value) -> TaskAdmission {
        cynic_protocol::task_admission::admit_task(
            payload,
            &self.open_reservations(),
            &self.envelope(),
            self.capacity_fresh(),
        )
    }
}

/// Queue-side rejection error carrying the honest reasons.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskAdmissionError {
    Unknown(Vec<String>),
    Blocked(Vec<String>),
}

impl std::fmt::Display for TaskAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(reasons) => write!(f, "unknown: {}", reasons.join(", ")),
            Self::Blocked(reasons) => write!(f, "blocked: {}", reasons.join(", ")),
        }
    }
}

impl TaskAdmissionError {
    pub fn from_decision(decision: &TaskAdmission) -> Option<Self> {
        match decision {
            TaskAdmission::AdmittedUnbudgeted | TaskAdmission::AdmittedWithReservation => None,
            TaskAdmission::Unknown { reasons } => Some(Self::Unknown(reasons.clone())),
            TaskAdmission::Blocked { reasons } => Some(Self::Blocked(reasons.clone())),
        }
    }
}
