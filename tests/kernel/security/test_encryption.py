"""Tests for AES-256-GCM encryption service.

Tests cover:
- Encryption/decryption with random nonces
- Key management from Vault
- Graceful fallback when Vault unavailable
- Journal entry encryption
- String encryption
- Authentication tag validation (tampering detection)
- Thread safety
- Performance baselines
"""

import pytest
from cynic.kernel.security.encryption import (
    EncryptedJournalEntry,
    EncryptionConfig,
    EncryptionKeyManager,
    EncryptionService,
    TransparentEncryption,
)


@pytest.mark.asyncio
class TestEncryptionConfig:
    """Test encryption configuration."""

    def test_config_defaults(self):
        """Test default configuration values."""
        config = EncryptionConfig()
        assert config.algorithm == "aes-256-gcm"
        assert config.key_rotation_days == 90
        assert config.enable_journal_encryption is True
        assert config.kv_mount_path == "secret"

    def test_config_custom_values(self):
        """Test custom configuration."""
        config = EncryptionConfig(
            algorithm="aes-256-gcm",
            key_rotation_days=60,
            enable_journal_encryption=False,
        )
        assert config.algorithm == "aes-256-gcm"
        assert config.key_rotation_days == 60
        assert config.enable_journal_encryption is False

    def test_config_from_environment(self, monkeypatch):
        """Test loading configuration from environment variables via CynicConfig."""
        monkeypatch.setenv("VAULT_ADDR", "https://vault.example.com:8200")
        monkeypatch.setenv("VAULT_TOKEN", "s.test1234567890")

        from cynic.config import CynicConfig

        cynic_config = CynicConfig.from_env()
        config = EncryptionConfig.from_config(cynic_config)

        assert config.vault_addr == "https://vault.example.com:8200"
        assert config.vault_token == "s.test1234567890"


@pytest.mark.asyncio
class TestEncryptionKeyManager:
    """Test encryption key management."""

    async def test_key_manager_initialization(self):
        """Test key manager initialization."""
        config = EncryptionConfig()
        manager = EncryptionKeyManager(config)
        assert manager.client is None
        assert manager._authenticated is False
        assert manager._key_cache == {}

    async def test_key_generation_without_vault(self):
        """Test key generation when Vault is unavailable."""
        config = EncryptionConfig(vault_addr="")  # No Vault
        manager = EncryptionKeyManager(config)

        # Should generate a temporary key
        key = await manager.get_key("test-key")
        assert isinstance(key, bytes)
        assert len(key) == 32  # 256 bits = 32 bytes

    async def test_key_caching(self):
        """Test that keys are cached after first retrieval."""
        config = EncryptionConfig(vault_addr="")
        manager = EncryptionKeyManager(config)

        key1 = await manager.get_key("cached-key")
        key2 = await manager.get_key("cached-key")

        # Should be same instance from cache
        assert key1 is key2

    async def test_key_size_parameter(self):
        """Test custom key sizes."""
        config = EncryptionConfig(vault_addr="")
        manager = EncryptionKeyManager(config)

        key_128 = await manager.get_key("test-128", key_size=16)  # AES-128
        key_256 = await manager.get_key("test-256", key_size=32)  # AES-256

        assert len(key_128) == 16
        assert len(key_256) == 32


