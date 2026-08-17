use serde::{Deserialize, Serialize};

pub const RUNTIME_CYCLE_ENVELOPE_SCHEMA: &str = "cynic.runtime.cycle_envelope.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePlaneLayer {
    Substrate,
    Inference,
    Agent,
    Coordination,
    Membrane,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeCycleEnvelope {
    pub schema: String,
    pub correlation_id: String,
    pub layer: RuntimePlaneLayer,
    pub direction: String,
    pub kind: String,
    pub status: String,
}

impl RuntimeCycleEnvelope {
    pub fn new(
        correlation_id: impl Into<String>,
        layer: RuntimePlaneLayer,
        direction: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            schema: RUNTIME_CYCLE_ENVELOPE_SCHEMA.into(),
            correlation_id: correlation_id.into(),
            layer,
            direction: direction.into(),
            kind: kind.into(),
            status: "observed".into(),
        }
    }
}
