//! Constitutional Court — meta-agent for internal policy coherence.
//!
//! When multiple surfaces (contracts, protocol, CLI, MCP, skills, skills) project
//! the same constitutional policies in different ways, this module provides the
//! types to detect drift, classify violations, and propose reconciliation — never
//! silent graceful degradation.
//!
//! Phase A: manual audit via Hermes LLM.
//! Phase C: autonomous daemon with cynic-inference (futur).

use serde::{Deserialize, Serialize};
use std::fmt;

pub const COURT_AUDIT_SCHEMA: &str = "cynic.court_audit.v1";
pub const POLICY_VIOLATION_SCHEMA: &str = "cynic.policy_violation.v1";
pub const COURT_MANDATE_SCHEMA: &str = "cynic.court_mandate.v1";

// ── Surface types ─────────────────────────────────────────────────────────────

/// The seven surfaces where constitutional policies are projected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceType {
    /// Constitutional foundation: contracts/foundation.json
    Contracts,
    /// Operational authority: infra/runtime-authority.json
    RuntimeAuthority,
    /// Canonical types: cynic-protocol/src/*.rs
    ProtocolTypes,
    /// Operator projection: cynic-cli/src/*_surface.rs
    CliSurface,
    /// Agent-facing tools: cynic-mcp/src/*.rs
    McpTool,
    /// Procedural memory: ~/.hermes/skills/cynic-*/SKILL.md
    Skill,
    /// Live state: /health, SSE events
    Runtime,
}

impl SurfaceType {
    /// All surfaces in canonical order.
    pub const ALL: &'static [Self] = &[
        Self::Contracts,
        Self::RuntimeAuthority,
        Self::ProtocolTypes,
        Self::CliSurface,
        Self::McpTool,
        Self::Skill,
        Self::Runtime,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contracts => "contracts",
            Self::RuntimeAuthority => "runtime_authority",
            Self::ProtocolTypes => "protocol_types",
            Self::CliSurface => "cli_surface",
            Self::McpTool => "mcp_tool",
            Self::Skill => "skill",
            Self::Runtime => "runtime",
        }
    }
}

impl fmt::Display for SurfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Violation nature ──────────────────────────────────────────────────────────

/// Classifies how a policy violation behaves from a reconciliation standpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationNature {
    /// Absolute block — no forward path without explicit reconciliation.
    /// Used for: graceful degradation, silent substitution, ad-hoc workaround.
    Blocked,
    /// Forward path exists via typed reconciliation.
    Recoverable,
    /// Direct violation of constitutional law (foundation.json).
    Constitutional,
    /// Agent behavior drifted from policy; requires agent-level resolution.
    AgentResolution,
}

impl ViolationNature {
    /// True if this nature prevents any forward progress until resolved.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Blocked | Self::Constitutional)
    }
}

// ── Verdict ───────────────────────────────────────────────────────────────────

/// Epistemic verdict on a gap between observed and expected state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// HOWL: critical gap — constitutional violation, immediate action required
    HOWL,
    /// WAG: warning — drift detected, forward path available
    WAG,
    /// GROWL: informational — minor alignment issue, no immediate risk
    GROWL,
    /// BARK: observation — potential concern, needs monitoring
    BARK,
}

impl Verdict {
    /// Q-score range associated with this verdict.
    pub fn q_score_range(&self) -> (f64, f64) {
        match self {
            Self::HOWL => (0.0, 25.0),
            Self::WAG => (25.0, 61.8),
            Self::GROWL => (61.8, 85.0),
            Self::BARK => (85.0, 100.0),
        }
    }
}

// ── Sovereign error codes ────────────────────────────────────────────────────

/// Typed error code following CYNIC-PROTOCOL-NNN convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CynicErrorCode {
    /// Domain: BUILD, KERNEL, AGENT, MCP, PROTOCOL, INFRA, COURT
    pub domain: String,
    /// 3-digit number within domain
    pub number: u16,
}

impl CynicErrorCode {
    pub fn new(domain: &str, number: u16) -> Self {
        Self {
            domain: domain.to_uppercase(),
            number,
        }
    }

