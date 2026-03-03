"""Tests for Vault secret management integration."""

from __future__ import annotations

import os
from datetime import datetime, timedelta, timezone

import pytest

from cynic.kernel.security.vault import (
    EnvironmentSecretStore,
    SecretManager,
    VaultConfig,
)


@pytest.fixture
def vault_config() -> VaultConfig:
    """Create Vault configuration."""
    return VaultConfig(
        vault_addr="",  # Empty to skip real Vault
        auto_rotate_days=30,
    )


@pytest.fixture
def secret_manager(vault_config: VaultConfig) -> SecretManager:
    """Create SecretManager instance."""
    return SecretManager(vault_config)


class TestVaultConfig:
    """Tests for Vault configuration."""

    def test_vault_config_defaults(self) -> None:
        """Vault config has sensible defaults."""
        config = VaultConfig()

        assert config.vault_addr == ""
        assert config.auto_rotate_days == 60
        assert config.kv_mount_path == "secret"

    def test_vault_config_custom(self) -> None:
        """Vault config can be customized."""
        config = VaultConfig(
            vault_addr="https://vault.example.com:8200",
            vault_token="s.1234567890abcdef",
            vault_namespace="admin",
            auto_rotate_days=45,
        )

        assert config.vault_addr == "https://vault.example.com:8200"
        assert config.vault_token == "s.1234567890abcdef"
        assert config.vault_namespace == "admin"
        assert config.auto_rotate_days == 45

    def test_vault_config_from_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Vault config reads from environment variables via CynicConfig."""
        monkeypatch.setenv("VAULT_ADDR", "https://vault.local:8200")
        monkeypatch.setenv("VAULT_TOKEN", "s.env_token")

        from cynic.config import CynicConfig

        cynic_config = CynicConfig.from_env()
        config = VaultConfig.from_config(cynic_config)

        assert config.vault_addr == "https://vault.local:8200"
        assert config.vault_token == "s.env_token"


class TestEnvironmentSecretStore:
    """Tests for environment variable secret store (fallback via CynicConfig)."""

    @pytest.mark.asyncio
    async def test_get_secret_from_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Secrets can be retrieved from environment variables via CynicConfig."""
        monkeypatch.setenv("ANTHROPIC_API_KEY", "secret123")

        from cynic.config import CynicConfig

        config = CynicConfig.from_env()
        store = EnvironmentSecretStore(config)

        # "anthropic.api.key" maps to "anthropic_api_key" attribute in CynicConfig
        secret = await store.get_secret("anthropic.api.key")
        assert secret == "secret123"

    @pytest.mark.asyncio
    async def test_get_secret_legacy_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Legacy secrets can still be retrieved directly from environment."""
        monkeypatch.setenv("SECRET_API_KEY", "legacy123")
        store = EnvironmentSecretStore()

        secret = await store.get_secret("api.key")
        assert secret == "legacy123"

    @pytest.mark.asyncio
    async def test_get_secret_not_found(self) -> None:
        """Non-existent secret returns None."""
        store = EnvironmentSecretStore()
        secret = await store.get_secret("nonexistent.key")
        assert secret is None

    @pytest.mark.asyncio
    async def test_put_secret_to_env(self) -> None:
        """Secrets can be stored in environment variables."""
        store = EnvironmentSecretStore()
        await store.put_secret("test.key", "test_value")

        # Verify it's in the environment
        env_key = "SECRET_TEST_KEY"
        assert os.getenv(env_key) == "test_value"

        # Cleanup
        if env_key in os.environ:
            del os.environ[env_key]

    @pytest.mark.asyncio
    async def test_delete_secret_from_env(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Secrets can be deleted from environment."""
        monkeypatch.setenv("SECRET_TEST_KEY", "test_value")
        store = EnvironmentSecretStore()

        await store.delete_secret("test.key")

        # Verify it's deleted
        assert os.getenv("SECRET_TEST_KEY") is None

    @pytest.mark.asyncio
    async def test_rotate_secret(self) -> None:
        """Secrets can be rotated."""
        store = EnvironmentSecretStore()
        new_value = await store.rotate_secret("test.key")

        # Verify new value is not empty and stored
        assert new_value is not None
        assert len(new_value) > 0

    @pytest.mark.asyncio
    async def test_health_check_always_ok(self) -> None:
        """Environment store health check always returns True."""
        store = EnvironmentSecretStore()
        health = await store.health_check()
        assert health is True


