"""
SovereigntyAgent — Amplifying creator impact.

Subscribes to VALUE_CREATED events and calculates multidimensional impact:
- Direct: Immediate utility of the creation.
- Indirect: How much it helps other creators (Symbiosis).
- Collective: Benefit to the whole Organism.

This agent turns raw actions into Sovereign Reputation (E-Score).
"""
from __future__ import annotations
import logging
import time
from typing import Any, Dict, List, Optional

from cynic.kernel.core.event_bus import get_core_bus, CoreEvent, Event
from cynic.kernel.core.unified_state import ValueCreation, ImpactMeasurement
from cynic.kernel.core.phi import PHI, PHI_INV

logger = logging.getLogger("cynic.kernel.organism.brain.agents.sovereignty")

class SovereigntyAgent:
    """
    Calculates and persists the impact of every creator in the system.
    """
    def __init__(self, state_manager: Any):
        self.state = state_manager
        self._total_value_observed = 0.0

    def start(self):
        """Subscribe to the nervous system."""
        bus = get_core_bus()
        bus.on(CoreEvent.VALUE_CREATED, self._on_value_created)
        logger.info("SovereigntyAgent active — measuring impact.")

    async def _on_value_created(self, event: Event) -> None:
        """A new creation event has occurred."""
        try:
            p = event.dict_payload
            creation = ValueCreation.model_validate(p)
            
            # Calculate Impact
            impact = await self.calculate_impact(creation)
            
            # Persist back to memory
            await self.state.add_impact_measurement(impact)
            
            # Signal the E-Score tracker to update reputation
            await get_core_bus().emit(Event.typed(
                CoreEvent.Q_TABLE_UPDATED, # Trigger reputation recalculation
                {
                    "agent_id": creation.creator_id,
                    "impact_score": impact.total_impact,
                    "source": "sovereignty_agent"
                }
            ))
            
            logger.info(
                "Sovereignty: Measured impact for %s (Total: %.2f)", 
                creation.creator_id, impact.total_impact
            )
            
        except Exception as e:
            logger.error("SovereigntyAgent failed to process value: %s", e)

    async def calculate_impact(self, creation: ValueCreation) -> ImpactMeasurement:
        """
        Multidimensional impact calculation based on PHI weighting.
        """
        # 1. Base scores (from metadata or analysis)
        direct = creation.direct_impact or 0.5
        indirect = creation.indirect_impact or 0.3
        collective = creation.collective_impact or 0.2
        temporal = creation.temporal_impact or 0.5
        
        # 2. Weighted Sum (PHI hierarchy)
        # Direct (PHI) > Indirect (1.0) > Collective (PHI_INV)
        total_impact = (direct * PHI) + (indirect * 1.0) + (collective * PHI_INV)
        
        # Normalize
        total_impact = total_impact / (PHI + 1.0 + PHI_INV)
        
        # 3. Governance Weight (how much say this person has)
        # Floor is PHI_INV_3 (0.236), scales with total impact
        gov_weight = 0.236 + (total_impact * 0.382)
        
        return ImpactMeasurement(
            human_id=creation.creator_id,
            total_impact=round(total_impact, 4),
            dimension_scores={
                "direct": direct,
                "indirect": indirect,
                "collective": collective,
                "temporal": temporal
            },
            governance_weight=round(gov_weight, 4)
        )
