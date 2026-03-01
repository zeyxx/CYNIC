"""
OrganismState — Single source of truth for all CYNIC state.

Unified state system with 3 layers:
  1. Memory (RAM): Fast, volatile state (Q-table, dogs, recent judgments)
  2. Persistent (DB/Graph): Long-term memory (Communities, Proposals, History)
  3. Checkpoint (Disk): Periodic snapshots for recovery
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING, Any, Dict, List, Optional, Union

from cynic.kernel.core.event_bus import CoreEvent, get_core_bus
from cynic.kernel.core.unified_state import (
    GovernanceCommunity,
    GovernanceProposal,
    GovernanceVote,
    UnifiedConsciousState,
    UnifiedJudgment,
    UnifiedLearningOutcome,
)

if TYPE_CHECKING:
    from cynic.kernel.core.storage.surreal import SurrealStorage

logger = logging.getLogger("cynic.kernel.organism.state_manager")


class StateLayer(Enum):
    MEMORY = "memory"
    PERSISTENT = "persistent"
    CHECKPOINT = "checkpoint"


@dataclass
class StateUpdate:
    key: str
    value: Any
    layer: StateLayer = StateLayer.MEMORY
    source: str = "internal"
    timestamp: float = field(default_factory=time.time)


@dataclass(frozen=True)
class OrganismSnapshot:
    """An immutable point-in-time view of the organism state."""
    total_judgments: int
    consciousness_level: str
    active_axioms: List[str]
    cycles: Dict[str, int]
    memory_keys: int
    persistent_keys: int
    timestamp: float = field(default_factory=lambda: time.time())


class OrganismState:
    """
    Manages the organism's memory and persistence.
    Requires an explicit instance_id for isolation.
    """

    def __init__(self, instance_id: str, storage_dir: str | None = None, storage: Optional[SurrealStorage] = None):
        self.instance_id = instance_id
        self.storage_dir = Path(
            storage_dir or os.path.join(os.path.expanduser("~"), ".cynic", "organism_state")
        )
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.storage = storage

        # Internal State Layers
        self._memory_state: dict[str, Any] = {}
        self._persistent_state: dict[str, Any] = {}
        self._checkpoint_state: dict[str, Any] = {}

        # The Living View
        self.consciousness = UnifiedConsciousState()
        self.total_judgments = 0
        self.total_spent_usd = 0.0

        # Cycle counters
        self.reflex_cycles = 0
        self.micro_cycles = 0
        self.macro_cycles = 0
        self.meta_cycles = 0
        self.total_cycles = 0

        # Axiom state
        self.active_axioms: list[str] = []
        self.emergent_states: dict[str, bool] = {}
        self.learned_weights: dict[str, dict[str, float]] = {}

        self._last_snapshot: Optional[OrganismSnapshot] = None
        self._update_queue: asyncio.Queue[Optional[StateUpdate]] = asyncio.Queue()
        self._processing = False
        self._loop_task: asyncio.Task | None = None
        self._lock = asyncio.Lock()

    async def get_stats(self) -> dict:
        """thread-safe statistics."""
        async with self._lock:
            return {
                "instance_id": self.instance_id,
                "total_judgments": self.total_judgments,
                "total_spent_usd": round(self.total_spent_usd, 4),
                "memory_keys": len(self._memory_state),
                "persistent_keys": len(self._persistent_state),
                "cycles": {
                    "reflex": self.reflex_cycles,
                    "micro": self.micro_cycles,
                    "macro": self.macro_cycles,
                    "meta": self.meta_cycles,
                    "total": self.total_cycles,
                },
            }

    async def record_action_cost(self, cost_usd: float) -> None:
        """Track metabolic spending (BURN)."""
        async with self._lock:
            self.total_spent_usd += cost_usd
        await self.update("total_spent_usd", self.total_spent_usd, layer=StateLayer.PERSISTENT)

    async def start_processing(self) -> None:
        if self._processing:
            return
        self._processing = True
        await self.recover()
        self._loop_task = asyncio.create_task(self._process_updates_loop())
        logger.info(f"[{self.instance_id}] State respiration started.")

    async def stop_processing(self) -> None:
        self._processing = False
        if self._loop_task:
            await self._update_queue.put(None)
            await self._loop_task
        await self.save_checkpoint()
        logger.info(f"[{self.instance_id}] State processing stopped.")

    async def update(self, key: str, value: Any, layer: StateLayer = StateLayer.MEMORY, source: str = "internal") -> bool:
        await self._update_queue.put(StateUpdate(key=key, value=value, layer=layer, source=source))
        return True

    async def query(self, key: str, default: Any = None) -> Any:
        async with self._lock:
            return self._memory_state.get(key, self._persistent_state.get(key, self._checkpoint_state.get(key, default)))

    async def add_judgment(self, judgment: Any) -> None:
        # Simplified for Round 5 focus
        async with self._lock:
            self.total_judgments += 1
        if self.storage:
            try:
                j_dict = judgment.to_dict() if hasattr(judgment, "to_dict") else str(judgment)
                asyncio.create_task(self.storage.judgments.save(j_dict))
            except Exception as e:
                logger.debug(f"SurrealDB save failed: {e}")

    async def _process_updates_loop(self) -> None:
        while self._processing:
            try:
                update = await asyncio.wait_for(self._update_queue.get(), timeout=0.5)
                if update is None: break
                async with self._lock:
                    if update.layer == StateLayer.MEMORY: self._memory_state[update.key] = update.value
                    elif update.layer == StateLayer.PERSISTENT: self._persistent_state[update.key] = update.value
                self._update_queue.task_done()
            except (asyncio.TimeoutError, TimeoutError): continue
            except Exception as e: logger.error(f"State update error: {e}")

    async def recover(self) -> None:
        cp_path = self.storage_dir / "state_checkpoint.json"
        if cp_path.exists():
            try:
                with open(cp_path) as f:
                    data = json.load(f)
                    async with self._lock: self._checkpoint_state = data
            except Exception: pass

    async def save_checkpoint(self) -> None:
        cp_path = self.storage_dir / "state_checkpoint.json"
        try:
            async with self._lock: snapshot = dict(self._persistent_state)
            with open(cp_path, "w") as f: json.dump(snapshot, f, indent=2, default=str)
        except Exception: pass
