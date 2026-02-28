"""Unified state manager collecting from all three entities: CYNIC, Human, Machine.

SymbioticStateManager aggregates observations from:
- HumanStateTracker: Energy, focus, intentions, feedback, values, growth areas
- MachineMonitor: CPU, memory, disk, network, temperature, health
- CYNIC Core: Observations, thinking, planning, confidence, E-Score

Returns a unified, immutable SymbioticState snapshot on demand.
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Optional, Any

from cynic.kernel.observability.human_state_tracker import HumanStateTracker
from cynic.kernel.observability.machine_monitor import MachineMonitor
from cynic.kernel.observability.models import SymbioticState

logger = logging.getLogger("cynic.kernel.observability.symbiotic")

# Global singleton instance
_INSTANCE: Optional[SymbioticStateManager] = None
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
        self._last_snapshot: Optional[SymbioticState] = None
        self._organism = None # Set via set_organism

    def set_organism(self, organism: Any) -> None:
        """Connect to the CYNIC organism."""
        self._organism = organism

    async def get_current_snapshot(self) -> SymbioticState:
        """
        Produce a unified snapshot of the symbiotic state.
        """
        # 1. Collect human metrics
        human = await self.human_tracker.get_snapshot()
        
        # 2. Collect machine metrics
        machine = await self.machine_monitor.get_snapshot()
        
        # 3. Collect CYNIC core metrics
        # (Mapping from OrganismState to SymbioticState expected format)
        cynic_metrics = {}
        if self._organism:
            stats = self._organism.state.get_stats()
            cynic_metrics = {
                "total_judgments": stats.get("total_judgments", 0),
                "consciousness_level": stats.get("consciousness_level", "REFLEX"),
                "confidence": 0.618, # Default if not computed
            }

        snapshot = SymbioticState(
            human=human,
            machine=machine,
            cynic=cynic_metrics,
            timestamp=time.time()
        )
        self._last_snapshot = snapshot
        return snapshot

async def get_symbiotic_state_manager() -> SymbioticStateManager:
    """Get or create the global SymbioticStateManager singleton."""
    global _INSTANCE
    async with _LOCK:
        if _INSTANCE is None:
            human = HumanStateTracker()
            machine = MachineMonitor()
            _INSTANCE = SymbioticStateManager(human, machine)
            
            # Try to auto-connect to global organism if available
            try:
                from cynic.interfaces.api.state import get_state
                _INSTANCE.set_organism(get_state())
            except Exception:
                pass
                
    return _INSTANCE
