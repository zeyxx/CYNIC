//! Core market types.

use serde::{Deserialize, Serialize};

/// Stable schema identifier for market actor.
pub const MARKET_ACTOR_SCHEMA: &str = "cynic.market_actor.v1";
pub const MARKET_PROPOSAL_SCHEMA: &str = "cynic.market_proposal.v1";
pub const MARKET_PREDICTION_SCHEMA: &str = "cynic.market_prediction.v1";
pub const MARKET_RESOLUTION_SCHEMA: &str = "cynic.market_resolution.v1";

/// Unique actor identifier.
pub type ActorId = String;
pub type ProposalId = String;
pub type PredictionId = String;
pub type ResolutionId = String;

/// Actor kind — determines trust mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    /// Large Language Model (Hermes, Codex, etc.)
    Llm,
    /// Deterministic algorithm (GC, pattern, scheduler).
    Algo,
    /// Statistical ML model (embedding, classifier).
    Ml,
    /// External oracle (Polymarket, Pyth, arxiv).
    Oracle,
    /// Pure system (hash, fs, build).
    System,
    /// Human behavior model (cynic-askesis).
    Askesis,
    /// Human operator (sovereign, veto power).
    Human,
}

/// Actor state — probationary period protects against Sybil/gaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorState {
    /// New actor (< 20 successful resolutions).
    /// Can predict but cannot execute level 2+ actions alone.
    Probationary,
    /// Active (>= 20 successful resolutions).
    /// Full prediction + execution capability.
    Active,
    /// Declining (3+ months without activity).
    /// Authority decays, requires read-back to recover.
    Declining,
    /// Suspended (contestation in progress).
    /// Predictions frozen, resolutions queued.
    Suspended,
}

/// A market actor — an entity that predicts consequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    #[serde(default = "default_schema")]
    pub schema: String,
    pub actor_id: ActorId,
    pub kind: ActorKind,
    pub state: ActorState,
    /// Authority score 0..1 (derived, never assigned).
    pub authority: f64,
    /// Calibration: total predictions made.
    pub predictions_total: u32,
    /// Calibration: predictions that matched reality.
    pub predictions_correct: u32,
    /// Weighted sum of blast radii for predictions made.
    pub blast_radius_weighted: f64,
    /// Crystal IDs associated with this actor's knowledge.
    pub crystal_ids: Vec<String>,
    /// First activity timestamp.
    pub first_active_unix_s: u64,
    /// Last activity timestamp.
    pub last_active_unix_s: u64,
    /// Decay rate per month of inactivity.
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f64,
    /// Probation config.
    #[serde(default)]
    pub probation: ProbationConfig,
}

fn default_schema() -> String {
    MARKET_ACTOR_SCHEMA.to_string()
}

fn default_decay_rate() -> f64 {
    0.01
}

/// Probation mechanism for new actors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbationConfig {
    pub resolutions_needed: u32,
    pub resolutions_completed: u32,
    pub promoted_at_unix_s: Option<u64>,
}

impl Default for ProbationConfig {
    fn default() -> Self {
        Self {
            resolutions_needed: 20,
            resolutions_completed: 0,
            promoted_at_unix_s: None,
        }
    }
}

impl Actor {
    pub fn new(actor_id: ActorId, kind: ActorKind) -> Self {
        let now = now_unix_s();
        Self {
            schema: default_schema(),
            actor_id,
            kind,
            state: ActorState::Probationary,
            authority: 0.0,
            predictions_total: 0,
            predictions_correct: 0,
            blast_radius_weighted: 0.0,
            crystal_ids: Vec::new(),
            first_active_unix_s: now,
            last_active_unix_s: now,
            decay_rate: default_decay_rate(),
            probation: ProbationConfig::default(),
        }
    }

    /// Check if actor is in probation.
    pub fn is_probationary(&self) -> bool {
        self.state == ActorState::Probationary
    }

    /// Record a successful resolution (may trigger promotion).
    pub fn record_success(&mut self, blast_radius: f64) {
        self.predictions_total += 1;
        self.predictions_correct += 1;
        self.blast_radius_weighted += blast_radius;
        self.last_active_unix_s = now_unix_s();

        if self.state == ActorState::Probationary {
            self.probation.resolutions_completed += 1;
            if self.probation.resolutions_completed >= self.probation.resolutions_needed {
                self.state = ActorState::Active;
                self.probation.promoted_at_unix_s = Some(now_unix_s());
            }
        }
    }

    /// Record a failed resolution.
    pub fn record_failure(&mut self) {
        self.predictions_total += 1;
        self.last_active_unix_s = now_unix_s();
    }

    /// Apply decay for inactivity.
    pub fn apply_decay(&mut self) {
        let months = months_since(self.last_active_unix_s);
        if months >= 3 {
            self.state = ActorState::Declining;
            self.authority *= (1.0 - self.decay_rate).powi(months as i32);
        }
    }
}

