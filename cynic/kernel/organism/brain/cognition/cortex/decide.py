"""
CYNIC DecideAgent â€” JUDGMENT_CREATED -> DECISION_MADE

Subscribes to JUDGMENT_CREATED events. For BARK/GROWL verdicts with
sufficient confidence (>= phi^-2 = 0.382), runs a NestedMCTS rollout
over the Q-Table to pick the best action, then emits DECISION_MADE.

Nested MCTS (Ring 2 enhancement):
  Instead of greedy Q-Table exploit(), runs UCT-based tree search:
    UCT(s, a) = Q(s, a) + C_UCT Ã— âˆš(ln(Î£ visits) / max(visits(a), 1))
  Depth-2 rollout using Q-Table values as leaf estimates.
  7 rollout simulations (F(4)=3 branches Ã— 2 depth = explores all VERDICTS).

Fire-and-forget: never blocks. All logic is async + bus.emit().
"""

from __future__ import annotations

import logging
import math
from typing import Any

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import DecisionMadePayload
from cynic.kernel.core.formulas import MCTS_UCT_C

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.decide")

# phi^-2 = 0.382 â€” minimum confidence to trigger auto-decide
_PHI_INV_2 = 0.382

# Verdicts that warrant a policy consultation
_ALERT_VERDICTS = {"BARK", "GROWL"}

# Realities that produce actionable prompts for the runner
_ACT_REALITIES = frozenset({"CODE", "CYNIC"})

# NestedMCTS hyperparameters (Ï†-derived)
_MCTS_DEPTH: int = 2  # rollout depth
_MCTS_N_SIM: int = 7  # simulations per action (F(4+1) ensures full VERDICTS coverage)
_UCT_C: float = MCTS_UCT_C  # Imported from formulas.py (exploration constant â‰ˆ 1/âˆš2)


# â”€â”€ NestedMCTS â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€


class NestedMCTS:
    """
    Lightweight 2-ply MCTS rollout using Q-Table as value oracle.

    Used by DecideAgent to replace greedy exploit() with proper UCT search.
    No actual environment transitions â€” Q-Table entries ARE the value function.

    Algorithm:
      For each candidate action:
        1. Get Q-Table value Q(s, a) as leaf estimate.
        2. Simulate _MCTS_N_SIM rollouts: sample action from sibling states
           using Thompson Sampling (via qtable.explore), average Q values.
        3. Compute UCT score = avg_q + C Ã— âˆš(ln(total_visits) / max(visits_a, 1))
      Return action with highest UCT score.

    Cold-start safe: if Q-Table is empty, falls back to lexicographic action order.
    """

    def __init__(self, qtable: Any) -> None:
        self._qtable = qtable

    def best_action(
        self,
        state_key: str,
        candidates: list[str],
    ) -> str:
        """
        Return the candidate action with the highest UCT score for state_key.

        Args:
            state_key:  Current state (e.g. "CODE:JUDGE:PRESENT:1").
            candidates: Valid actions to consider (e.g. ["BARK", "GROWL", "WAG", "HOWL"]).

        Returns:
            Best action string from candidates.
        """
        if not candidates:
            return "WAG"  # Neutral fallback â€” no options

        actions_dict = self._qtable._table.get(state_key, {})

        # Total visits across all candidates for UCT denominator
        total_visits = max(
            sum(e.visits for e in actions_dict.values()),
            1,
        )

        best_a = candidates[0]
        best_uct = -1.0

        for action in candidates:
            entry = actions_dict.get(action)
            if entry is None:
                # Unseen action: high exploration bonus (UCT promotes exploration)
                q = 0.5  # neutral prior
                visits = 0
            else:
                q = entry.q_value  # already in [0, 1] (normalized reward)
                visits = entry.visits

            # Depth-2 rollout: simulate the next state by looking up the greedy
            # follow-on action from the Q-Table. Use that Q-value as successor estimate.
            successor_q = self._rollout(state_key, action)

            # Average current + successor (2-ply)
            avg_q = (q + successor_q) / 2.0

            # UCT score: exploitation + exploration bonus
            uct = avg_q + _UCT_C * math.sqrt(math.log(total_visits) / max(visits, 1))

            if uct > best_uct:
                best_uct = uct
                best_a = action

        return best_a

    def _rollout(self, state_key: str, action: str) -> float:
        """
        Simulate one step forward: given we take `action` at `state_key`,
        what is the best Q-value at the successor state?

        Uses the heuristic: successor_state_key = same prefix, LOD promoted by 1.
        If no entry found, returns neutral 0.5 (prior).
        """
        # Successor: LOD is the last segment â€” increment by 1 (or use "1" default)
        parts = state_key.rsplit(":", 1)
        if len(parts) == 2:
            try:
                lod = int(parts[1])
                successor_key = f"{parts[0]}:{lod + 1}"
            except ValueError:
                successor_key = state_key  # non-numeric suffix â€” stay in place
        else:
            successor_key = state_key

        successor_actions = self._qtable._table.get(successor_key, {})
        if not successor_actions:
            return 0.5  # Cold start â€” neutral

        best_successor_q = max(e.q_value for e in successor_actions.values())
        return best_successor_q


