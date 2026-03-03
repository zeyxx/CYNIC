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
from cynic.kernel.security.siem_kill_chain import SiemCorrelationEngine, SecurityIncident, SiemPriority

logger = logging.getLogger("cynic.kernel.organism.metabolism.throttler")


class MetabolicThrottler:
    """
    Applies active backpressure (sleep) based on somatic pain and Security Incidents.
    """

    def __init__(self, body: Optional[HardwareBody] = None):
        self.body = body
        self.active_incident: Optional[SecurityIncident] = None

    def trigger_incident(self, anomaly_type: str, context: str) -> None:
        """SIEM L1/L2: Classify anomaly and update internal state."""
        incident = SiemCorrelationEngine.classify_anomaly(anomaly_type, context)
        logger.warning(f"🚨 SIEM ALERT: [Stage {incident.stage.value}] {incident.priority.name} - {incident.description}")
        
        if incident.priority in (SiemPriority.HIGH, SiemPriority.CRITICAL):
            self.active_incident = incident
            logger.critical("Metabolic Throttler engaging EMERGENCY FREEZE PROTOCOL.")

    async def wait_for_breath(self, level: ConsciousnessLevel) -> None:
        """
        Pause the current worker if metabolic cost is too high OR if an incident is active.
        """
        # SIEM L3: Incident Response (Freeze Protocol)
        if self.active_incident and not self.active_incident.remediation_applied:
            # Only reflexes are allowed to run during a critical incident (to try and heal)
            if level != ConsciousnessLevel.REFLEX:
                logger.debug(f"SIEM FREEZE: {level.name} suspended due to active incident.")
                await asyncio.sleep(5.0) # Massive penalty to stop the kill chain
                return

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
