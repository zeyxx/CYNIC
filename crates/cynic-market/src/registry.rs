//! Market registry — SSOT persistence for market state.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::types::*;

/// Market registry — persistent store for all market state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRegistry {
    #[serde(default = "default_schema")]
    schema: String,
    pub actors: HashMap<ActorId, Actor>,
    pub proposals: HashMap<ProposalId, Proposal>,
    pub predictions: HashMap<PredictionId, Prediction>,
    pub resolutions: HashMap<ResolutionId, Resolution>,
    pub events: Vec<MarketEvent>,
}

fn default_schema() -> String {
    "cynic.market_registry.v1".to_string()
}

impl Default for MarketRegistry {
    fn default() -> Self {
        Self {
            schema: default_schema(),
            actors: HashMap::new(),
            proposals: HashMap::new(),
            predictions: HashMap::new(),
            resolutions: HashMap::new(),
            events: Vec::new(),
        }
    }
}

impl MarketRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from disk or create empty.
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.exists() {
            let content =
                fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
            serde_json::from_str::<Self>(&content)
                .map_err(|e| format!("parse {}: {}", path.display(), e))
        } else {
            Ok(Self::default())
        }
    }

    /// Save to disk atomically (write temp + rename).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, json).map_err(|e| format!("{}: {}", tmp.display(), e))?;
        fs::rename(&tmp, path).map_err(|e| format!("rename {}: {}", path.display(), e))?;
        Ok(())
    }

    /// Register a new actor.
    pub fn register_actor(&mut self, actor: Actor) -> Result<(), String> {
        if self.actors.contains_key(&actor.actor_id) {
            return Err(format!("actor {} already exists", actor.actor_id));
        }
        let id = actor.actor_id.clone();
        self.actors.insert(id.clone(), actor);
        self.events.push(MarketEvent::ActorCreated { actor_id: id });
        Ok(())
    }

    /// Submit a proposal.
    pub fn submit_proposal(&mut self, proposal: Proposal) -> Result<(), String> {
        if self.proposals.contains_key(&proposal.proposal_id) {
            return Err(format!("proposal {} already exists", proposal.proposal_id));
        }
        let id = proposal.proposal_id.clone();
        self.proposals.insert(id.clone(), proposal);
        self.events
            .push(MarketEvent::ProposalCreated { proposal_id: id });
        Ok(())
    }

    /// Submit a prediction.
    pub fn submit_prediction(&mut self, prediction: Prediction) -> Result<(), String> {
        if self.predictions.contains_key(&prediction.prediction_id) {
            return Err(format!(
                "prediction {} already exists",
                prediction.prediction_id
            ));
        }
        // Validate actor exists
        if !self.actors.contains_key(&prediction.actor_id) {
            return Err(format!("actor {} not found", prediction.actor_id));
        }
        // Validate proposal exists
        if !self.proposals.contains_key(&prediction.proposal_id) {
            return Err(format!("proposal {} not found", prediction.proposal_id));
        }
        let id = prediction.prediction_id.clone();
        self.predictions.insert(id.clone(), prediction);
        self.events
            .push(MarketEvent::PredictionSubmitted { prediction_id: id });
        Ok(())
    }

    /// Record a resolution.
    pub fn record_resolution(&mut self, resolution: Resolution) -> Result<(), String> {
        if self.resolutions.contains_key(&resolution.resolution_id) {
            return Err(format!(
                "resolution {} already exists",
                resolution.resolution_id
            ));
        }
        let id = resolution.resolution_id.clone();
        self.resolutions.insert(id.clone(), resolution);
        self.events
            .push(MarketEvent::ResolutionCompleted { resolution_id: id });
        Ok(())
    }

    /// Get predictions for a proposal.
    pub fn predictions_for_proposal(&self, proposal_id: &str) -> Vec<String> {
        self.predictions
            .values()
            .filter(|p| p.proposal_id == proposal_id)
            .map(|p| p.prediction_id.clone())
            .collect()
    }

    /// Get actor by ID.
    pub fn get_actor(&self, actor_id: &str) -> Option<&Actor> {
        self.actors.get(actor_id)
    }

    /// Get mutable actor.
    pub fn get_actor_mut(&mut self, actor_id: &str) -> Option<&mut Actor> {
        self.actors.get_mut(actor_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("market.json");

        let mut reg = MarketRegistry::new();
        reg.register_actor(Actor::new("test".to_string(), ActorKind::Llm))
            .unwrap();
        reg.save(&path).unwrap();

        let loaded = MarketRegistry::load(&path).unwrap();
        assert!(loaded.actors.contains_key("test"));
    }

    #[test]
    fn duplicate_actor_rejected() {
        let mut reg = MarketRegistry::new();
        reg.register_actor(Actor::new("test".to_string(), ActorKind::Llm))
            .unwrap();
        let result = reg.register_actor(Actor::new("test".to_string(), ActorKind::Llm));
        assert!(result.is_err());
    }

    #[test]
    fn prediction_requires_actor_and_proposal() {
        let mut reg = MarketRegistry::new();
        reg.register_actor(Actor::new("actor1".to_string(), ActorKind::Llm))
            .unwrap();
        // Missing proposal
        let pred = Prediction {
            schema: "cynic.market_prediction.v1".to_string(),
            prediction_id: "pred1".to_string(),
            proposal_id: "missing".to_string(),
            actor_id: "actor1".to_string(),
            predicted_at_unix_s: 0,
            metric: "test".to_string(),
            predicted_value: "0.5".to_string(),
            confidence: 0.5,
            rationale: "test".to_string(),
            observation_window_days: 30,
        };
        assert!(reg.submit_prediction(pred).is_err());
    }
}
