"""NEAR key management"""

import base64

from nacl.signing import SigningKey


class KeyManager:
    """Manage NEAR ed25519 keys"""

    def __init__(self, private_key_b64: str):
        """
        Initialize with base64-encoded private key

        Args:
            private_key_b64: Base64-encoded 32-byte ed25519 seed
        """
        try:
            private_seed = base64.b64decode(private_key_b64)
            if len(private_seed) != 32:
                raise ValueError(f"Private key must be 32 bytes, got {len(private_seed)}")

            self.signing_key = SigningKey(private_seed)
            self.public_key = bytes(self.signing_key.verify_key)
        except Exception as e:
            raise ValueError(f"Failed to load NEAR private key: {e}")

    def sign(self, message: bytes) -> bytes:
        """Sign a message"""
        return bytes(self.signing_key.sign(message).signature)

    def public_key_b58(self) -> str:
        """Get public key in NEAR's ed25519: format"""
        import base58
        return f"ed25519:{base58.b58encode(self.public_key).decode()}"
