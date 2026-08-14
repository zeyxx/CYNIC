//! Application surface for inference runtime operations on nodes.
//!
//! Low-level command/probe adapters live in `inference_node_ops`. This module
//! gives callers a stable operation vocabulary without coupling them to adapter
//! function names.

use crate::inference_node_ops::{ModelProbeResult, attempt_node_recovery, attempt_node_start};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogOperationOutcome {
    pub node: String,
    pub status: String,
}

pub async fn start_dog_runtime(node: &str) -> DogOperationOutcome {
    DogOperationOutcome {
        node: node.to_string(),
        status: attempt_node_start(node).await,
    }
}

pub async fn remediate_dog_runtime(node: &str) -> DogOperationOutcome {
    DogOperationOutcome {
        node: node.to_string(),
        status: attempt_node_recovery(node).await,
    }
}

pub async fn probe_model_runtime(node: &str, expected_model: &str) -> ModelProbeResult {
    crate::inference_node_ops::probe_node_model(node, expected_model).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dog_operation_outcome_carries_node_and_status() {
        let outcome = DogOperationOutcome {
            node: "node-a".to_string(),
            status: "succeeded".to_string(),
        };
        assert_eq!(outcome.node, "node-a");
        assert_eq!(outcome.status, "succeeded");
    }
}
