"""Unified state manager collecting from all three entities: CYNIC, Human, Machine.

SymbioticStateManager aggregates observations from:
- HumanStateTracker: Energy, focus, intentions, feedback, values, growth areas
- MachineMonitor: CPU, memory, disk, network, temperature, health
- CYNIC Core: Observations, thinking, planning, confidence, E-Score

Returns a unified, immutable SymbioticState snapshot on demand.
Uses -NET Protocol for real-time synchronization.
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Any

from cynic.kernel.observability.human_state_tracker import HumanStateTracker
from cynic.kernel.observability.machine_monitor import MachineMonitor
from cynic.kernel.observability.models import SymbioticState
from cynic.kernel.protocol.kpulse import PulseMessage

logger = logging.getLogger("cynic.kernel.observability.symbiotic")

# Global singleton instance
_INSTANCE: SymbioticStateManager | None = None
_LOCK = asyncio.Lock()


class SymbioticStateManager:
    """Collects and unifies state from Human, Machine, and CYNIC."""

    def __init__(
        self,
        human_tracker: HumanStateTracker,
        machine_monitor: MachineMonitor,
    ):
        self.human_tracker = human_tracker
        self.machine_monitor = machine_monitor
        self._last_snapshot: SymbioticState | None = None
        self._organism = None  # Set via set_organism
        self.remote_mode = False
        self.api_url = "http://localhost:58765"

        # -NET Nerve Client (IPv6 Default)
        from cynic.kernel.protocol.knet_client import KNetClient

        self.knet = KNetClient(uri="ws://[::1]:58766")
        self.knet.on_pulse(self._on_knet_pulse)
        self._last_pulse_data: dict[str, Any] = {}

    def set_organism(self, organism: Any) -> None:
        """Connect to the CYNIC organism."""
        self._organism = organism
        self.remote_mode = False

    async def start_nerves(self):
        """Start the -NET listener."""
        await self.knet.connect()

    async def _on_knet_pulse(self, pulse: PulseMessage):
        """Callback when brain sends data."""
        self.remote_mode = True
        self._last_pulse_data = pulse.data
        logger.debug(f"Nerve: Received {pulse.type.value}")

    async def get_current_snapshot(self) -> SymbioticState:
        """
        Produce a unified snapshot of the symbiotic state.
        Ensures no partial failure crashes the caller.
        """
        try:
            # 0. Ensure nerves are active
            if not self.knet._running:
                try:
                    await asyncio.wait_for(self.start_nerves(), timeout=1.0)
                except Exception as _e:
                    logger.debug(f"Silenced: {_e}")
            # 1. Collect human metrics (Local) - Default to safe empty if fails
            try:
                human = await self.human_tracker.get_snapshot()
            except Exception:
                from cynic.kernel.observability.models import HumanState

                human = HumanState(
                    energy=0.5,
                    focus=0.5,
                    intentions=[],
                    values=[],
                    feedback=[],
                    growth_areas=[],
                    timestamp=time.time(),
                )

            # 2. Collect machine metrics (Local)
            try:
                machine = await self.machine_monitor.get_snapshot()
            except Exception:
                from cynic.kernel.observability.machine_monitor import MachineState

                machine = MachineState(
                    cpu_percent=0.0,
                    memory_percent=0.0,
                    disk_percent=0.0,
                    network_bandwidth=0.0,
                    temperature=0.0,
                    health={},
                    timestamp=time.time(),
                )

            # 3. Collect CYNIC core metrics
            cynic_thinking = "Idle"
            cynic_planning = []
            cynic_confidence = 0.618
            cynic_e_score = 50.0

            # Machine resources defaults to local
            machine_resources = {
                "cpu": machine.cpu_percent,
                "ram": machine.memory_percent,
                "disk": machine.disk_percent,
            }
            machine_health = machine.health

            # 3a. Local Instance
            if self._organism:
                try:
                    stats = await self._organism.state.get_stats()
                    cynic_thinking = stats.get("current_analysis", "Processing...")
                    cynic_confidence = stats.get("confidence", 0.618)
                    if (
                        hasattr(self._organism, "escore_tracker")
                        and self._organism.escore_tracker
                    ):
                        cynic_e_score = self._organism.escore_tracker.get_total_escore()
                except Exception as e:
                    logger.debug(f"SymbioticState Local fetch error: {e}")

            # 3b. Remote Instance via -NET (Pulse)
            elif self.remote_mode and self._last_pulse_data:
                try:
                    data = self._last_pulse_data
                    mind = data.get("mind", {})
                    cynic_thinking = mind.get("thinking", "Awake (-NET)")
                    cynic_confidence = mind.get("confidence", 0.618)
                    cynic_e_score = mind.get("e_score", 50.0)

                    # Map remote hardware if available in the pulse
                    hw = data.get("hardware", {})
                    if hw:
                        machine_resources = {
                            "cpu": hw.get("cpu", 0.0),
                            "ram": hw.get("ram", 0.0),
                            "disk": hw.get("disk", 0.0),
                        }
                        machine_health = {"is_healthy": True, "remote": True}
                except Exception as _e:
                    logger.debug(f"Silenced: {_e}")
            snapshot = SymbioticState(
                # CYNIC
                cynic_observations={},
                cynic_thinking=cynic_thinking,
                cynic_planning=cynic_planning,
                cynic_confidence=cynic_confidence,
                cynic_e_score=cynic_e_score,
                # Human
                human_energy=human.energy,
                human_focus=human.focus,
                human_intentions=human.intentions,
                human_values=human.values,
                human_feedback=human.feedback,
                human_growth_areas=human.growth_areas,
                # Machine
                machine_resources=machine_resources,
                machine_constraints={},
                machine_capability_delta=[],
                machine_health=machine_health,
                # Relationship
                alignment_score=0.618,
                conflicts=[],
                mutual_influences=[],
                shared_objectives=[],
                timestamp=time.time(),
            )
            self._last_snapshot = snapshot
            return snapshot
        except Exception as e:
            logger.error(f"Critical error in get_current_snapshot: {e}")
            # Return last good snapshot if available, or a fallback
            if self._last_snapshot:
                return self._last_snapshot
            raise


async def get_symbiotic_state_manager() -> SymbioticStateManager:
    """Get or create the global SymbioticStateManager singleton."""
    global _INSTANCE
    async with _LOCK:
        if _INSTANCE is None:
            human = HumanStateTracker()
            machine = MachineMonitor()
            _INSTANCE = SymbioticStateManager(human, machine)

            # 1. Try to auto-connect to global organism (In-process)
            try:
                from cynic.interfaces.api.state import get_state

                local_org = get_state()
                if local_org:
                    _INSTANCE.set_organism(local_org)
                    return _INSTANCE
            except Exception as _e:
                logger.debug(f"Silenced: {_e}")
    return _INSTANCE


async def get_current_state() -> SymbioticState:
    """Helper to get the current symbiotic snapshot from the singleton manager."""
    mgr = await get_symbiotic_state_manager()
    return await mgr.get_current_snapshot()
