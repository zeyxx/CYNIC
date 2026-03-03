"""
ActHandler " STEP 3 (DECIDE) + STEP 4 (ACT) unified action execution.

Extracted from JudgeOrchestrator._act_phase().

Responsibility:
- Call DecideAgent to recommend action (DECIDE phase)
- Validate decision through guardrails (PowerLimiter, Alignment, HumanGate, Audit)
- Filter by actionable realities (CODE, BALANCE, etc.)
- Execute action via Runner (ACT phase)
- Emit ACT_COMPLETED event + handle blocking
"""

from __future__ import annotations

import logging
import time
from typing import TYPE_CHECKING, Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus, EventBusError
from cynic.kernel.core.events_schema import ActCompletedPayload, DecisionMadePayload
from cynic.kernel.core.exceptions import CynicError
from cynic.kernel.core.judgment import Judgment
from cynic.kernel.organism.brain.cognition.cortex.decision_validator import BlockedDecision
from cynic.kernel.organism.brain.cognition.cortex.handlers.base import BaseHandler, HandlerResult

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgmentPipeline

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.handlers.act_executor")


class ActHandler(BaseHandler):
    """
    DECIDE + ACT phase handler.

    Injects:
    - decide_agent: DecideAgent (for judgment'decision recommendation)
    - decision_validator: DecisionValidator (optional, for guardrail checks)
    - runner: Runner (for action execution)
    """

    handler_id = "act_executor"
    version = "1.0"
    description = "ACT phase: DECIDE + execute actions with guardrails"

    def __init__(
        self,
        decide_agent: Any | None = None,
        decision_validator: Any | None = None,
        runner: Any | None = None,
        gasdf_executor: Any | None = None,
        agency_manager: Any | None = None,
        body: Any | None = None,
        motor_system: Any | None = None,
        bus: Optional[EventBus] = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(bus=bus)
        self.decide_agent = decide_agent
        self.decision_validator = decision_validator
        self.runner = runner
        self.gasdf_executor = gasdf_executor
        self.agency_manager = agency_manager
        self.body = body

        # Lazy import to avoid circular dependencies
        if motor_system is None and self.body:
            from cynic.kernel.organism.layers.motor import MotorSystem

            self.motor_system = MotorSystem(body=self.body)
        else:
            self.motor_system = motor_system

    async def execute(
        self,
        judgment: Judgment,
        pipeline: JudgmentPipeline | None = None,
        recent_judgments: list[Any] | None = None,
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
            # Inject agency_manager if provided
            if "agency_manager" in kwargs and self.agency_manager is None:
                self.agency_manager = kwargs["agency_manager"]

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
        pipeline: JudgmentPipeline | None,
        recent_judgments: list[Any] | None,
    ) -> dict | None:
        """
        STEP 3 (DECIDE) + STEP 4 (ACT) " unified action execution with guardrails.
        """
        # STEP 0: Agency Veto (Phase 3 Spec)
        if self.agency_manager:
            should_execute, reason = self.agency_manager.should_execute(judgment)
            if not should_execute:
                logger.warning("ACT: Manager VETO applied: %s", reason)
                # Emit ACT_FAILED to notify the bus that the action was intentionally blocked
                await self.bus.emit(
                    Event.typed(
                        CoreEvent.ACT_FAILED,
                        {
                            "action_id": judgment.judgment_id[:8],
                            "error": f"VETO: {reason}",
                            "is_veto": True,
                        },
                    )
                )
                return None

        if not self.decide_agent:
            return None

        # STEP 3: DECIDE " run DecideAgent synchronously
        decision = self.decide_agent.decide_for_judgment(judgment)
        if not decision:
            return None  # No action needed

        # Helper: emit DECISION_MADE for human review (consolidates duplicate emissions)
        async def emit_decision_made(trigger: str, error: str | None = None) -> None:
            await self.bus.emit(
                Event.typed(
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
                )
            )

        # GUARDRAIL VALIDATION " DecisionValidator chains all safety checks
        if self.decision_validator:
            try:
                scheduler = getattr(pipeline, "scheduler", None) if pipeline else None
                await self.decision_validator.validate_decision(
                    decision=decision,
                    judgment=judgment,
                    recent_judgments=recent_judgments or [],
                    scheduler=scheduler,
                )
                # Decision passed all guardrails
                logger.info(f"Decision validated: {decision['verdict']} ' proceeding to ACT")
            except BlockedDecision as e:
                # Decision blocked by guardrail
                logger.warning(
                    f"Decision BLOCKED [{e.guardrail}]: {e.reason} " f"' {e.recommendation}"
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
            logger.debug("No DecisionValidator available " skipping guardrail checks")

        # Filter: only execute for actionable realities
        from cynic.kernel.organism.brain.cognition.cortex.decide import _ACT_REALITIES

        is_governance = decision["reality"] == "SOCIAL"
        is_actionable = decision["reality"] in _ACT_REALITIES

        if not is_actionable and not (is_governance and self.gasdf_executor):
            # Emit for human review, but don't auto-execute
            await emit_decision_made(trigger="not_actionable_reality")
            return None

        # STEP 4: ACT " execute the action
        # 4a: GOVERNANCE ACTION (GASdf)
        if is_governance and self.gasdf_executor:
            t0 = time.perf_counter()
            try:
                # Extract proposal context for gasdf execution
                proposal_id = decision.get("judgment_id", "")
                verdict = decision.get("verdict", "UNKNOWN")
                q_score = decision.get("q_value", 0.0)

                # Metadata from cell if available
                cell = judgment.cell if hasattr(judgment, "cell") else None
                metadata = {}
                if cell and hasattr(cell, "context_dict"):
                    metadata = cell.context_dict()

                # Execute on-chain via GASdf
                result = await self.gasdf_executor.execute_verdict(
                    proposal_id=proposal_id,
                    verdict=verdict,
                    community_id=metadata.get("community_id", "cynic_dao"),
                    payment_token=metadata.get("payment_token", ""),
                    user_pubkey=metadata.get("user_pubkey", ""),
                    signed_transaction=metadata.get("signed_transaction", ""),
                    payment_token_account=metadata.get("payment_token_account", ""),
                    q_score=q_score,
                    proposal_context=metadata,
                )

                if result:
                    duration_ms = (time.perf_counter() - t0) * 1000
                    logger.info(
                        "ACT: GASdf execution completed (sig=%s, %.0fms)",
                        result.signature[:8],
                        duration_ms,
                    )

                    # Emit ACT_COMPLETED event
                    await self.bus.emit(
                        Event.typed(
                            CoreEvent.ACT_COMPLETED,
                            ActCompletedPayload(
                                success=True,
                                action_id=proposal_id[:8],
                                duration_ms=duration_ms,
                                metadata={"signature": result.signature, "status": result.status},
                            ),
                        )
                    )

                    return {
                        "action_id": proposal_id[:8],
                        "success": True,
                        "output": f"GASdf Signature: {result.signature}",
                        "duration_ms": duration_ms,
                    }
                else:
                    return None  # Skipped (not HOWL/WAG or low confidence)

            except Exception as e:
                duration_ms = (time.perf_counter() - t0) * 1000
                logger.error("ACT: GASdf execution failed: %s", e)
                return {
                    "action_id": decision.get("judgment_id", "")[:8],
                    "success": False,
                    "output": "",
                    "duration_ms": duration_ms,
                    "error": str(e),
                }

        # 4b: SYSTEM ACTION (Runner)
        if not self.runner:
            logger.warning("No runner available " cannot execute system action")
            return None

        t0 = time.perf_counter()
        try:
            # Execute as an Embodied Gesture if MotorSystem is available
            if self.motor_system:
                action_result = await self.motor_system.execute_gesture(
                    action_type="system_action",
                    effector=self.runner,
                    params={"prompt": decision["action_prompt"], "timeout": 30},
                    base_cost=0.01,
                )
            else:
                # Fallback to direct execution
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
            await self.bus.emit(
                Event.typed(
                    CoreEvent.ACT_COMPLETED,
                    ActCompletedPayload(
                        success=result["success"],
                        action_id=result["action_id"],
                        duration_ms=result["duration_ms"],
                        error=result["error"],
                    ),
                )
            )

            logger.info(
                "ACT: executed %s (success=%s, %.0fms)",
                result["action_id"],
                result["success"],
                duration_ms,
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