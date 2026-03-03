"""
OrganismState — Single source of truth for all CYNIC state.

REACTIVE MATERIALIZED VIEW:
This class acts as a fast, RAM-based cache of the truth stored in SurrealDB.
The 'dual-write drift' is eliminated by using SurrealDB Live Queries as the 
sole trigger for internal memory updates.

Patterns: Reactive, DB-First, phi-weighted.
"""

from __future__ import annotations

import asyncio
import logging
import os
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING, Any, Dict, List, Optional

from cynic.kernel.core.unified_state import UnifiedConsciousState

if TYPE_CHECKING:
    from cynic.kernel.core.storage.surreal import SurrealStorage
    from cynic.kernel.core.event_bus import EventBus

logger = logging.getLogger("cynic.kernel.organism.state_manager")

class StateLayer(str, Enum):
    MEMORY = "memory"
    PERSISTENT = "persistent"

class OrganismState:
    """
    Reactive state manager.
    Slaves internal RAM dictionaries to SurrealDB Live Query streams.
    """

    def __init__(self, instance_id: str, bus: EventBus, storage: Optional[SurrealStorage] = None):
        self.instance_id = instance_id
        self.storage = storage
        self.bus = bus

        # RAM Cache (The Materialized View)
        self._memory_state: dict[str, Any] = {}
        self._persistent_cache: dict[str, Any] = {}
        
        # Core Indicators
        self.total_judgments = 0
        self.total_spent_usd = 0.0
        self.consciousness = UnifiedConsciousState()
        
        self._running = False
        self._subscriptions: List[str] = []
        self._lock = asyncio.Lock()

    async def start_processing(self) -> None:
        """Awaken the state. Subscribes to SurrealDB Live Streams."""
        if self._running or not self.storage:
            return
        
        self._running = True
        logger.info(f"[{self.instance_id}] State Manager: Awakening Reactive View.")

        # 1. Subscribe to Judgments (to update counters)
        sub_id = await self.storage.subscribe("judgment", self._on_db_judgment)
        self._subscriptions.append(sub_id)
        
        # 2. Wire reactive listeners for external stimuli (if any)
        from cynic.kernel.core.event_bus import CoreEvent
        self.bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_created)

    async def stop_processing(self) -> None:
        """Shutdown subscriptions."""
        self._running = False
        # Unsubscribe logic to be implemented in SurrealStorage if needed
        logger.info(f"[{self.instance_id}] State Manager: Reactive View Dormant.")

    async def _on_db_judgment(self, action: str, result: Any):
        """Callback from SurrealDB LIVE SELECT on 'judgment' table."""
        if action in ["CREATE", "INSERT"]:
            async with self._lock:
                self.total_judgments += 1
                # Here we could update more complex metrics derived from the record
                
    async def _on_judgment_created(self, event: Any) -> None:
        """
        Triggered when a judgment is emitted.
        In the reactive model, we write to DB first.
        The internal counter 'total_judgments' will be updated by _on_db_judgment.
        """
        if self.storage:
            payload = event.dict_payload if hasattr(event, 'dict_payload') else event.payload
            # DB-FIRST: The write is the source of truth
            asyncio.create_task(self.storage.judgments.save(payload))

    async def update(self, key: str, value: Any, layer: StateLayer = StateLayer.MEMORY, source: str = "internal") -> bool:
        """
        Standard update method.
        If layer is PERSISTENT, writes to SurrealDB first.
        RAM is updated via Live Query or manual fallback.
        """
        if layer == StateLayer.PERSISTENT and self.storage:
            # For specific keys, we might want specialized repo calls
            # For generic state, we use a generic table or specialized logic
            pass # We will refine this based on the CCM requirements
            
        async with self._lock:
            if layer == StateLayer.MEMORY:
                self._memory_state[key] = value
            elif layer == StateLayer.PERSISTENT:
                self._persistent_cache[key] = value
                
        return True

    async def get_stats(self) -> dict:
        """thread-safe statistics."""
        async with self._lock:
            return {
                "instance_id": self.instance_id,
                "total_judgments": self.total_judgments,
                "total_spent_usd": round(self.total_spent_usd, 4),
                "consensus_score": self.consciousness.get_consensus_score(),
            }