    pub fn as_string(&self) -> String {
        format!("CYNIC-{}-{:03}", self.domain, self.number)
    }
}

impl fmt::Display for CynicErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

// ── Reconciliation path ──────────────────────────────────────────────────────

/// Typed path to resolve a violation — never ad-hoc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationPath {
    /// Modify contract to align with reality
    UpdateContract,
    /// Update canonical types in cynic-protocol
    UpdateProtocol,
    /// Update CLI surface to match policy
    UpdateCli,
    /// Update MCP tool exposure
    UpdateMcp,
    /// Update skill to match code reality
    UpdateSkill,
    /// Requires human operator authority escalation
    HumanAuthority,
    /// Escalate to court (meta-agent) for complex reconciliation
    EscalateToCourt,
}

// ── Policy violation ─────────────────────────────────────────────────────────

/// A typed gap between observed and expected state across surfaces.
///
/// Every substitution, silent fallback, or graceful degradation that bypasses
/// constitutional policy is a `PolicyViolation` — never invisible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Sovereign error code: CYNIC-{DOMAIN}-{NNN}
    pub code: CynicErrorCode,

    /// Nature of the violation
    pub nature: ViolationNature,

    /// Where the violation was observed
    pub surface: SurfaceType,

    /// Reference to the violated policy (file:line or contract section)
    pub policy_reference: String,

    /// What was observed (the drift)
    pub observed: String,

    /// What was expected (per policy)
    pub expected: String,

    /// Typed path to resolve
    pub required_reconciliation: ReconciliationPath,

    /// Epistemic Q-score (0-100, geometric mean of 7 axioms)
    pub q_score: f64,

    /// Verdict on severity
    pub verdict: Verdict,

    /// Optional: other surfaces involved in this drift
    pub related_surfaces: Vec<SurfaceType>,
}

impl PolicyViolation {
    /// Clamp q_score to valid range [0.0, 100.0], replacing NaN/Inf with 0.0.
    pub fn normalize_q_score(raw: f64) -> f64 {
        if raw.is_nan() || raw.is_infinite() || raw < 0.0 {
            0.0
        } else if raw > 100.0 {
            100.0
        } else {
            raw
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.nature.is_blocking()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} [{}] {}: observed={} expected={} q={:.1} verdict={}",
            self.code.as_string(),
            self.surface.as_str(),
            self.nature.as_str(),
            self.observed,
            self.expected,
            self.q_score,
            self.verdict.as_str(),
        )
    }
}

impl fmt::Display for ViolationNature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ViolationNature {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Recoverable => "recoverable",
            Self::Constitutional => "constitutional",
            Self::AgentResolution => "agent_resolution",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Verdict {
    fn as_str(&self) -> &'static str {
        match self {
            Self::HOWL => "HOWL",
            Self::WAG => "WAG",
            Self::GROWL => "GROWL",
            Self::BARK => "BARK",
        }
    }
}

// ── Court audit report ───────────────────────────────────────────────────────

/// Output of a constitutional court audit scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtAuditReport {
    /// Schema version
    pub schema: String,

    /// Surfaces scanned
    pub surfaces_scanned: Vec<SurfaceType>,

    /// Phase of audit (manual, cron, daemon)
    pub phase: String,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// All violations found
    pub violations: Vec<PolicyViolation>,

    /// Surfaces that could not be verified
    pub unknowns: Vec<(SurfaceType, String)>,

    /// Overall organism coherence score (geometric mean of Q-scores)
    pub coherence_score: f64,
}

impl CourtAuditReport {
    pub fn new(surfaces: Vec<SurfaceType>, phase: &str) -> Self {
        Self {
            schema: COURT_AUDIT_SCHEMA.to_string(),
            surfaces_scanned: surfaces,
            phase: phase.to_string(),
            timestamp: timestamp_now(),
            violations: Vec::new(),
            unknowns: Vec::new(),
            coherence_score: 100.0,
        }
    }

    pub fn add_violation(&mut self, violation: PolicyViolation) {
        self.violations.push(violation);
        self.recompute_coherence();
    }

