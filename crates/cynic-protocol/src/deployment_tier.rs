use serde::{Deserialize, Serialize};

pub const DEPLOYMENT_TIER_PROGRESSION_SCHEMA: &str = "cynic.deployment_tier_progression.v1";

/// Canonical deployment tiers according to `contracts/glossary.json`.
/// Replaces the historical and misnamed `RuntimeHarnessKind` and `HarnessScope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentTier {
    /// Isolated developer environment / local lab.
    LocalLab,
    /// Automated test conformance and contract validation gate.
    Conformance,
    /// Pre-production staging verification.
    Staging,
    /// Progressive fleet roll-out subset.
    Canary,
    /// Production execution environment.
    Production,
}

impl DeploymentTier {
    pub const ALL: [DeploymentTier; 5] = [
        DeploymentTier::LocalLab,
        DeploymentTier::Conformance,
        DeploymentTier::Staging,
        DeploymentTier::Canary,
        DeploymentTier::Production,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalLab => "local_lab",
            Self::Conformance => "conformance",
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
        }
    }

    pub fn parse_flexible(input: &str) -> Option<Self> {
        let normalized = input.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "locallab" | "local_lab" | "closedlocallab" | "closed_local_lab" | "observe"
            | "lab" => Some(Self::LocalLab),
            "conformance" | "conformance_gate" => Some(Self::Conformance),
            "staging" | "release_candidate" => Some(Self::Staging),
            "canary" | "fleet_canary" => Some(Self::Canary),
            "production" | "runtime_authority" | "prod" => Some(Self::Production),
            _ => None,
        }
    }
}

/// Backwards compatibility alias for the legacy `RuntimeHarnessKind`.
pub type RuntimeHarnessKind = DeploymentTier;

/// Required proof categories for environment tier progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofCategory {
    /// Bootstrap contracts satisfied (security, execution, observation).
    Bootstrap,
    /// Hardware baseline matches declared profile.
    Hardware,
    /// Host conformance report is green.
    HostConformance,
    /// Build evidence is reproducible.
    BuildReproduction,
    /// Network preflight verified.
    Network,
    /// Target attestation verified for the next boundary.
    TargetAttestation,
    /// Rollback intent declared.
    RollbackIntent,
}

impl ProofCategory {
    pub const ALL: [ProofCategory; 7] = [
        ProofCategory::Bootstrap,
        ProofCategory::Hardware,
        ProofCategory::HostConformance,
        ProofCategory::BuildReproduction,
        ProofCategory::Network,
        ProofCategory::TargetAttestation,
        ProofCategory::RollbackIntent,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Hardware => "hardware",
            Self::HostConformance => "host_conformance",
            Self::BuildReproduction => "build_reproduction",
            Self::Network => "network",
            Self::TargetAttestation => "target_attestation",
            Self::RollbackIntent => "rollback_intent",
        }
    }

    pub fn parse_flexible(input: &str) -> Option<Self> {
        let normalized = input.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "bootstrap" => Some(Self::Bootstrap),
            "hardware" => Some(Self::Hardware),
            "hostconformance" | "host_conformance" => Some(Self::HostConformance),
            "buildreproduction" | "build_reproduction" => Some(Self::BuildReproduction),
            "network" => Some(Self::Network),
            "targetattestation" | "target_attestation" => Some(Self::TargetAttestation),
            "rollbackintent" | "rollback_intent" => Some(Self::RollbackIntent),
            _ => None,
        }
    }
}

/// Evidence snapshot for tier progression evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierProgressionEvidence {
    pub current_kind: DeploymentTier,
    pub target_kind: DeploymentTier,
    pub satisfied: Vec<ProofCategory>,
    pub pending: Vec<ProofCategory>,
    pub unknown: Vec<ProofCategory>,
}

pub type HarnessProgressionEvidence = TierProgressionEvidence;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierProgressionVerdict {
    pub schema: &'static str,
    pub current_kind: DeploymentTier,
    pub target_kind: DeploymentTier,
    pub allowed: bool,
    pub blockers: Vec<String>,
    pub pending: Vec<ProofCategory>,
    pub unknown: Vec<ProofCategory>,
}

pub type HarnessProgressionVerdict = TierProgressionVerdict;

pub fn required_categories_for_tier(target: DeploymentTier) -> Vec<ProofCategory> {
    match target {
        DeploymentTier::LocalLab => vec![ProofCategory::Bootstrap],
        DeploymentTier::Conformance => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
            ]
        }
        DeploymentTier::Staging => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
            ]
        }
        DeploymentTier::Canary => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
            ]
        }
        DeploymentTier::Production => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
                ProofCategory::RollbackIntent,
            ]
        }
    }
}

pub fn decide_tier_progression(evidence: &TierProgressionEvidence) -> TierProgressionVerdict {
    let mut blockers = Vec::new();
    let required = required_categories_for_tier(evidence.target_kind);

    let mut missing_pending = Vec::new();
    let mut missing_unknown = Vec::new();

    for req in &required {
        if evidence.satisfied.contains(req) {
            continue;
        }
        if evidence.unknown.contains(req) {
            missing_unknown.push(*req);
            blockers.push(format!("unknown_proof:{}", req.as_str()));
        } else {
            missing_pending.push(*req);
            blockers.push(format!("missing_proof:{}", req.as_str()));
        }
    }

    if evidence.target_kind < evidence.current_kind {
        blockers.push("target_tier_precedes_current_tier".to_string());
    }

    TierProgressionVerdict {
        schema: DEPLOYMENT_TIER_PROGRESSION_SCHEMA,
        current_kind: evidence.current_kind,
        target_kind: evidence.target_kind,
        allowed: blockers.is_empty(),
        blockers,
        pending: missing_pending,
        unknown: missing_unknown,
    }
}

pub fn decide_harness_progression(evidence: &TierProgressionEvidence) -> TierProgressionVerdict {
    decide_tier_progression(evidence)
}
