"""
ActHandler — STEP 3 (DECIDE) + STEP 4 (ACT) unified action execution.

Extracted from JudgeOrchestrator._act_phase().

Responsibility:
- Call DecideAgent to recommend action (DECIDE phase)
- Validate decision through guardrails (PowerLimiter, Alignment, HumanGate, Audit)
- Filter by actionable realities (CODE, BALANCE, etc.)
- Execute action via Runner (ACT phase)
- Emit ACT_COMPLETED event + handle blocking
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Any, Optional, TYPE_CHECKING

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.cognition.cortex.decision_validator import BlockedDecision
from cynic.core.judgment import Judgment
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import DecisionMadePayload, ActCompletedPayload

if TYPE_CHECKING:
    from cynic.cognition.cortex.orchestrator import JudgmentPipeline

logger = logging.getLogger("cynic.cognition.cortex.handlers.act_executor")


class ActHandler(BaseHandler):
    """
    DECIDE + ACT phase handler.

    Injects:
    - decide_agent: DecideAgent (for judgment→decision recommendation)
    - decision_validator: DecisionValidator (optional, for guardrail checks)
    - runner: Runner (for action execution)
    """

    handler_id = "act_executor"
    version = "1.0"
    description = "ACT phase: DECIDE + execute actions with guardrails"

    def __init__(
        self,
        decide_agent: Optional[Any] = None,
        decision_validator: Optional[Any] = None,
        runner: Optional[Any] = None,
        **kwargs: Any,
    ) -> None:
        self.decide_agent = decide_agent
        self.decision_validator = decision_validator
        self.runner = runner

    async def execute(
        self,
        judgment: Judgment,
        pipeline: Optional[JudgmentPipeline] = None,
        recent_judgments: Optional[list[Any]] = None,
        **kwargs: Any,
    ) -> HandlerResult:
        """
        Execute DECIDE + ACT phases for a judgment.

        Args:
            judgment: Final judgment from JUDGE phase
            pipeline: JudgmentPipeline (optional, for context)
            recent_judgments: Recent judgments for guardrail context

        Returns:
            HandlerResult with action_result dict in output, or None if no action taken
        """
        t0 = time.perf_counter()
        try:
            action_result = await self._act_phase(judgment, pipeline, recent_judgments)
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution("act_phase_complete", f"action={bool(action_result)}")

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=action_result,
                duration_ms=duration_ms,
                metadata={
                    "judgment_id": judgment.judgment_id,
                    "action_executed": bool(action_result),
                },
            )
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_act", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )

    async def _act_phase(
        self,
        judgment: Judgment,
        pipeline: Optional[JudgmentPipeline],
        recent_judgments: Optional[list[Any]],
    ) -> Optional[dict]:
        """
        STEP 3 (DECIDE) + STEP 4 (ACT) — unified action execution with guardrails.

        1. Call DecideAgent.decide_for_judgment() to get action recommendation (DECIDE)
        2. Call DecisionValidator to pass all guardrails (PowerLimiter, Alignment, HumanGate, Audit)
        3. If validation passes and reality warrants action, call runner.execute() (ACT)
        4. Return action result or None if no action taken/blocked

        Args:
            judgment: Final judgment from JUDGE phase
            pipeline: JudgmentPipeline (optional, for context/logging)
            recent_judgments: Recent judgments for guardrail context

        Returns:
            {
                "action_id": str,
                "success": bool,
                "output": str,
                "duration_ms": float,
                "error": str or None,
            }
            or None if no action warranted/blocked
        """
        if not self.decide_agent:
            return None

        # STEP 3: DECIDE — run DecideAgent synchronously
        decision = self.decide_agent.decide_for_judgment(judgment)
        if not decision:
            return None  # No action needed

        # Helper: emit DECISION_MADE for human review (consolidates duplicate emissions)
        async def emit_decision_made(trigger: str, error: Optional[str] = None) -> None:
            await get_core_bus().emit(Event.typed(
                CoreEvent.DECISION_MADE,
                DecisionMadePayload(
                    verdict=decision["verdict"],
                    reality=decision["reality"],
                    state_key=decision.get("state_key", ""),
                    q_value=decision.get("q_value", 0.0),
                    confidence=decision.get("confidence", 0.0),
                    recommended_action=decision.get("recommended_action", ""),
                    action_prompt=decision.get("action_prompt", ""),
                    trigger=trigger,
                    mcts=True,
                    judgment_id=decision.get("judgment_id", ""),
                ),
                source="orchestrator_act_phase",
            ))

        # GUARDRAIL VALIDATION — DecisionValidator chains all safety checks
        if self.decision_validator:
            try:
                scheduler = getattr(pipeline, 'scheduler', None) if pipeline else None
                await self.decision_validator.validate_decision(
                    decision=decision,
                    judgment=judgment,
                    recent_judgments=recent_judgments or [],
                    scheduler=scheduler,
                )
                # Decision passed all guardrails
                logger.info(f"Decision validated: {decision['verdict']} → proceeding to ACT")
            except BlockedDecision as e:
                # Decision blocked by guardrail
                logger.warning(
                    f"Decision BLOCKED [{e.guardrail}]: {e.reason} "
                    f"→ {e.recommendation}"
                )
                await emit_decision_made(trigger="guardrail_blocked")
                # Return block result without executing
                return {
                    "action_id": decision.get("judgment_id", "")[:8],
                    "success": False,
                    "output": "",
                    "duration_ms": 0.0,
                    "error": f"[{e.guardrail}] {e.reason}",
                }
        else:
            logger.debug("No DecisionValidator available — skipping guardrail checks")

        # Filter: only execute for actionable realities
        from cynic.cognition.cortex.decide import _ACT_REALITIES

        if decision["reality"] not in _ACT_REALITIES:
            # Emit for human review, but don't auto-execute
            await emit_decision_made(trigger="not_actionable_reality")
            return None

        # STEP 4: ACT — execute the action
        if not self.runner:
            logger.warning("No runner available — cannot execute action")
            return None

        t0 = time.perf_counter()
        try:
            action_result = await self.runner.execute(
                prompt=decision["action_prompt"],
                timeout=30,
            )
            duration_ms = (time.perf_counter() - t0) * 1000

            result = {
                "action_id": decision.get("judgment_id", "")[:8],
                "success": action_result.get("success", False),
                "output": action_result.get("output", ""),
                "duration_ms": duration_ms,
                "error": action_result.get("error"),
            }

            # Emit ACT_COMPLETED event (for feedback loops L3, L4)
            await get_core_bus().emit(Event.typed(
                CoreEvent.ACT_COMPLETED,
                ActCompletedPayload(
                    success=result["success"],
                    action_id=result["action_id"],
                    duration_ms=result["duration_ms"],
                    error=result["error"],
                ),
            ))

            logger.info(
                "ACT: executed %s (success=%s, %.0fms)",
                result["action_id"], result["success"], duration_ms,
            )
            return result

        except CynicError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            logger.error("ACT: execution failed: %s (%.0fms)", e, duration_ms)
            return {
                "action_id": decision.get("judgment_id", "")[:8],
                "success": False,
                "output": "",
                "duration_ms": duration_ms,
                "error": str(e),
            }
