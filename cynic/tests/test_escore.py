"""
CYNIC EScoreTracker Tests (γ4)

Tests φ-weighted reputation tracking across 7 dimensions × 7 realities.
No LLM, no DB — pure in-memory.
"""
from __future__ import annotations

import math
import pytest

from cynic.core.phi import MAX_Q_SCORE, E_SCORE_WEIGHTS
from cynic.core.escore import (
    EScoreTracker,
    EntityScore,
    DimScore,
    EMA_ALPHA,
    DEFAULT_DIM_SCORE,
)


# ── DimScore ──────────────────────────────────────────────────────────────

class TestDimScore:
    def test_first_update_sets_value_directly(self):
        ds = DimScore(dimension="BUILD")
        ds.apply(80.0)
        assert ds.value == 80.0
        assert ds.updates == 1

    def test_ema_blends_new_with_old(self):
        ds = DimScore(dimension="JUDGE")
        ds.apply(60.0)   # First
        ds.apply(100.0)  # Second: EMA = 0.618*100 + 0.382*60
        expected = EMA_ALPHA * 100.0 + (1 - EMA_ALPHA) * 60.0
        assert abs(ds.value - expected) < 0.01

    def test_clamps_to_max(self):
        ds = DimScore(dimension="BURN")
        ds.apply(200.0)  # Over max
        assert ds.value <= MAX_Q_SCORE

    def test_clamps_to_zero(self):
        ds = DimScore(dimension="RUN")
        ds.apply(-50.0)
        assert ds.value >= 0.0

    def test_updates_counter(self):
        ds = DimScore(dimension="SOCIAL")
        for _ in range(5):
            ds.apply(50.0)
        assert ds.updates == 5


# ── EScoreTracker ─────────────────────────────────────────────────────────

class TestEScoreTrackerBasics:
    def test_initial_state(self):
        t = EScoreTracker()
        assert t.entity_count() == 0
        s = t.stats()
        assert s["entities"] == 0
        assert s["total_updates"] == 0

    def test_unknown_entity_returns_default_score(self):
        t = EScoreTracker()
        score = t.get_score("user:unknown")
        assert abs(score - DEFAULT_DIM_SCORE) < 0.01

    def test_update_creates_entity(self):
        t = EScoreTracker()
        t.update("user:alice", "BUILD", 70.0)
        assert t.entity_count() == 1

    def test_update_returns_aggregate_score(self):
        t = EScoreTracker()
        score = t.update("user:alice", "BUILD", 70.0)
        assert 0.0 <= score <= MAX_Q_SCORE

    def test_update_invalid_dimension_raises(self):
        t = EScoreTracker()
        with pytest.raises(ValueError, match="Invalid dimension"):
            t.update("user:alice", "INVALID_DIM", 50.0)

    def test_update_invalid_reality_raises(self):
        t = EScoreTracker()
        with pytest.raises(ValueError, match="Invalid reality"):
            t.update("user:alice", "BUILD", 50.0, reality="MARS")

    def test_all_valid_dimensions_accepted(self):
        t = EScoreTracker()
        for dim in E_SCORE_WEIGHTS:
            t.update("user:test", dim, 60.0)
        assert t.entity_count() == 1

    def test_all_valid_realities_accepted(self):
        from cynic.core.escore import REALITIES
        t = EScoreTracker()
        for reality in sorted(REALITIES):
            t.update("user:test", "BUILD", 60.0, reality=reality)

    def test_multiple_entities_tracked_independently(self):
        t = EScoreTracker()
        t.update("user:alice", "BUILD", 80.0)
        t.update("user:bob", "BURN", 20.0)
        assert t.entity_count() == 2
        alice = t.get_score("user:alice")
        bob = t.get_score("user:bob")
        # Alice BUILD=80, others default; Bob BURN=20, others default
        # Alice should score higher (BUILD has high base, BURN defaults at 50)
        assert alice != bob  # Different scores


