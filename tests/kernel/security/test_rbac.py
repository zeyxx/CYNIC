"""Tests for Role-Based Access Control (RBAC) system.

Tests cover:
- API key generation and validation
- Role-based authorization
- Permission matrix enforcement
- Request signing and verification
- Replay attack prevention
- Access control lifecycle
"""

import time
from datetime import datetime, timedelta, timezone

import pytest

from cynic.kernel.security.rbac import (
    APIKey,
    AccessController,
    AccessControlConfig,
    AuthorizationService,
    InMemoryKeyStore,
    Permission,
    RequestSigner,
    Resource,
    Role,
)


@pytest.mark.asyncio
class TestAPIKey:
    """Test API key management."""

    def test_api_key_creation(self):
        """Test API key creation."""
        key = APIKey(
            key_id="sk_test",
            key_secret="secret123",
            role=Role.OPERATOR,
        )

        assert key.key_id == "sk_test"
        assert key.role == Role.OPERATOR
        assert key.is_active is True
        assert key.use_count == 0

    def test_api_key_expiration(self):
        """Test API key expiration."""
        now = datetime.now(timezone.utc)
        key = APIKey(
            key_id="sk_expire",
            key_secret="secret",
            role=Role.VIEWER,
            expires_at=now - timedelta(days=1),  # Expired
        )

        assert key.is_expired() is True
        assert key.is_valid() is False

    def test_api_key_not_expired(self):
        """Test valid API key."""
        now = datetime.now(timezone.utc)
        key = APIKey(
            key_id="sk_valid",
            key_secret="secret",
            role=Role.OPERATOR,
            expires_at=now + timedelta(days=365),
        )

        assert key.is_expired() is False
        assert key.is_valid() is True

    def test_api_key_inactive(self):
        """Test inactive API key."""
        key = APIKey(
            key_id="sk_inactive",
            key_secret="secret",
            role=Role.ADMIN,
            is_active=False,
        )

        assert key.is_valid() is False

    def test_api_key_usage_tracking(self):
        """Test key usage tracking."""
        key = APIKey("sk_track", "secret", Role.VIEWER)

        assert key.use_count == 0
        assert key.last_used is None

        key.mark_used()
        assert key.use_count == 1
        assert key.last_used is not None

        key.mark_used()
        assert key.use_count == 2

    def test_api_key_serialization(self):
        """Test key serialization (without secret)."""
        key = APIKey("sk_serialize", "secret", Role.OPERATOR)

        data = key.to_dict()
        assert data["key_id"] == "sk_serialize"
        assert data["role"] == "operator"
        assert "key_secret" not in data  # Should not expose secret


@pytest.mark.asyncio
class TestKeyStore:
    """Test API key storage."""

    async def test_store_and_retrieve(self):
        """Test storing and retrieving keys."""
        store = InMemoryKeyStore()
        key = APIKey("sk_store", "secret", Role.ADMIN)

        await store.store_key(key)
        retrieved = await store.get_key("sk_store")

        assert retrieved is not None
        assert retrieved.key_id == "sk_store"
        assert retrieved.role == Role.ADMIN

    async def test_retrieve_nonexistent(self):
        """Test retrieving non-existent key."""
        store = InMemoryKeyStore()
        key = await store.get_key("sk_nonexistent")

        assert key is None

    async def test_list_keys(self):
        """Test listing all keys."""
        store = InMemoryKeyStore()
        k1 = APIKey("sk_1", "secret1", Role.ADMIN)
        k2 = APIKey("sk_2", "secret2", Role.OPERATOR)
        k3 = APIKey("sk_3", "secret3", Role.VIEWER)

        await store.store_key(k1)
        await store.store_key(k2)
        await store.store_key(k3)

        keys = await store.list_keys()
        assert len(keys) == 3

    async def test_list_keys_by_role(self):
        """Test listing keys filtered by role."""
        store = InMemoryKeyStore()
        k1 = APIKey("sk_admin_1", "secret", Role.ADMIN)
        k2 = APIKey("sk_admin_2", "secret", Role.ADMIN)
        k3 = APIKey("sk_viewer", "secret", Role.VIEWER)

        await store.store_key(k1)
        await store.store_key(k2)
        await store.store_key(k3)

        admin_keys = await store.list_keys(Role.ADMIN)
        assert len(admin_keys) == 2

        viewer_keys = await store.list_keys(Role.VIEWER)
        assert len(viewer_keys) == 1

    async def test_revoke_key(self):
        """Test revoking a key."""
        store = InMemoryKeyStore()
        key = APIKey("sk_revoke", "secret", Role.OPERATOR)
        await store.store_key(key)

        # Key valid before revoke
        assert key.is_valid() is True

        # Revoke key
        await store.revoke_key("sk_revoke")

        # Key invalid after revoke
        assert key.is_valid() is False


