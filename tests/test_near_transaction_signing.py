"""Tests for NEAR transaction signing"""

import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - governance_bot module not found")

import base64
import os

from governance_bot.near_integration import NonceManager, TransactionSigner
from governance_bot.near_keys import KeyManager


class TestKeyManagement:
    """Test NEAR key management"""

    def test_load_private_key_from_base64(self):
        """Test loading ed25519 private key from base64"""
        # Generate test key (32 bytes for ed25519 seed)
        test_seed = os.urandom(32)
        test_key_b64 = base64.b64encode(test_seed).decode()

        key_manager = KeyManager(private_key_b64=test_key_b64)

        # Should successfully load and expose public key
        assert key_manager.public_key is not None
        assert len(key_manager.public_key) == 32  # ed25519 public key is 32 bytes

    def test_sign_transaction(self):
        """Test signing a transaction"""
        test_seed = os.urandom(32)
        test_key_b64 = base64.b64encode(test_seed).decode()

        signer = TransactionSigner(private_key_b64=test_key_b64, account_id="test.testnet")

        # Sign a test message
        test_message = b"test transaction"
        signature = signer.sign(test_message)

        # Signature should be 64 bytes for ed25519
        assert len(signature) == 64

    def test_nonce_management(self):
        """Test nonce tracking for accounts"""
        nonce_mgr = NonceManager()

        # Get and increment nonce
        nonce_1 = nonce_mgr.get_next_nonce("test.testnet")
        nonce_2 = nonce_mgr.get_next_nonce("test.testnet")

        assert nonce_2 > nonce_1
