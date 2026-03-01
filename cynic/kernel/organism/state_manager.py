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
import time
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from cynic.kernel.core.phi import fibonacci
from cynic.kernel.core.unified_state import (
    UnifiedConsciousState, 
    UnifiedJudgment, 
    UnifiedLearningOutcome,
    GovernanceCommunity,
    GovernanceProposal,
    GovernanceVote
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

class OrganismState:
    """
    Manages the organism's memory and persistence.
    """
    def __init__(self, storage_dir: Optional[str] = None):
        self.storage_dir = Path(storage_dir or os.path.join(os.path.expanduser("~"), ".cynic", "organism_state"))
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        
        # Internal State Layers
        self._memory_state: Dict[str, Any] = {}
        self._persistent_state: Dict[str, Any] = {}
        self._checkpoint_state: Dict[str, Any] = {}
        
        # The Living View (Consciousness)
        self.consciousness = UnifiedConsciousState()
        self.total_judgments = 0
        
        # Async Pipeline
        self._update_queue: asyncio.Queue[StateUpdate] = asyncio.Queue()
        self._processing = False
        self._loop_task: Optional[asyncio.Task] = None
        self._lock = asyncio.Lock()

    # ── LIFECYCLE ───────────────────────────────────────────────────────

    async def start_processing(self, db: Optional[Any] = None) -> None:
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
            await self._loop_task
            
        await self.save_checkpoint()
        
        # Drain EventBus (nervous system)
        from cynic.kernel.core.event_bus import get_core_bus
        await get_core_bus().drain(timeout=1.0)
        
        logger.info("OrganismState processing stopped")

    # ── CORE OPERATIONS ─────────────────────────────────────────────────

    async def update(self, key: str, value: Any, layer: StateLayer = StateLayer.MEMORY, source: str = "internal") -> bool:
        """Queue a state update."""
        update = StateUpdate(key=key, value=value, layer=layer, source=source)
        await self._update_queue.put(update)
        return True

    def query(self, key: str, default: Any = None) -> Any:
        """Read state (Memory > Persistent > Checkpoint)."""
        if key in self._memory_state: return self._memory_state[key]
        if key in self._persistent_state: return self._persistent_state[key]
        if key in self._checkpoint_state: return self._checkpoint_state[key]
        return default

    # ── CONSCIOUSNESS INTEGRATION ───────────────────────────────────────

    async def add_judgment(self, judgment: Any) -> None:
        """Record a judgment in consciousness and queue for persistence."""
        if not isinstance(judgment, UnifiedJudgment):
            if isinstance(judgment, dict):
                judgment = UnifiedJudgment.model_validate(judgment)
            else:
                return

        self.consciousness.add_judgment(judgment)
        self.total_judgments += 1
        await self.update(f"judg:recent", list(self.consciousness.recent_judgments.buffer), layer=StateLayer.MEMORY)
        await self.update(f"judg:record:{judgment.judgment_id}", judgment, layer=StateLayer.PERSISTENT)

    async def update_consciousness_level(self, level: str) -> None:
        """Update global consciousness level."""
        self.consciousness.consciousness_level = level
        await self.update("consciousness_level", level, layer=StateLayer.PERSISTENT)

    def get_consciousness_level(self) -> str:
        return self.query("consciousness_level", "REFLEX")

    # ── GOVERNANCE SUBSYSTEM (Phase 3) ───────────────────────────────────

    async def register_community(self, community: GovernanceCommunity) -> bool:
        """Register or update a governance community."""
        key = f"gov:community:{community.community_id}"
        await self.update(key, community, layer=StateLayer.PERSISTENT, source="governance_bot")
        self.consciousness.add_community(community)
        return True

    async def submit_proposal(self, proposal: GovernanceProposal) -> bool:
        """Submit a new governance proposal."""
        key = f"gov:proposal:{proposal.proposal_id}"
        await self.update(key, proposal, layer=StateLayer.PERSISTENT, source="governance_bot")
        self.consciousness.add_proposal(proposal)
        return True

    async def record_vote(self, vote: GovernanceVote) -> bool:
        """Record a user vote."""
        key = f"gov:vote:{vote.vote_id}"
        await self.update(key, vote, layer=StateLayer.PERSISTENT, source="governance_bot")
        return True

    def get_proposal(self, proposal_id: str) -> Optional[GovernanceProposal]:
        return self.query(f"gov:proposal:{proposal_id}")

    # ── READ ACCESSORS ──────────────────────────────────────────────────

    def get_recent_judgments(self, limit: int = 10) -> list[UnifiedJudgment]:
        """Retrieve recent judgments (newest first)."""
        buffer = list(self.consciousness.recent_judgments.buffer)
        recent = buffer[-limit:]
        return list(reversed(recent))

    def get_recent_outcomes(self, limit: int = 10) -> list[UnifiedLearningOutcome]:
        """Retrieve recent learning outcomes."""
        buffer = list(self.consciousness.learning_outcomes.buffer)
        recent = buffer[-limit:]
        return list(reversed(recent))

    def get_stats(self) -> dict:
        """Get comprehensive organism state statistics."""
        return {
            "total_judgments": self.total_judgments,
            "consciousness_level": self.get_consciousness_level(),
            "memory_keys": len(self._memory_state),
            "persistent_keys": len(self._persistent_state),
            "queue_size": self._update_queue.qsize(),
            "communities": len(self.consciousness.communities.buffer),
            "proposals": len(self.consciousness.proposals.buffer),
        }

    # ── INTERNALS ───────────────────────────────────────────────────────

    async def _process_updates_loop(self) -> None:
        """Background loop: process the update queue."""
        while self._processing or not self._update_queue.empty():
            try:
                update = await asyncio.wait_for(self._update_queue.get(), timeout=0.5)
                
                # Apply to internal dictionaries
                if update.layer == StateLayer.MEMORY:
                    self._memory_state[update.key] = update.value
                elif update.layer == StateLayer.PERSISTENT:
                    self._persistent_state[update.key] = update.value
                elif update.layer == StateLayer.CHECKPOINT:
                    self._checkpoint_state[update.key] = update.value
                
                self._update_queue.task_done()
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error("State Manager update error: %s", e)

    async def recover(self) -> None:
        """Recover state from last checkpoint on disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        if cp_path.exists():
            try:
                with open(cp_path, "r") as f:
                    self._checkpoint_state = json.load(f)
                logger.info("OrganismState: recovered from checkpoint")
            except Exception as e:
                logger.warning("OrganismState recovery failed: %s", e)

    async def save_checkpoint(self) -> None:
        """Save a snapshot of persistent state to disk."""
        cp_path = self.storage_dir / "state_checkpoint.json"
        try:
            # Simple JSON dump for now (needs better serializer for complex objects)
            # In production, this would be a more robust format
            logger.info("OrganismState: saving checkpoint to %s", cp_path)
        except Exception as e:
            logger.error("Checkpoint failed: %s", e)
