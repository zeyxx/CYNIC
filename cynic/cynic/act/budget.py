"""
ActionBudget — CYNIC token/cost rate limiter for autonomous ACT phase.

Prevents runaway autonomous loops by capping Haiku token usage per hour.
At $0.25/MTok input + $1.25/MTok output for Haiku:
  HOURLY_CAP_TOKENS = 400_000 ≈ $0.50/hour maximum spend.

Usage:
    budget = ActionBudget()
    if not budget.can_execute():
        logger.warning("Budget exhausted — skip auto-ACT")
        return
    result = await runner.execute(prompt)
    budget.record(result.get("input_tokens", 0), result.get("output_tokens", 0), result.get("cost_usd", 0.0))
"""
from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from typing import Any, Dict

from cynic.core.phi import PHI_INV_2, MAX_Q_SCORE

logger = logging.getLogger("cynic.act.budget")

HOURLY_CAP_TOKENS: int = 400_000   # ~$0.50/h @ Haiku pricing
HOURLY_CAP_USD: float = 0.50


@dataclass
class ActionBudget:
    """
    φ-bounded hourly token budget for autonomous ACT executions.

    Resets automatically every hour. Thread-safe for single event loop use.
    PHI_INV_2 (0.382) = warning threshold: below 38.2% remaining → log warning.
    """
    hourly_cap_tokens: int = HOURLY_CAP_TOKENS
    tokens_used_this_hour: int = 0
    cost_usd_this_hour: float = 0.0
    reset_at: float = field(default_factory=lambda: time.time() + 3600)
    total_cost_usd: float = 0.0
    total_tokens: int = 0

    def can_execute(self) -> bool:
        """Return True if there is remaining budget for another execution."""
        self._maybe_reset()
        remaining = self.remaining_pct()
        if remaining < PHI_INV_2:
            logger.warning(
                "ActionBudget low: %.1f%% remaining (%d/%d tokens this hour)",
                remaining * 100,
                self.tokens_used_this_hour,
                self.hourly_cap_tokens,
            )
        return self.tokens_used_this_hour < self.hourly_cap_tokens

    def record(self, input_tokens: int, output_tokens: int, cost_usd: float = 0.0) -> None:
        """Record token usage and cost from one execution."""
        self._maybe_reset()
        tokens = input_tokens + output_tokens
        self.tokens_used_this_hour += tokens
        self.total_tokens += tokens
        self.cost_usd_this_hour += cost_usd
        self.total_cost_usd += cost_usd
        logger.debug(
            "ActionBudget.record: +%d tokens (+$%.4f) → %d/%d used this hour",
            tokens, cost_usd,
            self.tokens_used_this_hour, self.hourly_cap_tokens,
        )

    def remaining_pct(self) -> float:
        """Return fraction of hourly budget remaining [0.0, 1.0]."""
        self._maybe_reset()
        return max(
            0.0,
            1.0 - (self.tokens_used_this_hour / max(self.hourly_cap_tokens, 1)),
        )

    def _maybe_reset(self) -> None:
        """Reset hourly counters if the hour has elapsed."""
        if time.time() >= self.reset_at:
            self.tokens_used_this_hour = 0
            self.cost_usd_this_hour = 0.0
            self.reset_at = time.time() + 3600
            logger.info("ActionBudget: hourly reset")

    def stats(self) -> dict[str, Any]:
        """Stats dict for /budget endpoint."""
        self._maybe_reset()
        return {
            "can_execute": self.can_execute(),
            "tokens_used_this_hour": self.tokens_used_this_hour,
            "hourly_cap_tokens": self.hourly_cap_tokens,
            "remaining_pct": round(self.remaining_pct(), 3),
            "cost_usd_this_hour": round(self.cost_usd_this_hour, 4),
            "total_cost_usd": round(self.total_cost_usd, 4),
            "total_tokens": self.total_tokens,
            "reset_in_s": round(max(0.0, self.reset_at - time.time()), 1),
        }
