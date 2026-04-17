//! DomainTracker trait + registry.
//!
//! Phase 1: registry is deliberately empty. Body DomainTracker ships in Phase 2.

use chrono::NaiveTime;

pub trait DomainTracker: Send + Sync {
    fn name(&self) -> &str;
    fn log_prompt(&self) -> &str;
    fn audit_questions(&self) -> Vec<&str>;
    fn anchor_time(&self) -> NaiveTime;
}

/// Registry of domain trackers. Phase 1 returns an empty vector — intentional.
pub fn registry() -> Vec<Box<dyn DomainTracker>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_registry_is_empty() {
        assert!(registry().is_empty(), "Phase 1 MUST ship no domains");
    }
}
