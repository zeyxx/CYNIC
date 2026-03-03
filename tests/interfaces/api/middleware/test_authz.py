"""Tests for RBAC authorization middleware.

Tests cover:
- API key extraction from Authorization header
- Permission checking via AccessController
- Authorization grant/deny decisions
- Audit logging of all access decisions
- Integration with governance endpoints
- Error handling for missing/invalid credentials
"""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi import FastAPI, HTTPException
from fastapi.testclient import TestClient
from pydantic import BaseModel

from cynic.interfaces.api.middleware.authz import (
    RBACAuthConfig,
    RBACAuthorizer,
    get_authorized_key_id,
    require_authz,
)
from cynic.kernel.security.rbac import (
    AccessController,
    Permission,
    Resource,
    Role,
)
from cynic.kernel.security.audit_log import AuditLogger, AuditAction, AuditResult


class MockRequest:
    """Mock FastAPI request object for testing."""

    def __init__(self, auth_header: str = "", host: str = "127.0.0.1"):
        self.headers = {"Authorization": auth_header} if auth_header else {}
        self.method = "POST"
        self.url = MagicMock()
        self.url.path = "/api/proposals"
        self.client = MagicMock()
        self.client.host = host
        self.state = MagicMock()
        self.state.correlation_id = "test-correlation-123"


@pytest.mark.asyncio
class TestRBACAuthorizer:
    """Test RBAC authorizer functionality."""

    @pytest.fixture
    async def access_controller(self):
        """Create access controller with test API key."""
        controller = AccessController()
        await controller.startup()

        # Generate test key with OPERATOR role
        key_id, key_secret = await controller.generate_api_key(Role.OPERATOR)

        yield controller, key_id, key_secret

        await controller.shutdown()

    @pytest.fixture
    async def authorizer(self, access_controller):
        """Create authorizer with test controller."""
        controller, key_id, key_secret = access_controller
        audit_logger = AuditLogger("test-authz")

        authorizer = RBACAuthorizer(
            access_controller=controller,
            audit_logger=audit_logger,
        )
        return authorizer, key_id, key_secret

    async def test_authorize_with_valid_key(self, authorizer):
        """Test authorization with valid API key."""
        authorizer, key_id, key_secret = authorizer

        request = MockRequest(f"Bearer {key_id}:{key_secret}")

        is_auth, role, error = await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_auth is True
        assert role == Role.OPERATOR
        assert error == ""

    async def test_deny_with_invalid_key_format(self, authorizer):
        """Test authorization with invalid key format."""
        authorizer, _, _ = authorizer

        request = MockRequest("Bearer invalid_format")

        is_auth, role, error = await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_auth is False
        assert role is None
        assert "Invalid API key format" in error

    async def test_deny_with_missing_auth_header(self, authorizer):
        """Test authorization with missing Authorization header."""
        authorizer, _, _ = authorizer

        request = MockRequest("")

        is_auth, role, error = await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_auth is False
        assert role is None
        assert "Missing or invalid" in error

    async def test_deny_with_invalid_bearer_token(self, authorizer):
        """Test authorization with non-Bearer token."""
        authorizer, _, _ = authorizer

        request = MockRequest("Basic some_token")

        is_auth, role, error = await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        assert is_auth is False

    async def test_deny_unauthorized_resource_access(self):
        """Test authorization denies access to unauthorized resource."""
        controller = AccessController()
        await controller.startup()

        # Viewer role has no access to VAULT_SECRETS
        key_id, key_secret = await controller.generate_api_key(Role.VIEWER)

        authorizer = RBACAuthorizer(access_controller=controller)
        request = MockRequest(f"Bearer {key_id}:{key_secret}")

        is_auth, role, error = await authorizer.authorize(
            request,
            Resource.VAULT_SECRETS,
            Permission.READ,
        )

        assert is_auth is False
        assert "does not have" in error

        await controller.shutdown()

    async def test_audit_logging_on_grant(self, authorizer):
        """Test that successful authorization is logged."""
        authorizer, key_id, key_secret = authorizer

        # Mock audit logger to verify it's called
        mock_audit = AsyncMock()
        authorizer.audit_logger.log = mock_audit

        request = MockRequest(f"Bearer {key_id}:{key_secret}")

        await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        # Verify audit log was called with GRANT action
        mock_audit.assert_called_once()
        call_kwargs = mock_audit.call_args[1]
        assert call_kwargs["action"] == AuditAction.AUTHZ_CHECK_GRANTED
        assert call_kwargs["result"] == AuditResult.SUCCESS

    async def test_audit_logging_on_denial(self, authorizer):
        """Test that failed authorization is logged."""
        authorizer, _, _ = authorizer

        # Mock audit logger
        mock_audit = AsyncMock()
        authorizer.audit_logger.log = mock_audit

        request = MockRequest("Bearer bad:credentials")

        await authorizer.authorize(
            request,
            Resource.GOVERNANCE,
            Permission.WRITE,
        )

        # Verify audit log was called with DENIAL action
        mock_audit.assert_called_once()
        call_kwargs = mock_audit.call_args[1]
        assert call_kwargs["action"] == AuditAction.AUTHZ_CHECK_DENIED
        assert call_kwargs["result"] == AuditResult.DENIED


