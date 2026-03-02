"""GASdf REST API client using httpx and VascularSystem."""

from __future__ import annotations

from typing import Any, Optional
import httpx

from .types import GASdfError, GASdfQuote, GASdfStats, HealthResponse, TokenInfo, SubmitResponse
from cynic.kernel.core.vascular import VascularSystem


class GASdfClient:
    """Async HTTP client for GASdf REST API using VascularSystem connection pool."""

    def __init__(self, base_url: str = "https://www.asdfasdfa.tech", vascular: Optional[VascularSystem] = None, timeout: int = 30):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.vascular = vascular

    async def _get_client(self) -> httpx.AsyncClient:
        if self.vascular:
            return await self.vascular.get_client()
        return httpx.AsyncClient(timeout=float(self.timeout))

    async def health(self) -> dict[str, Any]:
        client = await self._get_client()
        response = await client.get(f"{self.base_url}/health")
        if response.status_code != 200:
            raise GASdfError(f"Health check failed: {response.status_code} {response.text}")
        try:
            validated = HealthResponse.model_validate(response.json())
            return validated.model_dump()
        except Exception as e:
            raise GASdfError(f"Invalid health response: {e}")

    async def get_tokens(self) -> list[dict[str, Any]]:
        client = await self._get_client()
        response = await client.get(f"{self.base_url}/v1/tokens")
        if response.status_code != 200:
            raise GASdfError(f"Get tokens failed: {response.status_code} {response.text}")
        try:
            data = response.json()
            tokens = data.get("tokens", []) if isinstance(data, dict) else data
            validated = [TokenInfo.model_validate(t) for t in tokens]
            return [t.model_dump() for t in validated]
        except Exception as e:
            raise GASdfError(f"Invalid tokens response: {e}")

    async def get_quote(self, payment_token: str, user_pubkey: str, amount: int) -> GASdfQuote:
        client = await self._get_client()
        payload = {
            "paymentToken": payment_token,
            "userPubkey": user_pubkey,
            "amount": amount,
        }
        response = await client.post(f"{self.base_url}/v1/quote", json=payload)
        if response.status_code != 200:
            raise GASdfError(f"Quote request failed: {response.status_code} {response.text}")
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
        client = await self._get_client()
        payload = {
            "quote_id": quote_id,
            "signed_transaction": signed_transaction,
            "payment_token_account": payment_token_account,
        }
        response = await client.post(f"{self.base_url}/v1/submit", json=payload)
        if response.status_code != 200:
            raise GASdfError(f"Submit failed: {response.status_code} {response.text}")
        try:
            validated = SubmitResponse.model_validate(response.json())
            return validated.model_dump()
        except Exception as e:
            raise GASdfError(f"Invalid submit response: {e}")

    async def get_stats(self) -> GASdfStats:
        client = await self._get_client()
        response = await client.get(f"{self.base_url}/v1/stats")
        if response.status_code != 200:
            raise GASdfError(f"Stats request failed: {response.status_code} {response.text}")
        data = response.json()
        return GASdfStats(
            total_burned=data["totalBurned"],
            total_transactions=data["totalTransactions"],
            burned_formatted=data["burnedFormatted"],
            treasury=cast(dict[str, object], data.get("treasury", {})),
        )
