"""
ProposalExecutor — Risk classification and dimension-specific execution handlers.

Classifies SelfProposal instances by risk level and executes them with appropriate
handlers for each dimension (QTABLE, METRICS, ESCORE, RESIDUAL, ARCHITECTURE).

Risk levels:
  LOW_RISK        — Severity < 0.2 (METRICS/QTABLE only); auto-executable
  REVIEW_REQUIRED — Severity >= 0.5 OR special dimensions (ESCORE/RESIDUAL/ARCH);
                     requires manual approval before execution
  NOT_EXECUTABLE  — No handler available for dimension

Execution flow:
  1. classify_risk(proposal) → RiskLevel
  2. execute(proposal) → ExecutionResult (success, dimension, message, new_value)

Usage:
    executor = ProposalExecutor()
    executor.set_qtable(qtable)
    executor.set_metrics_collector(collector)

    for proposal in self_prober.pending():
        risk = executor.classify_risk(proposal)
        if risk == RiskLevel.LOW_RISK:
            result = executor.execute(proposal)
            if result.success:
                print(f"Applied: {result.message}")
"""

from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass
from enum import Enum
from typing import Any, Optional

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.proposal_executor")


class RiskLevel(Enum):
    """Risk level classification for proposals."""
    LOW_RISK = "LOW_RISK"
    REVIEW_REQUIRED = "REVIEW_REQUIRED"
    NOT_EXECUTABLE = "NOT_EXECUTABLE"


@dataclass
class ExecutionResult:
    """Result of proposal execution."""
    success: bool
    proposal_id: str
    dimension: str
    message: str = ""
    error_message: str = ""
    old_value: Optional[float] = None
    new_value: Optional[float] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "success": self.success,
            "proposal_id": self.proposal_id,
            "dimension": self.dimension,
            "message": self.message,
            "error_message": self.error_message,
            "old_value": self.old_value,
            "new_value": self.new_value,
        }


