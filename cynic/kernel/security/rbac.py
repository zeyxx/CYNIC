"""Role-Based Access Control (RBAC) for CYNIC.

Implements zero-trust network access with:
- API key authentication + role-based authorization
- Request signing (HMAC-SHA256) for internal service calls
- Role hierarchy (admin > operator > viewer)
- Resource-level permissions (read, write, execute, admin)
- Time-limited credentials
- Audit logging on all access decisions

Architecture:
- All endpoints require authentication (API key + signature)
- All mutations require authorization (role + permission check)
- Internal service-to-service calls signed with HMAC
- No public endpoints (zero-trust model)
- Explicit allow (deny by default)

Integration points:
- API layer: FastAPI middleware validates requests
- Vault: API keys stored securely (task 1.2)
- Audit log: All access decisions logged (task 1.3)
- Encryption: API keys encrypted at rest (task 1.4)
"""

from __future__ import annotations

import hashlib
import hmac
import logging
import secrets
import time
from abc import ABC, abstractmethod
from datetime import datetime, timedelta, timezone
from enum import Enum
from typing import Any

logger = logging.getLogger(__name__)


class Role(str, Enum):
    """RBAC role hierarchy."""

    ADMIN = "admin"  # Full access
    OPERATOR = "operator"  # Read/write data, manage governance
    VIEWER = "viewer"  # Read-only access
    SERVICE = "service"  # Internal service-to-service
    ANONYMOUS = "anonymous"  # No access


class Permission(str, Enum):
    """Fine-grained resource permissions."""

    READ = "read"  # Get, list operations
    WRITE = "write"  # Create, update operations
    DELETE = "delete"  # Delete operations
    EXECUTE = "execute"  # Execute actions (judge, decide)
    ADMIN = "admin"  # Manage roles, keys, configs


class Resource(str, Enum):
    """Resources that can be protected."""

    JUDGMENTS = "judgments"
    DECISIONS = "decisions"
    GOVERNANCE = "governance"
    VAULT_SECRETS = "vault_secrets"
    AUDIT_LOG = "audit_log"
    CONFIGURATION = "configuration"
    USERS = "users"
    WALLETS = "wallets"
    BLOCKCHAIN = "blockchain"


# RBAC Matrix: Role  Permissions  Resources
RBAC_MATRIX = {
    Role.ADMIN: {
        Resource.JUDGMENTS: [
            Permission.READ,
            Permission.WRITE,
            Permission.DELETE,
            Permission.EXECUTE,
        ],
        Resource.DECISIONS: [
            Permission.READ,
            Permission.WRITE,
            Permission.DELETE,
            Permission.EXECUTE,
        ],
        Resource.GOVERNANCE: [
            Permission.READ,
            Permission.WRITE,
            Permission.DELETE,
            Permission.EXECUTE,
        ],
        Resource.VAULT_SECRETS: [
            Permission.READ,
            Permission.WRITE,
            Permission.DELETE,
            Permission.ADMIN,
        ],
        Resource.AUDIT_LOG: [Permission.READ, Permission.ADMIN],
        Resource.CONFIGURATION: [Permission.READ, Permission.WRITE, Permission.ADMIN],
        Resource.USERS: [
            Permission.READ,
            Permission.WRITE,
            Permission.DELETE,
            Permission.ADMIN,
        ],
        Resource.WALLETS: [Permission.READ, Permission.WRITE, Permission.ADMIN],
        Resource.BLOCKCHAIN: [
            Permission.READ,
            Permission.WRITE,
            Permission.EXECUTE,
            Permission.ADMIN,
        ],
    },
    Role.OPERATOR: {
        Resource.JUDGMENTS: [Permission.READ, Permission.WRITE, Permission.EXECUTE],
        Resource.DECISIONS: [Permission.READ, Permission.WRITE, Permission.EXECUTE],
        Resource.GOVERNANCE: [Permission.READ, Permission.WRITE, Permission.EXECUTE],
        Resource.VAULT_SECRETS: [],  # No access
        Resource.AUDIT_LOG: [Permission.READ],
        Resource.CONFIGURATION: [],  # No access
        Resource.USERS: [Permission.READ, Permission.WRITE],
        Resource.WALLETS: [Permission.READ],
        Resource.BLOCKCHAIN: [Permission.READ, Permission.EXECUTE],
    },
    Role.VIEWER: {
        Resource.JUDGMENTS: [Permission.READ],
        Resource.DECISIONS: [Permission.READ],
        Resource.GOVERNANCE: [Permission.READ],
        Resource.VAULT_SECRETS: [],
        Resource.AUDIT_LOG: [Permission.READ],
        Resource.CONFIGURATION: [],
        Resource.USERS: [Permission.READ],
        Resource.WALLETS: [Permission.READ],
        Resource.BLOCKCHAIN: [Permission.READ],
    },
    Role.SERVICE: {
        Resource.JUDGMENTS: [Permission.READ, Permission.WRITE, Permission.EXECUTE],
        Resource.DECISIONS: [Permission.READ, Permission.WRITE],
        Resource.GOVERNANCE: [Permission.READ],
        Resource.VAULT_SECRETS: [Permission.READ],
        Resource.AUDIT_LOG: [Permission.WRITE],
        Resource.CONFIGURATION: [Permission.READ],
        Resource.USERS: [Permission.READ],
        Resource.WALLETS: [Permission.READ],
        Resource.BLOCKCHAIN: [Permission.READ, Permission.EXECUTE],
    },
    Role.ANONYMOUS: {},  # No access to anything
}


