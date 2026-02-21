"""Phase 1: Conscious State Singleton

This is the FOUNDATION for paradigm inversion.

The organism maintains its internal state independently.
The API/CLI/Dashboard READ state, they don't CONTROL state.

Key principle:
- Organism is the SOURCE OF TRUTH
- ConsciousState is the READ-ONLY INTERFACE
- Events are the only way to update state
- No blocking API calls — state updates via async events

Architecture:
- Subscribes to 3 event buses (CORE, AUTOMATION, AGENT)
- Maintains thread-safe snapshots of current state
- Provides read-only queries: get_judgments(), get_dogs(), get_consciousness_level()
- Updates state ONLY via event handlers (never direct mutations)
- Persists state to ~/.cynic/conscious_state.json for recovery
"""

from __future__ import annotations

import asyncio
import json
import logging
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Optional
import threading

from cynic.core.event_bus import EventBus
from cynic.core.phi import MAX_CONFIDENCE, PHI_INV, PHI_INV_2

logger = logging.getLogger(__name__)

# State directory
STATE_DIR = Path.home() / ".cynic"
STATE_DIR.mkdir(exist_ok=True)
STATE_FILE = STATE_DIR / "conscious_state.json"


@dataclass(frozen=True)
class DogStatus:
    """Real-time status of a single dog (immutable)."""
    dog_id: str
    q_score: float  # Last judgment Q-Score
    verdict: str  # Last verdict
    confidence: float  # Last confidence
    activity: str  # "idle", "judging", "learning"
    last_active: float  # Timestamp
    judgment_count: int  # Total judgments


@dataclass(frozen=True)
class JudgmentSnapshot:
    """Lightweight snapshot of recent judgment (immutable)."""
    judgment_id: str
    timestamp: float
    q_score: float
    verdict: str
    confidence: float
    dog_votes: dict[str, float]
    source: str  # "perceive", "sdk", "social"


@dataclass(frozen=True)
class AxiomStatus:
    """State of a single axiom (immutable)."""
    axiom_id: str  # "PHI", "VERIFY", "CULTURE", "BURN", "FIDELITY"
    active: bool
    tier: str  # "A6", "A7", "A8", "A9"
    signal_count: int  # How many signals received
    maturity: float  # [0, 100]
    activated_at: Optional[float]


