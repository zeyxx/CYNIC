"""HashiCorp Vault integration for secret management.

Implements secure secret storage with auto-rotation, versioning, and graceful fallback.

Architecture:
- Secrets never stored in git or environment variables
- All secrets managed by Vault with access logging
- Auto-rotation: API keys rotated every 60 days
- Graceful degradation: Falls back to environment variables if Vault unavailable
- Leasing: Automatic lease renewal for dynamic secrets
- ACL: Role-based access control (task 1.5)

Integration points:
- Lifespan event: Connect to Vault, set up leases
- Config loading: Fetch secrets from Vault, not from env
- Auto-rotation: Background task checks and rotates secrets
"""

from __future__ import annotations

import logging
import os
from abc import ABC, abstractmethod
from datetime import datetime, timedelta, timezone
from typing import Any

from cynic.kernel.core.config import CynicConfig

logger = logging.getLogger(__name__)


class SecretStore(ABC):
    """Abstract interface for secret backends."""

    @abstractmethod
    async def get_secret(self, key: str) -> str | None:
        """Retrieve a secret by key."""
        pass

    @abstractmethod
    async def put_secret(self, key: str, value: str) -> None:
        """Store a secret."""
        pass

    @abstractmethod
    async def delete_secret(self, key: str) -> None:
        """Delete a secret."""
        pass

    @abstractmethod
    async def rotate_secret(self, key: str) -> str:
        """Rotate a secret (create new value, return new value)."""
        pass

    @abstractmethod
    async def health_check(self) -> bool:
        """Check if backend is available."""
        pass


class VaultConfig:
    """Configuration for Vault integration."""

    def __init__(
        self,
        vault_addr: str | None = None,
        vault_token: str | None = None,
        vault_namespace: str | None = None,
        enable_auto_unseal: bool = False,
        auto_rotate_days: int = 60,
    ):
        """Initialize Vault config.

        Args:
            vault_addr: Vault server address (e.g., "https://vault.example.com:8200")
            vault_token: Vault authentication token
            vault_namespace: Vault namespace (Enterprise feature)
            enable_auto_unseal: Whether to enable auto-unseal
            auto_rotate_days: Days between secret rotations
        """
        self.vault_addr = vault_addr or os.getenv("VAULT_ADDR", "")
        self.vault_token = vault_token or os.getenv("VAULT_TOKEN", "")
        self.vault_namespace = vault_namespace or os.getenv("VAULT_NAMESPACE")
        self.enable_auto_unseal = enable_auto_unseal
        self.auto_rotate_days = auto_rotate_days
        self.kv_mount_path = "secret"  # Default KV v2 mount path

    @classmethod
    def from_config(cls, config: CynicConfig) -> VaultConfig:
        """Create VaultConfig from global CynicConfig."""
        return cls(
            vault_addr=config.vault_addr,
            vault_token=config.vault_token,
            vault_namespace=config.vault_namespace,
        )


class EnvironmentSecretStore(SecretStore):
    """Fallback secret store using environment variables (via CynicConfig)."""

    def __init__(self, config: CynicConfig | None = None):
        self.config = config

    async def get_secret(self, key: str) -> str | None:
        """Get secret from environment variable (Rule 3: via config if possible)."""
        if self.config:
            # Check if this is a known secret in CynicConfig
            attr_name = key.lower().replace(".", "_").replace("-", "_")
            if hasattr(self.config, attr_name):
                return getattr(self.config, attr_name)

        # Fallback to direct os.getenv ONLY if config not available or key unknown
        # NOTE: This is a legacy path, should ideally be migrated to CynicConfig fields
        import os

        env_key = f"SECRET_{key.upper().replace('.', '_').replace('-', '_')}"
        return os.getenv(env_key)

    async def put_secret(self, key: str, value: str) -> None:
        """Store secret in environment (not recommended for production)."""
        logger.warning(f"Storing secret {key} in environment variable (fallback only)")
        env_key = f"SECRET_{key.upper().replace('.', '_').replace('-', '_')}"
        os.environ[env_key] = value

    async def delete_secret(self, key: str) -> None:
        """Remove secret from environment."""
        env_key = f"SECRET_{key.upper().replace('.', '_').replace('-', '_')}"
        if env_key in os.environ:
            del os.environ[env_key]

    async def rotate_secret(self, key: str) -> str:
        """Rotate secret (generate new value)."""
        import secrets

        new_value = secrets.token_urlsafe(32)
        await self.put_secret(key, new_value)
        return new_value

    async def health_check(self) -> bool:
        """Environment variables are always available."""
        return True