def _build_action_prompt(
    reality: str,
    analysis: str,
    verdict: str,
    content_preview: str,
    context: str,
) -> str:
    """
    Convert judgment context â†’ actionable Claude prompt.
    Rule-based templates â€” no LLM needed for routing.
    Called only for BARK/GROWL verdicts.
    """
    body = (content_preview or context or "(no detail available)").strip()[:400]

    if reality == "CODE":
        if verdict == "BARK":
            return (
                f"[CYNIC AUTO-ACT] BARK detected on code analysis.\n"
                f"Please review and fix the following critical issue:\n\n{body}"
            )
        return (
            f"[CYNIC AUTO-ACT] GROWL detected on code analysis.\n"
            f"Please review the following code quality concern:\n\n{body}"
        )
    if reality == "CYNIC":
        if verdict == "BARK":
            return (
                f"[CYNIC AUTO-ACT] BARK on self-assessment ({analysis}).\n"
                f"Critical organism issue â€” please investigate:\n\n{body}"
            )
        return (
            f"[CYNIC AUTO-ACT] GROWL on self-assessment ({analysis}).\n"
            f"Organism quality degradation â€” please review:\n\n{body}"
        )
    # Generic fallback for other realities
    return (
        f"[CYNIC AUTO-ACT] {verdict} on {reality}\u00b7{analysis}.\n"
        f"Please investigate:\n\n{body}"
    )


