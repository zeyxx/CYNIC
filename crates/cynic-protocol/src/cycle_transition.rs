use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CYCLE_TRANSITION_SCHEMA: &str = "cynic.runtime.cycle_transition.v1";

/// Canonical evidence shape for every bounded runtime cycle. The payloads are
/// typed by the owning layer; this envelope keeps the transaction auditable
/// without forcing host, inference, agent, and coordination domains together.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CycleTransitionRecord {
    pub schema: String,
    pub correlation_id: String,
    pub desired: Value,
    pub observed: Value,
    pub decision: Value,
    pub action: Option<Value>,
    pub proof: Option<Value>,
    pub status: String,
}

impl CycleTransitionRecord {
    pub fn planned(
        correlation_id: impl Into<String>,
        desired: Value,
        observed: Value,
        decision: Value,
    ) -> Self {
        Self {
            schema: CYCLE_TRANSITION_SCHEMA.into(),
            correlation_id: correlation_id.into(),
            desired,
            observed,
            decision,
            action: None,
            proof: None,
            status: "planned".into(),
        }
    }

    pub fn with_action(mut self, action: Value) -> Self {
        self.action = Some(action);
        self.status = "action_bounded".into();
        self
    }

    pub fn with_proof(mut self, proof: Value) -> Self {
        self.proof = Some(proof);
        self.status = "proved".into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_requires_explicit_read_back_for_proved_state() {
        let planned = CycleTransitionRecord::planned("r1", Value::Null, Value::Null, Value::Null);
        assert_eq!(planned.status, "planned");
        let action = planned.with_action(serde_json::json!({"argv":["start"]}));
        assert_eq!(action.status, "action_bounded");
        let proved = action.with_proof(serde_json::json!({"health":"ok"}));
        assert_eq!(proved.status, "proved");
    }
}
