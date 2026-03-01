"""
ActionProposer — The Strategy Layer.

Receives DECISION_MADE events from the JudgeOrchestrator and translates them
into actionable proposals for the organism's motor system (ActHandlers).

Responsibility:
- Maintain a priority queue of pending actions.
- Enforce deduplication (don't propose the same action twice).
- Persist proposals to disk/DB (~/.cynic/pending_actions.json).
"""
from __future__ import annotations

import json
import logging
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus
from cynic.kernel.core.events_schema import ActionProposedPayload, DecisionMadePayload

logger = logging.getLogger("cynic.kernel.brain.cognition.action_proposer")

@dataclass
class ProposedAction:
    action_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    judgment_id: str = ""
    verdict: str = ""
    reality: str = ""
    action_prompt: str = ""
    priority: int = 5
    status: str = "PENDING"  # PENDING, EXECUTING, COMPLETED, FAILED, BLOCKED
    proposed_at: float = field(default_factory=time.time)
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "action_id": self.action_id,
            "judgment_id": self.judgment_id,
            "verdict": self.verdict,
            "reality": self.reality,
            "action_prompt": self.action_prompt,
            "priority": self.priority,
            "status": self.status,
            "proposed_at": self.proposed_at,
            "metadata": self.metadata
        }

class ActionProposer:
    """
    Manages the lifecycle of proposed actions from judgment outcomes.
    """
    _STORAGE_PATH = Path.home() / ".cynic" / "pending_actions.json"

    def __init__(self, db_pool: Any | None = None):
        self.db_pool = db_pool
        self._pending: dict[str, ProposedAction] = {}
        self._load_from_disk()

    def start(self):
        """Subscribe to decision events."""
        bus = get_core_bus()
        bus.on(CoreEvent.DECISION_MADE, self.on_decision_made)
        logger.info("ActionProposer started — subscribed to DECISION_MADE")

    async def on_decision_made(self, event: Event) -> None:
        """Handle new decisions from the orchestrator."""
        try:
            payload = event.as_typed(DecisionMadePayload)
            
            # Filter: only propose actions for specific verdicts (e.g., ACT)
            if payload.verdict != "ACT":
                return

            action = ProposedAction(
                judgment_id=payload.judgment_id,
                verdict=payload.verdict,
                reality=payload.reality,
                action_prompt=payload.action_prompt,
                priority=1 if payload.q_value > 80 else 5,
                metadata={"q_value": payload.q_value, "confidence": payload.confidence}
            )

            await self.add_proposal(action)

        except Exception as e:
            logger.error("ActionProposer failed to process decision: %s", e)

    async def add_proposal(self, action: ProposedAction) -> bool:
        """Add a new proposal to the queue and notify the system."""
        # Deduplication check
        if any(p.judgment_id == action.judgment_id for p in self._pending.values()):
            return False

        self._pending[action.action_id] = action
        self._save_to_disk()

        # Emit ACTION_PROPOSED
        await get_core_bus().emit(Event.typed(
            CoreEvent.ACTION_PROPOSED,
            ActionProposedPayload(
                action_id=action.action_id,
                judgment_id=action.judgment_id,
                reality=action.reality,
                priority=action.priority,
                action_prompt=action.action_prompt
            ),
            source="action_proposer"
        ))
        
        logger.info("ACTION PROPOSED: %s (priority=%d)", action.action_id, action.priority)
        return True

    def get_next_action(self) -> ProposedAction | None:
        """Retrieve the highest priority pending action."""
        if not self._pending:
            return None
        
        pending_list = [a for a in self._pending.values() if a.status == "PENDING"]
        if not pending_list:
            return None
            
        return min(pending_list, key=lambda x: (x.priority, x.proposed_at))

    async def update_status(self, action_id: str, status: str) -> None:
        """Update action status and persist."""
        if action_id in self._pending:
            self._pending[action_id].status = status
            self._save_to_disk()

    def _save_to_disk(self):
        try:
            self._STORAGE_PATH.parent.mkdir(parents=True, exist_ok=True)
            data = [a.to_dict() for a in self._pending.values()]
            with open(self._STORAGE_PATH, "w") as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            logger.warning("ActionProposer failed to save: %s", e)

    def _load_from_disk(self):
        if not self._STORAGE_PATH.exists():
            return
        try:
            with open(self._STORAGE_PATH) as f:
                data = json.load(f)
                for item in data:
                    action = ProposedAction(**item)
                    self._pending[action.action_id] = action
            logger.info("ActionProposer: loaded %d actions from disk", len(self._pending))
        except Exception as e:
            logger.warning("ActionProposer failed to load: %s", e)

    def stats(self) -> dict:
        return {
            "pending_count": sum(1 for a in self._pending.values() if a.status == "PENDING"),
            "total_count": len(self._pending),
            "completed_count": sum(1 for a in self._pending.values() if a.status == "COMPLETED")
        }