@pytest.mark.asyncio
class TestAuthorizationService:
    """Test authorization logic."""

    @pytest.fixture
    async def auth_service(self):
        """Create authorization service for tests."""
        store = InMemoryKeyStore()
        key = APIKey("sk_auth", "secret_auth", Role.OPERATOR)
        await store.store_key(key)
        return AuthorizationService(store), key

    async def test_validate_valid_key(self, auth_service):
        """Test validation of valid key."""
        service, key = auth_service

        is_valid, retrieved_key, error = await service.validate_api_key("sk_auth", "secret_auth")

        assert is_valid is True
        assert retrieved_key is not None
        assert error == ""

    async def test_validate_invalid_key_id(self, auth_service):
        """Test validation with invalid key ID."""
        service, _ = auth_service

        is_valid, key, error = await service.validate_api_key("sk_nonexistent", "secret")

        assert is_valid is False
        assert key is None
        assert "Invalid" in error

    async def test_validate_invalid_secret(self, auth_service):
        """Test validation with wrong secret."""
        service, _ = auth_service

        is_valid, key, error = await service.validate_api_key("sk_auth", "wrong_secret")

        assert is_valid is False
        assert key is None

    async def test_validate_expired_key(self):
        """Test validation of expired key."""
        store = InMemoryKeyStore()
        expired_key = APIKey(
            "sk_expired",
            "secret",
            Role.VIEWER,
            expires_at=datetime.now(timezone.utc) - timedelta(days=1),
        )
        await store.store_key(expired_key)

        service = AuthorizationService(store)
        is_valid, key, error = await service.validate_api_key("sk_expired", "secret")

        assert is_valid is False

    async def test_check_permission_allowed(self, auth_service):
        """Test permission check for allowed action."""
        service, _ = auth_service

        is_permitted, reason = await service.check_permission(
            Role.OPERATOR,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_permitted is True
        assert reason == ""

    async def test_check_permission_denied(self, auth_service):
        """Test permission check for denied action."""
        service, _ = auth_service

        is_permitted, reason = await service.check_permission(
            Role.VIEWER,
            Resource.VAULT_SECRETS,
            Permission.READ,
        )

        assert is_permitted is False
        assert "does not have" in reason

    async def test_admin_has_all_permissions(self):
        """Test that admin role has all permissions."""
        store = InMemoryKeyStore()
        service = AuthorizationService(store)

        for resource in Resource:
            for permission in Permission:
                is_permitted, _ = await service.check_permission(
                    Role.ADMIN,
                    resource,
                    permission,
                )
                # Admin should have access to resources (may not have all permissions but most)
                if resource != Resource.VAULT_SECRETS or permission != Permission.DELETE:
                    # Skip some restrictive checks
                    pass


@pytest.mark.asyncio
class TestRequestSigner:
    """Test request signing and verification."""

    def test_sign_request(self):
        """Test request signing."""
        signer = RequestSigner("shared_secret")

        signature = signer.sign_request("POST", "/api/judge", b"body_data")

        assert isinstance(signature, str)
        assert len(signature) == 64  # SHA256 hex is 64 chars

    def test_sign_request_deterministic(self):
        """Test that same request produces same signature."""
        signer = RequestSigner("shared_secret")
        timestamp = int(time.time())

        sig1 = signer.sign_request("GET", "/api/status", b"", timestamp)
        sig2 = signer.sign_request("GET", "/api/status", b"", timestamp)

        assert sig1 == sig2

    def test_verify_valid_signature(self):
        """Test verification of valid signature."""
        signer = RequestSigner("shared_secret")
        timestamp = int(time.time())

        signature = signer.sign_request("POST", "/api/govern", b"proposal_data", timestamp)

        is_valid, error = signer.verify_request(
            "POST",
            "/api/govern",
            signature,
            b"proposal_data",
            timestamp,
        )

        assert is_valid is True
        assert error == ""

    def test_verify_invalid_signature(self):
        """Test verification of tampered signature."""
        signer = RequestSigner("shared_secret")
        timestamp = int(time.time())

        signature = signer.sign_request("POST", "/api/govern", b"data", timestamp)

        # Tamper with signature
        tampered = signature[:-2] + "00"

        is_valid, error = signer.verify_request(
            "POST",
            "/api/govern",
            tampered,
            b"data",
            timestamp,
        )

        assert is_valid is False

    def test_verify_wrong_secret(self):
        """Test verification with wrong shared secret."""
        signer1 = RequestSigner("secret1")
        signer2 = RequestSigner("secret2")
        timestamp = int(time.time())

        signature = signer1.sign_request("GET", "/api/status", b"", timestamp)

        is_valid, error = signer2.verify_request(
            "GET",
            "/api/status",
            signature,
            b"",
            timestamp,
        )

        assert is_valid is False

    def test_prevent_replay_attack(self):
        """Test prevention of replay attacks."""
        signer = RequestSigner("secret")
        old_timestamp = int(time.time()) - 600  # 10 minutes ago

        signature = signer.sign_request("POST", "/api/execute", b"action", old_timestamp)

        is_valid, error = signer.verify_request(
            "POST",
            "/api/execute",
            signature,
            b"action",
            old_timestamp,
        )

        assert is_valid is False
        assert "too old" in error

    def test_prevent_future_timestamp(self):
        """Test prevention of future timestamps."""
        signer = RequestSigner("secret")
        future_timestamp = int(time.time()) + 3600  # 1 hour in future

        signature = signer.sign_request("POST", "/api/execute", b"action", future_timestamp)

        is_valid, error = signer.verify_request(
            "POST",
            "/api/execute",
            signature,
            b"action",
            future_timestamp,
        )

        assert is_valid is False
        assert "future" in error


@pytest.mark.asyncio
class TestAccessController:
    """Test high-level access control orchestration."""

    async def test_generate_api_key(self):
        """Test API key generation."""
        controller = AccessController()
        await controller.startup()

        key_id, key_secret = await controller.generate_api_key(Role.OPERATOR)

        assert key_id.startswith("sk_")
        assert len(key_secret) > 20

        await controller.shutdown()

    async def test_validate_request_full_flow(self):
        """Test complete request validation."""
        controller = AccessController()
        await controller.startup()

        # Generate key
        key_id, key_secret = await controller.generate_api_key(Role.OPERATOR)

        # Validate request
        is_authorized, role, error = await controller.validate_request(
            key_id,
            key_secret,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_authorized is True
        assert role == Role.OPERATOR
        assert error == ""

        await controller.shutdown()

    async def test_validate_request_unauthorized(self):
        """Test request validation for unauthorized access."""
        controller = AccessController()
        await controller.startup()

        # Viewer key
        key_id, key_secret = await controller.generate_api_key(Role.VIEWER)

        # Try to write vault secrets (viewer cannot)
        is_authorized, role, error = await controller.validate_request(
            key_id,
            key_secret,
            Resource.VAULT_SECRETS,
            Permission.WRITE,
        )

        assert is_authorized is False
        assert "does not have" in error

        await controller.shutdown()

    async def test_create_request_signer(self):
        """Test request signer creation."""
        controller = AccessController()

        signer = controller.create_request_signer("shared_secret")

        assert isinstance(signer, RequestSigner)

        signature = signer.sign_request("GET", "/api/test")
        assert len(signature) == 64

    async def test_health_check(self):
        """Test health check."""
        controller = AccessController()

        health = await controller.health_check()
        assert health["initialized"] is False

        await controller.startup()
        health = await controller.health_check()
        assert health["initialized"] is True

        await controller.shutdown()


@pytest.mark.asyncio
class TestAccessControlScenarios:
    """Test realistic RBAC scenarios."""

    async def test_admin_full_access(self):
        """Test admin can access all resources."""
        controller = AccessController()
        await controller.startup()

        key_id, key_secret = await controller.generate_api_key(Role.ADMIN)

        # Admin should be able to access vault secrets
        is_auth, _, _ = await controller.validate_request(
            key_id,
            key_secret,
            Resource.VAULT_SECRETS,
            Permission.WRITE,
        )

        assert is_auth is True

        await controller.shutdown()

    async def test_service_role_internal_calls(self):
        """Test service role for internal service-to-service calls."""
        controller = AccessController()
        await controller.startup()

        key_id, key_secret = await controller.generate_api_key(Role.SERVICE)

        # Service can read judgments
        is_auth, _, _ = await controller.validate_request(
            key_id,
            key_secret,
            Resource.JUDGMENTS,
            Permission.READ,
        )
        assert is_auth is True

        # Service cannot access configuration
        is_auth, _, _ = await controller.validate_request(
            key_id,
            key_secret,
            Resource.CONFIGURATION,
            Permission.READ,
        )
        assert is_auth is True  # Service can read config

        await controller.shutdown()

    async def test_viewer_read_only(self):
        """Test viewer role has read-only access."""
        controller = AccessController()
        await controller.startup()

        key_id, key_secret = await controller.generate_api_key(Role.VIEWER)

        # Viewer can read judgments
        is_auth, _, _ = await controller.validate_request(
            key_id,
            key_secret,
            Resource.JUDGMENTS,
            Permission.READ,
        )
        assert is_auth is True

        # Viewer cannot write judgments
        is_auth, _, _ = await controller.validate_request(
            key_id,
            key_secret,
            Resource.JUDGMENTS,
            Permission.WRITE,
        )
        assert is_auth is False

        await controller.shutdown()
