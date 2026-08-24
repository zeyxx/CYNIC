//! Task admission — EnergyBudget gate for DomainTask push (plan §9 gap 4+5).
//!
//! Pure decision layer: a task carries an optional `EnergyBudget` in its
//! payload (`"energy_budget"` key). Admission compares the sum of open
//! window reservations plus the request against the node envelope, using
//! the latest CapacityObservation for per-field freshness (Unknown before
//! Blocked). A task WITHOUT a budget admits freely in V1 — the budget is
//! optional data, never a fabricated zero.

use serde::{Deserialize, Serialize};

pub const TASK_ADMISSION_SCHEMA: &str = "cynic.task_admission.v1";

/// The budget a task requests, extracted from its payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEnergyRequest {
    pub wall_time_ms: u64,
    #[serde(default)]
    pub inference_tokens: Option<u64>,
    #[serde(default)]
    pub cpu_ms: Option<u64>,
    #[serde(default)]
    pub ram_bytes_s: Option<u64>,
}

impl TaskEnergyRequest {
    /// Parse from a DomainTask payload. Returns None when the payload
    /// carries no budget — V1 semantics: un-budgeted tasks admit freely.
    pub fn from_payload(payload: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(payload.get("energy_budget")?.clone()).ok()
    }

    pub fn as_budget(&self) -> crate::task_pipeline::EnergyBudget {
        crate::task_pipeline::EnergyBudget {
            wall_time_ms: self.wall_time_ms,
            inference_tokens: self.inference_tokens,
            cpu_ms: self.cpu_ms,
            ram_bytes_s: self.ram_bytes_s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum TaskAdmission {
    /// No energy_budget in payload → admitted without accounting (V1).
    AdmittedUnbudgeted,
    /// Budget present and reservable → admitted, reservation recorded.
    AdmittedWithReservation,
    /// Capacity too stale to decide honestly → caller may retry later.
    Unknown { reasons: Vec<String> },
    /// Envelope exceeded or invalid request → refused.
    Blocked { reasons: Vec<String> },
}

/// Pure admission decision for one task against current capacity state.
///
/// - `open_reservations`: budgets of windows currently open on the host.
/// - `envelope`: node-declared concurrent ceilings (None dim = unbounded).
/// - `capacity_fresh`: whether the caller judged the CapacityObservation
///   fresh under the node policy (the observation itself lives host-side;
///   this layer stays pure on the decision, not the probe).
pub fn admit_task(
    payload: &serde_json::Value,
    open_reservations: &[crate::task_pipeline::EnergyBudget],
    envelope: &crate::task_pipeline::EnergyBudget,
    capacity_fresh: bool,
) -> TaskAdmission {
    let Some(request) = TaskEnergyRequest::from_payload(payload) else {
        return TaskAdmission::AdmittedUnbudgeted;
    };

    if !capacity_fresh {
        return TaskAdmission::Unknown {
            reasons: vec!["capacity_observation_stale".to_string()],
        };
    }

    let req = request.as_budget();
    if req.wall_time_ms == 0 {
        return TaskAdmission::Blocked {
            reasons: vec!["wall_time_budget_zero".to_string()],
        };
    }

    // Sum open reservations per additive dimension (wall time is bounded by
    // the envelope via fits_within, not summed across concurrent tasks).
    let mut reserved = [None; 3];
    for open in open_reservations {
        for (i, dim) in [open.inference_tokens, open.cpu_ms, open.ram_bytes_s]
            .into_iter()
            .enumerate()
        {
            reserved[i] = match (reserved[i], dim) {
                (None, None) => None,
                (Some(x), None) | (None, Some(x)) => Some(x),
                (Some(x), Some(y)) => Some(x.saturating_add(y)),
            };
        }
    }

    let request_dims = [req.inference_tokens, req.cpu_ms, req.ram_bytes_s];
    let names = ["inference_tokens", "cpu_ms", "ram_bytes_s"];
    let mut blocked = Vec::new();
    for i in 0..3 {
        let limit = envelope_dim(envelope, i);
        if let (Some(requested), Some(limit)) = (request_dims[i], limit) {
            let total = reserved[i].unwrap_or(0).saturating_add(requested);
            if total > limit {
                blocked.push(format!("envelope_exceeded:{}", names[i]));
            }
        }
    }

    if !blocked.is_empty() {
        return TaskAdmission::Blocked { reasons: blocked };
    }
    TaskAdmission::AdmittedWithReservation
}

fn envelope_dim(envelope: &crate::task_pipeline::EnergyBudget, i: usize) -> Option<u64> {
    match i {
        0 => envelope.inference_tokens,
        1 => envelope.cpu_ms,
        _ => envelope.ram_bytes_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope(tokens: u64) -> crate::task_pipeline::EnergyBudget {
        crate::task_pipeline::EnergyBudget {
            wall_time_ms: u64::MAX,
            inference_tokens: Some(tokens),
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    fn payload_with_budget(tokens: Option<u64>) -> serde_json::Value {
        json!({
            "energy_budget": {
                "wall_time_ms": 5_000,
                "inference_tokens": tokens,
            }
        })
    }

    #[test]
    fn unbudgeted_payload_admits_freely() {
        assert_eq!(
            admit_task(&json!({"any": "task"}), &[], &envelope(0), false),
            TaskAdmission::AdmittedUnbudgeted
        );
    }

    #[test]
    fn stale_capacity_is_unknown_even_with_room() {
        assert_eq!(
            admit_task(&payload_with_budget(Some(10)), &[], &envelope(100), false),
            TaskAdmission::Unknown {
                reasons: vec!["capacity_observation_stale".to_string()]
            }
        );
    }

    #[test]
    fn fresh_capacity_with_room_admits_with_reservation() {
        assert_eq!(
            admit_task(&payload_with_budget(Some(10)), &[], &envelope(100), true),
            TaskAdmission::AdmittedWithReservation
        );
    }

    #[test]
    fn reservations_accumulate_before_envelope_check() {
        let open = vec![
            crate::task_pipeline::EnergyBudget {
                wall_time_ms: 1_000,
                inference_tokens: Some(60),
                cpu_ms: None,
                ram_bytes_s: None,
            },
            crate::task_pipeline::EnergyBudget {
                wall_time_ms: 1_000,
                inference_tokens: Some(30),
                cpu_ms: None,
                ram_bytes_s: None,
            },
        ];
        // 60 + 30 + 20 = 110 > 100 → blocked with the dimension named.
        assert_eq!(
            admit_task(&payload_with_budget(Some(20)), &open, &envelope(100), true),
            TaskAdmission::Blocked {
                reasons: vec!["envelope_exceeded:inference_tokens".to_string()]
            }
        );
        // 60 + 30 + 10 = 100 ≤ 100 → fits exactly.
        assert_eq!(
            admit_task(&payload_with_budget(Some(10)), &open, &envelope(100), true),
            TaskAdmission::AdmittedWithReservation
        );
    }

    #[test]
    fn zero_wall_budget_is_blocked() {
        let payload = json!({
            "energy_budget": { "wall_time_ms": 0 }
        });
        assert_eq!(
            admit_task(&payload, &[], &envelope(100), true),
            TaskAdmission::Blocked {
                reasons: vec!["wall_time_budget_zero".to_string()]
            }
        );
    }
}
