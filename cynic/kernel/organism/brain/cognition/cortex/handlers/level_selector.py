"""
LevelSelector — Strategic Consciousness Allocation.

Arbitrates between Reality requirements (Anchors) and Hardware constraints (Profiler).
Ensures the organism chooses the highest possible level of detail without 
exhausting metabolic resources.
"""
from __future__ import annotations
import logging
from typing import Any, Callable, Optional

from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.kernel.core.consciousness import ConsciousnessLevel, get_anchor
from cynic.kernel.organism.brain.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.kernel.organism.metabolism.model_profiler import ModelProfiler

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.level_selector")

class LevelSelector(BaseHandler):
    """
    Decides the Consciousness Level for a judgment pipeline.
    """
    handler_id = "level_selector"

    def __init__(self, axiom_monitor: Any = None, lod_controller: Any = None, **kwargs):
        self.profiler = ModelProfiler()
        logger.info("LevelSelector active (Hardware-Aware)")

    async def execute(self, pipeline: Any, **kwargs) -> HandlerResult:
        """
        Select the optimal level based on Reality and Hardware.
        """
        cell = pipeline.cell
        anchor = get_anchor(cell.reality)
        metabolic_cap = self.profiler.estimate_capacity()
        
        # Strategy:
        # 1. Start with the Reality's default level
        selected_level = anchor.default_level
        
        # 2. Downgrade if Hardware Fit is poor (< 30%)
        if metabolic_cap.memory_fit_score < 0.3 and selected_level == ConsciousnessLevel.MACRO:
            logger.warning("LevelSelector: Poor memory fit (%.1f%%). Downgrading to MICRO.", metabolic_cap.memory_fit_score * 100)
            selected_level = ConsciousnessLevel.MICRO
            
        # 3. Upgrade if Priority is high and Hardware allows
        if anchor.metabolic_priority > 0.8 and metabolic_cap.cycles_per_second > 10.0:
            if selected_level == ConsciousnessLevel.MICRO:
                logger.info("LevelSelector: High priority reality (%s). Upgrading to MACRO.", cell.reality)
                selected_level = ConsciousnessLevel.MACRO

        pipeline.level = selected_level
        
        # Announce the choice to the bus
        from cynic.kernel.core.event_bus import get_core_bus
        await get_core_bus().emit(Event.typed(
            CoreEvent.LOD_CHANGED,
            {
                "level": selected_level.name, 
                "reality": cell.reality, 
                "fit": float(metabolic_cap.memory_fit_score)
            }
        ))

        return HandlerResult(
            success=True,
            handler_id=self.handler_id,
            output={"selected_level": selected_level.name}
        )