class APIKey:
    """API key with metadata and permissions."""

    def __init__(
        self,
        key_id: str,
        key_secret: str,
        role: Role,
        created_at: datetime | None = None,
        expires_at: datetime | None = None,
        is_active: bool = True,
    ):
        self.key_id = key_id
        self.key_secret = key_secret
        self.role = role
        self.created_at = created_at or datetime.now(timezone.utc)
        self.expires_at = expires_at or (self.created_at + timedelta(days=365))
        self.is_active = is_active
        self.last_used: datetime | None = None
        self.use_count = 0

    def is_expired(self) -> bool:
        """Check if key has expired."""
        return datetime.now(timezone.utc) > self.expires_at

    def is_valid(self) -> bool:
        """Check if key is valid and active."""
        return self.is_active and not self.is_expired()

    def mark_used(self) -> None:
        """Update last used timestamp and counter."""
        self.last_used = datetime.now(timezone.utc)
        self.use_count += 1

    def to_dict(self) -> dict[str, Any]:
        """Serialize key (without secret)."""
        return {
            "key_id": self.key_id,
            "role": self.role.value,
            "created_at": self.created_at.isoformat(),
            "expires_at": self.expires_at.isoformat(),
            "is_active": self.is_active,
            "is_expired": self.is_expired(),
            "last_used": self.last_used.isoformat() if self.last_used else None,
            "use_count": self.use_count,
        }


class KeyStore(ABC):
    """Abstract interface for API key storage."""

    @abstractmethod
    async def get_key(self, key_id: str) -> APIKey | None:
        """Retrieve key by ID."""
        pass

    @abstractmethod
    async def store_key(self, key: APIKey) -> None:
        """Store API key."""
        pass

    @abstractmethod
    async def list_keys(self, role: Role | None = None) -> list[APIKey]:
        """List all keys, optionally filtered by role."""
        pass

    @abstractmethod
    async def revoke_key(self, key_id: str) -> None:
        """Revoke (deactivate) a key."""
        pass