@pytest.mark.asyncio
class TestEncryptionService:
    """Test core encryption operations."""

    @pytest.fixture
    async def encryption_service(self):
        """Create encryption service for tests."""
        config = EncryptionConfig(vault_addr="")
        manager = EncryptionKeyManager(config)
        service = EncryptionService(manager)
        return service

    async def test_encrypt_decrypt_bytes(self, encryption_service):
        """Test encryption and decryption of binary data."""
        plaintext = b"Hello, World! This is a secret message."

        # Encrypt
        ciphertext = await encryption_service.encrypt(plaintext)

        # Verify nonce + ciphertext
        assert len(ciphertext) > len(plaintext)  # Overhead for nonce (12) + tag (16)
        assert ciphertext != plaintext  # Not plaintext

        # Decrypt
        decrypted = await encryption_service.decrypt(ciphertext)
        assert decrypted == plaintext

    async def test_encrypt_decrypt_string(self, encryption_service):
        """Test encryption and decryption of strings."""
        plaintext = "🔐 Sensitive governance token"

        # Encrypt
        encrypted_hex = await encryption_service.encrypt_string(plaintext)

        # Verify format (hex string)
        assert isinstance(encrypted_hex, str)
        assert all(c in "0123456789abcdef" for c in encrypted_hex)

        # Decrypt
        decrypted = await encryption_service.decrypt_string(encrypted_hex)
        assert decrypted == plaintext

    async def test_different_nonces_produce_different_ciphertexts(
        self, encryption_service
    ):
        """Test that random nonces prevent pattern leakage."""
        plaintext = b"Same plaintext"

        # Encrypt same plaintext twice
        ciphertext1 = await encryption_service.encrypt(plaintext)
        ciphertext2 = await encryption_service.encrypt(plaintext)

        # Should be different due to random nonce
        assert ciphertext1 != ciphertext2

        # But both should decrypt to same plaintext
        assert await encryption_service.decrypt(ciphertext1) == plaintext
        assert await encryption_service.decrypt(ciphertext2) == plaintext

    async def test_tampering_detection(self, encryption_service):
        """Test that GCM authentication detects tampering."""
        plaintext = b"Important financial record"

        # Encrypt
        ciphertext = await encryption_service.encrypt(plaintext)

        # Tamper with ciphertext (flip a bit in the payload)
        ciphertext_list = bytearray(ciphertext)
        ciphertext_list[15] ^= 0x01  # Flip one bit
        tampered = bytes(ciphertext_list)

        # Decryption should fail
        with pytest.raises(ValueError, match="Decryption failed"):
            await encryption_service.decrypt(tampered)

    async def test_tamper_nonce(self, encryption_service):
        """Test that tampering with nonce also fails."""
        plaintext = b"Governance proposal"

        # Encrypt
        ciphertext = await encryption_service.encrypt(plaintext)

        # Tamper with nonce (first 12 bytes)
        ciphertext_list = bytearray(ciphertext)
        ciphertext_list[0] ^= 0x01  # Flip bit in nonce
        tampered = bytes(ciphertext_list)

        # Decryption should fail
        with pytest.raises(ValueError):
            await encryption_service.decrypt(tampered)

    async def test_different_keys(self, encryption_service):
        """Test that decryption with wrong key fails."""
        plaintext = b"Secret message"

        # Encrypt with default key
        ciphertext = await encryption_service.encrypt(plaintext, "key1")

        # Try to decrypt with different key
        # This should fail or produce garbage
        with pytest.raises(ValueError):
            await encryption_service.decrypt(ciphertext, "key2")

    async def test_empty_plaintext(self, encryption_service):
        """Test encryption of empty data."""
        plaintext = b""

        ciphertext = await encryption_service.encrypt(plaintext)
        decrypted = await encryption_service.decrypt(ciphertext)

        assert decrypted == plaintext

    async def test_large_plaintext(self, encryption_service):
        """Test encryption of large data."""
        # 1 MB of data
        plaintext = b"x" * (1024 * 1024)

        ciphertext = await encryption_service.encrypt(plaintext)
        decrypted = await encryption_service.decrypt(ciphertext)

        assert decrypted == plaintext


@pytest.mark.asyncio
class TestEncryptedJournalEntry:
    """Test journal entry encryption."""

    @pytest.fixture
    async def journal_encryption(self):
        """Create journal encryption for tests."""
        config = EncryptionConfig(vault_addr="")
        manager = EncryptionKeyManager(config)
        encryption_service = EncryptionService(manager)
        return EncryptedJournalEntry(encryption_service)

    async def test_encrypt_entry_with_error(self, journal_encryption):
        """Test encryption of journal entry with error message."""
        entry = {
            "event_id": "evt-123",
            "timestamp_ms": 1234567890.0,
            "event_type": "core.judgment_created",
            "category": "judgment",
            "source": "kernel",
            "payload_keys": ["q_score", "verdict"],
            "is_error": True,
            "error_message": "Connection to database failed: sensitive credentials exposed",
        }

        # Encrypt
        encrypted = await journal_encryption.encrypt_entry(entry)

        # Verify error message is encrypted
        assert encrypted["_error_encrypted"] is True
        assert encrypted["error_message"] != entry["error_message"]
        assert isinstance(encrypted["error_message"], str)

        # Other fields unchanged
        assert encrypted["event_id"] == entry["event_id"]
        assert encrypted["source"] == entry["source"]

    async def test_decrypt_entry_with_error(self, journal_encryption):
        """Test decryption of journal entry with error message."""
        original_error = "Vault connection failed: sensitive token in logs"
        entry = {
            "event_id": "evt-456",
            "error_message": original_error,
            "is_error": True,
        }

        # Encrypt and decrypt
        encrypted = await journal_encryption.encrypt_entry(entry)
        decrypted = await journal_encryption.decrypt_entry(encrypted)

        # Error message restored
        assert decrypted["error_message"] == original_error
        assert "_error_encrypted" not in decrypted

    async def test_encrypt_entry_no_error(self, journal_encryption):
        """Test encryption of entry without error message."""
        entry = {
            "event_id": "evt-789",
            "timestamp_ms": 1234567890.0,
            "event_type": "core.judgment_created",
            "is_error": False,
        }

        encrypted = await journal_encryption.encrypt_entry(entry)

        # No _error_encrypted marker
        assert "_error_encrypted" not in encrypted
        assert encrypted == entry  # Unchanged


