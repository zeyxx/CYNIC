"""NEAR Protocol RPC client for blockchain interaction."""
from __future__ import annotations

from typing import Any, cast

import httpx

from .types import NEARError, NEARNetworkConfig


class NEARRPCClient:
    """Async HTTP client for NEAR Protocol RPC.

    Provides methods for querying blockchain state and submitting transactions.

    Attributes:
        config: NEAR network configuration
        timeout: Request timeout in seconds (default: 30)
    """

    def __init__(self, config: NEARNetworkConfig, timeout: int = 30):
        """Initialize NEAR RPC client.

        Args:
            config: Network configuration (mainnet/testnet)
            timeout: Request timeout in seconds
        """
        self.config = config
        self.timeout = timeout

    async def health(self) -> dict[str, Any]:
        """Check NEAR node health.

        Returns:
            Health check response dictionary

        Raises:
            NEARError: If the request fails
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            try:
                response = await client.post(
                    self.config.rpc_url,
                    json={
                        "jsonrpc": "2.0",
                        "id": "health",
                        "method": "status",
                        "params": [],
                    },
                )
                if response.status_code != 200:
                    raise NEARError(
                        f"Health check failed: {response.status_code}"
                    )
                return cast(dict[str, Any], response.json())
            except httpx.RequestError as e:
                raise NEARError(f"RPC connection failed: {e}")

    async def get_account(self, account_id: str) -> dict[str, Any]:
        """Get account information from NEAR.

        Args:
            account_id: NEAR account to query

        Returns:
            Account state dictionary

        Raises:
            NEARError: If the request fails
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.post(
                self.config.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": f"account_{account_id}",
                    "method": "query",
                    "params": {
                        "request_type": "view_account",
                        "account_id": account_id,
                        "finality": "optimistic",
                    },
                },
            )
            if response.status_code != 200:
                raise NEARError(
                    f"Query account failed: {response.status_code}"
                )
            data = response.json()
            if "error" in data:
                raise NEARError(f"RPC error: {data['error']}")
            return cast(dict[str, Any], data.get("result", {}))

    async def get_nonce(self, account_id: str) -> int:
        """Get next nonce for account.

        Args:
            account_id: NEAR account

        Returns:
            Next available nonce

        Raises:
            NEARError: If the request fails
        """
        account = await self.get_account(account_id)
        return int(account.get("nonce", 0)) + 1

    async def get_block_hash(self) -> str:
        """Get latest block hash for transaction signing.

        Returns:
            Block hash as base64 string

        Raises:
            NEARError: If the request fails
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.post(
                self.config.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": "block_hash",
                    "method": "block",
                    "params": {"finality": "optimistic"},
                },
            )
            if response.status_code != 200:
                raise NEARError(
                    f"Get block hash failed: {response.status_code}"
                )
            data = response.json()
            if "error" in data:
                raise NEARError(f"RPC error: {data['error']}")
            return cast(str, data["result"]["header"]["hash"])

    async def call_contract(
        self, contract_id: str, method: str, args: dict
    ) -> dict[str, Any]:
        """Call a read-only contract method.

        Args:
            contract_id: Contract account ID
            method: Method name
            args: Method arguments (as dict)

        Returns:
            Contract call result

        Raises:
            NEARError: If the request fails
        """
        import base64
        import json

        args_json = json.dumps(args)
        args_base64 = base64.b64encode(args_json.encode()).decode()

        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.post(
                self.config.rpc_url,
                json={
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
                },
            )
            if response.status_code != 200:
                raise NEARError(
                    f"Contract call failed: {response.status_code}"
                )
            data = response.json()
            if "error" in data:
                raise NEARError(f"RPC error: {data['error']}")
            return cast(dict[str, Any], data.get("result", {}))

    async def send_transaction(self, signed_tx: str) -> dict[str, Any]:
        """Send a signed transaction to NEAR.

        Args:
            signed_tx: Signed transaction (base64 encoded)

        Returns:
            Transaction submission result

        Raises:
            NEARError: If submission fails
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.post(
                self.config.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": "send_tx",
                    "method": "broadcast_tx_async",
                    "params": [signed_tx],
                },
            )
            if response.status_code != 200:
                raise NEARError(
                    f"Send transaction failed: {response.status_code}"
                )
            data = response.json()
            if "error" in data:
                raise NEARError(f"RPC error: {data['error']}")
            return cast(dict[str, Any], data.get("result", {}))

    async def get_transaction_result(
        self, tx_hash: str, account_id: str
    ) -> dict[str, Any]:
        """Get transaction result after execution.

        Args:
            tx_hash: Transaction hash
            account_id: Account that signed transaction

        Returns:
            Transaction result (outcome and receipts)

        Raises:
            NEARError: If query fails
        """
        async with httpx.AsyncClient(
            timeout=self.timeout, follow_redirects=True
        ) as client:
            response = await client.post(
                self.config.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": f"tx_result_{tx_hash}",
                    "method": "tx",
                    "params": [tx_hash, account_id],
                },
            )
            if response.status_code != 200:
                raise NEARError(
                    f"Query transaction failed: {response.status_code}"
                )
            data = response.json()
            if "error" in data:
                raise NEARError(f"RPC error: {data['error']}")
            return cast(dict[str, Any], data.get("result", {}))