class InMemoryKeyStore(KeyStore):
    """In-memory key store for testing."""

    def __init__(self):
        self._keys: dict[str, APIKey] = {}

    async def get_key(self, key_id: str) -> APIKey | None:
        return self._keys.get(key_id)

    async def store_key(self, key: APIKey) -> None:
        self._keys[key.key_id] = key

    async def list_keys(self, role: Role | None = None) -> list[APIKey]:
        keys = list(self._keys.values())
        if role:
            keys = [k for k in keys if k.role == role]
        return keys

    async def revoke_key(self, key_id: str) -> None:
        if key_id in self._keys:
            self._keys[key_id].is_active = False


class AuthorizationService:
    """Validates authentication and authorization."""

    def __init__(self, key_store: KeyStore):
        self.key_store = key_store

    async def validate_api_key(
        self, key_id: str, key_secret: str
    ) -> tuple[bool, APIKey | None, str]:
        """Validate API key credentials.

        Args:
            key_id: Key identifier
            key_secret: Key secret

        Returns:
            (is_valid, key, error_message)
        """
        # Fetch key from store
        key = await self.key_store.get_key(key_id)

        if not key:
            logger.warning(f"API key not found: {key_id}")
            return False, None, "Invalid API key"

        # Check if active and not expired
        if not key.is_valid():
            logger.warning(f"API key invalid/expired: {key_id}")
            return False, None, "API key expired or revoked"

        # Constant-time comparison to prevent timing attacks
        if not hmac.compare_digest(key.key_secret, key_secret):
            logger.warning(f"Invalid API key secret for: {key_id}")
            return False, None, "Invalid API key secret"

        # Mark as used
        key.mark_used()
        return True, key, ""

    async def check_permission(
        self,
        role: Role,
        resource: Resource,
        permission: Permission,
    ) -> tuple[bool, str]:
        """Check if role has permission for resource.

        Args:
            role: User role
            resource: Resource being accessed
            permission: Required permission

        Returns:
            (is_permitted, reason)
        """
        # Get allowed permissions for this role+resource
        allowed_perms = RBAC_MATRIX.get(role, {}).get(resource, [])

        if permission in allowed_perms:
            return True, ""

        reason = f"Role {role.value} does not have {permission.value} permission for {resource.value}"
        return False, reason

    async def can_access(
        self,
        role: Role,
        resource: Resource,
        permission: Permission,
    ) -> bool:
        """Quick check if role can access resource."""
        permitted, _ = await self.check_permission(role, resource, permission)
        return permitted


class RequestSigner:
    """Signs and verifies requests for service-to-service communication."""

    def __init__(self, service_key: str):
        """Initialize signer with service key.

        Args:
            service_key: Shared secret between services
        """
        self.service_key = (
            service_key.encode() if isinstance(service_key, str) else service_key
        )

    def sign_request(
        self,
        method: str,
        path: str,
        body: bytes = b"",
        timestamp: int | None = None,
    ) -> str:
        """Generate HMAC-SHA256 signature for request.

        Args:
            method: HTTP method (GET, POST, etc.)
            path: Request path
            body: Request body (empty for GET)
            timestamp: Unix timestamp (defaults to now)

        Returns:
            Base64-encoded signature
        """
        timestamp = timestamp or int(time.time())

        # Create canonical request: METHOD\nPATH\nTIMESTAMP\nBODY_HASH
        body_hash = hashlib.sha256(body).hexdigest()
        canonical_request = f"{method}\n{path}\n{timestamp}\n{body_hash}"

        # Sign with HMAC-SHA256
        signature = hmac.new(
            self.service_key,
            canonical_request.encode(),
            hashlib.sha256,
        ).digest()

        # Return hex-encoded signature
        return signature.hex()

    def verify_request(
        self,
        method: str,
        path: str,
        signature: str,
        body: bytes = b"",
        timestamp: int | None = None,
        max_age_seconds: int = 300,  # 5 minutes
    ) -> tuple[bool, str]:
        """Verify request signature.

        Args:
            method: HTTP method
            path: Request path
            signature: Provided signature (hex)
            body: Request body
            timestamp: Request timestamp
            max_age_seconds: Max age of request (replay attack prevention)

        Returns:
            (is_valid, error_message)
        """
        if timestamp is None:
            return False, "Missing timestamp"

        # Check timestamp freshness (prevent replay attacks)
        age = int(time.time()) - timestamp
        if age < 0:
            return False, "Request timestamp is in the future"
        if age > max_age_seconds:
            return False, "Request timestamp too old (possible replay)"

        # Compute expected signature
        expected_sig = self.sign_request(method, path, body, timestamp)

        # Constant-time comparison
        if not hmac.compare_digest(signature, expected_sig):
            logger.warning(f"Invalid signature for {method} {path}")
            return False, "Invalid request signature"

        return True, ""


