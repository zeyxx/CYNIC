//! AuditEngine trait + questions for Gemini+cynic-wisdom.

use async_trait::async_trait;

use crate::log::LogEntry;
use crate::reflection::Reflection;

#[async_trait]
pub trait AuditEngine: Send + Sync {
    /// Audit a corpus of logs against audit questions or directives.
    ///
    /// Must return `Reflection::degraded(reason)` on engine unavailability
    /// (per K14: poison/missing = degraded, never optimistic).
    async fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> crate::Result<Reflection>;
}

pub mod gemini_wisdom;

/// Default Phase 2 audit directives.
/// These guide Gemini to generate emergent questions based on the 7 CYNIC axioms.
pub fn default_phase2_directives() -> Vec<&'static str> {
    vec![
        "Analyze the logs through the lens of the 7 CYNIC axioms: FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY, and KENOSIS.",
        "For each axiom, identify the specific tension or success that emerges from the entries.",
        "Formulate the 'Question of the Week' that the context itself is posing.",
        "Detect patterns of self-deception vs. grounded reporting.",
    ]
}
