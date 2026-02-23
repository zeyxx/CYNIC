"""
OrganismState — Single source of truth for all CYNIC state.

VISION:
  One state system, three layers:
  1. Memory (RAM): Q-table, dogs, residuals — fast, lost on restart
  2. Persistent (SurrealDB): consciousness, actions — survives restart
  3. Checkpoint (File): recovery metadata — for startup

This consolidates 5 old systems:
  ✗ api/state.py::KernelState (9,925 LOC!)
  ✗ organism/conscious_state.py::ConsciousState (660 LOC)
  ✗ senses/checkpoint.py (functions)
  ✗ core/topology/topology_mirror.py (separate state)
  ✗ cognition/cortex (Q-table, residuals scattered)

ARCHITECTURE:
  OrganismState (this class)
  ├─ memory_state: dict (Q-table, dogs, residuals)
  ├─ persistent_state: dict (consciousness, actions)
  └─ checkpoint_state: dict (recovery metadata)

  All updates: state.update(key, value, layer="persistent")
  All queries: state.query(key) → value or None
  Consistency: write_through checks enabled
"""
from __future__ import annotations

import asyncio
import copy
import json
import logging
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional
from enum import Enum

logger = logging.getLogger("cynic.organism.state_manager")


class StateLayer(Enum):
    """Which layer to store state in."""
    MEMORY = "memory"          # RAM only, lost on restart
    PERSISTENT = "persistent"  # SurrealDB, survives restart
    CHECKPOINT = "checkpoint"  # File-based, for recovery


@dataclass
class StateUpdate:
    """Atomic state change with timestamp."""
    key: str
    value: Any
    layer: StateLayer
    timestamp: float = field(default_factory=time.time)
    source: str = ""  # Who made this change


@dataclass(frozen=True)
class StateSnapshot:
    """
    Immutable snapshot of OrganismState (Kani Criterion 2: Vacuity).

    Frozen dataclass prevents modification after creation, ensuring
    snapshot integrity and thread-safety.

    Attributes:
        memory: Copy of memory-layer state (dict[str, Any])
        persistent: Copy of persistent-layer state (dict[str, Any])
        checkpoint: Copy of checkpoint-layer state (dict[str, Any])
    """
    memory: dict[str, Any]
    persistent: dict[str, Any]
    checkpoint: dict[str, Any]


