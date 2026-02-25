"""GASdf REST API client using httpx."""
from __future__ import annotations

from typing import Any, cast

import httpx

from .types import GASdfError, GASdfQuote, GASdfStats


class GASdfClient:
    """Async HTTP client for GASdf REST API.

    The client manages connections to https://www.asdfasdfa.tech and provides
    methods for health checks, token queries, fee quoting, and transaction submission.

    Attributes:
        base_url: GASdf API base URL (default: https://www.asdfasdfa.tech)
        timeout: Request timeout in seconds (default: 30)
    """

    base_url: str = "https://www.asdfasdfa.tech"
    timeout: int = 30

    async def health(self) -> dict[str, Any]:
        """Check GASdf service health.

        Returns:
            Health check response dictionary

        Raises:
            GASdfError: If the request fails or returns non-200 status
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.get(f"{self.base_url}/health")
            if response.status_code != 200:
                raise GASdfError(
                    f"Health check failed: {response.status_code} {response.text}"
                )
            return cast(dict[str, Any], response.json())

    async def get_tokens(self) -> list[dict[str, Any]]:
        """Get list of accepted payment tokens.

        Returns:
            List of token information dictionaries, each containing token details

        Raises:
            GASdfError: If the request fails or returns non-200 status
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.get(f"{self.base_url}/v1/tokens")
            if response.status_code != 200:
                raise GASdfError(
                    f"Get tokens failed: {response.status_code} {response.text}"
                )
            data = response.json()
            # Extract tokens array from response
            tokens = data.get("tokens", []) if isinstance(data, dict) else data
            return cast(list[dict[str, Any]], tokens)

    async def get_quote(
        self, payment_token: str, user_pubkey: str, amount: int
    ) -> GASdfQuote:
        """Get a fee quote for a transaction.

        Args:
            payment_token: Token address for payment
            user_pubkey: User's public key
            amount: Transaction amount in lamports

        Returns:
            GASdfQuote with fee details and quote ID

        Raises:
            GASdfError: If the request fails or returns non-200 status
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            payload = {
                "paymentToken": payment_token,
                "userPubkey": user_pubkey,
                "amount": amount,
            }
            response = await client.post(
                f"{self.base_url}/v1/quote", json=payload
            )
            if response.status_code != 200:
                raise GASdfError(
                    f"Quote request failed: {response.status_code} {response.text}"
                )
            data = response.json()
            return GASdfQuote(
                quote_id=data["quote_id"],
                payment_token=data["payment_token"],
                fee_amount=data["fee_amount"],
                burn_amount=data["burn_amount"],
                user_pubkey=data["user_pubkey"],
            )

    async def submit(
        self,
        quote_id: str,
        signed_transaction: str,
        payment_token_account: str,
    ) -> dict[str, Any]:
        """Submit a signed transaction for execution.

        Args:
            quote_id: Quote ID from get_quote()
            signed_transaction: Base64-encoded signed transaction
            payment_token_account: Token account for fee payment

        Returns:
            Response dictionary with signature and status

        Raises:
            GASdfError: If the request fails or returns non-200 status
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            payload = {
                "quote_id": quote_id,
                "signed_transaction": signed_transaction,
                "payment_token_account": payment_token_account,
            }
            response = await client.post(
                f"{self.base_url}/v1/submit", json=payload
            )
            if response.status_code != 200:
                raise GASdfError(
                    f"Submit failed: {response.status_code} {response.text}"
                )
            return cast(dict[str, Any], response.json())

    async def get_stats(self) -> GASdfStats:
        """Get GASdf burn statistics.

        Returns:
            GASdfStats with cumulative burn and fee information

        Raises:
            GASdfError: If the request fails or returns non-200 status
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.get(f"{self.base_url}/v1/stats")
            if response.status_code != 200:
                raise GASdfError(
                    f"Stats request failed: {response.status_code} {response.text}"
                )
            data = response.json()
            return GASdfStats(
                total_burned=data["totalBurned"],
                total_transactions=data["totalTransactions"],
                burned_formatted=data["burnedFormatted"],
                treasury=cast(dict[str, object], data.get("treasury", {})),
            )