class TestEScoreAggregate:
    def test_aggregate_phi_bounded(self):
        """Aggregate score always in [0, MAX_Q_SCORE]."""
        t = EScoreTracker()
        for dim in E_SCORE_WEIGHTS:
            t.update("entity:x", dim, 100.0)
        score = t.get_score("entity:x")
        assert 0.0 <= score <= MAX_Q_SCORE

    def test_all_max_gives_high_score(self):
        """All dimensions at 100 → score close to MAX_Q_SCORE."""
        t = EScoreTracker()
        for dim in E_SCORE_WEIGHTS:
            for _ in range(10):  # Multiple updates to converge EMA
                t.update("entity:max", dim, 100.0)
        score = t.get_score("entity:max")
        assert score > 80.0, f"Expected high score, got {score}"

    def test_all_min_gives_low_score(self):
        """All dimensions at 1.0 → score near 1."""
        t = EScoreTracker()
        for dim in E_SCORE_WEIGHTS:
            for _ in range(10):
                t.update("entity:min", dim, 1.0)
        score = t.get_score("entity:min")
        assert score < 20.0, f"Expected low score, got {score}"

    def test_high_weight_dimension_matters_more(self):
        """BURN (φ³ weight) affects score more than HOLD (φ⁻³ weight)."""
        t1 = EScoreTracker()
        t2 = EScoreTracker()

        # t1: BURN=90 (high weight), HOLD=10 (low weight)
        for _ in range(5):
            t1.update("e1", "BURN", 90.0)
            t1.update("e1", "HOLD", 10.0)

        # t2: BURN=10 (high weight), HOLD=90 (low weight)
        for _ in range(5):
            t2.update("e2", "BURN", 10.0)
            t2.update("e2", "HOLD", 90.0)

        # e1 should score higher because BURN dominates
        s1 = t1.get_score("e1")
        s2 = t2.get_score("e2")
        assert s1 > s2, f"BURN-high should outscore HOLD-high: {s1} vs {s2}"


class TestEScoreReality:
    def test_reality_score_defaults_to_aggregate(self):
        """No reality-specific data → reality_score = aggregate."""
        t = EScoreTracker()
        t.update("user:alice", "BUILD", 70.0)  # No reality context
        agg = t.get_score("user:alice")
        reality = t.get_reality_score("user:alice", "CODE")
        assert abs(agg - reality) < 0.01

    def test_reality_score_uses_specific_data(self):
        """After reality-specific update, reality_score differs from aggregate."""
        t = EScoreTracker()
        # Global: BUILD=50
        t.update("user:alice", "BUILD", 50.0)
        # SOLANA-specific: BUILD=90
        t.update("user:alice", "BUILD", 90.0, reality="SOLANA")
        solana_score = t.get_reality_score("user:alice", "SOLANA")
        code_score = t.get_reality_score("user:alice", "CODE")
        # SOLANA-specific BUILD=90 vs CODE falling back to global BUILD≈50+EMA
        assert solana_score != code_score

    def test_unknown_entity_reality_score_returns_default(self):
        t = EScoreTracker()
        score = t.get_reality_score("nobody", "CODE")
        assert abs(score - DEFAULT_DIM_SCORE) < 0.01


class TestEScoreDetail:
    def test_detail_keys_present(self):
        t = EScoreTracker()
        t.update("user:alice", "JUDGE", 65.0)
        detail = t.get_detail("user:alice")
        assert "entity_id" in detail
        assert "aggregate" in detail
        assert "dimensions" in detail
        assert "reality_scores" in detail

    def test_detail_all_dimensions_present(self):
        """Detail always has all 7 dimensions, even unupdated ones."""
        t = EScoreTracker()
        t.update("user:bob", "BURN", 80.0)
        detail = t.get_detail("user:bob")
        for dim in E_SCORE_WEIGHTS:
            assert dim in detail["dimensions"]

    def test_detail_unknown_entity(self):
        t = EScoreTracker()
        detail = t.get_detail("nobody")
        assert detail["aggregate"] == DEFAULT_DIM_SCORE
        assert all(
            d["value"] == DEFAULT_DIM_SCORE
            for d in detail["dimensions"].values()
        )

    def test_detail_reality_scores_populated(self):
        t = EScoreTracker()
        t.update("user:alice", "BUILD", 75.0, reality="CODE")
        t.update("user:alice", "JUDGE", 80.0, reality="SOLANA")
        detail = t.get_detail("user:alice")
        assert "CODE" in detail["reality_scores"]
        assert "SOLANA" in detail["reality_scores"]


class TestTopEntities:
    def test_top_entities_sorted_descending(self):
        t = EScoreTracker()
        for _ in range(5):
            t.update("low", "BUILD", 10.0)
        for _ in range(5):
            t.update("high", "BUILD", 90.0)
            t.update("high", "BURN", 90.0)
        top = t.top_entities(n=2)
        assert top[0][0] == "high"
        assert top[0][1] >= top[1][1]

    def test_top_entities_n_limit(self):
        t = EScoreTracker()
        for i in range(10):
            t.update(f"entity:{i}", "BUILD", float(i * 5 + 10))
        top = t.top_entities(n=3)
        assert len(top) == 3

    def test_top_entities_empty_tracker(self):
        t = EScoreTracker()
        assert t.top_entities() == []
