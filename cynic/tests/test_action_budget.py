"""
Tests for ActionBudget — hourly token rate limiter.

Real behavior tests — no mocks of the class under test.
Time manipulation via direct reset_at assignment (standard pattern).
"""
import time
import pytest

from cynic.metabolism.budget import ActionBudget, HOURLY_CAP_TOKENS


class TestActionBudget:

    def test_fresh_budget_can_execute(self):
        """New budget with zero usage can always execute."""
        budget = ActionBudget()
        assert budget.can_execute() is True

    def test_budget_exhausted_blocks_execute(self):
        """Recording more than the hourly cap blocks execution."""
        budget = ActionBudget()
        # Exhaust the budget in one shot
        budget.record(HOURLY_CAP_TOKENS, 1, cost_usd=0.0)
        assert budget.can_execute() is False

    def test_record_increments_token_usage(self):
        """record() correctly accumulates input + output tokens."""
        budget = ActionBudget()
        budget.record(100, 50, cost_usd=0.01)
        assert budget.tokens_used_this_hour == 150
        assert budget.total_tokens == 150

    def test_record_increments_cost(self):
        """record() accumulates cost_usd correctly."""
        budget = ActionBudget()
        budget.record(100, 50, cost_usd=0.05)
        budget.record(200, 100, cost_usd=0.10)
        assert abs(budget.total_cost_usd - 0.15) < 1e-6
        assert abs(budget.cost_usd_this_hour - 0.15) < 1e-6

    def test_remaining_pct_decreases_with_usage(self):
        """remaining_pct() decreases as tokens are recorded."""
        budget = ActionBudget()
        assert budget.remaining_pct() == pytest.approx(1.0, abs=0.01)
        budget.record(HOURLY_CAP_TOKENS // 2, 0)
        assert budget.remaining_pct() == pytest.approx(0.5, abs=0.01)

    def test_remaining_pct_zero_when_exhausted(self):
        """remaining_pct() never goes below 0.0."""
        budget = ActionBudget()
        budget.record(HOURLY_CAP_TOKENS + 1000, 0)
        assert budget.remaining_pct() == 0.0

    def test_hourly_reset_restores_capacity(self):
        """Manipulating reset_at to the past triggers reset on next call."""
        budget = ActionBudget()
        budget.record(HOURLY_CAP_TOKENS, 0)
        assert budget.can_execute() is False

        # Simulate hour having elapsed
        budget.reset_at = time.time() - 1.0
        budget._maybe_reset()

        assert budget.tokens_used_this_hour == 0
        assert budget.cost_usd_this_hour == 0.0
        assert budget.can_execute() is True

    def test_total_cost_survives_reset(self):
        """total_cost_usd accumulates across hourly resets (never reset)."""
        budget = ActionBudget()
        budget.record(100, 0, cost_usd=0.25)

        # Force hourly reset
        budget.reset_at = time.time() - 1.0
        budget._maybe_reset()

        # Hourly cost resets, total does not
        assert budget.cost_usd_this_hour == 0.0
        assert abs(budget.total_cost_usd - 0.25) < 1e-6

    def test_stats_returns_dict(self):
        """stats() returns a dict with all expected keys."""
        budget = ActionBudget()
        budget.record(1000, 500, cost_usd=0.001)
        s = budget.stats()
        assert "can_execute" in s
        assert "tokens_used_this_hour" in s
        assert "remaining_pct" in s
        assert "total_cost_usd" in s
        assert s["tokens_used_this_hour"] == 1500
