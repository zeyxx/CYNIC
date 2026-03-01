"""
Motor System — CYNIC's Muscles.
Anatomy: Central Nervous System (Motor Cortex) to Effectors.

Coordinates actions with metabolic costs and hardware constraints.
Axiom Alignment: BURN — gestures must have a measurable physical impact and cost.
"""

from __future__ import annotations

import logging
import time
from typing import Any, Optional

from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event

logger = logging.getLogger("cynic.kernel.organism.layers.motor")


class MotorSystem:
    """
    The 'Muscles' of CYNIC.
    Coordinates gestures (actions) with the physical body and energy budget.
    """

    def __init__(self, bus: EventBus, body: Any | None = None, state_manager: Any | None = None):
        self.bus = bus
        self.body = body  # HardwareBody
        self.state = state_manager
        self._gestures_executed = 0
        
        # Survival Thresholds
        self.MAX_ACTION_COST = 0.50  # USD
        self.DAILY_BUDGET = 10.00   # USD

    async def execute_gesture(
        self,
        action_type: str,
        effector: Any,
        params: dict[str, Any],
        base_cost: float = 0.01,  # USD or Joules
    ) -> dict[str, Any]:
        """
        Execute an embodied gesture with metabolic oversight.
        """
        t0 = time.perf_counter()

        # 1. Metabolic Calculation (The price of incarnation)
        multiplier = 1.0
        if self.body:
            # Hardware fatigue or temperature might increase cost
            multiplier = self.body.get_metabolic_cost()

        actual_cost = base_cost * multiplier

        # 2. Budget Check (Energy survival)
        if actual_cost > self.MAX_ACTION_COST:
            logger.warning(f"[{self.bus.instance_id}] 🚫 Action blocked: cost {actual_cost:.4f} > MAX {self.MAX_ACTION_COST}")
            return {"success": False, "error": "cost_too_high"}

        if self.state:
            # Check cumulative spend (SRE lens: Prevent runaway automation)
            stats = await self.state.get_stats()
            total_spent = stats.get("total_spent_usd", 0.0)
            if total_spent + actual_cost > self.DAILY_BUDGET:
                logger.critical(f"[{self.bus.instance_id}] 🛑 DAILY BUDGET EXHAUSTED ({total_spent:.2f}). Blocking action.")
                return {"success": False, "error": "budget_exhausted"}

        # 3. Effector execution
        logger.info(
            "[%s] *muscle flex* Executing %s (Cost=%.4f)",
            self.bus.instance_id,
            action_type,
            actual_cost,
        )

        try:
            await self.bus.emit(Event.typed(
                CoreEvent.ACT_REQUESTED,
                {"type": action_type, "cost": actual_cost},
                source="motor"
            ))

            # Real work happens here
            result = await effector.execute(**params)

            duration_ms = (time.perf_counter() - t0) * 1000
            self._gestures_executed += 1

            # Update state with real spend
            if self.state and result.get("success"):
                # Record the BURN
                await self.state.record_action_cost(actual_cost)

            await self.bus.emit(Event.typed(
                CoreEvent.ACT_COMPLETED,
                {
                    "type": action_type,
                    "success": result.get("success", False),
                    "duration_ms": duration_ms,
                    "cost": actual_cost
                },
                source="motor"
            ))

            return {**result, "metabolic_cost": actual_cost, "duration_ms": duration_ms}

        except Exception as e:
            logger.error("[%s] Motor failure: %s", self.bus.instance_id, e)
            return {"success": False, "error": str(e)}

    def stats(self) -> dict[str, Any]:
        return {
            "gestures_count": self._gestures_executed,
            "body_linked": self.body is not None,
            "instance_id": self.bus.instance_id
        }