    fn recompute_coherence(&mut self) {
        if self.violations.is_empty() {
            self.coherence_score = 100.0;
            return;
        }
        let product: f64 = self
            .violations
            .iter()
            .map(|v| (v.q_score / 100.0).max(0.01))
            .product();
        let n = self.violations.len() as f64;
        self.coherence_score = product.powf(1.0 / n) * 100.0;
    }

    pub fn blocking_count(&self) -> usize {
        self.violations.iter().filter(|v| v.is_blocking()).count()
    }
}

// ── Court mandate ─────────────────────────────────────────────────────────────

/// Mandate for the constitutional court meta-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtMandate {
    /// Unique mandate: policy alignment across surfaces
    pub primary_mandate: String,

    /// Maximum inference budget per cycle (tokens)
    pub inference_budget_per_cycle: u64,

    /// Authority level (always ReadOnlyPropose in Phase A)
    pub authority_level: String,

    /// Surfaces under surveillance
    pub scope: Vec<SurfaceType>,

    /// Self-audit disabled to prevent recursion
    pub self_audit: bool,

    /// Phase: manual, cron, daemon
    pub phase: String,
}

impl Default for CourtMandate {
    fn default() -> Self {
        Self {
            primary_mandate: "policy_alignment".to_string(),
            inference_budget_per_cycle: 50_000,
            authority_level: "ReadOnlyPropose".to_string(),
            scope: SurfaceType::ALL.to_vec(),
            self_audit: false,
            phase: "manual".to_string(),
        }
    }
}

impl CourtMandate {
    pub fn can_apply(&self) -> bool {
        false // Phase A: never auto-apply
    }

    pub fn with_phase(mut self, phase: &str) -> Self {
        self.phase = phase.to_string();
        self
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn timestamp_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_type_roundtrip() {
        for surface in SurfaceType::ALL {
            let s = surface.as_str();
            let parsed: SurfaceType = serde_json::from_str(&format!("\"{}\"", s)).unwrap();
            assert_eq!(*surface, parsed);
        }
    }

    #[test]
    fn violation_nature_blocking() {
        assert!(ViolationNature::Blocked.is_blocking());
        assert!(ViolationNature::Constitutional.is_blocking());
        assert!(!ViolationNature::Recoverable.is_blocking());
        assert!(!ViolationNature::AgentResolution.is_blocking());
    }

    #[test]
    fn cynic_error_code_format() {
        let code = CynicErrorCode::new("BUILD", 42);
        assert_eq!(code.as_string(), "CYNIC-BUILD-042");
    }

    #[test]
    fn verdict_q_score_ranges() {
        let (lo, hi) = Verdict::HOWL.q_score_range();
        assert!(lo < hi);
        assert_eq!(lo, 0.0);
        let (_, hi) = Verdict::BARK.q_score_range();
        assert_eq!(hi, 100.0);
    }

    #[test]
    fn court_mandate_default() {
        let mandate = CourtMandate::default();
        assert_eq!(mandate.primary_mandate, "policy_alignment");
        assert!(!mandate.can_apply());
        assert!(!mandate.self_audit);
        assert_eq!(mandate.scope.len(), 7);
    }

    #[test]
    fn court_audit_report_empty() {
        let report = CourtAuditReport::new(SurfaceType::ALL.to_vec(), "manual");
        assert_eq!(report.coherence_score, 100.0);
        assert_eq!(report.blocking_count(), 0);
    }

    #[test]
    fn court_audit_report_with_violation() {
        let mut report = CourtAuditReport::new(SurfaceType::ALL.to_vec(), "manual");
        report.add_violation(PolicyViolation {
            code: CynicErrorCode::new("COURT", 1),
            nature: ViolationNature::Blocked,
            surface: SurfaceType::CliSurface,
            policy_reference: "foundation.json:hard_boundaries#NoManualBlockerBypass".to_string(),
            observed: "used --yes instead of --apply".to_string(),
            expected: "use --apply with explicit admission grant".to_string(),
            required_reconciliation: ReconciliationPath::UpdateCli,
            q_score: 15.0,
            verdict: Verdict::HOWL,
            related_surfaces: vec![SurfaceType::Contracts],
        });
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.blocking_count(), 1);
        assert!(report.coherence_score < 100.0);
    }
}
