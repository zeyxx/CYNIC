"""NEAR Protocol RPC client for blockchain interaction."""

from __future__ import annotations

from typing import Any, cast, Optional
import httpx

from .types import NEARError, NEARNetworkConfig
from cynic.kernel.core.vascular import VascularSystem


class NEARRPCClient:
    """Async HTTP client for NEAR Protocol RPC using VascularSystem pooling."""

    def __init__(self, config: NEARNetworkConfig, vascular: Optional[VascularSystem] = None, timeout: int = 30):
        self.config = config
        self.timeout = timeout
        self.vascular = vascular

    async def _get_client(self) -> httpx.AsyncClient:
        if self.vascular:
            return await self.vascular.get_client()
        return httpx.AsyncClient(timeout=float(self.timeout))

    async def _post(self, payload: dict) -> dict[str, Any]:
        """Centralized POST for NEAR RPC."""
        try:
            client = await self._get_client()
            response = await client.post(
                self.config.rpc_url,
                json=payload,
                timeout=float(self.timeout)
            )
            if response.status_code != 200:
                raise NEARError(f"RPC request failed: {response.status_code}")
            return cast(dict[str, Any], response.json())
        except httpx.RequestError as e:
            raise NEARError(f"RPC connection failed: {e}")

    async def health(self) -> dict[str, Any]:
        return await self._post({
            "jsonrpc": "2.0",
            "id": "health",
            "method": "status",
            "params": [],
        })

    async def get_account(self, account_id: str) -> dict[str, Any]:
        data = await self._post({
            "jsonrpc": "2.0",
            "id": f"account_{account_id}",
            "method": "query",
            "params": {
                "request_type": "view_account",
                "account_id": account_id,
                "finality": "optimistic",
            },
        })
        if "error" in data:
            raise NEARError(f"RPC error: {data['error']}")
        return cast(dict[str, Any], data.get("result", {}))

    async def get_nonce(self, account_id: str) -> int:
        account = await self.get_account(account_id)
        return int(account.get("nonce", 0)) + 1

    async def get_block_hash(self) -> str:
        data = await self._post({
            "jsonrpc": "2.0",
            "id": "block_hash",
            "method": "block",
            "params": {"finality": "optimistic"},
        })
        if "error" in data:
            raise NEARError(f"RPC error: {data['error']}")
        return cast(str, data["result"]["header"]["hash"])

    async def call_contract(self, contract_id: str, method: str, args: dict) -> dict[str, Any]:
        import base64
        import json
        args_json = json.dumps(args)
        args_base64 = base64.b64encode(args_json.encode()).decode()

        data = await self._post({
            "jsonrpc": "2.0",
            "id": f"call_{method}",
            "method": "query",
            "params": {
                "request_type": "call_function",
                "account_id": contract_id,
                "method_name": method,
                "args_base64": args_base64,
                "finality": "optimistic",
            },
        })
        if "error" in data:
            raise NEARError(f"RPC error: {data['error']}")
        return cast(dict[str, Any], data.get("result", {}))

    async def send_transaction(self, signed_tx: str) -> dict[str, Any]:
        data = await self._post({
            "jsonrpc": "2.0",
            "id": "send_tx",
            "method": "broadcast_tx_async",
            "params": [signed_tx],
        })
        if "error" in data:
            raise NEARError(f"Send transaction failed: {data['error']}")
        return cast(dict[str, Any], data.get("result", {}))

    async def get_transaction_result(self, tx_hash: str, account_id: str) -> dict[str, Any]:
        data = await self._post({
            "jsonrpc": "2.0",
            "id": f"tx_result_{tx_hash}",
            "method": "tx",
            "params": [tx_hash, account_id],
        })
        if "error" in data:
            raise NEARError(f"Query transaction failed: {data['error']}")
        return cast(dict[str, Any], data.get("result", {}))
