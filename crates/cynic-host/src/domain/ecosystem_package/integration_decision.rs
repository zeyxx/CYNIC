use super::integration_plan_types::IntegrationPlanAction;
use super::integration_types::{IntegrationObservation, IntegrationPrincipal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationDecisionSurface {
    Observe,
    Connect,
    Adopt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationDecision {
    pub action: IntegrationPlanAction,
    pub surface: IntegrationDecisionSurface,
}

pub fn decide_integration_action(observation: &IntegrationObservation) -> IntegrationDecision {
    match observation.id.as_str() {
        "github" if observation.status != "ready" => IntegrationDecision {
            action: IntegrationPlanAction::ConnectOperatorCli,
            surface: IntegrationDecisionSurface::Connect,
        },
        "huggingface"
            if observation.observed_principal == Some(IntegrationPrincipal::RuntimeUser)
                && observation.principal_matches
                && observation.status != "ready" =>
        {
            IntegrationDecision {
                action: IntegrationPlanAction::ConnectRuntimeCli,
                surface: IntegrationDecisionSurface::Connect,
            }
        }
        "huggingface" if observation.status != "ready" => IntegrationDecision {
            action: IntegrationPlanAction::AdoptRuntimeCli,
            surface: IntegrationDecisionSurface::Adopt,
        },
        _ => IntegrationDecision {
            action: IntegrationPlanAction::None,
            surface: IntegrationDecisionSurface::Observe,
        },
    }
}
