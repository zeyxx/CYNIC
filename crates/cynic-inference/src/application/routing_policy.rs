use cynic_workload::DataClassification;

use crate::domain::backend_identity::{BackendHealthSnapshot, BackendIdentity};
use crate::domain::routing::{RoutingConstraint, RoutingDecision, RoutingIntent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyRoutingError {
    NonSovereignBackendForPrivateData {
        backend_name: String,
        classification: DataClassification,
    },
}

/// Pure rule: validates whether an inference backend is admissible for a given intent's data classification.
/// Private and Sensitive data must never be routed to non-sovereign backends (e.g. remote SaaS/OpenAI).
pub fn validate_privacy_admissibility(
    intent: &RoutingIntent,
    backend: &BackendIdentity,
) -> Result<(), PrivacyRoutingError> {
    for constraint in &intent.constraints {
        if let RoutingConstraint::Classification(class) = constraint {
            if matches!(
                class,
                DataClassification::Private | DataClassification::Sensitive
            ) && !backend.sovereign
            {
                return Err(PrivacyRoutingError::NonSovereignBackendForPrivateData {
                    backend_name: backend.backend_name.clone(),
                    classification: *class,
                });
            }
        }
    }
    Ok(())
}

pub fn select_backend(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    let requires_sovereign = intent
        .constraints
        .iter()
        .any(|constraint| match constraint {
            RoutingConstraint::SovereignOnly => true,
            RoutingConstraint::Classification(DataClassification::Private)
            | RoutingConstraint::Classification(DataClassification::Sensitive) => true,
            _ => false,
        });

    backends
        .iter()
        .find(|snapshot| {
            snapshot.status.is_available()
                && (!requires_sovereign || snapshot.identity.sovereign)
                && validate_privacy_admissibility(intent, &snapshot.identity).is_ok()
        })
        .map(|snapshot| RoutingDecision {
            backend_name: snapshot.identity.backend_name.clone(),
            reason: if requires_sovereign {
                "sovereign_backend_enforced_by_privacy_classification".to_string()
            } else {
                "first_available_backend".to_string()
            },
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::backend_identity::{
        BackendCapability, BackendHealthSnapshot, BackendIdentity,
    };
    use crate::domain::inference::BackendStatus;
    use crate::domain::routing::{InferenceClass, RoutingConstraint, RoutingIntent};
    fn sovereign_backend() -> BackendHealthSnapshot {
        BackendHealthSnapshot {
            identity: BackendIdentity {
                backend_name: "llama-local".to_string(),
                display_name: "Local Llama".to_string(),
                model_name: Some("qwen2.5-coder".to_string()),
                sovereign: true,
                capabilities: vec![BackendCapability::Chat],
            },
            status: BackendStatus::Healthy,
        }
    }

    fn remote_backend() -> BackendHealthSnapshot {
        BackendHealthSnapshot {
            identity: BackendIdentity {
                backend_name: "openai-remote".to_string(),
                display_name: "OpenAI API".to_string(),
                model_name: Some("gpt-4o".to_string()),
                sovereign: false,
                capabilities: vec![BackendCapability::Chat],
            },
            status: BackendStatus::Healthy,
        }
    }

    #[test]
    fn public_intent_can_route_to_remote_or_sovereign() {
        let backends = vec![remote_backend(), sovereign_backend()];
        let intent = RoutingIntent {
            class: InferenceClass::General,
            constraints: vec![RoutingConstraint::Classification(
                DataClassification::Public,
            )],
        };
        let decision = select_backend(&backends, &intent).expect("routing decision");
        assert_eq!(decision.backend_name, "openai-remote");
    }

    #[test]
    fn private_and_sensitive_intent_strictly_rejects_remote_backend() {
        let remote = remote_backend();
        let local = sovereign_backend();

        let private_intent = RoutingIntent {
            class: InferenceClass::General,
            constraints: vec![RoutingConstraint::Classification(
                DataClassification::Private,
            )],
        };

        // Only remote available -> returns None (refuses leakage)
        assert!(select_backend(&[remote.clone()], &private_intent).is_none());

        // Both available -> routes strictly to local sovereign backend
        let decision = select_backend(&[remote.clone(), local.clone()], &private_intent)
            .expect("routes to sovereign");
        assert_eq!(decision.backend_name, "llama-local");
        assert_eq!(
            decision.reason,
            "sovereign_backend_enforced_by_privacy_classification"
        );

        // Direct admissibility validation
        assert!(validate_privacy_admissibility(&private_intent, &local.identity).is_ok());
        assert!(matches!(
            validate_privacy_admissibility(&private_intent, &remote.identity),
            Err(PrivacyRoutingError::NonSovereignBackendForPrivateData { .. })
        ));
    }
}
