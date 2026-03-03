"""Tests for governance encryption (PHASE 1B).

Tests cover:
- Encryption/decryption of community tokens
- Encryption/decryption of treasury addresses
- Transparent encryption in API responses
- Database storage of encrypted values
- Error handling for tampering/decryption failures
- Vault integration for key management
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.interfaces.bots.governance.encryption import (
    EncryptedGovernanceData,
    EncryptedCommunityModel,
)
from cynic.kernel.security.encryption import (
    EncryptionService,
    EncryptionKeyManager,
    EncryptionConfig,
)


@pytest.mark.asyncio
class TestEncryptedGovernanceData:
    """Test encryption/decryption of governance data."""

    @pytest.fixture
    async def encryption_service(self):
        """Create test encryption service."""
        config = EncryptionConfig(
            vault_addr="http://localhost:8200",
            vault_token="test-token",
        )
        key_manager = EncryptionKeyManager(config)
        service = EncryptionService(key_manager)
        return service

    @pytest.fixture
    async def encrypted_data(self, encryption_service):
        """Create EncryptedGovernanceData instance with service."""
        EncryptedGovernanceData.set_encryption_service(encryption_service)
        return EncryptedGovernanceData()

    async def test_encrypt_community_token(self, encrypted_data, encryption_service):
        """Test encrypting community token."""
        plaintext_token = "sk_test_abc123xyz"

        encrypted = await encrypted_data.encrypt_token(plaintext_token)

        assert encrypted is not None
        assert encrypted != plaintext_token
        assert isinstance(encrypted, str)
        # Encrypted token should be hex string (twice the binary length)
        assert len(encrypted) > len(plaintext_token)

    async def test_decrypt_community_token(self, encrypted_data, encryption_service):
        """Test decrypting community token."""
        plaintext_token = "sk_test_abc123xyz"

        encrypted = await encrypted_data.encrypt_token(plaintext_token)
        decrypted = await encrypted_data.decrypt_token(encrypted)

        assert decrypted == plaintext_token

    async def test_encrypt_decrypt_roundtrip(self, encrypted_data):
        """Test encrypt/decrypt roundtrip preserves data."""
        test_values = [
            "sk_prod_token_123",
            "0xdeadbeef123456",
            "complex!@#$%^&*()token",
        ]

        for value in test_values:
            encrypted = await encrypted_data.encrypt_token(value)
            decrypted = await encrypted_data.decrypt_token(encrypted)
            assert decrypted == value

    async def test_decrypt_none_token(self, encrypted_data):
        """Test decrypting None returns None."""
        result = await encrypted_data.decrypt_token(None)
        assert result is None

    async def test_encrypt_treasury_address(self, encrypted_data):
        """Test encrypting treasury address."""
        plaintext_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f123456"

        encrypted = await encrypted_data.encrypt_address(plaintext_addr)

        assert encrypted is not None
        assert encrypted != plaintext_addr
        assert isinstance(encrypted, str)

    async def test_decrypt_treasury_address(self, encrypted_data):
        """Test decrypting treasury address."""
        plaintext_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f123456"

        encrypted = await encrypted_data.encrypt_address(plaintext_addr)
        decrypted = await encrypted_data.decrypt_address(encrypted)

        assert decrypted == plaintext_addr

    async def test_encryption_service_not_configured(self):
        """Test graceful degradation when encryption service not set."""
        EncryptedGovernanceData.set_encryption_service(None)
        data = EncryptedGovernanceData()

        # Should return plaintext if service not available
        result = await data.encrypt_token("test_token")
        assert result == "test_token"

    async def test_tampered_ciphertext_fails(self, encrypted_data):
        """Test that tampered ciphertext fails decryption."""
        plaintext_token = "sk_test_token_123"

        encrypted = await encrypted_data.encrypt_token(plaintext_token)

        # Tamper with the ciphertext by flipping bits
        tampered = "0" + encrypted[1:]

        with pytest.raises(ValueError, match="Decryption failed"):
            await encrypted_data.decrypt_token(tampered)

    async def test_empty_token_returns_none(self, encrypted_data):
        """Test encrypting empty token returns None."""
        result = await encrypted_data.encrypt_token("")
        assert result is None

    async def test_empty_address_returns_none(self, encrypted_data):
        """Test encrypting empty address returns None."""
        result = await encrypted_data.encrypt_address("")
        assert result is None


@pytest.mark.asyncio
class TestEncryptedCommunityModel:
    """Test encryption in community model operations."""

    @pytest.fixture
    async def encryption_service(self):
        """Create test encryption service."""
        config = EncryptionConfig(
            vault_addr="http://localhost:8200",
            vault_token="test-token",
        )
        key_manager = EncryptionKeyManager(config)
        return EncryptionService(key_manager)

    @pytest.fixture
    async def community_model(self, encryption_service):
        """Create EncryptedCommunityModel with service."""
        return EncryptedCommunityModel(encryption_service)

    async def test_prepare_create_encrypts_token(self, community_model):
        """Test prepare_create encrypts community token."""
        data = {
            "community_id": "com_123",
            "community_name": "Test DAO",
            "platform": "discord",
            "community_token": "sk_test_token_abc",
        }

        prepared = await community_model.prepare_create(data)

        assert "_community_token_encrypted" in prepared
        assert "community_token" not in prepared
        assert prepared["_community_token_encrypted"] != data["community_token"]

    async def test_prepare_create_encrypts_address(self, community_model):
        """Test prepare_create encrypts treasury address."""
        data = {
            "community_id": "com_123",
            "community_name": "Test DAO",
            "platform": "discord",
            "treasury_address": "0xdeadbeef123456",
        }

        prepared = await community_model.prepare_create(data)

        assert "_treasury_address_encrypted" in prepared
        assert "treasury_address" not in prepared
        assert prepared["_treasury_address_encrypted"] != data["treasury_address"]

    async def test_prepare_create_encrypts_both(self, community_model):
        """Test prepare_create encrypts both token and address."""
        data = {
            "community_id": "com_123",
            "community_name": "Test DAO",
            "platform": "discord",
            "community_token": "sk_test_token",
            "treasury_address": "0xdeadbeef123456",
        }

        prepared = await community_model.prepare_create(data)

        assert "_community_token_encrypted" in prepared
        assert "_treasury_address_encrypted" in prepared
        assert "community_token" not in prepared
        assert "treasury_address" not in prepared

    async def test_prepare_create_skips_encryption_if_no_service(self):
        """Test prepare_create skips encryption if service not available."""
        model = EncryptedCommunityModel(None)
        data = {
            "community_id": "com_123",
            "community_token": "plaintext_token",
        }

        prepared = await model.prepare_create(data)

        # Should keep plaintext if no encryption service
        assert prepared["community_token"] == "plaintext_token"

    async def test_prepare_response_decrypts_token(self, community_model):
        """Test prepare_response decrypts community token."""
        # Create encrypted community mock
        community = MagicMock()
        community.community_id = "com_123"
        community.community_name = "Test DAO"
        community.platform = "discord"
        community.token_symbol = "CYNIC"
        community.voting_period_hours = 72
        community.execution_delay_hours = 24
        community.quorum_percentage = 25.0
        community.approval_threshold_percentage = 50.0
        community.gasdf_enabled = True
        community.cynic_enabled = True
        community.created_at = None

        # Encrypt a token for storage
        test_token = "sk_test_response_token"
        encrypted_token = await community_model.encryption_service.encrypt_string(
            test_token,
            key_id="governance-community-token",
        )
        community._community_token_encrypted = encrypted_token
        community._treasury_address_encrypted = None

        response = await community_model.prepare_response(community)

        assert response["community_token"] == test_token
        assert response["treasury_address"] is None

    async def test_prepare_response_decrypts_address(self, community_model):
        """Test prepare_response decrypts treasury address."""
        community = MagicMock()
        community.community_id = "com_123"
        community.community_name = "Test DAO"
        community.platform = "discord"
        community.token_symbol = "CYNIC"
        community.voting_period_hours = 72
        community.execution_delay_hours = 24
        community.quorum_percentage = 25.0
        community.approval_threshold_percentage = 50.0
        community.gasdf_enabled = True
        community.cynic_enabled = True
        community.created_at = None
        community._community_token_encrypted = None

        # Encrypt an address for storage
        test_address = "0xdeadbeef123456"
        encrypted_address = await community_model.encryption_service.encrypt_string(
            test_address,
            key_id="governance-treasury-address",
        )
        community._treasury_address_encrypted = encrypted_address

        response = await community_model.prepare_response(community)

        assert response["community_token"] is None
        assert response["treasury_address"] == test_address

    async def test_prepare_response_handles_missing_encrypted(self, community_model):
        """Test prepare_response handles missing encrypted fields gracefully."""
        community = MagicMock()
        community.community_id = "com_123"
        community.community_name = "Test DAO"
        community.platform = "discord"
        community.token_symbol = "CYNIC"
        community.voting_period_hours = 72
        community.execution_delay_hours = 24
        community.quorum_percentage = 25.0
        community.approval_threshold_percentage = 50.0
        community.gasdf_enabled = True
        community.cynic_enabled = True
        community.created_at = None
        community._community_token_encrypted = None
        community._treasury_address_encrypted = None

        response = await community_model.prepare_response(community)

        assert response["community_token"] is None
        assert response["treasury_address"] is None

    async def test_prepare_create_handles_none_values(self, community_model):
        """Test prepare_create handles None values gracefully."""
        data = {
            "community_id": "com_123",
            "community_name": "Test DAO",
            "community_token": None,
            "treasury_address": None,
        }

        prepared = await community_model.prepare_create(data)

        # None values should be handled gracefully
        assert "_community_token_encrypted" not in prepared
        assert "_treasury_address_encrypted" not in prepared


@pytest.mark.asyncio
class TestEncryptionDataProtection:
    """Test that encryption protects data at rest."""

    async def test_encrypted_value_not_readable_without_key(self):
        """Test that encrypted value is not human-readable."""
        config = EncryptionConfig()
        key_manager = EncryptionKeyManager(config)
        service = EncryptionService(key_manager)

        plaintext = "sensitive_governance_token"

        encrypted = await service.encrypt_string(plaintext, key_id="test")

        # Encrypted value should not contain plaintext
        assert plaintext not in encrypted
        # Should be hex-encoded
        assert all(c in "0123456789abcdef" for c in encrypted.lower())

    async def test_different_encryptions_same_plaintext_differ(self):
        """Test that same plaintext encrypts to different ciphertext (due to random nonce)."""
        config = EncryptionConfig()
        key_manager = EncryptionKeyManager(config)
        service = EncryptionService(key_manager)

        plaintext = "test_token_123"

        encrypted1 = await service.encrypt_string(plaintext, key_id="test")
        encrypted2 = await service.encrypt_string(plaintext, key_id="test")

        # Different nonces should produce different ciphertexts
        assert encrypted1 != encrypted2

        # But both should decrypt to same plaintext
        assert await service.decrypt_string(encrypted1, key_id="test") == plaintext
        assert await service.decrypt_string(encrypted2, key_id="test") == plaintext