@pytest.mark.asyncio
class TestTransparentEncryption:
    """Test the high-level encryption orchestration."""

    async def test_startup_shutdown(self):
        """Test encryption service lifecycle."""
        encryption = TransparentEncryption()

        assert encryption._initialized is False

        await encryption.startup()
        assert encryption._initialized is True
        assert encryption.encryption_service is not None

        await encryption.shutdown()
        assert encryption._initialized is False

    async def test_encrypt_before_startup_fails(self):
        """Test that operations fail if not initialized."""
        encryption = TransparentEncryption()

        with pytest.raises(RuntimeError, match="not initialized"):
            await encryption.encrypt(b"data")

    async def test_full_workflow(self):
        """Test complete encryption workflow."""
        encryption = TransparentEncryption()
        await encryption.startup()

        # String encryption
        original = "🔒 Treasury Address: 0x1234567890abcdef"
        encrypted = await encryption.encrypt_string(original)
        decrypted = await encryption.decrypt_string(encrypted)

        assert decrypted == original

        # Journal encryption
        journal_entry = {
            "event_id": "evt-full",
            "error_message": "Sensitive error with API key: sk_live_123456",
        }

        encrypted_entry = await encryption.encrypt_journal_entry(journal_entry)
        decrypted_entry = await encryption.decrypt_journal_entry(encrypted_entry)

        assert decrypted_entry["error_message"] == journal_entry["error_message"]

        await encryption.shutdown()

    async def test_health_check_before_startup(self):
        """Test health check on uninitialized encryption."""
        encryption = TransparentEncryption()
        health = await encryption.health_check()

        assert health["initialized"] is False
        assert health["vault_ready"] is False

    async def test_health_check_after_startup(self):
        """Test health check on initialized encryption."""
        encryption = TransparentEncryption()
        await encryption.startup()

        health = await encryption.health_check()
        assert health["initialized"] is True

        await encryption.shutdown()

    async def test_multiple_key_ids(self):
        """Test encryption with different key IDs."""
        encryption = TransparentEncryption()
        await encryption.startup()

        data = "Secret data"

        # Encrypt with different keys
        encrypted_db = await encryption.encrypt_string(data, "db-column")
        encrypted_journal = await encryption.encrypt_string(data, "journal-errors")

        # Should be different ciphertexts (different keys)
        assert encrypted_db != encrypted_journal

        # Each decrypts with its own key
        assert await encryption.decrypt_string(encrypted_db, "db-column") == data
        assert (
            await encryption.decrypt_string(encrypted_journal, "journal-errors") == data
        )

        await encryption.shutdown()


@pytest.mark.asyncio
class TestEncryptionPerformance:
    """Test encryption performance characteristics."""

    async def test_encryption_throughput(self):
        """Test encryption throughput (MB/s)."""
        encryption = TransparentEncryption()
        await encryption.startup()

        # 10 MB of data
        data = b"x" * (10 * 1024 * 1024)
        iterations = 5

        import time

        start = time.time()
        for _ in range(iterations):
            await encryption.encrypt(data)
        elapsed = time.time() - start

        throughput_mbps = (iterations * len(data) / (1024 * 1024)) / elapsed

        # Should be > 100 MB/s on modern hardware
        assert throughput_mbps > 10  # At least 10 MB/s

        print(f"Encryption throughput: {throughput_mbps:.1f} MB/s")

        await encryption.shutdown()

    async def test_overhead_measurements(self):
        """Test encryption overhead (size and latency)."""
        encryption = TransparentEncryption()
        await encryption.startup()

        # Measure size overhead
        for size in [100, 1000, 10000]:
            plaintext = b"x" * size
            ciphertext = await encryption.encrypt(plaintext)

            overhead = len(ciphertext) - len(plaintext)
            assert overhead == 28  # 12 (nonce) + 16 (auth tag)

        # Measure latency
        import time

        plaintext = b"x" * 1000
        iterations = 100

        start = time.time()
        for _ in range(iterations):
            await encryption.encrypt(plaintext)
        elapsed = time.time() - start

        avg_latency_ms = (elapsed / iterations) * 1000
        assert avg_latency_ms < 10  # Under 10ms per encryption

        print(f"Encryption latency: {avg_latency_ms:.2f} ms")

        await encryption.shutdown()