class TestSecretManager:
    """Tests for SecretManager with fallback."""

    @pytest.mark.asyncio
    async def test_startup_without_vault(self, vault_config: VaultConfig) -> None:
        """SecretManager starts without Vault (uses fallback)."""
        vault_config.vault_addr = ""  # No Vault
        manager = SecretManager(vault_config)

        await manager.startup()

        assert manager._initialized is True
        assert manager.primary_store is not None

    @pytest.mark.asyncio
    async def test_get_secret_from_env_fallback(
        self, secret_manager: SecretManager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """SecretManager retrieves from environment fallback."""
        await secret_manager.startup()
        monkeypatch.setenv("SECRET_TEST_KEY", "test_value")

        secret = await secret_manager.get_secret("test.key")

        assert secret == "test_value"

    @pytest.mark.asyncio
    async def test_put_secret(self, secret_manager: SecretManager) -> None:
        """SecretManager stores secrets."""
        await secret_manager.startup()

        await secret_manager.put_secret("test.key", "test_value")

        # Verify it was stored in fallback
        secret = await secret_manager.get_secret("test.key")
        assert secret == "test_value"

        # Cleanup
        if "SECRET_TEST_KEY" in os.environ:
            del os.environ["SECRET_TEST_KEY"]

    @pytest.mark.asyncio
    async def test_delete_secret(self, secret_manager: SecretManager) -> None:
        """SecretManager deletes secrets."""
        await secret_manager.startup()

        # Store first
        await secret_manager.put_secret("test.key", "test_value")
        assert await secret_manager.get_secret("test.key") is not None

        # Delete
        await secret_manager.delete_secret("test.key")
        assert await secret_manager.get_secret("test.key") is None

    @pytest.mark.asyncio
    async def test_rotate_secret(self, secret_manager: SecretManager) -> None:
        """SecretManager rotates secrets."""
        await secret_manager.startup()

        # Store initial
        await secret_manager.put_secret("test.key", "initial_value")
        initial = await secret_manager.get_secret("test.key")

        # Rotate
        new_value = await secret_manager.rotate_secret("test.key")

        # Verify changed
        assert new_value != initial
        assert new_value is not None

        # Verify in rotation schedule
        assert "test.key" in secret_manager._rotation_schedule

        # Cleanup
        await secret_manager.delete_secret("test.key")

    @pytest.mark.asyncio
    async def test_check_rotations(self, secret_manager: SecretManager) -> None:
        """SecretManager can check rotation schedule."""
        await secret_manager.startup()

        # Register a secret as rotated long ago
        secret_manager.register_rotation_schedule(
            "old.key",
            rotated_at=datetime.now(timezone.utc) - timedelta(days=70),
        )

        # Register a recent rotation
        secret_manager.register_rotation_schedule(
            "new.key",
            rotated_at=datetime.now(timezone.utc),
        )

        # Check which ones need rotation
        due = await secret_manager.check_rotations()

        # old.key should be due (70 days > 60 day config)
        assert "old.key" in due
        assert "new.key" not in due

    @pytest.mark.asyncio
    async def test_health_check(self, secret_manager: SecretManager) -> None:
        """SecretManager health check."""
        await secret_manager.startup()

        health = await secret_manager.health_check()

        assert "primary" in health
        assert "fallback" in health
        assert health["fallback"] is True  # Fallback always OK

    @pytest.mark.asyncio
    async def test_not_initialized_raises(self) -> None:
        """Using uninitialized SecretManager raises error."""
        manager = SecretManager()

        with pytest.raises(RuntimeError, match="not initialized"):
            await manager.get_secret("test.key")

    @pytest.mark.asyncio
    async def test_shutdown(self, secret_manager: SecretManager) -> None:
        """SecretManager can be shut down."""
        await secret_manager.startup()
        await secret_manager.shutdown()
        # Should not raise


class TestSecretRotation:
    """Tests for secret rotation scheduling."""

    @pytest.mark.asyncio
    async def test_rotation_schedule_tracking(
        self, secret_manager: SecretManager
    ) -> None:
        """Rotation schedule is tracked correctly."""
        await secret_manager.startup()

        # Register secret
        secret_manager.register_rotation_schedule("db.password")

        assert "db.password" in secret_manager._rotation_schedule

    @pytest.mark.asyncio
    async def test_custom_rotation_time(self, secret_manager: SecretManager) -> None:
        """Custom rotation time can be set."""
        await secret_manager.startup()

        custom_time = datetime.now(timezone.utc) - timedelta(days=10)
        secret_manager.register_rotation_schedule("api.key", rotated_at=custom_time)

        assert secret_manager._rotation_schedule["api.key"] == custom_time
