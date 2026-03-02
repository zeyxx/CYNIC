"""
ActionProposer - The Strategy Layer.

Receives DECISION_MADE events from the JudgeOrchestrator and translates them
into actionable proposals for the organism's motor system (ActHandlers).

Memory Unification: Now uses SurrealDB ActionProposalRepo instead of JSON files.
"""

from __future__ import annotations

import logging
import time
import uuid
from dataclasses import dataclass, field
from typing import Any

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import ActionProposedPayload, DecisionMadePayload
from cynic.kernel.core.storage.interface import ActionProposalRepoInterface

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
            "metadata": self.metadata,
        }


class ActionProposer:
    """
    Manages the lifecycle of proposed actions from judgment outcomes.
    Persistence is fully handled by SurrealDB via the injected repository.
    """

    def __init__(self, bus: EventBus, repo: ActionProposalRepoInterface):
        self.repo = repo
        self._last_stats = {"pending": 0, "total": 0}
        self._bus = bus

    def start(self):
        """Subscribe to decision events."""
        self._bus.on(CoreEvent.DECISION_MADE, self.on_decision_made)
        logger.info("ActionProposer started - linked to SurrealDB")

    def stop(self) -> None:
        """Unregister from bus decision events."""
        try:
            self._bus.off(CoreEvent.DECISION_MADE, self.on_decision_made)
        except Exception as e:
            logger.debug(f"Error unregistering ActionProposer listener: {e}")
        logger.info("ActionProposer stopped")

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
                metadata={"q_value": payload.q_value, "confidence": payload.confidence},
            )

            await self.add_proposal(action)

        except Exception as e:
            logger.error("ActionProposer failed to process decision: %s", e)

    async def add_proposal(self, action: ProposedAction) -> bool:
        """Add a new proposal to SurrealDB and notify the system."""
        try:
            # Persistence (SurrealDB)
            await self.repo.upsert(action.to_dict())

            # Emit ACTION_PROPOSED
            await self._bus.emit(
                Event.typed(
                    CoreEvent.ACTION_PROPOSED,
                    ActionProposedPayload(
                        action_id=action.action_id,
                        judgment_id=action.judgment_id,
                        reality=action.reality,
                        priority=action.priority,
                        action_prompt=action.action_prompt,
                    ),
                    source="action_proposer",
                )
            )

            logger.info(
                "ACTION PROPOSED: %s (priority=%d) -> Stored in SurrealDB",
                action.action_id,
                action.priority,
            )
            return True
        except Exception as e:
            logger.error(f"ActionProposer: Failed to persist to SurrealDB: {e}")
            return False

    async def get_next_action(self) -> ProposedAction | None:
        """Retrieve the highest priority pending action from SurrealDB."""
        try:
            pending = await self.repo.all_pending()
            if not pending:
                return None

            # Map back to dataclass
            data = pending[0]
            return ProposedAction(**data)
        except Exception:
            return None

    async def update_status(self, action_id: str, status: str) -> None:
        """Update action status in SurrealDB."""
        try:
            await self.repo.update_status(action_id, status)
        except Exception as e:
            logger.error(f"Failed to update action status in SurrealDB: {e}")

    async def refresh_stats(self):
        """Update cached stats from DB."""
        try:
            all_actions = await self.repo.all()
            pending = [a for a in all_actions if a["status"] == "PENDING"]
            self._last_stats = {"pending": len(pending), "total": len(all_actions)}
        except Exception:
            pass

    def stats(self) -> dict:
        return {
            "pending_count": self._last_stats["pending"],
            "total_count": self._last_stats["total"],
        }
