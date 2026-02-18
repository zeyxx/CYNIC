"""
Tests for HeuristicFacetScorer — verifies non-flat, signal-driven axiom scoring.

Key invariants:
  - Clean code context → higher VERIFY/BURN/FIDELITY than smelly code
  - Dangerous ACT context → scores suppressed below 40 (BARK zone)
  - CYNIC self-state (healthy) → scores above 50 (WAG zone)
  - All scores in [0, 100]
  - Never returns flat 50.0 for signal-rich contexts
"""
import pytest
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.axioms import AxiomArchitecture


# ── Fixtures ──────────────────────────────────────────────────────────────

@pytest.fixture
def scorer() -> HeuristicFacetScorer:
    return HeuristicFacetScorer()


# Probe contexts (from judge/probes.py)
CTX_CLEAN = "Well-structured utility function with type hints and docstring."
CTX_SMELLY = "God class: 20-parameter method, wildcard imports, no type hints, magic numbers."
CTX_DANGER = "Irreversible destructive operation: unconfirmed, global blast radius, no backup."
CTX_SELF = "CYNIC self-state: all 11 dogs active, memory healthy, learning nominal."
CTX_SOLANA = "Standard SPL token transfer: fee 5000 lamports, 3 accounts, success."
CTX_NEUTRAL = "Some context without specific signals."


# ── Range tests ─────────────────────────────────────────────────────────

class TestOutputRange:
    def test_always_in_0_100(self, scorer):
        """All outputs must be in [0, 100]."""
        contexts = [CTX_CLEAN, CTX_SMELLY, CTX_DANGER, CTX_SELF, CTX_SOLANA, CTX_NEUTRAL]
        axioms = ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]
        for ctx in contexts:
            for ax in axioms:
                s = scorer(ax, ax, ctx)
                assert 0.0 <= s <= 100.0, f"Out of range: {ax}+'{ctx[:30]}' → {s}"

    def test_neutral_context_stays_near_50(self, scorer):
        """Context with no signals should stay close to 50."""
        for ax in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]:
            s = scorer(ax, ax, CTX_NEUTRAL)
            assert 40.0 <= s <= 60.0, f"Neutral context shifted too far: {ax} → {s}"


# ── Signal differentiation tests ────────────────────────────────────────

class TestSignalDifferentiation:
    def test_clean_code_beats_smelly_verify(self, scorer):
        """Clean code should score higher on VERIFY than smelly code."""
        clean_score = scorer("VERIFY", "VERIFIABILITY", CTX_CLEAN)
        smelly_score = scorer("VERIFY", "VERIFIABILITY", CTX_SMELLY)
        assert clean_score > smelly_score, (
            f"VERIFY: clean={clean_score} should > smelly={smelly_score}"
        )

    def test_clean_code_beats_smelly_burn(self, scorer):
        """Clean code should score higher on BURN than god-class code."""
        clean_score = scorer("BURN", "EFFICIENCY", CTX_CLEAN)
        smelly_score = scorer("BURN", "EFFICIENCY", CTX_SMELLY)
        assert clean_score > smelly_score

    def test_clean_code_beats_smelly_fidelity(self, scorer):
        """Clean code with type hints should score higher on FIDELITY."""
        clean_score = scorer("FIDELITY", "CANDOR", CTX_CLEAN)
        smelly_score = scorer("FIDELITY", "CANDOR", CTX_SMELLY)
        assert clean_score > smelly_score

    def test_danger_suppresses_all_axioms(self, scorer):
        """Dangerous ACT context should score below 40 on all core axioms."""
        for ax in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]:
            s = scorer(ax, ax, CTX_DANGER)
            assert s < 50.0, f"Danger not suppressing {ax}: {s}"

    def test_danger_scores_below_clean(self, scorer):
        """Danger context must score lower than clean code on all axioms."""
        for ax in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]:
            clean = scorer(ax, ax, CTX_CLEAN)
            danger = scorer(ax, ax, CTX_DANGER)
            assert clean > danger, f"{ax}: clean={clean} should > danger={danger}"

    def test_self_state_scores_above_neutral(self, scorer):
        """Healthy CYNIC self-state should score above neutral on all axioms."""
        for ax in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]:
            s = scorer(ax, ax, CTX_SELF)
            assert s > 50.0, f"CYNIC self-state should be above 50 for {ax}: {s}"

    def test_solana_success_above_neutral(self, scorer):
        """Successful Solana tx should score above neutral."""
        for ax in ["FIDELITY", "VERIFY"]:
            s = scorer(ax, ax, CTX_SOLANA)
            assert s > 50.0, f"Solana success should be above 50 for {ax}: {s}"


