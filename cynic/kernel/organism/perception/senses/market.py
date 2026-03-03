"""
CYNIC Market Sensor " Financial Reality Perception (1).

Polls external market data (Solana, BTC, etc.) and injects it into
the Organism's nervous system as a MARKET reality.
"""
from __future__ import annotations

import asyncio
import logging
import random
from typing import Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.realities import MarketPayload

logger = logging.getLogger("cynic.senses.market")


class MarketSensor:
    """
    Connects CYNIC to the pulse of the markets.
    """
    def __init__(self, bus: EventBus, interval_s: float = 60.0, vascular: Optional[Any] = None):
        self.interval_s = interval_s
        self.vascular = vascular
        self._running = False
        self._task: asyncio.Task | None = None
        self._bus = bus

    def start(self):
        if self._running:
            return
        self._running = True
        self._task = asyncio.create_task(self._run_loop())
        logger.info("MarketSensor: Started polling.")

    def stop(self):
        self._running = False
        if self._task:
            self._task.cancel()

    async def _run_loop(self):
        while self._running:
            try:
                await self.perceive_market()
                await asyncio.sleep(self.interval_s)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"MarketSensor: Error: {e}")
                await asyncio.sleep(10)

    async def perceive_market(self):
        """Fetch and emit market data."""
        # For this E2E proof, we simulate a 'Real' market call
        # In production, this would use httpx.get('https://api.binance.com/...')
        
        # Simulated SOL price with high volatility to trigger CYNIC
        price = 144.0 + random.uniform(-10, 10) 
        volatility = random.uniform(0, 1.0)
        
        payload = MarketPayload(
            symbol="SOL",
            price=price,
            change_24h=random.uniform(-5, 5),
            volatility=volatility,
            source="simulated_binance"
        )
        
        logger.debug(f"MarketSensor: Perceived SOL at ${price:.2f} (vol={volatility:.2f})")
        
        # Emit to NERVES
        await self._bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            payload.model_dump(),
            source="market_sensor"
        ))
