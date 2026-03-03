"""
AccountAgent - The Organism's Auditor.

Tracks the total cost of all LLM judgments and manages the session budget.
Emits BUDGET_WARNING and BUDGET_EXHAUSTED events when thresholds are crossed.

-Law: BURN - resources are finite. Monitoring them is an Axiomatic requirement.
"""

from __future__ import annotations

import logging
from typing import Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import BudgetExhaustedPayload, BudgetWarningPayload
from cynic.kernel.core.formulas import BUDGET_HARD_CAP_USD, BUDGET_WARNING_PCT

logger = logging.getLogger("cynic.kernel.brain.cognition.account")


class AccountAgent:
    """
    Tracks expenditures and enforces the financial boundary of the organism.
    """

    def __init__(self, bus: EventBus, budget_limit_usd: Optional[float] = None):
        self.limit = budget_limit_usd or BUDGET_HARD_CAP_USD
        self.total_cost_usd = 0.0
        self._warning_sent = False
        self._exhausted_sent = False
        self.escore_tracker = None
        self._bus = bus

    def set_escore_tracker(self, tracker: Any) -> None:
        """Inject E-Score tracker for reputation updates."""
        self.escore_tracker = tracker

    def start(self):
        """Subscribe to judgment events to track costs."""
        self._bus.on(CoreEvent.JUDGMENT_CREATED, self.on_judgment_created)
        logger.info("AccountAgent started - budget=$%.2f, EScore=wired", self.limit)

    def stop(self) -> None:
        """Unregister from bus judgment events."""
        try:
            self._bus.off(CoreEvent.JUDGMENT_CREATED, self.on_judgment_created)
        except Exception as e:
            logger.debug(f"Error unregistering AccountAgent listener: {e}")
        logger.info("AccountAgent stopped")

    async def on_judgment_created(self, event: Event) -> None:
        """Accumulate cost from a new judgment."""
        try:
            payload = event.dict_payload
            cost = payload.get("cost_usd", 0.0)
            self.total_cost_usd += cost

            await self._check_thresholds()
        except Exception as e:
            logger.error("AccountAgent failed to update cost: %s", e)

    async def _check_thresholds(self) -> None:
        """Check if we've crossed budget boundaries."""
        # Use instance-specific bus
        bus = self._bus

        # 1. Budget Warning (e.g. 61.8% of limit reached)
        warning_threshold = self.limit * (BUDGET_WARNING_PCT / 100.0)
        if self.total_cost_usd >= warning_threshold and not self._warning_sent:
            self._warning_sent = True
            logger.warning(
                "BUDGET WARNING: %.2f USD spent (limit=%.2f)",
                self.total_cost_usd,
                self.limit,
            )
            await bus.emit(
                Event.typed(
                    CoreEvent.BUDGET_WARNING,
                    BudgetWarningPayload(
                        current_cost=self.total_cost_usd, limit=self.limit
                    ),
                )
            )

        # 2. Budget Exhausted
        if self.total_cost_usd >= self.limit and not self._exhausted_sent:
            self._exhausted_sent = True
            logger.error("BUDGET EXHAUSTED: %.2f USD spent", self.total_cost_usd)
            await bus.emit(
                Event.typed(
                    CoreEvent.BUDGET_EXHAUSTED,
                    BudgetExhaustedPayload(
                        total_cost=self.total_cost_usd, limit=self.limit
                    ),
                )
            )

    def stats(self) -> dict:
        return {
            "total_cost_usd": round(self.total_cost_usd, 4),
            "limit_usd": self.limit,
            "remaining_usd": round(max(0, self.limit - self.total_cost_usd), 4),
            "usage_pct": round((self.total_cost_usd / self.limit) * 100, 2)
            if self.limit > 0
            else 0,
        }
