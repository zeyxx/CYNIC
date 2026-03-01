"""
Motor System — CYNIC's Muscles.
Anatomy: Central Nervous System (Motor Cortex) to Effectors.

Coordinates actions with metabolic costs and hardware constraints.
Axiom Alignment: BURN — gestures must have a measurable physical impact and cost.
"""

from __future__ import annotations

import logging
import time
from typing import Any

from cynic.kernel.core.event_bus import Event, get_core_bus

logger = logging.getLogger("cynic.kernel.organism.layers.motor")

class MotorSystem:
    """
    The 'Muscles' of CYNIC.
    Coordinates gestures (actions) with the physical body and energy budget.
    """

    def __init__(self, body: Any | None = None, budget_manager: Any | None = None) -> None:
        self.body = body # HardwareBody
        self.budget_manager = budget_manager
        self._bus = get_core_bus()
        self._gestures_executed = 0

    async def execute_gesture(
        self, 
        action_type: str, 
        effector: Any, 
        params: dict[str, Any],
        base_cost: float = 0.01 # USD or Joules
    ) -> dict[str, Any]:
        """
        Execute an embodied gesture.
        
        1. Calculate Metabolic Penalty
        2. Check Energy Budget
        3. Execute physical movement (Effector)
        4. Record fatigue
        """
        t0 = time.perf_counter()
        
        # 1. Metabolic Calculation (The price of incarnation)
        multiplier = 1.0
        if self.body:
            multiplier = self.body.get_metabolic_cost()
        
        actual_cost = base_cost * multiplier
        
        # 2. Budget Check (Energy survival)
        # TODO: Link with real budget manager if available
        
        # 3. Effector execution (The actual muscle contraction)
        logger.info(
            "*muscle flex* Executing %s gesture (Metabolic Multiplier=%.2f, Cost=%.4f)",
            action_type, multiplier, actual_cost
        )
        
        try:
            # Emit GESTURE_STARTED
            await self._bus.emit(Event(
                topic="organism.motor.gesture_started",
                payload={"type": action_type, "cost": actual_cost}
            ))
            
            # Real work happens here
            result = await effector.execute(**params)
            
            duration_ms = (time.perf_counter() - t0) * 1000
            self._gestures_executed += 1
            
            # Emit GESTURE_COMPLETED
            await self._bus.emit(Event(
                topic="organism.motor.gesture_completed",
                payload={
                    "type": action_type, 
                    "success": result.get("success", False),
                    "duration_ms": duration_ms,
                    "metabolic_effort": duration_ms * multiplier
                }
            ))
            
            return {
                **result,
                "metabolic_cost": actual_cost,
                "effort": duration_ms * multiplier
            }

        except Exception as e:
            logger.error("Motor failure: gesture %s failed: %s", action_type, e)
            return {"success": False, "error": str(e)}

    def stats(self) -> dict[str, Any]:
        return {
            "gestures_count": self._gestures_executed,
            "body_linked": self.body is not None,
        }
