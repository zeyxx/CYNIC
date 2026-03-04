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
from cynic.kernel.core.supervisor import KernelSupervisor
from cynic.kernel.infrastructure.orchestrator import InfrastructureOrchestrator
from cynic.kernel.organism.metabolism.embodiment import UniversalHardwareBody
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm

logger = logging.getLogger("cynic.daemon")


class CynicDaemon:
    """The master controller of the CYNIC organism."""

    def __init__(self, config: Optional[CynicConfig] = None):
        self.config = config or CynicConfig()
        self.infra = InfrastructureOrchestrator()
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

        self.supervisor = KernelSupervisor()
        self._running = False

    async def run(self, cell: Any, level: Any):
        """Bridge method for the scheduler to execute cognitive tasks."""
        # This will be replaced by the real Orchestrator later
        logger.debug(f"Daemon: Processing cell at level {level}")
        await asyncio.sleep(0.01)

    async def _somatic_loop(self):
        """Maintains the physical heartbeat."""
        while self._running:
            await self.body.pulse()
            await asyncio.sleep(1.0)

    async def _cognitive_loop(self):
        """Maintains the thought rhythm."""
        self.rhythm.start()
        try:
            # Wait forever until the supervisor cancels this task
            await asyncio.Event().wait()
        except asyncio.CancelledError:
            await self.rhythm.stop()
            raise

    async def start(self):
        """Unified boot sequence."""
        if self._running:
            return
        self._running = True

        print("--- CYNIC DAEMON: AWAKENING ---")
        logger.info(f"Starting CYNIC Daemon [{self.config.instance_id}]")

        # 0. Boot Physical Infrastructure
        infra_ready = await self.infra.provision_reality()
        if not infra_ready:
            logger.critical("Daemon boot halted: Infrastructure provision failed.")
            self._running = False
            return

        # 1. Start Vascular (Pooling) - usually synchronous or self-managing

        # 2. Register organs with Supervisor
        self.supervisor.register("somatic_pulse", self._somatic_loop)
        self.supervisor.register("cognitive_rhythm", self._cognitive_loop)

        logger.info("Organism is fully awake and supervised.")

        try:
            # The supervisor will run indefinitely until stopped
            await self.supervisor.start_all()
            while self._running:
                await asyncio.sleep(0.1)
        except asyncio.CancelledError:
            await self.stop()

    async def stop(self):
        """Graceful shutdown of all organs."""
        if not self._running:
            return
        self._running = False
        print("\n--- CYNIC DAEMON: PUTTING TO SLEEP ---")
        await self.supervisor.stop_all()
        await self.vascular.close()
        logger.info("Shutdown complete.")


async def main():
    daemon = CynicDaemon()

    # Handle OS signals (gracefully handle Windows limitation)
    loop = asyncio.get_running_loop()
    try:
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, lambda: asyncio.create_task(daemon.stop()))
    except NotImplementedError:
        # Windows does not support add_signal_handler
        pass

    try:
        await daemon.start()
    except Exception as e:
        logger.critical(f"Daemon CRASHED: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
