"""
Metabolic Throttler - Robotics / SRE Lens.

Implements somatic backpressure. When the HardwareBody feels pain (high CPU/RAM),
the Throttler slows down the internal consciousness loops.

Axiom Alignment: BURN - Prevent exhaustion by breathing.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Optional

from cynic.kernel.organism.metabolism.embodiment import HardwareBody
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.phi import PHI, PHI_INV

logger = logging.getLogger("cynic.kernel.organism.metabolism.throttler")


class MetabolicThrottler:
    """
    Applies active backpressure (sleep) based on somatic pain.
    """

    def __init__(self, body: Optional[HardwareBody] = None):
        self.body = body

    async def wait_for_breath(self, level: ConsciousnessLevel) -> None:
        """
        Pause the current worker if metabolic cost is too high.
        Higher consciousness levels (META, MACRO) are throttled more than basic reflexes.
        """
        if not self.body:
            return  # No body, no pain

        # Base cost is 1.0. If CPU > 90%, cost can spike to 3.0+
        cost = self.body.get_metabolic_cost()

        # If system is healthy, do not throttle at all (cost <= 1.1)
        if cost < 1.1:
            return

        # Calculate pain multiplier
        pain = cost - 1.0

        # Different levels have different pain tolerance
        # Reflexes need to stay fast, so they are barely throttled
        # Meta thoughts can wait if CPU is burning
        level_multipliers = {
            ConsciousnessLevel.REFLEX: 0.1 * PHI_INV,
            ConsciousnessLevel.MICRO: 0.5 * PHI_INV,
            ConsciousnessLevel.MACRO: 1.0 * PHI,
            ConsciousnessLevel.META: 2.0 * PHI,
        }

        multiplier = level_multipliers.get(level, 1.0)

        # Sleep time is proportional to pain and consciousness level
        sleep_time = pain * multiplier

        if sleep_time > 0.05:
            logger.debug(
                f"Somatic Throttling: Level {level.name} sleeping for {sleep_time:.3f}s (Cost: {cost:.2f})"
            )
            await asyncio.sleep(sleep_time)