class OrganismState:
    """
    Single source of truth for CYNIC state.

    Features:
    - Three-layer state (memory/persistent/checkpoint)
    - Write-through consistency checking
    - Async updates with queue
    - Query interface for read-only access
    - Recovery from persistence layer
    """

    def __init__(self, storage_path: Optional[str | Path] = None):
        """
        Initialize OrganismState with optional persistent storage path.

        Args:
            storage_path: Optional path for persistence files. Defaults to ~/.cynic/organism_state
        """
        # Three-layer state
        self._memory_state: dict[str, Any] = {}
        self._persistent_state: dict[str, Any] = {}
        self._checkpoint_state: dict[str, Any] = {}

        # Q-Table: dict[state_key: str, dict[action: str, q_value: float]]
        # Stored in memory layer (fast, lost on restart per Phase 2)
        self._qtable: dict[str, dict[str, float]] = {}

        # Update queue (for batching writes)
        self._update_queue: asyncio.Queue[StateUpdate] = asyncio.Queue()
        self._processing = False

        # Write-through consistency
        self._consistency_errors: list[str] = []
        self._last_update: dict[str, float] = {}  # key → timestamp

        # Thread safety (Kani Criterion 4: Invariant)
        self._lock = asyncio.Lock()

        # Persistence backend (injected)
        self._db = None  # Will be set by organism

        # Storage paths (Task 8: Lifecycle integration)
        self.storage_path = Path(storage_path or Path.home() / ".cynic" / "organism_state")
        self.storage_path.mkdir(parents=True, exist_ok=True)
        self._checkpoint_path = self.storage_path / "state_checkpoint.json"

        logger.info("OrganismState initialized (3-layer, write-through, storage=%s)", self.storage_path)

    # ── LIFECYCLE ────────────────────────────────────────────────────────

    async def initialize(self) -> bool:
        """
        Initialize OrganismState after construction.

        Returns:
            True if initialization successful
        """
        try:
            # Attempt to recover state from persistence
            await self.recover_from_persistent()
            logger.info("OrganismState initialization complete")
            return True
        except asyncio.TimeoutError as e:
            logger.warning("Failed to recover state during init: %s", e)
            # Initialize with empty state
            return True

    async def persist(self) -> None:
        """
        Persist consciousness level to disk (PERSISTENT layer).

        Task 8: Lifecycle integration — save state to file for recovery after restart.
        """
        async with self._lock:
            # Get consciousness level from persistent state
            consciousness_level = self._persistent_state.get("consciousness_level", "REFLEX")

            persist_file = self.storage_path / "consciousness.json"
            consciousness_data = {
                "level": consciousness_level,
                "timestamp": time.time(),
            }
            persist_file.write_text(json.dumps(consciousness_data, indent=2))
            logger.info(f"Persisted consciousness level: {consciousness_level}")

    async def recover(self) -> None:
        """
        Recover consciousness level from disk (PERSISTENT layer).

        Task 8: Lifecycle integration — restore state after restart.
        Clears MEMORY layer (judgments, residuals, actions) as expected on restart.
        """
        async with self._lock:
            persist_file = self.storage_path / "consciousness.json"
            if not persist_file.exists():
                logger.info("No persistent state to recover")
                return

            try:
                data = json.loads(persist_file.read_text())
                level = data.get("level", "REFLEX")
                self._persistent_state["consciousness_level"] = level
                logger.info(f"Recovered consciousness level: {level}")
            except Exception as e:
                logger.error(f"Failed to recover consciousness level: {e}")

    def snapshot(self) -> StateSnapshot:
        """
        Create immutable snapshot of current state (Kani Criterion 2: Vacuity).

        Returns:
            StateSnapshot with copies of all three layers
        """
        return StateSnapshot(
            memory=dict(self._memory_state),
            persistent=dict(self._persistent_state),
            checkpoint=dict(self._checkpoint_state),
        )

    # ── SYNCHRONOUS GETTER/SETTER (for TDD tests) ────────────────────────

    def get_value(
        self,
        key: str,
        layer: Optional[StateLayer] = None,
        default: Any = None,
    ) -> Any:
        """
        Read-only synchronous getter.

        Args:
            key: State key
            layer: Specific layer to query, or None for search
            default: Return value if not found

        Returns:
            State value or default
        """
        if layer == StateLayer.MEMORY or layer is None:
            if key in self._memory_state:
                return self._memory_state[key]

        if layer == StateLayer.PERSISTENT or layer is None:
            if key in self._persistent_state:
                return self._persistent_state[key]

        if layer == StateLayer.CHECKPOINT or layer is None:
            if key in self._checkpoint_state:
                return self._checkpoint_state[key]

        return default

    async def set_value(
        self,
        key: str,
        value: Any,
        layer: StateLayer = StateLayer.PERSISTENT,
        source: str = "",
    ) -> bool:
        """
        Asynchronous setter with thread-safety (Kani Criterion 4).

        Args:
            key: State key
            value: New value
            layer: Where to store (memory/persistent/checkpoint)
            source: Who triggered this update (for debugging)

        Returns:
            True if successful
        """
        async with self._lock:
            # Consistency check
            if not self._consistency_check(key, value, layer):
                logger.error("Consistency check failed for key=%s", key)
                return False

            # Write to appropriate layer
            if layer == StateLayer.MEMORY:
                self._memory_state[key] = value
            elif layer == StateLayer.PERSISTENT:
                self._persistent_state[key] = value
            elif layer == StateLayer.CHECKPOINT:
                self._checkpoint_state[key] = value

            self._last_update[key] = time.time()
            logger.debug("Set %s=%s (%s)", key, type(value).__name__, layer.value)
            return True

    # ── WRITE OPERATIONS ─────────────────────────────────────────────────

    async def update(
        self,
        key: str,
        value: Any,
        layer: StateLayer = StateLayer.PERSISTENT,
        source: str = "",
    ) -> bool:
        """
        Atomically update state in specified layer.

        Args:
            key: State key (e.g., "q_table", "consciousness_level")
            value: New value
            layer: Where to store (memory/persistent/checkpoint)
            source: Who triggered this update (for debugging)

        Returns:
            True if successful, False if consistency check failed
        """
        update = StateUpdate(key=key, value=value, layer=layer, source=source)

        # Write-through: consistency check BEFORE queuing
        if not self._consistency_check(key, value, layer):
            self._consistency_errors.append(
                f"Consistency check failed: {key}={value} ({layer.value})"
            )
            logger.error("CONSISTENCY VIOLATION: %s", self._consistency_errors[-1])
            return False

        # Queue for async processing
        await self._update_queue.put(update)
        return True

    async def update_many(
        self,
        updates: dict[str, Any],
        layer: StateLayer = StateLayer.PERSISTENT,
    ) -> bool:
        """
        Batch update multiple keys in same layer.

        Args:
            updates: {key: value} dict
            layer: Where to store all of them

        Returns:
            True if all updates successful
        """
        results = []
        for key, value in updates.items():
            result = await self.update(key, value, layer=layer)
            results.append(result)
        return all(results)

    # ── READ OPERATIONS ─────────────────────────────────────────────────

    def query(
        self,
        key: str,
        layer: Optional[StateLayer] = None,
        default: Any = None,
    ) -> Any:
        """
        Read-only query of state.

        Searches layers in order: memory → persistent → checkpoint
        (unless specific layer requested)

        Args:
            key: State key
            layer: Specific layer to query, or None for search
            default: Return value if not found

        Returns:
            State value or default
        """
        if layer == StateLayer.MEMORY or layer is None:
            if key in self._memory_state:
                return self._memory_state[key]

        if layer == StateLayer.PERSISTENT or layer is None:
            if key in self._persistent_state:
                return self._persistent_state[key]

        if layer == StateLayer.CHECKPOINT or layer is None:
            if key in self._checkpoint_state:
                return self._checkpoint_state[key]

        return default

    def query_all(self, layer: Optional[StateLayer] = None) -> dict[str, Any]:
        """
        Query entire state in a layer (or merged view).

        Args:
            layer: Specific layer, or None for merged view

        Returns:
            {key: value} dict
        """
        if layer == StateLayer.MEMORY:
            return dict(self._memory_state)
        elif layer == StateLayer.PERSISTENT:
            return dict(self._persistent_state)
        elif layer == StateLayer.CHECKPOINT:
            return dict(self._checkpoint_state)
        else:
            # Merged view: checkpoint ← persistent ← memory (memory wins)
            result = {}
            result.update(self._checkpoint_state)
            result.update(self._persistent_state)
            result.update(self._memory_state)
            return result

    # ── Q-TABLE SUBSYSTEM (Task 3: Migration) ────────────────────────────────

    async def update_qtable_entry(
        self,
        state_key: str,
        action: str,
        q_value: float,
    ) -> None:
        """
        Update a single Q-Table entry. Clamps q_value to [0.0, 1.0].

        Args:
            state_key: State identifier (e.g., "CODE:JUDGE:PRESENT:1")
            action: Action/verdict (e.g., "BARK", "GROWL", "WAG", "HOWL")
            q_value: Q-value to store, will be clamped to [0.0, 1.0]

        Returns:
            None
        """
        # Clamp q_value to [0.0, 1.0]
        clamped_q = max(0.0, min(1.0, q_value))

        # Ensure state exists in Q-table
        if state_key not in self._qtable:
            self._qtable[state_key] = {}

        # Set the Q-value
        self._qtable[state_key][action] = clamped_q
        logger.debug(
            "Updated Q-Table[%s][%s] = %.3f (clamped from %.3f)",
            state_key, action, clamped_q, q_value,
        )

    def get_qtable_entry(
        self,
        state_key: str,
        action: str,
    ) -> float:
        """
        Retrieve Q-value for (state, action).
        Returns 0.5 (neutral default) if not found.

        Args:
            state_key: State identifier
            action: Action/verdict

        Returns:
            Q-value in [0.0, 1.0], or 0.5 if not found
        """
        if state_key not in self._qtable:
            return 0.5

        return self._qtable[state_key].get(action, 0.5)

    def get_qtable_entries(
        self,
        state_key: str,
    ) -> dict[str, float]:
        """
        Retrieve all actions for a given state.

        Args:
            state_key: State identifier

        Returns:
            dict[action: str, q_value: float], empty dict if state not found
        """
        if state_key not in self._qtable:
            return {}

        # Return a copy to prevent external modification
        return dict(self._qtable[state_key])

    async def clear_qtable(self) -> None:
        """
        Clear entire Q-Table (for testing/reset).

        All entries are forgotten. Can continue adding entries after clear.

        Returns:
            None
        """
        self._qtable.clear()
        logger.info("Cleared entire Q-Table")

    # ── CONSCIOUSNESS SUBSYSTEM (Task 4: Migration) ──────────────────────

    async def add_judgment(self, judgment: dict) -> None:
        """
        Add judgment to recent_judgments (keep last 100).

        Args:
            judgment: Judgment dict with keys: judgment_id, q_score, verdict, etc.

        Returns:
            None
        """
        async with self._lock:
            if "recent_judgments" not in self._memory_state:
                self._memory_state["recent_judgments"] = []

            judgments = self._memory_state["recent_judgments"]
            judgments.append(judgment)

            # Keep only last 100 judgments (Fibonacci F(11) = 89, so 100 is safe)
            if len(judgments) > 100:
                self._memory_state["recent_judgments"] = judgments[-100:]

            logger.debug("Added judgment: %s", judgment.get("judgment_id", "unknown"))

    def get_recent_judgments(self, limit: int = 10) -> list[dict]:
        """
        Retrieve recent judgments (default last 10).

        Args:
            limit: Maximum number of judgments to return (default 10)

        Returns:
            List of judgment dicts (newest first)
        """
        judgments = self._memory_state.get("recent_judgments", [])
        # Return newest first
        return list(reversed(judgments[-limit:]))

    def get_consciousness_level(self) -> str:
        """
        Get current consciousness level.

        Returns:
            Consciousness level string (REFLEX, MICRO, MACRO, META)
        """
        return self._persistent_state.get("consciousness_level", "REFLEX")

    async def update_consciousness_level(self, level: str) -> None:
        """
        Update consciousness level (validates REFLEX|MICRO|MACRO|META).

        Args:
            level: New consciousness level

        Raises:
            ValueError: if level is not valid
        """
        valid_levels = {"REFLEX", "MICRO", "MACRO", "META"}
        if level not in valid_levels:
            raise ValueError(
                f"Invalid consciousness level: {level}. Must be one of {valid_levels}"
            )

        async with self._lock:
            old_level = self._persistent_state.get("consciousness_level", "REFLEX")
            self._persistent_state["consciousness_level"] = level
            logger.info(
                "Updated consciousness level: %s → %s", old_level, level
            )

    # ── DOGS REGISTRY SUBSYSTEM (Task 5: Migration) ───────────────────────

    async def set_dogs(self, dogs: dict) -> None:
        """
        Set dog registry (readonly after set).

        Args:
            dogs: Dict of dog configurations

        Returns:
            None
        """
        async with self._lock:
            self._memory_state["dogs"] = dogs
            logger.debug("Set dogs registry with %d dogs", len(dogs))

    def get_dogs(self) -> dict:
        """
        Get all dogs.

        Returns:
            Dict of all registered dogs (deep copy)
        """
        return copy.deepcopy(self._memory_state.get("dogs", {}))

    def get_dog(self, dog_id: str) -> Any:
        """
        Get single dog by ID, returns None if not found.

        Args:
            dog_id: ID of the dog to retrieve

        Returns:
            Dog config dict or None if not found
        """
        dogs = self._memory_state.get("dogs", {})
        return dogs.get(dog_id)

    # ── RESIDUALS SUBSYSTEM (Task 6: Migration) ─────────────────────────

    async def update_residual(self, residual_id: str, residual_state: dict) -> None:
        """
        Update residual tracking entry.

        Args:
            residual_id: ID of the residual
            residual_state: State dict for the residual

        Returns:
            None
        """
        async with self._lock:
            if "residuals" not in self._memory_state:
                self._memory_state["residuals"] = {}

            self._memory_state["residuals"][residual_id] = residual_state
            logger.debug("Updated residual: %s", residual_id)

    def get_residual(self, residual_id: str) -> dict:
        """
        Get single residual, returns {} if not found.

        Args:
            residual_id: ID of the residual to retrieve

        Returns:
            Residual state dict or empty dict if not found
        """
        residuals = self._memory_state.get("residuals", {})
        return residuals.get(residual_id, {})

    def get_all_residuals(self) -> dict:
        """
        Get all residuals.

        Returns:
            Dict of all residuals (deep copy)
        """
        return copy.deepcopy(self._memory_state.get("residuals", {}))

    async def clear_residuals(self) -> None:
        """
        Clear all residuals.

        Returns:
            None
        """
        async with self._lock:
            self._memory_state["residuals"] = {}
            logger.info("Cleared all residuals")

    # ── ACTIONS QUEUE SUBSYSTEM (Task 7: Migration) ──────────────────────

    async def add_action(self, action: dict) -> None:
        """
        Add action to pending queue (capped at 89, BURN axiom).

        Args:
            action: Action dict to queue

        Returns:
            None
        """
        async with self._lock:
            if "pending_actions" not in self._memory_state:
                self._memory_state["pending_actions"] = []

            actions = self._memory_state["pending_actions"]
            actions.append(action)

            # Cap at 89 (Fibonacci F(11) = 89, BURN axiom principle)
            if len(actions) > 89:
                self._memory_state["pending_actions"] = actions[-89:]

            logger.debug("Added action: %s", action.get("action_id", "unknown"))

    def get_pending_actions(self) -> list[dict]:
        """
        Get all pending actions in FIFO order.

        Returns:
            List of action dicts in order (deep copy)
        """
        return copy.deepcopy(self._memory_state.get("pending_actions", []))

    async def remove_action(self, action_id: str) -> bool:
        """
        Remove action by ID. Returns True if found.

        Args:
            action_id: ID of action to remove

        Returns:
            True if action was found and removed, False otherwise
        """
        async with self._lock:
            actions = self._memory_state.get("pending_actions", [])
            for idx, action in enumerate(actions):
                if action.get("action_id") == action_id:
                    actions.pop(idx)
                    logger.debug("Removed action: %s", action_id)
                    return True
            return False

    async def clear_actions(self) -> None:
        """
        Clear all pending actions.

        Returns:
            None
        """
        async with self._lock:
            self._memory_state["pending_actions"] = []
            logger.info("Cleared all pending actions")

    # ── CONSISTENCY ──────────────────────────────────────────────────────

    def _consistency_check(
        self,
        key: str,
        value: Any,
        layer: StateLayer,
    ) -> bool:
        """
        Write-through consistency check.

        Rules:
        1. Memory and persistent shouldn't both have same key (usually)
        2. Values should be serializable (JSON-safe)
        3. Keys shouldn't be massive (>10MB each)

        Returns:
            True if OK, False if violation
        """
        # Rule 1: Don't write to persistent if already in memory (usually)
        if layer == StateLayer.PERSISTENT and key in self._memory_state:
            # Exception: OK if value is same
            if self._memory_state.get(key) != value:
                logger.warning(
                    "CONSISTENCY: Writing %s to persistent but already in memory",
                    key,
                )
                # Not a hard error, just warn

        # Rule 2: Check serializability
        try:
            json.dumps(value, default=str)
        except json.JSONDecodeError as e:
            logger.error("CONSISTENCY: %s not JSON-serializable: %s", key, e)
            return False

        # Rule 3: Check size (rough check)
        size_bytes = len(json.dumps(value, default=str))
        if size_bytes > 10 * 1024 * 1024:  # 10MB
            logger.error(
                "CONSISTENCY: %s is %dMB (>10MB limit)",
                key,
                size_bytes / 1024 / 1024,
            )
            return False

        return True

    def get_consistency_errors(self) -> list[str]:
        """Return all consistency errors encountered."""
        return list(self._consistency_errors)

    def clear_consistency_errors(self) -> None:
        """Clear error log (after handling them)."""
        self._consistency_errors.clear()

    # ── PERSISTENCE ──────────────────────────────────────────────────────

    async def sync_to_persistent(self) -> bool:
        """
        Sync memory state to persistent backend (SurrealDB).

        This is called periodically to back up Q-table, dogs, etc.

        Returns:
            True if successful
        """
        if not self._db:
            logger.warning("No persistent backend configured, skipping sync")
            return False

        try:
            # Serialize memory state
            payload = {
                "layer": "memory",
                "data": dict(self._memory_state),
                "timestamp": time.time(),
            }

            # Write to DB (assumes _db.save_state() exists)
            await self._db.save_state("organism_memory", payload)
            logger.debug("Synced memory state to persistent")
            return True
        except asyncpg.Error as e:
            logger.error("Failed to sync memory state: %s", e)
            return False

    async def recover_from_persistent(self) -> bool:
        """
        Recover state from persistent backend after restart.

        Loads:
        1. Memory state from DB (Q-table, dogs)
        2. Persistent state from DB (consciousness, actions)
        3. Checkpoint state from file

        Returns:
            True if recovery successful
        """
        if not self._db:
            logger.warning("No persistent backend, starting fresh")
            return False

        try:
            # Load memory state from DB
            memory_data = await self._db.load_state("organism_memory")
            if memory_data:
                self._memory_state = memory_data.get("data", {})
                logger.info("Recovered memory state (%d keys)", len(self._memory_state))

            # Load persistent state from DB
            persistent_data = await self._db.load_state("organism_persistent")
            if persistent_data:
                self._persistent_state = persistent_data.get("data", {})
                logger.info(
                    "Recovered persistent state (%d keys)",
                    len(self._persistent_state),
                )

            # Load checkpoint from file
            if self._checkpoint_path.exists():
                with open(self._checkpoint_path) as f:
                    checkpoint = json.load(f)
                    self._checkpoint_state = checkpoint.get("data", {})
                    logger.info(
                        "Recovered checkpoint (%d keys)",
                        len(self._checkpoint_state),
                    )

            return True
        except ValidationError as e:
            logger.error("Failed to recover state: %s", e)
            return False

    async def save_checkpoint(self) -> bool:
        """
        Save recovery checkpoint to file (called on shutdown).

        Stores metadata needed to restart organism:
        - Last known consciousness level
        - Last judgment timestamp
        - Handler health status

        Returns:
            True if successful
        """
        try:
            checkpoint = {
                "timestamp": time.time(),
                "data": {
                    # Copy essential state for recovery
                    "consciousness_level": self.query("consciousness_level"),
                    "last_judgment_time": self.query("last_judgment_time"),
                    "q_table_hash": hash(
                        json.dumps(self.query("q_table", default={}), default=str)
                    ),
                },
            }

            self._checkpoint_path.parent.mkdir(parents=True, exist_ok=True)
            with open(self._checkpoint_path, "w") as f:
                json.dump(checkpoint, f, indent=2)

            logger.info("Saved checkpoint to %s", self._checkpoint_path)
            return True
        except OSError as e:
            logger.error("Failed to save checkpoint: %s", e)
            return False

    # ── LIFECYCLE ────────────────────────────────────────────────────────

    async def start_processing(self, db: Any) -> None:
        """
        Start async processing of state updates.

        Args:
            db: Persistent backend (implements save_state/load_state)
        """
        self._db = db
        self._processing = True
        await self.recover_from_persistent()
        asyncio.create_task(self._process_updates_loop())
        logger.info("OrganismState processing started")

    async def stop_processing(self) -> None:
        """
        Stop async processing and flush all pending updates.

        Called during shutdown to ensure no data loss.
        """
        self._processing = False
        await self._flush_queue()
        await self.sync_to_persistent()
        await self.save_checkpoint()
        logger.info("OrganismState processing stopped")

    async def _process_updates_loop(self) -> None:
        """
        Background loop: process state updates from queue.

        Batches updates together for efficiency.
        """
        batch: dict[str, StateUpdate] = {}
        batch_timeout = 1.0  # Flush every 1 second
        last_flush = time.time()

        while self._processing:
            try:
                # Try to get update with timeout
                try:
                    update = await asyncio.wait_for(
                        self._update_queue.get(),
                        timeout=batch_timeout - (time.time() - last_flush),
                    )
                    batch[update.key] = update
                except asyncio.TimeoutError:
                    pass

                # Flush batch if timeout reached or batch is large
                if (
                    time.time() - last_flush > batch_timeout
                    or len(batch) > 100
                ):
                    await self._flush_batch(batch)
                    batch.clear()
                    last_flush = time.time()

            except asyncpg.Error as e:
                logger.error("Error in update loop: %s", e)
                await asyncio.sleep(1.0)

    async def _flush_batch(self, batch: dict[str, StateUpdate]) -> None:
        """
        Flush a batch of updates to appropriate layers.

        Args:
            batch: {key: StateUpdate} dict
        """
        memory_updates = {}
        persistent_updates = {}
        checkpoint_updates = {}

        for key, update in batch.items():
            if update.layer == StateLayer.MEMORY:
                memory_updates[key] = update.value
            elif update.layer == StateLayer.PERSISTENT:
                persistent_updates[key] = update.value
            elif update.layer == StateLayer.CHECKPOINT:
                checkpoint_updates[key] = update.value

            self._last_update[key] = update.timestamp

        # Apply to in-memory storage
        self._memory_state.update(memory_updates)
        self._persistent_state.update(persistent_updates)
        self._checkpoint_state.update(checkpoint_updates)

        # Async: sync to DB if batch has persistent updates
        if persistent_updates and self._db:
            try:
                payload = {
                    "layer": "persistent",
                    "data": persistent_updates,
                    "timestamp": time.time(),
                }
                await self._db.save_state("organism_persistent", payload)
            except asyncpg.Error as e:
                logger.error("Failed to persist updates: %s", e)

    async def _flush_queue(self) -> None:
        """Flush all remaining updates before shutdown."""
        batch: dict[str, StateUpdate] = {}
        while not self._update_queue.empty():
            update = self._update_queue.get_nowait()
            batch[update.key] = update
        if batch:
            await self._flush_batch(batch)

    # ── DIAGNOSTICS ──────────────────────────────────────────────────────

    def get_stats(self) -> dict[str, Any]:
        """Return diagnostic stats about state."""
        return {
            "memory_keys": len(self._memory_state),
            "persistent_keys": len(self._persistent_state),
            "checkpoint_keys": len(self._checkpoint_state),
            "queue_size": self._update_queue.qsize(),
            "consistency_errors": len(self._consistency_errors),
            "last_updates": dict(
                sorted(
                    self._last_update.items(),
                    key=lambda x: x[1],
                    reverse=True,
                )[:10]
            ),
        }

    def __repr__(self) -> str:
        stats = self.get_stats()
        return (
            f"OrganismState(memory={stats['memory_keys']}, "
            f"persistent={stats['persistent_keys']}, "
            f"checkpoint={stats['checkpoint_keys']}, "
            f"queue={stats['queue_size']})"
        )
