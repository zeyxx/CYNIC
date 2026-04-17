//! AuditEngine trait + questions for Gemini+cynic-wisdom.

use async_trait::async_trait;

use crate::log::LogEntry;
use crate::reflection::Reflection;

#[async_trait]
pub trait AuditEngine: Send + Sync {
    /// Audit a corpus of logs against audit questions.
    ///
    /// Must return `Reflection::degraded(reason)` on engine unavailability
    /// (per K14: poison/missing = degraded, never optimistic).
    async fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> crate::Result<Reflection>;
}

/// Default Phase 1 audit questions (generic, not domain-specific).
/// Phase 2+ domains provide their own via DomainTracker::audit_questions.
pub fn default_phase1_questions() -> Vec<&'static str> {
    vec![
        "What patterns of self-deception vs honest reporting?",
        "What has Zey stopped doing? (KENOSIS check)",
        "Where is authenticity strongest, where weakest?",
        "Is the language concrete and grounded, or abstract and smoothing?",
    ]
}
