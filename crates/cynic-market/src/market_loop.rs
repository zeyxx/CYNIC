//! Market loop — proposal → prediction → resolution → scoring.

use crate::registry::MarketRegistry;
use crate::scoring::ScoringEngine;
use crate::types::*;

/// Market loop engine.
pub struct MarketLoop {
    scoring: ScoringEngine,
}

impl MarketLoop {
    pub fn new() -> Self {
        Self {
            scoring: ScoringEngine::new(),
        }
    }

    /// Create a proposal and register it.
    pub fn submit_proposal(
        &self,
        registry: &mut MarketRegistry,
        title: String,
        description: String,
        scale: ProposalScale,
        proposed_by: String,
        metrics: Vec<TargetMetric>,
    ) -> Result<ProposalId, String> {
        let proposal_id = format!("prop_{}", registry.proposals.len() + 1);
        let proposal = Proposal {
            schema: "cynic.market_proposal.v1".to_string(),
            proposal_id: proposal_id.clone(),
            proposed_by,
            proposed_at_unix_s: now_unix_s(),
            title,
            description,
            scale,
            blast_radius: BlastRadius {
                scope: "proposal".to_string(),
                targets: vec![],
                downstream: vec![],
                estimated_crates: 0,
            },
            observation_window: ObservationWindow {
                estimated_days: 30,
                min_days: 7,
                max_days: 90,
                velocity_basis: "default".to_string(),
            },
            target_metrics: metrics,
            axioms: vec![],
            admission_level: 1,
            admission_status: AdmissionStatus::Pending,
            value_frame: "frame_debt_first".to_string(),
        };
        registry.submit_proposal(proposal)?;
        Ok(proposal_id)
    }

    /// Submit a prediction (binary yes/no on a metric).
    pub fn submit_prediction(
        &self,
        registry: &mut MarketRegistry,
        proposal_id: &str,
        actor_id: &str,
        metric: &str,
        predicts_yes: bool,
        confidence: f64,
        rationale: &str,
    ) -> Result<PredictionId, String> {
        let prediction_id = format!("pred_{}_{}_{}", proposal_id, actor_id, metric);
        let prediction = Prediction {
            schema: "cynic.market_prediction.v1".to_string(),
            prediction_id: prediction_id.clone(),
            proposal_id: proposal_id.to_string(),
            actor_id: actor_id.to_string(),
            predicted_at_unix_s: now_unix_s(),
            metric: metric.to_string(),
            predicted_value: if predicts_yes { "yes" } else { "no" }.to_string(),
            confidence,
            rationale: rationale.to_string(),
            observation_window_days: 30,
        };
        registry.submit_prediction(prediction)?;
        Ok(prediction_id)
    }

    /// Resolve a prediction against ground truth.
    pub fn resolve_prediction(
        &self,
        registry: &mut MarketRegistry,
        prediction_id: &str,
        actual_value: &str,
    ) -> Result<ResolutionId, String> {
        let prediction = registry
            .predictions
            .get(prediction_id)
            .ok_or_else(|| format!("prediction {} not found", prediction_id))?
            .clone();

        // Error = 0 if prediction matches actual, 1 if not
        let error = if prediction.predicted_value == actual_value {
            0.0
        } else {
            1.0
        };

        // Authority delta: positive if correct, negative if wrong
        let authority_delta = if error < 0.5 {
            (1.0 - error) * 0.05
        } else {
            -(error - 0.5) * 0.1
        };

        let resolution_id = format!("res_{}", prediction_id);
        let resolution = Resolution {
            schema: "cynic.market_resolution.v1".to_string(),
            resolution_id: resolution_id.clone(),
            prediction_id: prediction_id.to_string(),
            resolved_at_unix_s: now_unix_s(),
            actual_value: actual_value.to_string(),
            predicted_value: prediction.predicted_value.clone(),
            error,
            authority_delta,
            proof_ref: "manual_resolution".to_string(),
            contested: false,
            contestation_ref: None,
        };

        registry.record_resolution(resolution)?;

        // Update actor authority
        if let Some(actor) = registry.get_actor_mut(&prediction.actor_id) {
            actor.authority = (actor.authority + authority_delta).clamp(0.0, 1.0);
            if error < 0.5 {
                actor.record_success(1.0);
            } else {
                actor.record_failure();
            }
        }

        Ok(resolution_id)
    }