class AccessControlConfig:
    """Configuration for RBAC system."""

    def __init__(
        self,
        default_role: Role = Role.VIEWER,
        max_key_age_days: int = 365,
        request_signature_ttl_seconds: int = 300,
        audit_all_access: bool = True,
    ):
        self.default_role = default_role
        self.max_key_age_days = max_key_age_days
        self.request_signature_ttl_seconds = request_signature_ttl_seconds
        self.audit_all_access = audit_all_access


class AccessController:
    """Orchestrates authentication, authorization, and auditing."""

    def __init__(
        self,
        config: AccessControlConfig | None = None,
        key_store: KeyStore | None = None,
    ):
        self.config = config or AccessControlConfig()
        self.key_store = key_store or InMemoryKeyStore()
        self.auth_service = AuthorizationService(self.key_store)
        self._initialized = False

    async def startup(self) -> None:
        """Initialize access control system."""
        logger.info("Starting access control system...")
        self._initialized = True
        logger.info(" Access control system started")

    async def shutdown(self) -> None:
        """Clean up resources."""
        logger.info("Stopping access control system...")
        self._initialized = False

    async def generate_api_key(
        self,
        role: Role,
        expires_in_days: int | None = None,
    ) -> tuple[str, str]:
        """Generate new API key.

        Args:
            role: Role for the key
            expires_in_days: Days until expiration

        Returns:
            (key_id, key_secret) - secret should be shown once
        """
        key_id = f"sk_{secrets.token_urlsafe(20)}"
        key_secret = secrets.token_urlsafe(32)

        expires_at = None
        if expires_in_days:
            expires_at = datetime.now(timezone.utc) + timedelta(days=expires_in_days)

        key = APIKey(
            key_id=key_id,
            key_secret=key_secret,
            role=role,
            expires_at=expires_at,
        )

        await self.key_store.store_key(key)
        logger.info(f"Generated API key {key_id} for role {role.value}")

        return key_id, key_secret

    async def validate_request(
        self,
        key_id: str,
        key_secret: str,
        resource: Resource,
        permission: Permission,
    ) -> tuple[bool, Role | None, str]:
        """Validate request authentication and authorization.

        Args:
            key_id: API key ID
            key_secret: API key secret
            resource: Resource being accessed
            permission: Required permission

        Returns:
            (is_authorized, role, error_message)
        """
        # Step 1: Authenticate
        is_valid, key, auth_error = await self.auth_service.validate_api_key(
            key_id, key_secret
        )
        if not is_valid:
            return False, None, auth_error

        # Step 2: Authorize
        is_permitted, authz_error = await self.auth_service.check_permission(
            key.role, resource, permission
        )
        if not is_permitted:
            logger.warning(f"Authorization denied for {key.role.value}: {authz_error}")
            return False, key.role, authz_error

        # Step 3: Success
        logger.debug(
            f"Access granted: {key.role.value}  {resource.value}:{permission.value}"
        )
        return True, key.role, ""

    def create_request_signer(self, service_key: str) -> RequestSigner:
        """Create request signer for service-to-service calls."""
        return RequestSigner(service_key)

    async def health_check(self) -> dict[str, bool]:
        """Check access control system health."""
        return {
            "initialized": self._initialized,
            "key_store_ready": True,
        }