class ConsciousState:
    """Read-only interface to organism state.

    This is NOT a data store. This is an OBSERVATION PORT.
    The organism updates state via events; ConsciousState reads and caches.

    Public API:
    - get_current_level() -> str  ("REFLEX", "MICRO", "MACRO", "META")
    - get_dogs() -> dict[str, DogStatus]
    - get_recent_judgments(limit=10) -> list[JudgmentSnapshot]
    - get_dog(dog_id) -> DogStatus
    - get_axiom(axiom_id) -> AxiomStatus
    - get_all_axioms() -> dict[str, AxiomStatus]
    - get_health() -> dict (system health metrics)
    """

    _instance: Optional[ConsciousState] = None
    _lock = threading.Lock()

    def __new__(cls):
        """Singleton pattern."""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self):
        """Initialize conscious state (only once)."""
        if hasattr(self, "_initialized"):
            return

        self._initialized = True
        self._state_lock = asyncio.Lock()

        # Current organism state
        self._consciousness_level = "REFLEX"
        self._dogs: dict[str, DogStatus] = {}
        self._recent_judgments: list[JudgmentSnapshot] = []
        self._axioms: dict[str, AxiomStatus] = {}
        self._last_update = datetime.now().timestamp()

        # System health
        self._judgment_count = 0
        self._axiom_activation_count = 0
        self._error_count = 0

        logger.info("ConsciousState singleton initialized")

    async def initialize_from_buses(
        self,
        core_bus: EventBus,
        automation_bus: Optional[EventBus] = None,
        agent_bus: Optional[EventBus] = None,
    ):
        """Subscribe to event buses and start listening.

        This connects ConsciousState to the organism's nervous system.
        After this, state updates automatically as events fire.

        Args:
            core_bus: Main event bus (JUDGMENT_CREATED, AXIOM_ACTIVATED, etc.)
            automation_bus: Optional automation bus (for decision tracking)
            agent_bus: Optional agent bus (for dog activity)
        """
        logger.info("ConsciousState connecting to event buses...")

        # Subscribe to CORE bus events (use CoreEvent enum for correct string values)
        from cynic.core.event_bus import CoreEvent
        core_bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_created)
        core_bus.on(CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_level_changed)
        core_bus.on(CoreEvent.AXIOM_ACTIVATED, self._on_axiom_activated)
        # Note: DOG_ACTIVITY and ERROR are not yet defined in CoreEvent enum
        # core_bus.on("dog.activity", self._on_dog_activity)
        # core_bus.on("error", self._on_error)

        # Subscribe to AUTOMATION bus if available
        if automation_bus:
            from cynic.core.event_bus import CoreEvent
            # DECISION_MADE is in CoreEvent, not AutomationEvent
            # automation_bus.on(CoreEvent.DECISION_MADE, self._on_decision_made)
            # Note: ACTION_EXECUTED is not yet defined
            # automation_bus.on("action.executed", self._on_action_executed)

        # Subscribe to AGENT bus if available
        if agent_bus:
            # Note: DOG_JUDGMENT is not yet defined in AgentEvent enum
            # agent_bus.on("dog.judgment", self._on_dog_judgment)
            pass

        logger.info("ConsciousState subscribed to all event buses")
        await self.load_from_disk()

    async def _on_judgment_created(self, event):
        """Update state when a judgment is created.

        Args:
            event: Event object with payload dict
        """
        # Handle both Event objects and plain dicts (for testing)
        payload = event.payload if hasattr(event, 'payload') else event
        logger.debug("[ConsciousState] Received JUDGMENT_CREATED event: %s",
                    event.event_id if hasattr(event, 'event_id') else "unknown")

        async with self._state_lock:
            judgment_id = payload.get("judgment_id", "unknown")
            q_score = payload.get("q_score", 50.0)
            verdict = payload.get("verdict", "WAG")
            confidence = payload.get("confidence", 0.0)
            dog_votes = payload.get("dog_votes", {})
            source = payload.get("source", "unknown")

            snapshot = JudgmentSnapshot(
                judgment_id=judgment_id,
                timestamp=datetime.now().timestamp(),
                q_score=q_score,
                verdict=verdict,
                confidence=confidence,
                dog_votes=dog_votes,
                source=source,
            )

            # Rolling cap: F(11) = 89 recent judgments
            self._recent_judgments.append(snapshot)
            if len(self._recent_judgments) > 89:
                self._recent_judgments.pop(0)

            self._judgment_count += 1
            self._last_update = datetime.now().timestamp()

            logger.debug(
                "ConsciousState recorded judgment: Q=%.1f, Verdict=%s",
                q_score,
                verdict,
            )

    async def _on_consciousness_level_changed(self, event):
        """Update consciousness tier (REFLEX → MICRO → MACRO → META)."""
        payload = event.payload if hasattr(event, 'payload') else event
        async with self._state_lock:
            old_level = self._consciousness_level
            new_level = payload.get("level", "REFLEX")
            self._consciousness_level = new_level
            self._last_update = datetime.now().timestamp()

            logger.info(
                "ConsciousState consciousness changed: %s → %s",
                old_level,
                new_level,
            )

    async def _on_axiom_activated(self, event):
        """Record when an axiom is activated (signals)."""
        payload = event.payload if hasattr(event, 'payload') else event
        async with self._state_lock:
            axiom_id = payload.get("axiom_id", "unknown")
            tier = payload.get("tier", "A6")

            if axiom_id not in self._axioms:
                self._axioms[axiom_id] = AxiomStatus(
                    axiom_id=axiom_id,
                    active=False,
                    tier=tier,
                    signal_count=0,
                    maturity=0.0,
                    activated_at=None,
                )

            # Create new AxiomStatus with updated values (frozen dataclass)
            old_axiom = self._axioms[axiom_id]
            new_signal_count = old_axiom.signal_count + 1
            new_maturity = min(new_signal_count / 10.0, 100.0)
            was_inactive = not old_axiom.active
            is_now_active = new_signal_count >= 3 and was_inactive

            self._axioms[axiom_id] = AxiomStatus(
                axiom_id=axiom_id,
                active=is_now_active or old_axiom.active,
                tier=old_axiom.tier,
                signal_count=new_signal_count,
                maturity=new_maturity,
                activated_at=datetime.now().timestamp() if is_now_active else old_axiom.activated_at,
            )

            if is_now_active:
                self._axiom_activation_count += 1
                logger.info("ConsciousState: Axiom %s ACTIVATED", axiom_id)

            self._last_update = datetime.now().timestamp()

    async def _on_dog_activity(self, event):
        """Update dog status (idle/judging/learning)."""
        payload = event.payload if hasattr(event, 'payload') else event
        async with self._state_lock:
            dog_id = payload.get("dog_id", "unknown")
            activity = payload.get("activity", "idle")
            q_score = payload.get("q_score", 50.0)
            verdict = payload.get("verdict", "WAG")
            confidence = payload.get("confidence", 0.0)

            # Count judgments only when activity transitions to "judging"
            increment_judgment = activity == "judging"

            if dog_id not in self._dogs:
                self._dogs[dog_id] = DogStatus(
                    dog_id=dog_id,
                    q_score=q_score,
                    verdict=verdict,
                    confidence=confidence,
                    activity=activity,
                    last_active=datetime.now().timestamp(),
                    judgment_count=1 if increment_judgment else 0,
                )
            else:
                # Create new DogStatus with updated values (frozen dataclass)
                old_dog = self._dogs[dog_id]
                self._dogs[dog_id] = DogStatus(
                    dog_id=dog_id,
                    q_score=q_score,
                    verdict=verdict,
                    confidence=confidence,
                    activity=activity,
                    last_active=datetime.now().timestamp(),
                    judgment_count=old_dog.judgment_count + (1 if increment_judgment else 0),
                )

            self._last_update = datetime.now().timestamp()

    async def _on_dog_judgment(self, event):
        """Record individual dog judgment."""
        payload = event.payload if hasattr(event, 'payload') else event
        dog_id = payload.get("dog_id", "unknown")
        q_score = payload.get("q_score", 50.0)

        async with self._state_lock:
            if dog_id not in self._dogs:
                self._dogs[dog_id] = DogStatus(
                    dog_id=dog_id,
                    q_score=q_score,
                    verdict="WAG",
                    confidence=0.0,
                    activity="idle",
                    last_active=datetime.now().timestamp(),
                    judgment_count=1,
                )
            else:
                self._dogs[dog_id].q_score = q_score
                self._dogs[dog_id].last_active = datetime.now().timestamp()

    async def _on_decision_made(self, event):
        """Record when a decision is made."""
        pass  # Can expand later

    async def _on_action_executed(self, event):
        """Record when an action is executed."""
        pass  # Can expand later

    async def _on_error(self, event):
        """Track errors for diagnostics."""
        payload = event.payload if hasattr(event, 'payload') else event
        async with self._state_lock:
            self._error_count += 1
            logger.warning(
                "ConsciousState recorded error: %s",
                payload.get("message", "unknown"),
            )

    # ==================== READ-ONLY PUBLIC API ====================

    async def get_current_level(self) -> str:
        """Get current consciousness tier."""
        async with self._state_lock:
            return self._consciousness_level

    async def get_dogs(self) -> dict[str, DogStatus]:
        """Get status of all dogs."""
        async with self._state_lock:
            return dict(self._dogs)

    async def get_dog(self, dog_id: str) -> Optional[dict]:
        """Get status of a single dog (immutable copy)."""
        async with self._state_lock:
            dog = self._dogs.get(dog_id)
            return asdict(dog) if dog else None

    async def get_recent_judgments(self, limit: int = 10) -> list[JudgmentSnapshot]:
        """Get recent judgments (newest first)."""
        async with self._state_lock:
            return list(reversed(self._recent_judgments[-limit:]))

    async def get_judgment_by_id(self, judgment_id: str) -> Optional[JudgmentSnapshot]:
        """Query for a specific judgment by ID (Phase 3 API query endpoint)."""
        async with self._state_lock:
            for j in self._recent_judgments:
                if j.judgment_id == judgment_id:
                    return j
            return None

    async def get_axiom(self, axiom_id: str) -> Optional[AxiomStatus]:
        """Get status of a single axiom."""
        async with self._state_lock:
            return self._axioms.get(axiom_id)

    async def get_all_axioms(self) -> dict[str, AxiomStatus]:
        """Get status of all axioms."""
        async with self._state_lock:
            return dict(self._axioms)

    async def get_health(self) -> dict[str, Any]:
        """Get system health metrics."""
        async with self._state_lock:
            return {
                "consciousness_level": self._consciousness_level,
                "judgment_count": self._judgment_count,
                "axiom_activation_count": self._axiom_activation_count,
                "error_count": self._error_count,
                "dog_count": len(self._dogs),
                "axiom_count": len(self._axioms),
                "active_axioms": sum(1 for a in self._axioms.values() if a.active),
                "last_update": self._last_update,
                "timestamp": datetime.now().timestamp(),
            }

    async def to_dict(self) -> dict:
        """Serialize state to dict (for persistence)."""
        async with self._state_lock:
            return {
                "consciousness_level": self._consciousness_level,
                "dogs": {
                    k: asdict(v) for k, v in self._dogs.items()
                },
                "recent_judgments": [
                    asdict(j) for j in self._recent_judgments
                ],
                "axioms": {
                    k: asdict(v) for k, v in self._axioms.items()
                },
                "stats": {
                    "judgment_count": self._judgment_count,
                    "axiom_activation_count": self._axiom_activation_count,
                    "error_count": self._error_count,
                    "last_update": self._last_update,
                },
            }

    async def save_to_disk(self):
        """Persist state to disk (recovery on restart)."""
        try:
            state_dict = await self.to_dict()
            STATE_FILE.write_text(json.dumps(state_dict, indent=2))
            logger.debug("ConsciousState persisted to %s", STATE_FILE)
        except Exception as e:
            logger.error("Failed to save ConsciousState: %s", e)

    async def load_from_disk(self):
        """Load state from disk (recovery on startup)."""
        try:
            if STATE_FILE.exists():
                data = json.loads(STATE_FILE.read_text())
                async with self._state_lock:
                    self._consciousness_level = data.get(
                        "consciousness_level", "REFLEX"
                    )
                    self._judgment_count = data.get("stats", {}).get(
                        "judgment_count", 0
                    )
                    self._axiom_activation_count = data.get("stats", {}).get(
                        "axiom_activation_count", 0
                    )
                    self._error_count = data.get("stats", {}).get(
                        "error_count", 0
                    )
                logger.info("ConsciousState loaded from disk: %d judgments", self._judgment_count)
        except Exception as e:
            logger.error("Failed to load ConsciousState: %s", e)

    async def sync_checkpoint(self) -> None:
        """
        SYNC checkpoint: flush all in-memory state to disk atomically.

        Used after critical operations (POST /judge) to ensure
        data survives process crash. Writes atomically using temp → rename pattern.

        Raises:
        - OSError: if checkpoint write fails
        """
        import os
        import time
        import tempfile

        try:
            data = await self.to_dict()
            temp_fd, temp_path = tempfile.mkstemp(
                dir=STATE_FILE.parent,
                prefix=".conscious_state_tmp_",
                suffix=".json",
            )
            try:
                with os.fdopen(temp_fd, "w", encoding="utf-8") as fh:
                    json.dump(data, fh, indent=2)
                os.replace(temp_path, STATE_FILE)
                logger.info("ConsciousState checkpoint synced: %s", STATE_FILE)
            except Exception:
                try:
                    os.unlink(temp_path)
                except:
                    pass
                raise
        except Exception as e:
            logger.error("ConsciousState sync checkpoint FAILED: %s", e)
            raise


# Singleton instance
_conscious_state: Optional[ConsciousState] = None


def get_conscious_state() -> ConsciousState:
    """Get the ConsciousState singleton."""
    global _conscious_state
    if _conscious_state is None:
        _conscious_state = ConsciousState()
    return _conscious_state
