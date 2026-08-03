use super::{
    decode_transaction, migrate_odysseum_json_bytes, DecodedTransaction, PumpProfileRegistry,
    SolanaObservation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservedTrace {
    pub observation: SolanaObservation,
    pub decoded: DecodedTransaction,
    pub status: TraceStatus,
}

pub fn trace_legacy_json_bytes(
    bytes: &[u8],
    registry: &PumpProfileRegistry,
) -> Result<ObservedTrace, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let observation = migrate_odysseum_json_bytes(bytes).map_err(|error| format!("{error:?}"))?;
    let decoded = decode_transaction(registry, &value);
    let status = if decoded.instructions.is_empty() {
        TraceStatus::Unknown
    } else if decoded.unresolved_paths.is_empty() {
        TraceStatus::Complete
    } else {
        TraceStatus::Partial
    };
    Ok(ObservedTrace {
        observation,
        decoded,
        status,
    })
}

pub fn trace_legacy_json_file(
    path: impl AsRef<Path>,
    registry: &PumpProfileRegistry,
) -> Result<ObservedTrace, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    trace_legacy_json_bytes(&bytes, registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{InstructionDecodeStatus, PUMP_PROGRAM_ID};
    use serde_json::json;

    #[test]
    fn trace_connects_legacy_observation_to_decoded_instruction() {
        let value = json!({
            "observation": {"signature": "sig", "slot": 10},
            "transaction": {
                "transaction": {
                    "message": {
                        "accountKeys": [{"pubkey": PUMP_PROGRAM_ID}],
                        "instructions": [{
                            "programIdIndex": 0,
                            "parsed": {"type": "buy_v2"}
                        }]
                    }
                }
            }
        });
        let bytes = serde_json::to_vec(&value).expect("fixture serializes");
        let trace = trace_legacy_json_bytes(&bytes, &PumpProfileRegistry::canonical())
            .expect("fixture traces");
        assert_eq!(trace.observation.signature, "sig");
        assert_eq!(trace.decoded.instructions.len(), 1);
        assert_eq!(trace.status, TraceStatus::Complete);
        assert_eq!(
            trace.decoded.instructions[0].status,
            InstructionDecodeStatus::Classified
        );
    }

    #[test]
    fn trace_marks_unknown_instruction_as_partial() {
        let value = json!({
            "observation": {"signature": "sig", "slot": 10},
            "transaction": {
                "transaction": {
                    "message": {
                        "instructions": [{"programId": PUMP_PROGRAM_ID, "name": "future"}]
                    }
                }
            }
        });
        let bytes = serde_json::to_vec(&value).expect("fixture serializes");
        let trace = trace_legacy_json_bytes(&bytes, &PumpProfileRegistry::canonical())
            .expect("fixture traces");
        assert_eq!(trace.status, TraceStatus::Partial);
    }
}