    /// Get all predictions for a proposal.
    pub fn get_predictions(&self, registry: &MarketRegistry, proposal_id: &str) -> Vec<Prediction> {
        registry
            .predictions_for_proposal(proposal_id)
            .iter()
            .filter_map(|id| registry.predictions.get(id).cloned())
            .collect()
    }

    /// Calculate consensus (fraction of yes predictions) for a proposal.
    pub fn calculate_consensus(&self, registry: &MarketRegistry, proposal_id: &str) -> Option<f64> {
        let predictions = self.get_predictions(registry, proposal_id);
        if predictions.is_empty() {
            return None;
        }
        let yes_count = predictions
            .iter()
            .filter(|p| p.predicted_value == "yes")
            .count();
        Some(yes_count as f64 / predictions.len() as f64)
    }

    /// Scoring access.
    pub fn scoring(&self) -> &ScoringEngine {
        &self.scoring
    }
}

impl Default for MarketLoop {
    fn default() -> Self {
        Self::new()
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_lifecycle() {
        let mut registry = MarketRegistry::new();
        registry
            .register_actor(Actor::new("actor1".to_string(), ActorKind::Llm))
            .unwrap();

        let market = MarketLoop::new();
        let proposal_id = market
            .submit_proposal(
                &mut registry,
                "Test Proposal".to_string(),
                "Test Description".to_string(),
                ProposalScale::Material,
                "operator".to_string(),
                vec![],
            )
            .unwrap();

        let pred_id = market
            .submit_prediction(
                &mut registry,
                &proposal_id,
                "actor1",
                "debt_violations",
                true,
                0.7,
                "Test rationale",
            )
            .unwrap();

        let _res_id = market
            .resolve_prediction(&mut registry, &pred_id, "yes")
            .unwrap();

        let actor = registry.get_actor("actor1").unwrap();
        assert!(actor.authority > 0.0);
    }

    #[test]
    fn resolve_wrong_prediction() {
        let mut registry = MarketRegistry::new();
        registry
            .register_actor(Actor::new("actor1".to_string(), ActorKind::Llm))
            .unwrap();

        let market = MarketLoop::new();
        let proposal_id = market
            .submit_proposal(
                &mut registry,
                "Test".to_string(),
                "Test".to_string(),
                ProposalScale::Material,
                "operator".to_string(),
                vec![],
            )
            .unwrap();

        let pred_id = market
            .submit_prediction(
                &mut registry,
                &proposal_id,
                "actor1",
                "test",
                true,
                0.8,
                "rationale",
            )
            .unwrap();

        market
            .resolve_prediction(&mut registry, &pred_id, "no")
            .unwrap();

        let actor = registry.get_actor("actor1").unwrap();
        assert_eq!(actor.predictions_total, 1);
        assert_eq!(actor.predictions_correct, 0);
    }

    #[test]
    fn consensus_calculation() {
        let mut registry = MarketRegistry::new();
        registry
            .register_actor(Actor::new("actor1".to_string(), ActorKind::Llm))
            .unwrap();
        registry
            .register_actor(Actor::new("actor2".to_string(), ActorKind::Llm))
            .unwrap();
        registry
            .register_actor(Actor::new("actor3".to_string(), ActorKind::Llm))
            .unwrap();

        let market = MarketLoop::new();
        let proposal_id = market
            .submit_proposal(
                &mut registry,
                "Test".to_string(),
                "Test".to_string(),
                ProposalScale::Material,
                "operator".to_string(),
                vec![],
            )
            .unwrap();

        market
            .submit_prediction(
                &mut registry,
                &proposal_id,
                "actor1",
                "test",
                true,
                0.7,
                "rationale",
            )
            .unwrap();
        market
            .submit_prediction(
                &mut registry,
                &proposal_id,
                "actor2",
                "test",
                true,
                0.6,
                "rationale",
            )
            .unwrap();
        market
            .submit_prediction(
                &mut registry,
                &proposal_id,
                "actor3",
                "test",
                false,
                0.8,
                "rationale",
            )
            .unwrap();

        let consensus = market.calculate_consensus(&registry, &proposal_id).unwrap();
        assert!((consensus - 0.6667).abs() < 0.01);
    }
}