/// A proposal — an action whose consequences are to be predicted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    #[serde(default = "default_proposal_schema")]
    pub schema: String,
    pub proposal_id: ProposalId,
    pub proposed_by: String,
    pub proposed_at_unix_s: u64,
    pub title: String,
    pub description: String,
    pub scale: ProposalScale,
    pub blast_radius: BlastRadius,
    pub observation_window: ObservationWindow,
    pub target_metrics: Vec<TargetMetric>,
    pub axioms: Vec<String>,
    pub admission_level: u8,
    pub admission_status: AdmissionStatus,
    pub value_frame: String,
}

fn default_proposal_schema() -> String {
    MARKET_PROPOSAL_SCHEMA.to_string()
}

/// Scale of the proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalScale {
    /// Material (extraction, deployment, refactor).
    Material,
    /// Projection (publication, research, sentiment).
    Projection,
    /// Economic (token, stake, financial).
    Economic,
}

/// Blast radius — scope of impact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    pub scope: String,
    pub targets: Vec<String>,
    pub downstream: Vec<String>,
    pub estimated_crates: u32,
}

/// Observation window — time to verify consequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationWindow {
    pub estimated_days: u32,
    pub min_days: u32,
    pub max_days: u32,
    pub velocity_basis: String,
}

/// A target metric to be measured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetric {
    pub name: String,
    pub target: String,
    pub proof: String,
}

/// Admission status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionStatus {
    Pending,
    Approved,
    Rejected,
    Contested,
}

/// A prediction — an actor's bet on a proposal's consequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    #[serde(default = "default_prediction_schema")]
    pub schema: String,
    pub prediction_id: PredictionId,
    pub proposal_id: ProposalId,
    pub actor_id: ActorId,
    pub predicted_at_unix_s: u64,
    pub metric: String,
    pub predicted_value: String,
    pub confidence: f64,
    pub rationale: String,
    pub observation_window_days: u32,
}

fn default_prediction_schema() -> String {
    MARKET_PREDICTION_SCHEMA.to_string()
}

/// A resolution — the outcome of a prediction vs reality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    #[serde(default = "default_resolution_schema")]
    pub schema: String,
    pub resolution_id: ResolutionId,
    pub prediction_id: PredictionId,
    pub resolved_at_unix_s: u64,
    pub actual_value: String,
    pub predicted_value: String,
    pub error: f64,
    pub authority_delta: f64,
    pub proof_ref: String,
    pub contested: bool,
    pub contestation_ref: Option<String>,
}

fn default_resolution_schema() -> String {
    MARKET_RESOLUTION_SCHEMA.to_string()
}

/// Market event for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketEvent {
    ActorCreated { actor_id: ActorId },
    ActorPromoted { actor_id: ActorId },
    ProposalCreated { proposal_id: ProposalId },
    PredictionSubmitted { prediction_id: PredictionId },
    ResolutionCompleted { resolution_id: ResolutionId },
    CollusionDetected { actor_ids: Vec<ActorId> },
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn months_since(unix_s: u64) -> u32 {
    let now = now_unix_s();
    let seconds_per_month = 30 * 24 * 3600;
    ((now - unix_s) / seconds_per_month).max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_starts_probationary() {
        let actor = Actor::new("test-llm".to_string(), ActorKind::Llm);
        assert_eq!(actor.state, ActorState::Probationary);
        assert_eq!(actor.authority, 0.0);
        assert!(actor.is_probationary());
    }

    #[test]
    fn actor_promotes_after_20_successes() {
        let mut actor = Actor::new("test-llm".to_string(), ActorKind::Llm);
        for _ in 0..20 {
            actor.record_success(1.0);
        }
        assert_eq!(actor.state, ActorState::Active);
        assert!(!actor.is_probationary());
        assert_eq!(actor.probation.promoted_at_unix_s.is_some(), true);
    }

    #[test]
    fn actor_decay_after_3_months() {
        let mut actor = Actor::new("test-llm".to_string(), ActorKind::Llm);
        actor.authority = 0.8;
        actor.last_active_unix_s = now_unix_s() - (4 * 30 * 24 * 3600); // 4 months ago
        actor.apply_decay();
        assert_eq!(actor.state, ActorState::Declining);
        assert!(actor.authority < 0.8);
    }

    #[test]
    fn proposal_schema_set() {
        let proposal = Proposal {
            schema: default_proposal_schema(),
            proposal_id: "prop_001".to_string(),
            proposed_by: "operator".to_string(),
            proposed_at_unix_s: now_unix_s(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            scale: ProposalScale::Material,
            blast_radius: BlastRadius {
                scope: "crate".to_string(),
                targets: vec!["cynic-kernel".to_string()],
                downstream: vec![],
                estimated_crates: 1,
            },
            observation_window: ObservationWindow {
                estimated_days: 30,
                min_days: 7,
                max_days: 90,
                velocity_basis: "test".to_string(),
            },
            target_metrics: vec![],
            axioms: vec![],
            admission_level: 1,
            admission_status: AdmissionStatus::Pending,
            value_frame: "frame_debt_first".to_string(),
        };
        assert_eq!(proposal.schema, MARKET_PROPOSAL_SCHEMA);
    }
}
