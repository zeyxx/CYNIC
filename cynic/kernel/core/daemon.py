"""
CYNIC Daemon - The Sovereign Life Loop.
Respects Backend, SRE & Solutions Architect Lenses.

The central entry point that orchestrates the entire organism's lifecycle.
Ensures that all systems (Vascular, Somatic, Cognitive) are synchronized.
"""

from __future__ import annotations

import asyncio
import logging
import signal
import sys
from typing import Optional, Any

from cynic.config import CynicConfig
from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.core.event_bus import EventBus
from cynic.kernel.organism.metabolism.embodiment import UniversalHardwareBody
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm

logger = logging.getLogger("cynic.daemon")


class CynicDaemon:
    """The master controller of the CYNIC organism."""

    def __init__(self, config: Optional[CynicConfig] = None):
        self.config = config or CynicConfig()
        self.bus = EventBus(bus_id=self.config.instance_id)
        self.vascular = VascularSystem(
            instance_id=self.config.instance_id, redis_url=self.config.redis_url
        )
        self.body = UniversalHardwareBody(bus=self.bus)

        # Scheduler requires orchestrator, which we'll bridge here
        self.rhythm = ConsciousnessRhythm(
            orchestrator=self,  # Bridges to run()
            body=self.body,
            bus=self.bus,
        )

        self._running = False

    async def run(self, cell: Any, level: Any):
        """Bridge method for the scheduler to execute cognitive tasks."""
        # This will be replaced by the real Orchestrator later
        logger.debug(f"Daemon: Processing cell at level {level}")
        await asyncio.sleep(0.01)

    async def start(self):
        """Unified boot sequence."""
        if self._running:
            return
        self._running = True

        print("--- 🌀 CYNIC DAEMON: AWAKENING ---")
        logger.info(f"Starting CYNIC Daemon [{self.config.instance_id}]")

        # 1. Start Vascular (Pooling)
        # 2. Start Body (Senses)
        # 3. Start Rhythm (Cognition)
        self.rhythm.start()

        logger.info("Organism is fully awake and synchronized.")

        try:
            while self._running:
                # Standard pulse
                await self.body.pulse()
                await asyncio.sleep(1.0)
        except asyncio.CancelledError:
            await self.stop()

    async def stop(self):
        """Graceful shutdown of all organs."""
        if not self._running:
            return
        self._running = False
        print("\n--- 💤 CYNIC DAEMON: PUTTING TO SLEEP ---")
        await self.rhythm.stop()
        await self.vascular.close()
        logger.info("Shutdown complete.")


async def main():
    daemon = CynicDaemon()

    # Handle OS signals
    loop = asyncio.get_running_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, lambda: asyncio.create_task(daemon.stop()))

    try:
        await daemon.start()
    except Exception as e:
        logger.critical(f"Daemon CRASHED: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
