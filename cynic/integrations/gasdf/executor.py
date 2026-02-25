"""GASdf executor for CYNIC governance verdicts."""
from __future__ import annotations

from .client import GASdfClient
from .types import GASdfError, GASdfExecutionResult


class GASdfExecutor:
    """Executes governance verdicts via GASdf gasless transactions.

    The executor bridges CYNIC governance decisions to on-chain execution
    through the GASdf API, allowing verdicts to be reflected in community
    treasury management with non-extractive fee handling.

    Attributes:
        client: GASdfClient instance for API communication
    """

    def __init__(self, client: GASdfClient) -> None:
        """Initialize GASdfExecutor.

        Args:
            client: GASdfClient instance for API calls
        """
        self.client = client

    async def execute_verdict(
        self,
        proposal_id: str,
        verdict: str,
        community_id: str,
        payment_token: str,
        user_pubkey: str,
        signed_transaction: str,
        payment_token_account: str,
    ) -> GASdfExecutionResult | None:
        """Execute a governance verdict on-chain via GASdf.

        Only executes for APPROVED and TENTATIVE_APPROVE verdicts.
        Other verdicts (CAUTION, REJECT) do not trigger execution.

        Args:
            proposal_id: Unique proposal identifier
            verdict: Verdict string (APPROVED, TENTATIVE_APPROVE, CAUTION, REJECT)
            community_id: Community identifier
            payment_token: Token address for fee payment
            user_pubkey: User's public key
            signed_transaction: Base64-encoded signed transaction
            payment_token_account: Token account for fee deduction

        Returns:
            GASdfExecutionResult if execution occurred, None if verdict skipped

        Raises:
            GASdfError: If API calls fail
        """
        # Only execute for approving verdicts
        if verdict not in ("APPROVED", "TENTATIVE_APPROVE"):
            return None

        try:
            # Request quote for the transaction
            quote = await self.client.get_quote(
                payment_token=payment_token,
                user_pubkey=user_pubkey,
                amount=1000000,  # Placeholder amount (actual determined by proposal)
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

            return result

        except GASdfError as e:
            raise GASdfError(
                f"Execution failed for proposal {proposal_id}: {str(e)}"
            ) from e
