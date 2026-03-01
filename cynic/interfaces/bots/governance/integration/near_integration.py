"""NEAR transaction integration"""

from typing import Any

from governance_bot.near_keys import KeyManager


class TransactionSigner:
    """Sign NEAR transactions"""

    def __init__(self, private_key_b64: str, account_id: str):
        self.key_manager = KeyManager(private_key_b64)
        self.account_id = account_id

    def sign(self, message: bytes) -> bytes:
        """Sign a message"""
        return self.key_manager.sign(message)

    def create_transaction(
        self,
        receiver_id: str,
        actions: list,
        nonce: int,
        block_hash: str
    ) -> dict[str, Any]:
        """Create a signed transaction"""
        # Transaction structure for NEAR
        transaction = {
            "signer_id": self.account_id,
            "public_key": self.key_manager.public_key_b58(),
            "nonce": nonce,
            "receiver_id": receiver_id,
            "block_hash": block_hash,
            "actions": actions
        }

        return transaction


class NonceManager:
    """Track account nonces for transactions"""

    def __init__(self):
        self.nonces: dict[str, int] = {}

    def get_next_nonce(self, account_id: str) -> int:
        """Get next nonce for account"""
        if account_id not in self.nonces:
            self.nonces[account_id] = 1
        else:
            self.nonces[account_id] += 1

        return self.nonces[account_id]

    def set_nonce(self, account_id: str, nonce: int):
        """Set nonce for account"""
        self.nonces[account_id] = nonce


class NearRpcClient:
    """NEAR RPC client for blockchain interaction"""

    def __init__(self, rpc_url: str):
        self.rpc_url = rpc_url
        self.session = None

    async def __aenter__(self):
        import aiohttp
        self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()

    async def _call_rpc(self, method: str, params: dict[str, Any]) -> dict:
        """Call NEAR RPC method"""
        import aiohttp
        if not self.session:
            self.session = aiohttp.ClientSession()

        payload = {
            "jsonrpc": "2.0",
            "id": "1",
            "method": method,
            "params": params
        }

        async with self.session.post(self.rpc_url, json=payload) as resp:
            return await resp.json()

    async def get_account_nonce(self, account_id: str) -> int:
        """Get current nonce for account"""
        result = await self._call_rpc("query", {
            "request_type": "view_account",
            "account_id": account_id,
            "finality": "final"
        })

        return result["result"]["nonce"]

    async def send_transaction(self, transaction: dict[str, Any]) -> str:
        """Send transaction and return hash"""
        result = await self._call_rpc("broadcast_tx_commit", {
            "signed_tx": transaction
        })

        if "result" in result:
            return result["result"]["hash"]
        else:
            raise RuntimeError(f"Transaction failed: {result}")

    async def poll_transaction_status(
        self,
        tx_hash: str,
        max_polls: int = 10,
        poll_interval: float = 1.0
    ) -> dict[str, Any]:
        """Poll for transaction confirmation"""
        import asyncio
        for attempt in range(max_polls):
            result = await self._call_rpc("tx", {
                "hash": tx_hash,
                "wait_until": "FINAL"
            })

            if result.get("result"):
                return result["result"]

            if attempt < max_polls - 1:
                await asyncio.sleep(poll_interval)

        raise TimeoutError(f"Transaction {tx_hash} did not confirm within {max_polls * poll_interval}s")

    async def get_block_hash(self) -> str:
        """Get latest block hash for transactions"""
        result = await self._call_rpc("block", {
            "finality": "final"
        })

        return result["result"]["header"]["hash"]