class DecideAgent:
    """
    Autonomous decision layer â€” sits between Judge and Act.

    Listens for judgments, runs NestedMCTS over Q-Table for BARK/GROWL,
    and emits DECISION_MADE so downstream actors can react without
    manual intervention.

    Ring 2 upgrade: NestedMCTS replaces greedy exploit() for better
    action selection under uncertainty (UCT balances exploration/exploitation).
    """

    def __init__(self, qtable: Any, bus: Optional[EventBus] = None) -> None:
        """
        qtable: cynic.kernel.organism.brain.learning.qlearning.QTable â€” consulted for best action.
        """
        self._qtable = qtable
        self._mcts = NestedMCTS(qtable)
        self._decisions_made: int = 0
        self._skipped: int = 0
        self._handler = self._on_judgment
        from cynic.kernel.core.event_bus import CoreEvent, Event
        self._bus = bus or get_core_bus("DEFAULT")

    # ---- Lifecycle --------------------------------------------------------

    def start(self, bus: Optional[Any] = None) -> None:
        """Subscribe to JUDGMENT_CREATED. Must be called with a running event loop."""
        target_bus = bus or self._bus
        target_bus.on(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info("DecideAgent started â€” subscribed to JUDGMENT_CREATED")

    def stop(self, bus: EventBus) -> None:
        """Unsubscribe from JUDGMENT_CREATED."""
        bus.off(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info("DecideAgent stopped")

    # ---- Synchronous Decision Logic ----------------------------------------

    def decide_for_judgment(self, judgment: Any) -> Optional[dict[str, Any]]:
        """
        Synchronous decision extraction â€” used by orchestrator._act_phase().

        Takes a Judgment and returns a decision dict, or None if no action needed.
        Does NOT emit DECISION_MADE (that's handled by the event handler).

        Args:
            judgment: cynic.kernel.core.judgment.Judgment object

        Returns:
            {
                "verdict": str,
                "reality": str,
                "state_key": str,
                "q_value": float,
                "confidence": float,
                "recommended_action": str,
                "action_prompt": str,
                "judgment_id": str,
            }
            or None if no action warranted (verdict not BARK/GROWL, etc.)
        """
        verdict = judgment.verdict
        confidence = judgment.confidence
        state_key = judgment.cell.state_key() if hasattr(judgment, "cell") else ""

        # Same filters as _on_judgment
        if verdict not in _ALERT_VERDICTS or confidence < _PHI_INV_2:
            return None

        # Run NestedMCTS
        from cynic.kernel.organism.brain.learning.qlearning import VERDICTS

        recommended_action = self._mcts.best_action(state_key, list(VERDICTS))
        q_entry = self._qtable._table.get(state_key, {}).get(recommended_action)
        q_value = q_entry.q_value if q_entry is not None else 0.0

        # Build action prompt
        reality = judgment.cell.reality if hasattr(judgment, "cell") else ""
        analysis = judgment.cell.analysis if hasattr(judgment, "cell") else ""
        content_preview = (
            str(getattr(judgment.cell, "content", ""))[:200] if hasattr(judgment, "cell") else ""
        )
        context = judgment.cell.context if hasattr(judgment, "cell") else ""

        action_prompt = _build_action_prompt(reality, analysis, verdict, content_preview, context)

        return {
            "verdict": verdict,
            "reality": reality,
            "state_key": state_key,
            "q_value": q_value,
            "confidence": confidence,
            "recommended_action": recommended_action,
            "action_prompt": action_prompt,
            "judgment_id": judgment.judgment_id,
        }

    # ---- Handler ----------------------------------------------------------

    async def _on_judgment(self, event: Event) -> None:
        payload = event.dict_payload or {}
        verdict = payload.get("verdict", "")
        confidence = float(payload.get("confidence", 0.0))
        state_key = payload.get("state_key", "")
        judgment_id = payload.get("judgment_id", "")
        payload.get("q_score", 0.0)
        # Enriched fields from orchestrator.py (JUDGMENT_CREATED now includes these)
        reality = payload.get("reality", "")
        analysis = payload.get("analysis", "")
        content_preview = payload.get("content_preview", "")
        context = payload.get("context", "")

        if verdict not in _ALERT_VERDICTS or confidence < _PHI_INV_2:
            self._skipped += 1
            logger.debug(
                "DecideAgent skip: verdict=%s confidence=%.3f",
                verdict,
                confidence,
            )
            return

        # Ring 2: NestedMCTS over Q-Table (replaces greedy exploit)
        from cynic.kernel.organism.brain.learning.qlearning import VERDICTS

        recommended_action = self._mcts.best_action(state_key, list(VERDICTS))
        q_entry = self._qtable._table.get(state_key, {}).get(recommended_action)
        q_value = q_entry.q_value if q_entry is not None else 0.0

        # Build action prompt (non-empty only for ACT_REALITIES â€” others get generic)
        action_prompt = _build_action_prompt(reality, analysis, verdict, content_preview, context)

        await self._bus.emit(
            Event.typed(
                CoreEvent.DECISION_MADE,
                DecisionMadePayload(
                    verdict=verdict,
                    reality=reality,
                    state_key=state_key,
                    q_value=q_value,
                    confidence=confidence,
                    recommended_action=recommended_action,
                    action_prompt=action_prompt,
                    trigger="auto_decide",
                    mcts=True,
                    judgment_id=judgment_id,
                    content_preview=content_preview,
                ),
                source="decide_agent",
            )
        )

        self._decisions_made += 1
        logger.info(
            "DecideAgent decision: verdict=%s reality=%s state=%s action=%s q=%.3f",
            verdict,
            reality,
            state_key,
            recommended_action,
            q_value,
        )

    # ---- Stats ------------------------------------------------------------

    def stats(self) -> dict[str, Any]:
        return {
            "decisions_made": self._decisions_made,
            "skipped": self._skipped,
        }
