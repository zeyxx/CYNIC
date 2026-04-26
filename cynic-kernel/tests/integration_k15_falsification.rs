#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! K15 Falsification E2E: Crystal context changes verdicts or fails.
//!
//! Uses the ONE crystallized pattern known to exist (141 obs, certainty=0.999)
//! to test whether Dogs actually USE injected crystal context.
//!
//! **Setup:**
//! - Mock storage that returns a mature crystallized crystal for domain "general"
//! - Two identical stimuli: one with inject_crystals=false, one with inject_crystals=true
//! - Compare q_scores and verdict kinds
//!
//! **Falsification:**
//! If q_1 ≈ q_0 (delta < φ⁻⁴) and kind_1 == kind_0, then K15 fails:
//! Dogs either ignore context or consumer is broken.

#[cfg(test)]
mod k15_e2e_falsification {
    use cynic_kernel::domain::ccm::{Crystal, CrystalState, MatureCrystal};
    use cynic_kernel::domain::dog::{PHI_INV, PHI_INV4};

    /// Build a test crystal matching the one known to exist (141 obs, certainty=0.999).
    /// This is the ONLY mature crystal in production as of 2026-04-25.
    fn make_test_crystallized_pattern() -> MatureCrystal {
        let crystal = Crystal {
            id: "prod-crystallized-001".to_string(),
            // Actual content unknown from handoff, but we test with a reasonable substitute.
            // The pattern is from repeated observations of some concept.
            content: "Pattern emerged from 141 observations: this has proven true consistently across varying conditions".to_string(),
            domain: "general".to_string(),
            confidence: 0.95, // high but within phi-bounded
            observations: 141,
            state: CrystalState::Crystallized,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            updated_at: "2026-04-25T10:00:00Z".to_string(),
            contributing_verdicts: (0..141)
                .map(|i| format!("v-{i:03}"))
                .collect(),
            certainty: 0.999,  // matches handoff claim
            variance_m2: 0.005, // very low variance — highly consensual
            mean_quorum: 3.5,
            howl_count: 100,
            wag_count: 41,
            growl_count: 0,
            bark_count: 0,
        };

        MatureCrystal::try_from(crystal).expect("test crystal must be mature")
    }

    #[test]
    fn k15_format_crystal_context_produces_injection_text() {
        // STEP 1: Prove that crystal can be formatted into Dog context.
        let crystal = make_test_crystallized_pattern();
        let formatted = cynic_kernel::domain::ccm::format_crystal_context(
            &[crystal],
            "general",
            1100, // standard budget
        );

        assert!(
            formatted.is_some(),
            "mature crystal must produce context for injection"
        );

        let ctx = formatted.unwrap();
        assert!(ctx.contains("CRYSTALLIZED"), "context should label state");
        assert!(ctx.contains("0.99"), "context should show high certainty");
        assert!(ctx.contains("141"), "context should show observation count");

        println!("✓ Step 1: Crystal formats correctly");
        println!("  Context:\n{ctx}\n");
    }

    #[test]
    fn k15_empty_crystal_list_produces_no_injection() {
        // NEGATIVE TEST: when no crystals, no context is injected.
        let formatted = cynic_kernel::domain::ccm::format_crystal_context(&[], "general", 1100);

        assert!(
            formatted.is_none(),
            "empty crystal list should produce no context"
        );
        println!("✓ Step 2: Empty crystal list → no injection");
    }

    #[test]
    fn k15_domain_filter_respects_boundaries() {
        // ISOLATION TEST: crystal for domain "chess" should not inject into "trading" domain.
        let crystal = make_test_crystallized_pattern();

        let formatted_chess = cynic_kernel::domain::ccm::format_crystal_context(
            std::slice::from_ref(&crystal),
            "chess",
            1100,
        );
        assert!(
            formatted_chess.is_none(),
            "general-domain crystal should not inject for specific domain without match"
        );

        let formatted_general = cynic_kernel::domain::ccm::format_crystal_context(
            std::slice::from_ref(&crystal),
            "general",
            1100,
        );
        assert!(
            formatted_general.is_some(),
            "crystal.domain='general' should inject for domain='general'"
        );

        println!("✓ Step 3: Domain filtering works");
    }

    #[test]
    fn k15_falsification_setup_document() {
        // DOCUMENTATION OF FALSIFICATION PROTOCOL.
        //
        // This test documents what WOULD be falsified if infrastructure were available.
        // Actual falsification requires:
        // - Live Judge with all Dogs
        // - Real embedding for the stimulus
        // - Verdict caching disabled (to force fresh evaluation each time)
        //
        // PROTOCOL:
        // 1. Create stimulus: "unknown concept X"
        // 2. Run verdict WITHOUT crystal: run(..., inject_crystals=false, ...)
        //    → Capture q_score_no_crystal, verdict_kind_no_crystal
        // 3. Inject the test crystal above into storage
        // 4. Run verdict WITH crystal: run(..., inject_crystals=true, ...)
        //    → Capture q_score_with_crystal, verdict_kind_with_crystal
        // 5. Falsify:
        //    a) delta = |q_with - q_no| >= PHI_INV4 * 0.5 → K15 PASSES (consumer works)
        //    b) OR verdict_kind changed → K15 PASSES
        //    c) OTHERWISE → K15 FAILS (Dogs ignore crystal context)
        //
        // THRESHOLD: φ⁻⁴ ≈ 0.146. Accept delta >= 0.07 (half of that).
        // Rationale: crystal represents certainty=0.999 (consensus), should shift verdict > noise.

        let crystal = make_test_crystallized_pattern();
        let certainty = crystal.certainty();
        let observations = crystal.observations();

        println!("K15 FALSIFICATION SETUP:");
        println!("  Test crystal certainty: {certainty:.3}");
        println!("  Test crystal observations: {observations}");
        println!("  Threshold for passing delta: >= {:.3}", PHI_INV4 * 0.5);
        println!();
        println!("To run full falsification:");
        println!("  1. Boot kernel + Dogs + SurrealDB");
        println!("  2. Inject test crystal via /storage endpoint");
        println!("  3. Run stimulus twice (with/without inject_crystals)");
        println!("  4. Measure delta and compare to threshold");
        println!();

        assert!(
            certainty > PHI_INV,
            "test crystal must exceed crystallization threshold"
        );
        assert!(
            observations > 100,
            "test crystal must have many observations"
        );
    }

    #[test]
    fn k15_docstring_summary() {
        // SUMMARY: K15 acts as follows:
        //
        // PRODUCER: observe_crystal_for_verdict() in crystal_observer.rs
        //   Input: every verdict from evaluate_progressive()
        //   Output: crystals stored in SurrealDB
        //   Mechanism: quorum gate (voter_count >= 2) + epistemic gate (disagreement < φ⁻²)
        //
        // INJECTION: format_crystal_context() in domain/ccm/engine.rs
        //   Input: list of MatureCrystal (only Crystallized|Canonical)
        //   Output: text context to inject into Dog prompt
        //   Mechanism: format_crystal_context() → enriched_context → Stimulus.context
        //
        // CONSUMER: Dogs receive stimulus.context in their prompt (dogs/inference.rs)
        //   Input: Stimulus with context field
        //   Output: AxiomScores (hopefully adjusted by context)
        //   Mechanism: context embedded in user prompt alongside stimulus.content
        //
        // THE QUESTION: Do Dogs actually USE the context to change their scores?
        // This has never been measured. The test above documents the falsification
        // protocol, but running it requires live infrastructure.
        //
        // STATUS: UNTESTED CONSUMER (code exists, impact unknown)

        println!("K15 consumer chain documented and ready for empirical validation");
    }
}
