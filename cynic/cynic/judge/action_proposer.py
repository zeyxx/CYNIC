"""
CYNIC ActionProposer — DECISION_MADE → ProposedAction queue

Translates automated decisions into human-readable proposed actions.
Writes ~/.cynic/pending_actions.json so the CLI shows pending work
and the human can accept/reject before execution.

Action lifecycle:
    PENDING → ACCEPTED (human approved) → execution attempted
    PENDING → REJECTED (human declined)
    PENDING → AUTO_EXECUTED (runner already fired it automatically)

Action types (verdict × reality → priority):
    BARK × CODE/CYNIC   → INVESTIGATE  (priority 1 — critical, must fix)
    GROWL × CODE/CYNIC  → REFACTOR     (priority 2 — quality concern)
    BARK × other        → ALERT        (priority 2 — non-code alert)
    GROWL × other       → MONITOR      (priority 3 — watch and see)

Priority scale (φ-derived, 1=highest urgency):
    1 → critical (BARK on core realities)
    2 → important (GROWL on core, BARK elsewhere)
    3 → normal (GROWL elsewhere)
    4 → FYI (informational only)

Queue: in-memory + ~/.cynic/pending_actions.json (disk)
Capped at F(11)=89 entries (rolling BURN axiom).

Usage:
    proposer = ActionProposer()
    proposer.start(get_core_bus())
    # → now auto-writes on every DECISION_MADE

    pending = proposer.pending()          # list of PENDING ProposedActions
    proposer.accept("abc12345")           # mark as ACCEPTED
    proposer.reject("abc12345")           # mark as REJECTED
    proposer.mark_auto_executed("abc12345")  # called by _on_decision_made in server.py
"""
from __future__ import annotations

import json
import logging
import os
import time
import uuid
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional

from cynic.core.event_bus import CoreEvent, Event, EventBus, get_core_bus
from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.judge.action_proposer")

_QUEUE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "pending_actions.json")
_MAX_QUEUE  = fibonacci(11)  # 89 — BURN axiom: rolling window

# Realities that produce code-level actions (INVESTIGATE/REFACTOR)
_CODE_REALITIES = frozenset({"CODE", "CYNIC"})


# ── ProposedAction dataclass ─────────────────────────────────────────────────

@dataclass
class ProposedAction:
    """
    A human-reviewable action proposal generated from a DECISION_MADE event.

    Fields:
        action_id:    8-char UUID prefix — stable cross-session identifier
        judgment_id:  Traceability back to the original judgment
        state_key:    Q-Table state key the decision was made for
        verdict:      BARK or GROWL that triggered this proposal
        reality:      CODE / CYNIC / MARKET / SOCIAL / HUMAN / SOLANA / COSMOS
        action_type:  INVESTIGATE / REFACTOR / ALERT / MONITOR
        description:  Human-readable one-liner (≤120 chars)
        prompt:       Full action prompt (Claude-ready)
        q_score:      Q-value at time of proposal (context for human)
        priority:     1=critical … 4=FYI
        proposed_at:  Unix timestamp
        status:       PENDING / ACCEPTED / REJECTED / AUTO_EXECUTED
    """
    action_id:   str
    judgment_id: str
    state_key:   str
    verdict:     str
    reality:     str
    action_type: str
    description: str
    prompt:      str
    q_score:     float
    priority:    int
    proposed_at: float
    status:      str = "PENDING"
    parent_action_id: Optional[str] = None
    chain_depth:      int = 0
    generated_by:     str = "JUDGE"  # JUDGE | VERIFY | SELF

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "ProposedAction":
        return ProposedAction(**{k: v for k, v in d.items() if k in ProposedAction.__dataclass_fields__})


# ── Helpers ───────────────────────────────────────────────────────────────────

def _action_type_and_priority(verdict: str, reality: str) -> tuple[str, int]:
    """Map verdict × reality → (action_type, priority)."""
    if verdict == "BARK":
        if reality in _CODE_REALITIES:
            return "INVESTIGATE", 1
        return "ALERT", 2
    # GROWL
    if reality in _CODE_REALITIES:
        return "REFACTOR", 2
    return "MONITOR", 3


def _description(verdict: str, reality: str, action_type: str, content_preview: str) -> str:
    """Generate a ≤120-char human-readable description."""
    preview = (content_preview or "").strip().replace("\n", " ")[:60]
    if action_type == "INVESTIGATE":
        base = f"[{verdict}] Investigate critical issue in {reality}"
    elif action_type == "REFACTOR":
        base = f"[{verdict}] Refactor quality concern in {reality}"
    elif action_type == "ALERT":
        base = f"[{verdict}] Alert: {reality} signal requires attention"
    else:
        base = f"[{verdict}] Monitor {reality} — quality below threshold"
    if preview:
        return f"{base}: {preview}"[:120]
    return base[:120]


