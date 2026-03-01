"""
OrganismState — Single source of truth for all CYNIC state.

Unified state system with 3 layers:
  1. Memory (RAM): Fast, volatile state (Q-table, dogs, recent judgments)
  2. Persistent (DB/Graph): Long-term memory (Communities, Proposals, History)
  3. Checkpoint (Disk): Periodic snapshots for recovery

Implements the "Hippocampus" of the organism.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from cynic.kernel.core.unified_state import (
    GovernanceCommunity,
    GovernanceProposal,
    GovernanceVote,
    UnifiedConsciousState,
    UnifiedJudgment,
    UnifiedLearningOutcome,
)

logger = logging.getLogger("cynic.kernel.organism.state_manager")


class StateLayer(Enum):
    MEMORY = "memory"
    PERSISTENT = "persistent"
    CHECKPOINT = "checkpoint"


@dataclass
class StateUpdate:
    key: str
    value: Any
    layer: StateLayer
    source: str
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
    timestamp: float = field(default_factory=time.time)


class OrganismState:
    """
    Manages the organism's memory and persistence.
    Thread-safe implementation for concurrent UI/Kernel access.
    """

    def __init__(self, storage_dir: str | None = None):
        self.storage_dir = Path(
            storage_dir or os.path.join(os.path.expanduser("~"), ".cynic", "organism_state")
        )
        self.storage_dir.mkdir(parents=True, exist_ok=True)

        # Internal State Layers
        self._memory_state: dict[str, Any] = {}
        self._persistent_state: dict[str, Any] = {}
        self._checkpoint_state: dict[str, Any] = {}

        # The Living View (Consciousness)
        self.consciousness = UnifiedConsciousState()
        self.total_judgments = 0

        # Cycle counters
        self.reflex_cycles = 0
        self.micro_cycles = 0
        self.macro_cycles = 0
        self.meta_cycles = 0
        self.total_cycles = 0

        # Axiom state (for AxiomArchitecture)
        self.active_axioms: list[str] = []
        self.emergent_states: dict[str, bool] = {}
        self.activation_log: list[dict[str, Any]] = []
        self.learned_weights: dict[str, dict[str, float]] = {}

        # The Snapshot (Double Buffer for Scalability)
        self._last_snapshot: Optional[OrganismSnapshot] = None

        # Async Pipeline
        self._update_queue: asyncio.Queue[StateUpdate] = asyncio.Queue()
        self._processing = False
        self._loop_task: asyncio.Task | None = None

        # Concurrency protection
        self._lock = threading.RLock()  # Use threading.RLock for synchronous accessors

    def get_snapshot(self) -> OrganismSnapshot:
        """Return the latest immutable snapshot. Lock-free after first take."""
        if self._last_snapshot is None:
            self.take_snapshot()
        return self._last_snapshot

    def take_snapshot(self) -> None:
        """Capture current state into an immutable snapshot."""
        with self._lock:
            self._last_snapshot = OrganismSnapshot(
                total_judgments=self.total_judgments,
                consciousness_level=self.get_consciousness_level(),
                active_axioms=list(self.active_axioms),
                cycles={
                    "reflex": self.reflex_cycles,
                    "micro": self.micro_cycles,
                    "macro": self.macro_cycles,
                    "meta": self.meta_cycles,
                    "total": self.total_cycles,
                },
                memory_keys=len(self._memory_state),
                persistent_keys=len(self._persistent_state),
            )

    # ── LIFECYCLE ───────────────────────────────────────────────────────

    async def start_processing(self, db: Any | None = None) -> None:
        """Start the background respiration (update loop)."""
        if self._processing:
            return

        self._processing = True
        # Load last checkpoint if exists
        await self.recover()

        self._loop_task = asyncio.create_task(self._process_updates_loop())
        logger.info("OrganismState respiration started (storage=%s)", self.storage_dir)

    async def stop_processing(self) -> None:
        """Graceful shutdown: flush and save."""
        self._processing = False
        if self._loop_task:
            # Wake up the loop if it's waiting on the queue
            self._update_queue.put_nowait(None)  # type: ignore
            await self._loop_task

        await self.save_checkpoint()

        # Drain EventBus (nervous system)
        from cynic.kernel.core.event_bus import get_core_bus

        await get_core_bus().drain(timeout=1.0)

        logger.info("OrganismState processing stopped")

    # ── CORE OPERATIONS ─────────────────────────────────────────────────

    async def update(
        self, key: str, value: Any, layer: StateLayer = StateLayer.MEMORY, source: str = "internal"
    ) -> bool:
        """Queue a state update."""
        update = StateUpdate(key=key, value=value, layer=layer, source=source)
        await self._update_queue.put(update)
        return True

    def query(self, key: str, default: Any = None) -> Any:
        """Read state (Memory > Persistent > Checkpoint)."""
        with self._lock:
            if key in self._memory_state:
                return self._memory_state[key]
            if key in self._persistent_state:
                return self._persistent_state[key]
            if key in self._checkpoint_state:
                return self._checkpoint_state[key]
            return default

    # ── CONSCIOUSNESS INTEGRATION ───────────────────────────────────────

    async def add_judgment(self, judgment: Any) -> None:
        """Record a judgment in consciousness and queue for persistence."""
        if not isinstance(judgment, UnifiedJudgment):
            if isinstance(judgment, dict):
                # Convert dict to UnifiedJudgment (dataclass, not Pydantic)
                # Filter dict to only include fields that UnifiedJudgment expects
                import dataclasses

                unified_fields = {f.name for f in dataclasses.fields(UnifiedJudgment)}
                filtered_dict = {k: v for k, v in judgment.items() if k in unified_fields}
                try:
                    judgment = UnifiedJudgment(**filtered_dict)
                except (TypeError, ValueError) as e:
                    logger.debug(f"Could not create UnifiedJudgment from dict: {e}")
                    return
            else:
                return

        with self._lock:
            self.consciousness.add_judgment(judgment)
            self.total_judgments += 1

        await self.update(
            "judg:recent", list(self.consciousness.recent_judgments.buffer), layer=StateLayer.MEMORY
        )
        await self.update(
            f"judg:record:{judgment.judgment_id}", judgment, layer=StateLayer.PERSISTENT
        )

    async def update_consciousness_level(self, level: str) -> None:
        """Update global consciousness level."""
        with self._lock:
            self.consciousness.consciousness_level = level
        await self.update("consciousness_level", level, layer=StateLayer.PERSISTENT)

    def get_consciousness_level(self) -> str:
        return self.query("consciousness_level", "REFLEX")

    # ── GOVERNANCE SUBSYSTEM (Phase 3) ───────────────────────────────────

    async def register_community(self, community: GovernanceCommunity) -> bool:
        """Register or update a governance community."""
        key = f"gov:community:{community.community_id}"
        await self.update(key, community, layer=StateLayer.PERSISTENT, source="governance_bot")
        with self._lock:
            self.consciousness.add_community(community)
        return True

    async def submit_proposal(self, proposal: GovernanceProposal) -> bool:
        """Submit a new governance proposal."""
        key = f"gov:proposal:{proposal.proposal_id}"
        await self.update(key, proposal, layer=StateLayer.PERSISTENT, source="governance_bot")
        with self._lock:
            self.consciousness.add_proposal(proposal)
        return True

    async def record_vote(self, vote: GovernanceVote) -> bool:
        """Record a user vote."""
        key = f"gov:vote:{vote.vote_id}"
        await self.update(key, vote, layer=StateLayer.PERSISTENT, source="governance_bot")
        return True

    def get_proposal(self, proposal_id: str) -> GovernanceProposal | None:
        return self.query(f"gov:proposal:{proposal_id}")

    # ── READ ACCESSORS ──────────────────────────────────────────────────

    def get_recent_judgments(self, limit: int = 10) -> list[UnifiedJudgment]:
        """Retrieve recent judgments (newest first)."""
        with self._lock:
            buffer = list(self.consciousness.recent_judgments.buffer)
            recent = buffer[-limit:]
            return list(reversed(recent))

    def get_recent_outcomes(self, limit: int = 10) -> list[UnifiedLearningOutcome]:
        """Retrieve recent learning outcomes."""
        with self._lock:
            buffer = list(self.consciousness.learning_outcomes.buffer)
            recent = buffer[-limit:]
            return list(reversed(recent))

    def get_stats(self) -> dict:
        """Get comprehensive organism state statistics (thread-safe)."""
        with self._lock:
            return {
                "total_judgments": self.total_judgments,
                "consciousness_level": self.get_consciousness_level(),
                "memory_keys": len(self._memory_state),
                "persistent_keys": len(self._persistent_state),
                "queue_size": self._update_queue.qsize(),
                "communities": len(self.consciousness.communities.buffer),
                "proposals": len(self.consciousness.proposals.buffer),
                "cycles": {
                    "reflex": self.reflex_cycles,
                    "micro": self.micro_cycles,
                    "macro": self.macro_cycles,
                    "meta": self.meta_cycles,
                    "total": self.total_cycles,
                },
            }

    # ── INTERNALS ───────────────────────────────────────────────────────

    async def _process_updates_loop(self) -> None:
        """Background loop: process the update queue."""
        while self._processing:
            try:
                update = await asyncio.wait_for(self._update_queue.get(), timeout=0.5)
                if update is None:  # Shutdown signal
                    break

                # Apply to internal dictionaries with lock
                with self._lock:
                    if update.layer == StateLayer.MEMORY:
                        self._memory_state[update.key] = update.value
                    elif update.layer == StateLayer.PERSISTENT:
                        self._persistent_state[update.key] = update.value
                    elif update.layer == StateLayer.CHECKPOINT:
                        self._checkpoint_state[update.key] = update.value

                self._update_queue.task_done()
            except TimeoutError:
                continue
            except Exception as e:
                logger.error("State Manager update error: %s", e)

    async def recover(self) -> None:
        """Recover state from last checkpoint on disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        if cp_path.exists():
            try:
                with open(cp_path) as f:
                    data = json.load(f)
                    with self._lock:
                        self._checkpoint_state = data
                logger.info("OrganismState: recovered from checkpoint")
            except Exception as e:
                logger.warning("OrganismState recovery failed: %s", e)

    async def save_checkpoint(self) -> None:
        """Save a snapshot of persistent state to disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        try:
            # Snapshot state while locked
            with self._lock:
                snapshot = dict(self._persistent_state)

            # Write to disk (outside lock)
            with open(cp_path, "w") as f:
                json.dump(snapshot, f, indent=2, default=str)
            logger.info("OrganismState: saved checkpoint to %s", cp_path)
        except Exception as e:
            logger.error("Checkpoint failed: %s", e)