class ProposalExecutor:
    """
    Risk classifier and executor for SelfProposal instances.

    Provides:
    - Risk classification based on severity and dimension
    - Dimension-specific execution handlers
    - Graceful failure when dependencies not available
    """

    def __init__(self) -> None:
        self._qtable: Any | None = None
        self._metrics_collector: Any | None = None
        self._escore_tracker: Any | None = None

        # Rate limiting and circuit breaker
        self._rate_limit: float | None = None  # Executions per second
        self._last_execution_times: list[float] = []  # Last 10 execution times
        self._circuit_breaker_threshold: int | None = None  # Max failures before opening
        self._consecutive_failures: int = 0
        self._circuit_open: bool = False
        self._circuit_open_at: float | None = None  # Timestamp when circuit opened
        self._circuit_reset_timeout_s: float = 300.0  # 5 minutes

    # — Injection —————————————————————————————————————————————————————————————

    def set_qtable(self, qtable: Any) -> None:
        """Inject QTable for QTABLE dimension execution."""
        self._qtable = qtable

    def set_metrics_collector(self, collector: Any) -> None:
        """Inject EventMetricsCollector for METRICS dimension execution."""
        self._metrics_collector = collector

    def set_escore_tracker(self, tracker: Any) -> None:
        """Inject EScore tracker for ESCORE dimension execution."""
        self._escore_tracker = tracker

    def set_rate_limit(self, max_per_second: float) -> None:
        """
        Set rate limit for auto-apply.

        Args:
            max_per_second: Maximum executions per second (e.g., 1.0 = 1 execution per second)
        """
        self._rate_limit = max_per_second
        logger.info("ProposalExecutor: Rate limit set to %.2f executions/sec", max_per_second)

    def set_circuit_breaker_threshold(self, max_failures: int) -> None:
        """
        Set circuit breaker threshold.

        After N consecutive failures, disable auto-apply entirely.

        Args:
            max_failures: Number of consecutive failures before opening circuit
        """
        self._circuit_breaker_threshold = max_failures
        logger.info("ProposalExecutor: Circuit breaker threshold set to %d failures", max_failures)

    def is_circuit_open(self) -> bool:
        """
        Check if circuit breaker is open.

        Returns:
            True if circuit is open (too many recent failures), False otherwise
        """
        return self._circuit_open

    def _check_circuit_reset(self) -> bool:
        """
        Check if circuit breaker should auto-reset after timeout.

        After the circuit opens due to consecutive failures, it automatically
        transitions to a closed state after _circuit_reset_timeout_s seconds,
        allowing one test execution to verify recovery.

        Returns:
            True if circuit was auto-reset, False otherwise
        """
        if self._circuit_open and self._circuit_open_at is not None:
            elapsed = time.time() - self._circuit_open_at
            if elapsed >= self._circuit_reset_timeout_s:
                logger.info(
                    "ProposalExecutor: Circuit breaker auto-reset after %.1f seconds timeout",
                    self._circuit_reset_timeout_s,
                )
                self._circuit_open = False
                self._circuit_open_at = None
                self._consecutive_failures = 0
                return True
        return False

    def reset_circuit(self) -> None:
        """
        Manually reset circuit breaker (for emergency recovery).

        Can be called by operators to force recovery from failure cascade.
        """
        if self._circuit_open:
            logger.warning("ProposalExecutor: Circuit breaker manually reset")
            self._circuit_open = False
            self._circuit_open_at = None
            self._consecutive_failures = 0

    async def _apply_rate_limit(self) -> None:
        """
        Enforce rate limit before execution.

        If rate_limit is set, ensures we don't exceed max_per_second.
        Uses last 10 execution timestamps to calculate current rate.
        """
        if self._rate_limit is None or self._rate_limit <= 0:
            return

        now = time.time()
        # Keep only recent timestamps (within 1 second window)
        self._last_execution_times = [t for t in self._last_execution_times if now - t < 1.0]

        # Calculate current rate
        executions_in_window = len(self._last_execution_times)
        if executions_in_window >= self._rate_limit:
            # Need to wait
            oldest = self._last_execution_times[0]
            wait_time = 1.0 - (now - oldest) + 0.01  # Small buffer
            if wait_time > 0:
                logger.debug("ProposalExecutor: Rate limit wait %.3fs", wait_time)
                await asyncio.sleep(wait_time)
                now = time.time()

        # Record execution time
        self._last_execution_times.append(now)
        # Keep only last 10
        if len(self._last_execution_times) > 10:
            self._last_execution_times.pop(0)

    # — Risk Classification ————————————————————————————————————————————————————

    def classify_risk(self, proposal: Any) -> RiskLevel:
        """
        Classify proposal by risk level.

        Rules:
        - ESCORE/RESIDUAL/ARCHITECTURE → always REVIEW_REQUIRED
        - METRICS/QTABLE + severity < 0.2 → LOW_RISK
        - METRICS/QTABLE + severity >= 0.5 → REVIEW_REQUIRED
        - METRICS/QTABLE + 0.2 <= severity < 0.5 → REVIEW_REQUIRED (cautious)
        - Other dimensions → NOT_EXECUTABLE

        Args:
            proposal: SelfProposal instance

        Returns:
            RiskLevel enum value
        """
        dimension = proposal.dimension
        severity = proposal.severity

        # Step 1: Special dimensions always require review
        if dimension in ("ESCORE", "RESIDUAL", "ARCHITECTURE", "CONFIG", "COUPLING"):
            return RiskLevel.REVIEW_REQUIRED

        # Step 2: METRICS and QTABLE depend on severity
        if dimension in ("METRICS", "QTABLE"):
            if severity < 0.2:
                return RiskLevel.LOW_RISK
            else:
                # >= 0.2: be cautious, require review
                return RiskLevel.REVIEW_REQUIRED

        # Step 3: Unknown dimensions
        return RiskLevel.NOT_EXECUTABLE

    # — Execution ——————————————————————————————————————————————————————————————

    async def execute(self, proposal: Any) -> ExecutionResult:
        """
        Execute a proposal based on its dimension.

        Args:
            proposal: SelfProposal instance

        Returns:
            ExecutionResult with success flag and outcome message
        """
        # Check if circuit should auto-reset
        self._check_circuit_reset()

        # Check circuit breaker state
        if self._circuit_open:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension=proposal.dimension,
                message="Execution blocked",
                error_message="Circuit breaker is open (too many recent failures)",
            )

        # Apply rate limiting
        await self._apply_rate_limit()

        dimension = proposal.dimension

        # Execute based on dimension
        result: ExecutionResult
        if dimension == "QTABLE":
            result = await self._execute_qtable(proposal)
        elif dimension == "METRICS":
            result = await self._execute_metrics(proposal)
        elif dimension == "ESCORE":
            result = await self._execute_escore(proposal)
        elif dimension == "RESIDUAL":
            result = await self._execute_residual(proposal)
        elif dimension == "ARCHITECTURE" or dimension == "COUPLING":
            result = await self._execute_architecture(proposal)
        else:
            result = ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension=dimension,
                message=f"Unknown dimension: {dimension}",
            )

        # Update circuit breaker state based on result
        if self._circuit_breaker_threshold is not None:
            if result.success:
                self._consecutive_failures = 0
            else:
                self._consecutive_failures += 1
                if self._consecutive_failures >= self._circuit_breaker_threshold:
                    self._circuit_open = True
                    self._circuit_open_at = time.time()  # Record timestamp for timeout
                    logger.warning(
                        "ProposalExecutor: Circuit breaker opened after %d failures (will auto-reset after %.1f seconds)",
                        self._consecutive_failures,
                        self._circuit_reset_timeout_s,
                    )

        return result

    # — Dimension Handlers —————————————————————————————————————————————————————

    async def _execute_qtable(self, proposal: Any) -> ExecutionResult:
        """
        Execute QTABLE dimension proposal.

        Updates Q-value for the state-action pair to suggested_value.
        Format: target = "state_key:action"

        Args:
            proposal: SelfProposal with dimension="QTABLE"

        Returns:
            ExecutionResult
        """
        if self._qtable is None:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                message="QTable not injected; cannot execute QTABLE proposal",
            )

        try:
            # Parse target: "state_key:action"
            parts = proposal.target.split(":")
            if len(parts) != 2:
                return ExecutionResult(
                    success=False,
                    proposal_id=proposal.probe_id,
                    dimension="QTABLE",
                    message=f"Invalid target format: {proposal.target} (expected 'state:action')",
                )

            state_key, action = parts
            old_value = proposal.current_value
            new_value = proposal.suggested_value

            # Try to use update() method if available
            if hasattr(self._qtable, "update") and callable(self._qtable.update):
                try:
                    self._qtable.update(state_key, action, new_value)
                    logger.info(
                        "ProposalExecutor: Updated QTable[%s][%s] = %.4f",
                        state_key,
                        action,
                        new_value,
                    )
                    return ExecutionResult(
                        success=True,
                        proposal_id=proposal.probe_id,
                        dimension="QTABLE",
                        message=f"Updated QTable[{state_key}][{action}] from {old_value:.4f} to {new_value:.4f}",
                        old_value=old_value,
                        new_value=new_value,
                    )
                except Exception as e:
                    logger.warning("ProposalExecutor: QTable.update() failed: %s", e)
                    # Fall through to direct access attempt

            # Fall back to direct table access
            if hasattr(self._qtable, "_table"):
                table = self._qtable._table
                if state_key in table and action in table[state_key]:
                    table[state_key][action]["value"] = new_value
                    logger.info(
                        "ProposalExecutor: Updated QTable[%s][%s] = %.4f (direct access)",
                        state_key,
                        action,
                        new_value,
                    )
                    return ExecutionResult(
                        success=True,
                        proposal_id=proposal.probe_id,
                        dimension="QTABLE",
                        message=f"Updated QTable[{state_key}][{action}] from {old_value:.4f} to {new_value:.4f}",
                        old_value=old_value,
                        new_value=new_value,
                    )

            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                message=f"QTable entry not found: {state_key}:{action}",
            )

        except Exception as e:
            logger.warning("ProposalExecutor._execute_qtable error: %s", e)
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                message=f"Execution error: {str(e)}",
                error_message=str(e),
            )

    async def _execute_metrics(self, proposal: Any) -> ExecutionResult:
        """
        Execute METRICS dimension proposal.

        For metrics proposals, actual parameter changes are deferred (complex).
        This logs the proposal as a record for manual implementation.

        Args:
            proposal: SelfProposal with dimension="METRICS"

        Returns:
            ExecutionResult
        """
        try:
            old_value = proposal.current_value
            new_value = proposal.suggested_value
            message = (
                f"METRICS proposal recorded: {proposal.target} "
                f"({proposal.pattern_type}): {proposal.recommendation} "
                f"(current={old_value:.2f}, "
                f"suggested={new_value:.2f})"
            )
            logger.info("ProposalExecutor: %s", message)
            return ExecutionResult(
                success=True,
                proposal_id=proposal.probe_id,
                dimension="METRICS",
                message=message,
                old_value=old_value,
                new_value=new_value,
            )

        except Exception as e:
            logger.warning("ProposalExecutor._execute_metrics error: %s", e)
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="METRICS",
                message=f"Execution error: {str(e)}",
                error_message=str(e),
            )

    async def _execute_escore(self, proposal: Any) -> ExecutionResult:
        """
        Execute ESCORE dimension proposal.

        ESCORE proposals require manual review and cannot be auto-executed.

        Args:
            proposal: SelfProposal with dimension="ESCORE"

        Returns:
            ExecutionResult (always fails, requires manual review)
        """
        old_value = proposal.current_value
        new_value = proposal.suggested_value
        message = (
            f"ESCORE proposal requires manual review: {proposal.target} "
            f"(current={old_value:.1f}, "
            f"suggested={new_value:.1f})"
        )
        logger.info("ProposalExecutor: %s", message)
        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension="ESCORE",
            message=message,
            old_value=old_value,
            new_value=new_value,
        )

    async def _execute_residual(self, proposal: Any) -> ExecutionResult:
        """
        Execute RESIDUAL dimension proposal.

        RESIDUAL proposals require manual review and cannot be auto-executed.

        Args:
            proposal: SelfProposal with dimension="RESIDUAL"

        Returns:
            ExecutionResult (always fails, requires manual review)
        """
        message = (
            f"RESIDUAL proposal requires manual review: {proposal.target} "
            f"({proposal.recommendation})"
        )
        logger.info("ProposalExecutor: %s", message)
        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension="RESIDUAL",
            message=message,
        )

    async def _execute_architecture(self, proposal: Any) -> ExecutionResult:
        """
        Execute ARCHITECTURE dimension proposal.

        ARCHITECTURE proposals require manual review and cannot be auto-executed.

        Args:
            proposal: SelfProposal with dimension="ARCHITECTURE"

        Returns:
            ExecutionResult (always fails, requires manual review)
        """
        message = (
            f"ARCHITECTURE proposal requires manual review: {proposal.target} "
            f"({proposal.recommendation})"
        )
        logger.info("ProposalExecutor: %s", message)
        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension="ARCHITECTURE" if proposal.dimension == "ARCHITECTURE" else "COUPLING",
            message=message,
        )
