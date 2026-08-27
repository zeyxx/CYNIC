use crate::domain::backend_identity::{BackendCapability, BackendHealthSnapshot};
use crate::domain::routing::{RoutingConstraint, RoutingDecision, RoutingIntent};

/// Selects an optimal available backend adhering to routing constraints,
/// with automatic failover if the requested primary model is degraded or down.
pub fn select_backend(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    let requires_sovereign = intent
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, RoutingConstraint::SovereignOnly));

    let requires_tool_use = intent
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, RoutingConstraint::ToolUseRequired));

    let requires_json_mode = intent
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, RoutingConstraint::JsonModeRequired));

    let requested_model = intent.constraints.iter().find_map(|c| match c {
        RoutingConstraint::ModelName(name) => Some(name.as_str()),
        _ => None,
    });

    let satisfies_capabilities = |snapshot: &BackendHealthSnapshot| {
        let caps = &snapshot.identity.capabilities;
        let meets_sovereign = !requires_sovereign || snapshot.identity.sovereign;
        let meets_tool_use = !requires_tool_use || caps.contains(&BackendCapability::ToolUse);
        let meets_json_mode = !requires_json_mode || caps.contains(&BackendCapability::JsonMode);
        meets_sovereign && meets_tool_use && meets_json_mode
    };

    // 1. If a specific model was requested, attempt exact match first
    if let Some(requested) = requested_model {
        if let Some(exact_match) = backends.iter().find(|b| {
            (b.identity.backend_name == requested
                || b.identity.model_name.as_deref() == Some(requested))
                && b.status.is_available()
                && satisfies_capabilities(b)
        }) {
            return Some(RoutingDecision {
                backend_name: exact_match.identity.backend_name.clone(),
                reason: format!("requested_model_exact_match:{requested}"),
            });
        }

        // 2. Primary model requested but unavailable -> Trigger automatic failover
        if let Some(fallback) = backends
            .iter()
            .find(|b| b.status.is_available() && satisfies_capabilities(b))
        {
            let is_gpu_oom = requested.contains("gpu") || requested.contains("vram");
            let failover_reason = if is_gpu_oom
                && (fallback.identity.backend_name.contains("cpu")
                    || fallback.identity.display_name.contains("CPU"))
            {
                format!("gpu_oom_cpu_fallback_selected:primary_{requested}_down")
            } else {
                format!(
                    "primary_model_unavailable_fallback_selected:requested_{requested}_to_{}",
                    fallback.identity.backend_name
                )
            };

            return Some(RoutingDecision {
                backend_name: fallback.identity.backend_name.clone(),
                reason: failover_reason,
            });
        }

        return None;
    }

    // 3. General selection: first available backend matching capability constraints
    backends
        .iter()
        .find(|b| b.status.is_available() && satisfies_capabilities(b))
        .map(|snapshot| RoutingDecision {
            backend_name: snapshot.identity.backend_name.clone(),
            reason: "first_available_backend".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::backend_identity::{BackendCapability, BackendIdentity};
    use crate::domain::inference::BackendStatus;
    use crate::domain::routing::InferenceClass;

    fn make_snapshot(
        name: &str,
        model: &str,
        sovereign: bool,
        caps: Vec<BackendCapability>,
        status: BackendStatus,
    ) -> BackendHealthSnapshot {
        BackendHealthSnapshot {
            identity: BackendIdentity {
                backend_name: name.to_string(),
                display_name: name.to_string(),
                model_name: Some(model.to_string()),
                sovereign,
                capabilities: caps,
            },
            status,
        }
    }

    #[test]
    fn test_primary_model_down_fallback_selected() {
        let backends = vec![
            make_snapshot(
                "primary-qwen-gpu",
                "qwen35-9b",
                true,
                vec![BackendCapability::Chat, BackendCapability::ToolUse],
                BackendStatus::Critical, // primary is down
            ),
            make_snapshot(
                "fallback-qwen-backup",
                "qwen3-4b",
                true,
                vec![BackendCapability::Chat, BackendCapability::ToolUse],
                BackendStatus::Healthy, // fallback is healthy
            ),
        ];

        let intent = RoutingIntent {
            class: InferenceClass::ToolUse,
            constraints: vec![
                RoutingConstraint::ModelName("primary-qwen-gpu".to_string()),
                RoutingConstraint::ToolUseRequired,
            ],
        };

        let decision = select_backend(&backends, &intent).expect("failover should succeed");
        assert_eq!(decision.backend_name, "fallback-qwen-backup");
        assert!(decision.reason.contains("primary_model_unavailable_fallback_selected"));
    }

    #[test]
    fn test_gpu_oom_cpu_fallback_selected() {
        let backends = vec![
            make_snapshot(
                "qwen35-gpu-high-vram",
                "qwen35-35b-gpu",
                true,
                vec![BackendCapability::Chat],
                BackendStatus::Critical, // GPU OOM crash
            ),
            make_snapshot(
                "qwen35-cpu-fallback",
                "qwen35-35b-cpu",
                true,
                vec![BackendCapability::Chat],
                BackendStatus::Healthy, // CPU backend is up
            ),
        ];

        let intent = RoutingIntent {
            class: InferenceClass::General,
            constraints: vec![RoutingConstraint::ModelName("qwen35-gpu-high-vram".to_string())],
        };

        let decision = select_backend(&backends, &intent).expect("CPU failover should succeed");
        assert_eq!(decision.backend_name, "qwen35-cpu-fallback");
        assert!(decision.reason.contains("gpu_oom_cpu_fallback_selected"));
    }

    #[test]
    fn test_tool_use_constraint_respected_during_failover() {
        let backends = vec![
            make_snapshot(
                "primary-model",
                "model-a",
                true,
                vec![BackendCapability::Chat, BackendCapability::ToolUse],
                BackendStatus::Critical, // down
            ),
            make_snapshot(
                "text-only-model",
                "model-b",
                true,
                vec![BackendCapability::Chat], // lacks tool use
                BackendStatus::Healthy,
            ),
            make_snapshot(
                "tool-capable-model",
                "model-c",
                true,
                vec![BackendCapability::Chat, BackendCapability::ToolUse],
                BackendStatus::Healthy,
            ),
        ];

        let intent = RoutingIntent {
            class: InferenceClass::ToolUse,
            constraints: vec![
                RoutingConstraint::ModelName("primary-model".to_string()),
                RoutingConstraint::ToolUseRequired,
            ],
        };

        let decision = select_backend(&backends, &intent).expect("should select tool capable fallback");
        assert_eq!(decision.backend_name, "tool-capable-model");
    }
}
