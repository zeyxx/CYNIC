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

import logging
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
    dimension: str
    message: str
    new_value: Optional[float] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "success": self.success,
            "dimension": self.dimension,
            "message": self.message,
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

    def execute(self, proposal: Any) -> ExecutionResult:
        """
        Execute a proposal based on its dimension.

        Args:
            proposal: SelfProposal instance

        Returns:
            ExecutionResult with success flag and outcome message
        """
        dimension = proposal.dimension

        if dimension == "QTABLE":
            return self._execute_qtable(proposal)
        elif dimension == "METRICS":
            return self._execute_metrics(proposal)
        elif dimension == "ESCORE":
            return self._execute_escore(proposal)
        elif dimension == "RESIDUAL":
            return self._execute_residual(proposal)
        elif dimension == "ARCHITECTURE" or dimension == "COUPLING":
            return self._execute_architecture(proposal)
        else:
            return ExecutionResult(
                success=False,
                dimension=dimension,
                message=f"Unknown dimension: {dimension}",
            )

    # — Dimension Handlers —————————————————————————————————————————————————————

    def _execute_qtable(self, proposal: Any) -> ExecutionResult:
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
                dimension="QTABLE",
                message="QTable not injected; cannot execute QTABLE proposal",
            )

        try:
            # Parse target: "state_key:action"
            parts = proposal.target.split(":")
            if len(parts) != 2:
                return ExecutionResult(
                    success=False,
                    dimension="QTABLE",
                    message=f"Invalid target format: {proposal.target} (expected 'state:action')",
                )

            state_key, action = parts
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
                        dimension="QTABLE",
                        message=f"Updated QTable[{state_key}][{action}] from {proposal.current_value:.4f} to {new_value:.4f}",
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
                        dimension="QTABLE",
                        message=f"Updated QTable[{state_key}][{action}] from {proposal.current_value:.4f} to {new_value:.4f}",
                        new_value=new_value,
                    )

            return ExecutionResult(
                success=False,
                dimension="QTABLE",
                message=f"QTable entry not found: {state_key}:{action}",
            )

        except Exception as e:
            logger.warning("ProposalExecutor._execute_qtable error: %s", e)
            return ExecutionResult(
                success=False,
                dimension="QTABLE",
                message=f"Execution error: {str(e)}",
            )

    def _execute_metrics(self, proposal: Any) -> ExecutionResult:
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
            message = (
                f"METRICS proposal recorded: {proposal.target} "
                f"({proposal.pattern_type}): {proposal.recommendation} "
                f"(current={proposal.current_value:.2f}, "
                f"suggested={proposal.suggested_value:.2f})"
            )
            logger.info("ProposalExecutor: %s", message)
            return ExecutionResult(
                success=True,
                dimension="METRICS",
                message=message,
                new_value=proposal.suggested_value,
            )

        except Exception as e:
            logger.warning("ProposalExecutor._execute_metrics error: %s", e)
            return ExecutionResult(
                success=False,
                dimension="METRICS",
                message=f"Execution error: {str(e)}",
            )

    def _execute_escore(self, proposal: Any) -> ExecutionResult:
        """
        Execute ESCORE dimension proposal.

        ESCORE proposals require manual review and cannot be auto-executed.

        Args:
            proposal: SelfProposal with dimension="ESCORE"

        Returns:
            ExecutionResult (always fails, requires manual review)
        """
        message = (
            f"ESCORE proposal requires manual review: {proposal.target} "
            f"(current={proposal.current_value:.1f}, "
            f"suggested={proposal.suggested_value:.1f})"
        )
        logger.info("ProposalExecutor: %s", message)
        return ExecutionResult(
            success=False,
            dimension="ESCORE",
            message=message,
        )

    def _execute_residual(self, proposal: Any) -> ExecutionResult:
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
            dimension="RESIDUAL",
            message=message,
        )

    def _execute_architecture(self, proposal: Any) -> ExecutionResult:
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
            dimension="ARCHITECTURE" if proposal.dimension == "ARCHITECTURE" else "COUPLING",
            message=message,
        )