@pytest.mark.asyncio
class TestAuthzDependency:
    """Test require_authz dependency function."""

    def test_require_authz_returns_callable(self):
        """Test that require_authz returns a callable dependency."""
        dep = require_authz(Resource.GOVERNANCE, Permission.WRITE)

        # Should be a Depends object
        assert hasattr(dep, "dependency")

    async def test_get_authorized_key_id_with_valid_auth(self):
        """Test extracting key ID from valid Authorization header."""
        request = MockRequest("Bearer test_key_id:test_secret")

        key_id = await get_authorized_key_id(request)

        assert key_id == "test_key_id"

    async def test_get_authorized_key_id_missing_auth(self):
        """Test key extraction fails with missing auth header."""
        request = MockRequest("")

        with pytest.raises(HTTPException) as exc_info:
            await get_authorized_key_id(request)

        assert exc_info.value.status_code == 401

    async def test_get_authorized_key_id_invalid_format(self):
        """Test key extraction fails with invalid format."""
        request = MockRequest("Bearer invalid")

        with pytest.raises(HTTPException) as exc_info:
            await get_authorized_key_id(request)

        assert exc_info.value.status_code == 401


@pytest.mark.asyncio
class TestGovernanceEndpointProtection:
    """Integration tests for protected governance endpoints."""

    @pytest.fixture
    def app_with_protected_routes(self):
        """Create FastAPI app with protected governance routes."""
        app = FastAPI()

        class ProposalRequest(BaseModel):
            title: str
            description: str

        @app.post("/api/governance/proposals")
        async def submit_proposal(
            req: ProposalRequest,
            authz: RBACAuthorizer = require_authz(
                Resource.GOVERNANCE, Permission.WRITE
            ),
        ):
            return {"status": "SUCCESS", "proposal_id": "prop_123"}

        @app.post("/api/governance/proposals/{proposal_id}/vote")
        async def cast_vote(
            proposal_id: str,
            authz: RBACAuthorizer = require_authz(
                Resource.GOVERNANCE, Permission.WRITE
            ),
        ):
            return {"status": "SUCCESS", "vote_id": "vote_123"}

        @app.post("/api/governance/proposals/{proposal_id}/outcome")
        async def record_outcome(
            proposal_id: str,
            authz: RBACAuthorizer = require_authz(
                Resource.GOVERNANCE, Permission.WRITE
            ),
        ):
            return {"status": "SUCCESS"}

        @app.post("/api/governance/votes")
        async def record_vote(
            authz: RBACAuthorizer = require_authz(
                Resource.GOVERNANCE, Permission.WRITE
            ),
        ):
            return {"status": "SUCCESS", "vote_id": "vote_456"}

        return app

    def test_post_proposals_without_auth(self, app_with_protected_routes):
        """Test POST /proposals returns 403 without authentication."""
        client = TestClient(app_with_protected_routes)

        response = client.post(
            "/api/governance/proposals",
            json={"title": "Test", "description": "Test proposal"},
        )

        assert response.status_code == 403
        assert "Missing or invalid" in response.json()["detail"]

    @pytest.mark.asyncio
    async def test_post_proposals_with_valid_auth(self, app_with_protected_routes):
        """Test POST /proposals succeeds with valid API key."""
        # Setup access controller with OPERATOR key
        controller = AccessController()
        await controller.startup()
        key_id, key_secret = await controller.generate_api_key(Role.OPERATOR)

        client = TestClient(app_with_protected_routes)

        # Patch the global authorizer to use our test controller
        with patch(
            "cynic.interfaces.api.middleware.authz.get_global_authorizer"
        ) as mock_get_auth:
            authorizer = RBACAuthorizer(access_controller=controller)
            mock_get_auth.return_value = authorizer

            response = client.post(
                "/api/governance/proposals",
                json={"title": "Test", "description": "Test proposal"},
                headers={"Authorization": f"Bearer {key_id}:{key_secret}"},
            )

        assert response.status_code == 200
        assert response.json()["proposal_id"] == "prop_123"

        await controller.shutdown()

    def test_post_vote_without_auth(self, app_with_protected_routes):
        """Test POST /vote returns 403 without authentication."""
        client = TestClient(app_with_protected_routes)

        response = client.post(
            "/api/governance/proposals/prop_123/vote",
            json={},
        )

        assert response.status_code == 403

    def test_post_outcome_without_auth(self, app_with_protected_routes):
        """Test POST /outcome returns 403 without authentication."""
        client = TestClient(app_with_protected_routes)

        response = client.post(
            "/api/governance/proposals/prop_123/outcome",
            json={"outcome": "APPROVED"},
        )

        assert response.status_code == 403

    def test_post_record_vote_without_auth(self, app_with_protected_routes):
        """Test POST /votes returns 403 without authentication."""
        client = TestClient(app_with_protected_routes)

        response = client.post("/api/governance/votes")

        assert response.status_code == 403


@pytest.mark.asyncio
class TestRBACConfig:
    """Test RBAC authorization configuration."""

    def test_config_defaults(self):
        """Test RBACAuthConfig uses sensible defaults."""
        config = RBACAuthConfig()

        assert config.require_auth is True
        assert config.default_role == Role.VIEWER
        assert config.audit_log_dir is None

    def test_config_custom_values(self):
        """Test RBACAuthConfig accepts custom values."""
        config = RBACAuthConfig(
            require_auth=False,
            audit_log_dir="/tmp/audit",
            default_role=Role.OPERATOR,
        )

        assert config.require_auth is False
        assert config.audit_log_dir == "/tmp/audit"
        assert config.default_role == Role.OPERATOR
