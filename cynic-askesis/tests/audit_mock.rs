#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use cynic_askesis::audit::{AuditEngine, default_phase1_questions};
use cynic_askesis::log::LogEntry;
use cynic_askesis::reflection::{Reflection, Verdict};

struct StaticMock(Reflection);

#[async_trait]
impl AuditEngine for StaticMock {
    async fn audit(
        &self,
        _logs: &[LogEntry],
        _questions: &[&str],
    ) -> cynic_askesis::Result<Reflection> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn mock_audit_returns_static_reflection() {
    let expected = Reflection {
        verdict: Verdict::Wag,
        prose: "ok".into(),
        patterns_detected: vec![],
        kenosis_candidate: None,
        confidence: 0.5,
    };
    let engine = StaticMock(expected.clone());
    let result = engine
        .audit(&[], &default_phase1_questions())
        .await
        .unwrap();
    assert_eq!(result, expected);
}

#[test]
fn default_phase1_questions_are_non_empty() {
    let questions = default_phase1_questions();
    assert!(!questions.is_empty());
    assert!(
        questions
            .iter()
            .any(|q| q.to_lowercase().contains("kenosis"))
    );
}
