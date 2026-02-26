"""NEAR transaction integration"""

from governance_bot.near_keys import KeyManager
from typing import Dict, Any


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
    ) -> Dict[str, Any]:
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
        self.nonces: Dict[str, int] = {}

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
