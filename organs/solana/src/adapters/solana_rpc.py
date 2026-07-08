"""Small JSON-RPC client for live Solana evidence probes."""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from typing import Any


DEFAULT_SOLANA_RPC_URL = "https://api.mainnet-beta.solana.com"


class SolanaRpcClient:
    def __init__(self, endpoint: str | None = None, timeout: int = 20):
        self.endpoint = endpoint or _endpoint_from_env()
        self.timeout = timeout
        self._next_id = 1

    def call(self, method: str, params: list[Any]) -> dict[str, Any]:
        payload = {
            "jsonrpc": "2.0",
            "id": self._next_id,
            "method": method,
            "params": params,
        }
        self._next_id += 1
        request = urllib.request.Request(
            self.endpoint,
            data=json.dumps(payload).encode("utf-8"),
            headers={"content-type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                data = json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"Solana RPC HTTP {exc.code}: {detail}") from exc
        if "error" in data:
            raise RuntimeError(f"Solana RPC {method} error: {data['error']}")
        return data

    def get_mint_account(self, mint: str) -> dict[str, Any]:
        return self.call(
            "getAccountInfo",
            [
                mint,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed",
                },
            ],
        )

    def get_signatures(self, address: str, limit: int) -> list[dict[str, Any]]:
        result = self.call(
            "getSignaturesForAddress",
            [
                address,
                {
                    "limit": limit,
                    "commitment": "confirmed",
                },
            ],
        ).get("result", [])
        return result if isinstance(result, list) else []

    def get_transaction(self, signature: str) -> dict[str, Any]:
        return self.call(
            "getTransaction",
            [
                signature,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed",
                    "maxSupportedTransactionVersion": 0,
                },
            ],
        )


def _endpoint_from_env() -> str:
    explicit = os.getenv("SOLANA_RPC_URL")
    if explicit:
        return explicit
    helius = os.getenv("HELIUS_API_KEY")
    if helius:
        return f"https://mainnet.helius-rpc.com/?api-key={helius}"
    return DEFAULT_SOLANA_RPC_URL
