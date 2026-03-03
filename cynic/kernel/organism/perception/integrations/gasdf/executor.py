"""GASdf executor for CYNIC governance verdicts."""

from __future__ import annotations

import logging
from typing import Any

from .client import GASdfClient
from .types import GASdfError, GASdfExecutionResult

logger = logging.getLogger(__name__)

# Verdict execution mapping: which verdicts trigger execution and how
VERDICT_EXECUTION_MAP = {
    "HOWL": {"execute": True, "priority": "high"},  # Strong yes
    "WAG": {"execute": True, "priority": "medium"},  # Yes
    "GROWL": {"execute": False, "priority": "low"},  # Caution
    "BARK": {"execute": False, "priority": "none"},  # Reject
}


class GASdfExecutor:
    """Executes governance verdicts via GASdf gasless transactions.

    The executor bridges CYNIC governance decisions to on-chain execution
    through the GASdf API, allowing verdicts to be reflected in community
    treasury management with non-extractive fee handling.

    Integrates with CYNIC's learning loop to provide fee burn statistics
    as reward signals for verdict quality evaluation.

    Attributes:
        client: GASdfClient instance for API communication
    """

    def __init__(self, client: GASdfClient) -> None:
        """Initialize GASdfExecutor.

        Args:
            client: GASdfClient instance for API calls
        """
        self.client = client

    async def should_execute_verdict(self, verdict: str, q_score: float = 0.5) -> bool:
        """Determine if a verdict should be executed on-chain.

        Execution is based on verdict type and confidence (Q-Score).
        High-confidence verdicts are more likely to execute.

        Args:
            verdict: CYNIC verdict (HOWL/WAG/GROWL/BARK)
            q_score: CYNIC confidence score (0-1, -bounded at 0.618)

        Returns:
            True if verdict should execute, False otherwise
        """
        config = VERDICT_EXECUTION_MAP.get(verdict, {"execute": False})
        should_execute = config.get("execute", False)

        # Only execute approving verdicts with confidence > 50%
        if should_execute and q_score < 0.5:
            logger.info(
                "Verdict %s has low confidence (%.2f), skipping execution",
                verdict,
                q_score,
            )
            return False

        return should_execute

    async def execute_verdict(
        self,
        proposal_id: str,
        verdict: str,
        community_id: str,
        payment_token: str,
        user_pubkey: str,
        signed_transaction: str,
        payment_token_account: str,
        q_score: float = 0.5,
        proposal_context: dict[str, Any] | None = None,
    ) -> GASdfExecutionResult | None:
        """Execute a governance verdict on-chain via GASdf.

        Evaluates verdict type and confidence before execution. Only high-
        confidence HOWL and WAG verdicts trigger on-chain execution. Lower-
        confidence verdicts or GROWL/BARK verdicts are logged but not executed.

        Args:
            proposal_id: Unique proposal identifier
            verdict: CYNIC verdict (HOWL/WAG/GROWL/BARK)
            community_id: Community identifier
            payment_token: Token address for fee payment
            user_pubkey: User's public key
            signed_transaction: Base64-encoded signed transaction
            payment_token_account: Token account for fee deduction
            q_score: CYNIC confidence score (0-1, default 0.5)
            proposal_context: Additional proposal metadata for logging

        Returns:
            GASdfExecutionResult if execution occurred, None if verdict skipped

        Raises:
            GASdfError: If API calls fail during execution
        """
        context = proposal_context or {}

        # Check if verdict should execute
        if not await self.should_execute_verdict(verdict, q_score):
            logger.info(
                "Skipping execution: proposal=%s verdict=%s q_score=%.2f",
                proposal_id,
                verdict,
                q_score,
            )
            return None

        try:
            logger.info(
                "Executing verdict: proposal=%s verdict=%s community=%s q_score=%.2f",
                proposal_id,
                verdict,
                community_id,
                q_score,
            )

            # Request quote for the transaction
            quote = await self.client.get_quote(
                payment_token=payment_token,
                user_pubkey=user_pubkey,
                amount=context.get("amount", 1000000),
            )

            logger.debug(
                "Fee quote obtained: quote_id=%s fee=%s token=%s",
                quote.quote_id,
                quote.fee_amount,
                quote.payment_token,
            )

            # Submit the signed transaction with the quote
            response = await self.client.submit(
                quote_id=quote.quote_id,
                signed_transaction=signed_transaction,
                payment_token_account=payment_token_account,
            )

            # Create result object
            result = GASdfExecutionResult(
                signature=response.get("signature", ""),
                status=response.get("status", "unknown"),
                fee_amount=quote.fee_amount,
                fee_token=quote.payment_token,
                quote_id=quote.quote_id,
            )

            # Calculate burn amount (76.4% of fee)
            burn_amount = int(quote.fee_amount * 0.764)

            logger.info(
                "Verdict executed: proposal=%s signature=%s " "fee=%s burn=%s status=%s",
                proposal_id,
                result.signature,
                quote.fee_amount,
                burn_amount,
                result.status,
            )

            return result

        except GASdfError as e:
            logger.error(
                "Execution failed: proposal=%s verdict=%s error=%s",
                proposal_id,
                verdict,
                str(e),
            )
            raise GASdfError(f"Execution failed for proposal {proposal_id}: {str(e)}") from e

    async def get_execution_reward(self) -> dict[str, Any]:
        """Get cumulative stats for CYNIC learning feedback.

        Queries GASdf to obtain fee burn statistics, which serve as reward
        signals for the Q-Learning algorithm. Higher burn = better governance.

        Returns:
            Dictionary with reward metrics:
            - total_burned: Cumulative tokens burned
            - total_transactions: Cumulative execution count
            - average_fee: Average fee per transaction
            - treasury_health: Qualitative assessment

        Raises:
            GASdfError: If stats query fails
        """
        try:
            stats = await self.client.get_stats()

            # Calculate reward signal: burn per transaction
            avg_reward = (
                stats.total_burned / stats.total_transactions if stats.total_transactions > 0 else 0
            )

            # Assess treasury health based on burn rate
            if stats.total_burned > 10_000_000:  # >10M burned
                treasury_health = "excellent"
            elif stats.total_burned > 1_000_000:  # >1M burned
                treasury_health = "good"
            elif stats.total_burned > 100_000:  # >100K burned
                treasury_health = "fair"
            else:
                treasury_health = "poor"

            reward_data = {
                "total_burned": stats.total_burned,
                "total_transactions": stats.total_transactions,
                "average_fee": avg_reward,
                "treasury_health": treasury_health,
            }

            logger.debug(
                "Execution reward computed: health=%s avg_fee=%s",
                treasury_health,
                avg_reward,
            )

            return reward_data

        except GASdfError as e:
            logger.warning("Failed to compute execution reward: %s", str(e))
            return {
                "total_burned": 0,
                "total_transactions": 0,
                "average_fee": 0,
                "treasury_health": "unknown",
            }