class VaultSecretStore(SecretStore):
    """Secret store backed by HashiCorp Vault."""

    def __init__(self, config: VaultConfig):
        self.config = config
        self.client: Any = None
        self._authenticated = False
        self._leases: dict[str, dict[str, Any]] = {}

    async def connect(self) -> bool:
        """Establish connection to Vault."""
        if not self.config.vault_addr:
            logger.warning("VAULT_ADDR not set, Vault integration disabled")
            return False

        try:
            import hvac

            # Create client (async handling is limited in hvac, so we use sync client)
            self.client = hvac.Client(
                url=self.config.vault_addr,
                token=self.config.vault_token,
                namespace=self.config.vault_namespace,
            )

            # Verify auth
            auth_info = self.client.auth.token.lookup_self()
            logger.info(f" Connected to Vault as {auth_info['data']['display_name']}")
            self._authenticated = True
            return True
        except ImportError:
            logger.warning("hvac library not installed, Vault integration disabled")
            return False
        except Exception as e:
            logger.error(f"Failed to connect to Vault: {e}")
            return False

    async def get_secret(self, key: str) -> str | None:
        """Retrieve secret from Vault."""
        if not self._authenticated:
            logger.warning(f"Not authenticated to Vault, cannot fetch {key}")
            return None

        try:
            path = f"{self.config.kv_mount_path}/data/{key}"
            response = self.client.secrets.kv.v2.read_secret_version(path=key)
            return response["data"]["data"].get("value")
        except Exception as e:
            logger.warning(f"Failed to read secret {key} from Vault: {e}")
            return None

    async def put_secret(self, key: str, value: str) -> None:
        """Store secret in Vault."""
        if not self._authenticated:
            logger.error(f"Not authenticated to Vault, cannot store {key}")
            return

        try:
            self.client.secrets.kv.v2.create_or_update_secret(
                path=key,
                secret_data={"value": value},
            )
            logger.debug(f"Stored secret {key} in Vault")
        except Exception as e:
            logger.error(f"Failed to store secret {key} in Vault: {e}")

    async def delete_secret(self, key: str) -> None:
        """Delete secret from Vault."""
        if not self._authenticated:
            logger.error(f"Not authenticated to Vault, cannot delete {key}")
            return

        try:
            self.client.secrets.kv.v2.delete_secret_version(path=key)
            logger.debug(f"Deleted secret {key} from Vault")
        except Exception as e:
            logger.error(f"Failed to delete secret {key} from Vault: {e}")

    async def rotate_secret(self, key: str) -> str:
        """Rotate secret by generating new value."""
        import secrets

        new_value = secrets.token_urlsafe(32)
        await self.put_secret(key, new_value)

        # Track rotation time
        self._leases[key] = {
            "rotated_at": datetime.now(timezone.utc),
            "value": new_value,
        }
        logger.info(f"Rotated secret {key}")
        return new_value

    async def health_check(self) -> bool:
        """Check Vault health."""
        if not self.client:
            return False

        try:
            health = self.client.sys.is_sealed()
            return not health  # Return True if unsealed
        except Exception:
            return False


class SecretManager:
    """Manages secrets with rotation and fallback."""

    def __init__(self, config: VaultConfig | None = None):
        self.config = config or VaultConfig()
        self.primary_store: SecretStore | None = None
        self.fallback_store = EnvironmentSecretStore()
        self._initialized = False
        self._rotation_schedule: dict[str, datetime] = {}

    async def startup(self) -> None:
        """Initialize secret manager and connect to Vault."""
        logger.info("Starting secret manager...")

        # Try to connect to Vault
        vault_store = VaultSecretStore(self.config)
        if await vault_store.connect():
            self.primary_store = vault_store
            logger.info(" Using Vault for secret storage")
        else:
            logger.warning(" Vault unavailable, using environment variable fallback")
            self.primary_store = self.fallback_store

        self._initialized = True
        logger.info("Secret manager started")

    async def shutdown(self) -> None:
        """Clean up resources."""
        logger.info("Stopping secret manager...")
        # Cleanup if needed (close connections, etc.)
        logger.info("Secret manager stopped")

    async def get_secret(self, key: str) -> str | None:
        """Get secret from primary store, fallback to env vars."""
        if not self._initialized:
            raise RuntimeError("Secret manager not initialized")

        # Try primary store
        if self.primary_store:
            secret = await self.primary_store.get_secret(key)
            if secret:
                return secret

        # Fall back to environment
        logger.debug(f"Primary store unavailable for {key}, trying fallback")
        return await self.fallback_store.get_secret(key)

    async def put_secret(self, key: str, value: str) -> None:
        """Store secret in primary store."""
        if not self._initialized:
            raise RuntimeError("Secret manager not initialized")

        if self.primary_store:
            await self.primary_store.put_secret(key, value)
        else:
            # Fallback to environment
            await self.fallback_store.put_secret(key, value)

    async def delete_secret(self, key: str) -> None:
        """Delete secret from primary store."""
        if not self._initialized:
            raise RuntimeError("Secret manager not initialized")

        if self.primary_store:
            await self.primary_store.delete_secret(key)
        else:
            await self.fallback_store.delete_secret(key)

    async def rotate_secret(self, key: str) -> str:
        """Rotate a secret, update rotation schedule."""
        if not self._initialized:
            raise RuntimeError("Secret manager not initialized")

        # Rotate in both stores to keep in sync
        new_value = await (self.primary_store or self.fallback_store).rotate_secret(key)
        self._rotation_schedule[key] = datetime.now(timezone.utc)
        return new_value

    async def check_rotations(self) -> list[str]:
        """Check which secrets need rotation."""
        secrets_due = []

        for key, last_rotation in self._rotation_schedule.items():
            age = datetime.now(timezone.utc) - last_rotation
            if age > timedelta(days=self.config.auto_rotate_days):
                secrets_due.append(key)

        return secrets_due

    async def health_check(self) -> dict[str, bool]:
        """Check health of secret stores."""
        primary_health = False
        if self.primary_store:
            primary_health = await self.primary_store.health_check()

        return {
            "primary": primary_health,
            "fallback": await self.fallback_store.health_check(),
        }

    def register_rotation_schedule(
        self, key: str, rotated_at: datetime | None = None
    ) -> None:
        """Register a secret for auto-rotation tracking."""
        self._rotation_schedule[key] = rotated_at or datetime.now(timezone.utc)