# ── Non-flat test ────────────────────────────────────────────────────────

class TestNotFlat:
    def test_clean_code_not_neutral_verify(self, scorer):
        """Clean code must produce non-neutral VERIFY score."""
        s = scorer("VERIFY", "ACCURACY", CTX_CLEAN)
        assert s != 50.0, "Expected non-neutral score for clean code VERIFY"
        assert s > 55.0, f"Expected > 55 for clean code VERIFY, got {s}"

    def test_smelly_code_not_neutral_burn(self, scorer):
        """Smelly code must produce non-neutral BURN score."""
        s = scorer("BURN", "UTILITY", CTX_SMELLY)
        assert s != 50.0, "Expected non-neutral score for smelly code BURN"
        assert s < 45.0, f"Expected < 45 for smelly code BURN, got {s}"

    def test_danger_not_neutral_fidelity(self, scorer):
        """Dangerous ACT must produce non-neutral FIDELITY score."""
        s = scorer("FIDELITY", "COMMITMENT", CTX_DANGER)
        assert s != 50.0
        assert s < 50.0, f"Expected < 50 for danger FIDELITY, got {s}"


# ── Integration with AxiomArchitecture ──────────────────────────────────

class TestAxiomArchitectureIntegration:
    def test_clean_code_q_score_wag_zone(self):
        """Clean code via AxiomArchitecture with heuristic scorer → WAG zone."""
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
        result = arch.score_and_compute(domain="CODE", context=CTX_CLEAN)
        # With signal-aware scoring, clean code should reach WAG (≥61.8)
        assert result.q_score >= 55.0, (
            f"Clean code Q-Score should be in WAG zone, got {result.q_score}"
        )

    def test_smelly_code_q_score_lower_than_clean(self):
        """Smelly code Q-Score must be lower than clean code."""
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
        clean_result = arch.score_and_compute(domain="CODE", context=CTX_CLEAN)
        smelly_result = arch.score_and_compute(domain="CODE", context=CTX_SMELLY)
        assert clean_result.q_score > smelly_result.q_score, (
            f"Clean ({clean_result.q_score}) should > Smelly ({smelly_result.q_score})"
        )

    def test_danger_q_score_suppressed(self):
        """Dangerous ACT should produce GROWL/BARK verdict via heuristic scorer."""
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        from cynic.core.axioms import Verdict
        arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
        result = arch.score_and_compute(domain="CODE", context=CTX_DANGER)
        assert result.verdict in (Verdict.GROWL, Verdict.BARK), (
            f"Danger should be GROWL/BARK, got {result.verdict} (Q={result.q_score})"
        )

    def test_flat_default_vs_heuristic(self):
        """Heuristic scorer produces different results than flat 50.0 default."""
        arch_flat = AxiomArchitecture()  # Default flat scorer
        arch_heur = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())

        flat_clean = arch_flat.score_and_compute(domain="CODE", context=CTX_CLEAN)
        heur_clean = arch_heur.score_and_compute(domain="CODE", context=CTX_CLEAN)

        # Heuristic should produce different score than flat default
        assert flat_clean.q_score != heur_clean.q_score, (
            "Heuristic scorer must produce different results than flat 50.0"
        )
        # Heuristic should be higher for clean code
        assert heur_clean.q_score >= flat_clean.q_score, (
            f"Heuristic ({heur_clean.q_score}) should ≥ flat ({flat_clean.q_score}) for clean code"
        )