# ── ActionProposer ───────────────────────────────────────────────────────────

class ActionProposer:
    """
    Subscribes to DECISION_MADE, creates ProposedAction entries,
    and maintains a rolling queue in ~/.cynic/pending_actions.json.

    Thread-safe assumption: single asyncio event loop (standard for FastAPI).
    """

    def __init__(self, queue_path: Optional[str] = None) -> None:
        self._path = queue_path or _QUEUE_PATH
        self._queue: List[ProposedAction] = []
        self._proposed_total: int = 0
        self._handler = self._on_decision_made
        self._load()

    # ── Lifecycle ─────────────────────────────────────────────────────────────

    def start(self, bus: EventBus) -> None:
        bus.on(CoreEvent.DECISION_MADE, self._handler)
        logger.info("ActionProposer started — subscribed to DECISION_MADE")

    def stop(self, bus: EventBus) -> None:
        bus.off(CoreEvent.DECISION_MADE, self._handler)
        logger.info("ActionProposer stopped")

    # ── Handler ───────────────────────────────────────────────────────────────

    async def _on_decision_made(self, event: Event) -> None:
        p = event.payload or {}
        verdict        = p.get("recommended_action", "")  # WAG/BARK/etc from QTable
        judgment_id    = p.get("judgment_id", "")
        state_key      = p.get("state_key", "")
        reality        = p.get("reality", "CODE")
        content_preview = p.get("content_preview", "")
        prompt         = p.get("action_prompt", "")
        q_value        = float(p.get("q_value", 0.0))

        # Chain depth inheritance: if event carries parent info, propagate depth
        parent_action_id: Optional[str] = p.get("parent_action_id", None)
        parent_chain_depth: int = int(p.get("chain_depth", 0))
        generated_by: str = p.get("generated_by", "JUDGE")
        chain_depth = parent_chain_depth + 1 if parent_action_id else 0

        if not verdict:
            return

        action_type, priority = _action_type_and_priority(verdict, reality)
        desc = _description(verdict, reality, action_type, content_preview)

        action = ProposedAction(
            action_id   = uuid.uuid4().hex[:8],
            judgment_id = judgment_id,
            state_key   = state_key,
            verdict     = verdict,
            reality     = reality,
            action_type = action_type,
            description = desc,
            prompt      = prompt[:1000],  # cap prompt length in JSON
            q_score     = round(q_value, 3),
            priority    = priority,
            proposed_at = time.time(),
            status      = "PENDING",
            parent_action_id = parent_action_id,
            chain_depth      = chain_depth,
            generated_by     = generated_by,
        )

        self._queue.append(action)
        self._proposed_total += 1

        # Rolling cap (BURN axiom)
        if len(self._queue) > _MAX_QUEUE:
            self._queue = self._queue[-_MAX_QUEUE:]

        self._save()

        logger.info(
            "ActionProposer: %s [%s] %s (priority=%d) → pending_actions.json",
            action.action_id, action.action_type, action.description[:60], priority,
        )

        # Emit ACTION_PROPOSED so other components can react
        await get_core_bus().emit(Event(
            type=CoreEvent.ACTION_PROPOSED,
            payload={
                "action_id":   action.action_id,
                "action_type": action.action_type,
                "verdict":     verdict,
                "reality":     reality,
                "priority":    priority,
                "description": desc,
            },
            source="action_proposer",
        ))

    # ── L4→P5 bridge: SelfProposal → ProposedAction ──────────────────────────

    def propose_self_improvement(self, proposal: Dict[str, Any]) -> Optional[ProposedAction]:
        """
        Convert a SelfProposal dict → ProposedAction (priority=4, action_type=IMPROVE).

        Called by state.py on SELF_IMPROVEMENT_PROPOSED.
        Closes the L4→P5 feedback loop: self-insight → human-visible action queue.

        proposal: SelfProposal.to_dict() — keys: probe_id, target, recommendation,
                  dimension, pattern_type, severity, current_value, suggested_value.
        """
        try:
            rec   = (proposal.get("recommendation") or "").strip()
            target = (proposal.get("target") or "unknown").strip()
            dim   = proposal.get("dimension", "UNKNOWN")
            sev   = float(proposal.get("severity", 0.5))
            if not rec:
                return None

            # Severity → priority: HIGH (≥0.618) → 2, MEDIUM (≥0.382) → 3, LOW → 4
            from cynic.core.phi import PHI_INV, PHI_INV_2
            if sev >= PHI_INV:
                priority = 2
            elif sev >= PHI_INV_2:
                priority = 3
            else:
                priority = 4

            desc = f"[IMPROVE/{dim}] {rec}"[:120]
            prompt = (
                f"Self-improvement proposal from L4 analysis.\n"
                f"Target: {target}\n"
                f"Pattern: {proposal.get('pattern_type', '?')} (severity={sev:.2f})\n"
                f"Current: {proposal.get('current_value', '?')} → Suggested: {proposal.get('suggested_value', '?')}\n"
                f"Action: {rec}"
            )

            action = ProposedAction(
                action_id   = uuid.uuid4().hex[:8],
                judgment_id = proposal.get("probe_id", ""),
                state_key   = target,
                verdict     = "IMPROVE",
                reality     = "CYNIC",
                action_type = "IMPROVE",
                description = desc,
                prompt      = prompt[:1000],
                q_score     = round(sev * 100.0, 1),  # severity → Q-scale
                priority    = priority,
                proposed_at = float(proposal.get("proposed_at", time.time())),
                status      = "PENDING",
            )

            self._queue.append(action)
            self._proposed_total += 1
            if len(self._queue) > _MAX_QUEUE:
                self._queue = self._queue[-_MAX_QUEUE:]
            self._save()
            logger.info(
                "ActionProposer: %s [IMPROVE/%s] %s (priority=%d) via L4",
                action.action_id, dim, desc[:50], priority,
            )
            return action
        except Exception as exc:
            logger.debug("propose_self_improvement failed: %s", exc)
            return None

    # ── Query / Mutation ──────────────────────────────────────────────────────

    def pending(self) -> List[ProposedAction]:
        """All actions with status=PENDING, sorted by priority (1=first)."""
        return sorted(
            [a for a in self._queue if a.status == "PENDING"],
            key=lambda a: (a.priority, a.proposed_at),
        )

    def all_actions(self) -> List[ProposedAction]:
        """Full queue, most recent last."""
        return list(self._queue)

    def get(self, action_id: str) -> Optional[ProposedAction]:
        for a in self._queue:
            if a.action_id == action_id:
                return a
        return None

    def accept(self, action_id: str) -> Optional[ProposedAction]:
        """Mark action as ACCEPTED. Returns the action or None if not found."""
        return self._set_status(action_id, "ACCEPTED")

    def reject(self, action_id: str) -> Optional[ProposedAction]:
        """Mark action as REJECTED. Returns the action or None if not found."""
        return self._set_status(action_id, "REJECTED")

    def mark_auto_executed(self, action_id: str) -> Optional[ProposedAction]:
        """Mark action as AUTO_EXECUTED (runner fired it automatically)."""
        return self._set_status(action_id, "AUTO_EXECUTED")

    def mark_completed(self, action_id: str, success: bool) -> Optional[ProposedAction]:
        """Mark action as COMPLETED or FAILED after execution result arrives (T30)."""
        status = "COMPLETED" if success else "FAILED"
        return self._set_status(action_id, status)

    def _set_status(self, action_id: str, status: str) -> Optional[ProposedAction]:
        for a in self._queue:
            if a.action_id == action_id:
                a.status = status
                self._save()
                logger.info("ActionProposer: %s → %s", action_id, status)
                return a
        logger.warning("ActionProposer: action_id %s not found", action_id)
        return None

    # ── Persistence ───────────────────────────────────────────────────────────

    def _save(self) -> None:
        try:
            os.makedirs(os.path.dirname(self._path), exist_ok=True)
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump([a.to_dict() for a in self._queue], fh, indent=2)
        except Exception as exc:
            logger.debug("ActionProposer: save failed: %s", exc)

    def _load(self) -> None:
        try:
            if not os.path.exists(self._path):
                return
            with open(self._path, encoding="utf-8") as fh:
                raw = json.load(fh)
            if isinstance(raw, list):
                self._queue = [ProposedAction.from_dict(d) for d in raw]
                logger.info("ActionProposer: loaded %d actions from disk", len(self._queue))
        except Exception as exc:
            logger.debug("ActionProposer: load failed: %s", exc)
            self._queue = []

    # ── Stats ─────────────────────────────────────────────────────────────────

    def stats(self) -> Dict[str, Any]:
        statuses: Dict[str, int] = {}
        for a in self._queue:
            statuses[a.status] = statuses.get(a.status, 0) + 1
        return {
            "proposed_total": self._proposed_total,
            "queue_size":     len(self._queue),
            "pending":        statuses.get("PENDING", 0),
            "accepted":       statuses.get("ACCEPTED", 0),
            "rejected":       statuses.get("REJECTED", 0),
            "auto_executed":  statuses.get("AUTO_EXECUTED", 0),
            "completed":      statuses.get("COMPLETED", 0),
            "failed":         statuses.get("FAILED", 0),
        }
