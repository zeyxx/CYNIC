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
    timestamp: float = field(default_factory=time.time)


class OrganismState:
    """
    Manages the organism's memory and persistence.
    Thread-safe implementation for concurrent UI/Kernel access.
    """

    def __init__(self, storage_dir: str | None = None, storage: Optional[SurrealStorage] = None):
        self.storage_dir = Path(
            storage_dir or os.path.join(os.path.expanduser("~"), ".cynic", "organism_state")
        )
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.storage = storage  # Real persistence layer (SurrealDB)

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
        self._lock = asyncio.Lock()

    async def get_snapshot(self) -> OrganismSnapshot:
        """Return the latest immutable snapshot. Lock-free after first take."""
        if self._last_snapshot is None:
            await self.take_snapshot()
        return self._last_snapshot

    async def take_snapshot(self) -> None:
        """Capture current state into an immutable snapshot."""
        async with self._lock:
            self._last_snapshot = OrganismSnapshot(
                total_judgments=self.total_judgments,
                consciousness_level=await self.get_consciousness_level(),
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

        # Subscribe to internal events
        from cynic.kernel.core.event_bus import get_core_bus
        bus = get_core_bus()
        bus.on("organism.somatic_sensation", self._on_somatic_sensation)

        self._loop_task = asyncio.create_task(self._process_updates_loop())
        logger.info("OrganismState respiration started (storage=%s)", self.storage_dir)

    async def _on_somatic_sensation(self, event: Event) -> None:
        """Update internal stats from somatic sensors."""
        p = event.dict_payload or {}
        async with self._lock:
            self._memory_state["machine_cpu"] = p.get("cpu", 0.0)
            self._memory_state["machine_ram"] = p.get("ram", 0.0)

    async def _on_judgment_created(self, event: Event) -> None:
        """Internal handler for judgments."""
        await self.add_judgment(event.dict_payload)

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

    # ── Track E: Polling Support ────────────────────────────────────────

    async def record_pending_judgment(self, judgment_id: str) -> None:
        """Record a judgment ID as pending (Track E)."""
        # Store immediately in memory state for fast lookup
        key = f"judgment:{judgment_id}:status"
        async with self._lock:
            self._memory_state[key] = "PENDING"

        # Also queue for persistence
        await self.update(
            key=key,
            value="PENDING",
            source="api"
        )

    async def get_judgment_status(self, judgment_id: str) -> str:
        """Retrieve status of a judgment (PENDING/COMPLETED/etc)."""
        return await self.query(f"judgment:{judgment_id}:status", default="UNKNOWN")

    # ── CORE OPERATIONS ─────────────────────────────────────────────────

    async def update(
        self, key: str, value: Any, layer: StateLayer = StateLayer.MEMORY, source: str = "internal"
    ) -> bool:
        """Queue a state update."""
        update = StateUpdate(key=key, value=value, layer=layer, source=source)
        await self._update_queue.put(update)
        return True

    async def query(self, key: str, default: Any = None) -> Any:
        """Read state (Memory > Persistent > Checkpoint)."""
        async with self._lock:
            if key in self._memory_state:
                return self._memory_state[key]
            if key in self._persistent_state:
                return self._persistent_state[key]
            if key in self._checkpoint_state:
                return self._checkpoint_state[key]
            return default

    def query_sync(self, key: str, default: Any = None) -> Any:
        """Synchronous version of query. DANGEROUS: may block event loop if lock held."""
        # Check if we're in an async loop
        try:
            loop = asyncio.get_running_loop()
            if loop.is_running():
                # We are in an async loop, this is risky
                pass
        except RuntimeError:
            pass

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

        async with self._lock:
            self.consciousness.add_judgment(judgment)
            self.total_judgments += 1

        # Real Persistence (SurrealDB)
        if self.storage:
            try:
                # 1. Save Judgment record
                j_dict = judgment.to_dict() if hasattr(judgment, "to_dict") else vars(judgment)
                asyncio.create_task(self.storage.judgments.save(j_dict))
                
                # 2. Update Q-Table if it was a learning outcome
                # (Normally handled by LEARNING_EVENT, but we ensure consistency here)
                state_key = j_dict.get("state_key")
                if state_key:
                    asyncio.create_task(self.storage.qtable.update(
                        state_key=state_key,
                        action=j_dict.get("verdict", "WAG"),
                        q_value=j_dict.get("q_score", 0.0)
                    ))
            except Exception as e:
                logger.debug("SurrealDB persistence failed: %s", e)

        await self.update(
            "judg:recent", list(self.consciousness.recent_judgments.buffer), layer=StateLayer.MEMORY
        )
        await self.update(
            f"judg:record:{judgment.judgment_id}", judgment, layer=StateLayer.PERSISTENT
        )
        # Mark as completed for polling
        await self.update(
            f"judgment:{judgment.judgment_id}:status", "COMPLETED", layer=StateLayer.MEMORY
        )

    async def update_consciousness_level(self, level: str) -> None:
        """Update global consciousness level."""
        async with self._lock:
            self.consciousness.consciousness_level = level
        await self.update("consciousness_level", level, layer=StateLayer.PERSISTENT)

    async def get_consciousness_level(self) -> str:
        return await self.query("consciousness_level", "REFLEX")

    # ── GOVERNANCE SUBSYSTEM (Phase 3) ───────────────────────────────────

    async def register_community(self, community: GovernanceCommunity) -> bool:
        """Register or update a governance community."""
        key = f"gov:community:{community.community_id}"
        await self.update(key, community, layer=StateLayer.PERSISTENT, source="governance_bot")
        async with self._lock:
            self.consciousness.add_community(community)
        return True

    async def submit_proposal(self, proposal: GovernanceProposal) -> bool:
        """Submit a new governance proposal."""
        key = f"gov:proposal:{proposal.proposal_id}"
        await self.update(key, proposal, layer=StateLayer.PERSISTENT, source="governance_bot")
        async with self._lock:
            self.consciousness.add_proposal(proposal)
        return True

    async def record_vote(self, vote: GovernanceVote) -> bool:
        """Record a user vote."""
        key = f"gov:vote:{vote.vote_id}"
        await self.update(key, vote, layer=StateLayer.PERSISTENT, source="governance_bot")
        return True

    async def get_proposal(self, proposal_id: str) -> GovernanceProposal | None:
        return await self.query(f"gov:proposal:{proposal_id}")

    # ── READ ACCESSORS ──────────────────────────────────────────────────

    async def get_recent_judgments(self, limit: int = 10) -> list[UnifiedJudgment]:
        """Retrieve recent judgments (newest first)."""
        async with self._lock:
            buffer = list(self.consciousness.recent_judgments.buffer)
            recent = buffer[-limit:]
            return list(reversed(recent))

    async def get_recent_outcomes(self, limit: int = 10) -> list[UnifiedLearningOutcome]:
        """Retrieve recent learning outcomes."""
        async with self._lock:
            buffer = list(self.consciousness.learning_outcomes.buffer)
            recent = buffer[-limit:]
            return list(reversed(recent))

    async def get_stats(self) -> dict:
        """Get comprehensive organism state statistics (thread-safe)."""
        async with self._lock:
            return {
                "total_judgments": self.total_judgments,
                "consciousness_level": await self.get_consciousness_level(),
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
                async with self._lock:
                    if update.layer == StateLayer.MEMORY:
                        self._memory_state[update.key] = update.value
                    elif update.layer == StateLayer.PERSISTENT:
                        self._persistent_state[update.key] = update.value
                    elif update.layer == StateLayer.CHECKPOINT:
                        self._checkpoint_state[update.key] = update.value

                self._update_queue.task_done()
            except TimeoutError:
                continue
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("State Manager update error: %s", e)

    async def recover(self) -> None:
        """Recover state from last checkpoint on disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        if cp_path.exists():
            try:
                with open(cp_path) as f:
                    data = json.load(f)
                    async with self._lock:
                        self._checkpoint_state = data
                logger.info("OrganismState: recovered from checkpoint")
            except Exception as e:
                logger.warning("OrganismState recovery failed: %s", e)

    async def save_checkpoint(self) -> None:
        """Save a snapshot of persistent state to disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        try:
            # Snapshot state while locked
            async with self._lock:
                snapshot = dict(self._persistent_state)

            # Write to disk (outside lock)
            with open(cp_path, "w") as f:
                json.dump(snapshot, f, indent=2, default=str)
            logger.info("OrganismState: saved checkpoint to %s", cp_path)
        except Exception as e:
            logger.error("Checkpoint failed: %s", e)
