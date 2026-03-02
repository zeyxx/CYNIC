"""
Test Stimulus Generator — Inject test market data for E2E testing.

Simulates market events without requiring external sources.
Allows CYNIC to perceive, judge, and respond to synthetic stimulus.
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Optional

from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event
from cynic.kernel.core.events_schema import PerceptionReceivedPayload

logger = logging.getLogger("cynic.perception.test_stimulus")


class TestStimulusGenerator:
    """Inject synthetic market data for testing."""

    def __init__(self, bus: EventBus):
        self.bus = bus
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._last_multiplier = 1.0

    async def inject_market_spike(self, multiplier: float = 42.0):
        """Inject a single market data point (simulating cannon.pumpparty.com)."""
        perception = PerceptionReceivedPayload(
            reality="GAMBLING",
            analysis="PERCEIVE",
            data={
                "game_id": "cannon-test",
                "multiplier": multiplier,
                "timestamp": time.time(),
            },
            risk=0.5,
            run_judgment=True,
        )

        await self.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                perception,
                source="test_stimulus",
            )
        )

        logger.info(f"TestStimulus: Injected market spike (multiplier={multiplier})")

    async def inject_anomaly(self, anomaly_type: str = "LATENCY_SPIKE", value: float = 0.85):
        """Inject a system anomaly for self-prober detection."""
        perception = PerceptionReceivedPayload(
            reality="INTERNAL",
            analysis="ANOMALY",
            data=f"Test anomaly: {anomaly_type}={value}",
            risk=0.7,
            run_judgment=True,
        )

        await self.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                perception,
                source="test_stimulus",
            )
        )

        logger.info(f"TestStimulus: Injected anomaly ({anomaly_type}={value})")

    async def start_market_simulation(self, duration_s: float = 30.0):
        """Run market simulation for testing."""
        if self._running:
            return

        self._running = True
        logger.info(f"TestStimulus: Starting market simulation ({duration_s}s)")

        start = time.time()
        iteration = 0

        try:
            while self._running and (time.time() - start) < duration_s:
                # Simulate market movement
                base = 1.0
                if iteration == 5:
                    mult = 42.0  # Spike
                elif iteration == 10:
                    mult = 0.5   # Crash
                else:
                    mult = base + (iteration % 3) * 0.5

                await self.inject_market_spike(mult)

                # Occasional anomalies
                if iteration % 7 == 0:
                    await self.inject_anomaly("RATE_SPIKE", 0.8)

                await asyncio.sleep(2)  # Stimulus every 2 seconds
                iteration += 1

        finally:
            self._running = False
            logger.info("TestStimulus: Market simulation complete")

    async def stop(self):
        """Stop stimulus generation."""
        self._running = False
        if self._task:
            self._task.cancel()
