#![allow(clippy::unwrap_used, clippy::expect_used)]
//! K15 Consumer Testing: Does crystal injection actually change verdicts?
//!
//! **Hypothesis:** Crystals are injected into Dog context. If they're read and used,
//! verdicts should differ between inject_crystals=true and inject_crystals=false.
//!
//! **Falsification:** If Q-scores are identical (or nearly identical within φ⁻⁴),
//! the consumer is broken or Dogs ignore the context.

#[cfg(test)]
mod k15_consumer_impact {
    /// Stub test — would require live Dogs + storage + full pipeline setup.
    /// Demonstrates the test pattern that SHOULD run in integration environment.
    #[test]
    #[ignore = "requires live Dogs + SurrealDB; use in CI after kernel boots"]
    #[allow(clippy::print_stdout)] // WHY: stub test body uses println! to communicate intent; no logging infra in test context
    fn k15_crystal_injection_changes_verdict_kind() {
        // PATTERN FOR INTEGRATION TEST:
        //
        // 1. Setup: inject a test crystal (certainty >= φ⁻¹, state=Crystallized)
        // 2. First run (inject_crystals=false): measure Q-score and kind
        // 3. Second run (inject_crystals=true): measure Q-score and kind
        // 4. Falsify: |q_1 - q_0| >= φ⁻⁴ or kind changed

        println!("Stub test — see pattern above for integration setup");
        // This test will be implemented when Dogs are reliably live in test environment
    }

    /// Structural verification: format_crystal_context produces real output
    /// from a crystallized MatureCrystal. Tests the plumbing, not Dog behavior.
    #[test]
    fn k15_format_crystal_context_produces_nonempty_output() {
        use cynic_kernel::domain::ccm::{
            Crystal, CrystalState, MatureCrystal, format_crystal_context,
        };

        let crystal = Crystal {
            id: "test-crystal-001".into(),
            content: "Italian Opening leads to tactical play".into(),
            domain: "chess".into(),
            confidence: 0.85,
            observations: 50,
            state: CrystalState::Crystallized,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            contributing_verdicts: vec!["v-001".into()],
            certainty: 0.72,
            variance_m2: 0.5,
            mean_quorum: 3.5,
            howl_count: 30,
            wag_count: 15,
            growl_count: 3,
            bark_count: 2,
        };
        let mature = MatureCrystal::try_from(crystal).expect("test crystal must be mature");

        let context = format_crystal_context(&[mature], "chess", 4096);
        assert!(
            context.is_some(),
            "format_crystal_context must produce output for crystallized data"
        );
        let text = context.unwrap();
        assert!(
            text.contains("Italian Opening"),
            "crystal content must appear in formatted context"
        );
    }
}
