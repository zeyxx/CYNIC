"""
Tests for authentication and authorization systems

Tests:
- JWT token creation and verification
- Role-based access control
- Permission checking
- Role hierarchy
"""

import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture: module imports not available in V5"
)

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)


import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture removed in V5 - governance_bot module not found"
)

from governance_bot.auth import AuthManager
from governance_bot.config import Config
from governance_bot.roles import RoleManager, UserRole


class TestAuthenticationSetup:
    """Test authentication setup and configuration"""

    @pytest.fixture
    def auth_config(self, monkeypatch):
        """Create test config with JWT settings"""
        monkeypatch.setenv("AUTH_JWT_SECRET", "test_secret_key_12345")
        monkeypatch.setenv("AUTH_JWT_ALGORITHM", "HS256")
        monkeypatch.setenv("AUTH_JWT_EXPIRY_HOURS", "24")
        monkeypatch.setenv("AUTH_ENABLE_AUTH", "true")
        return Config()

    def test_auth_config_loaded(self, auth_config):
        """Test that auth config is properly loaded"""
        assert auth_config.auth is not None
        assert auth_config.auth.jwt_secret == "test_secret_key_12345"
        assert auth_config.auth.jwt_algorithm == "HS256"
        assert auth_config.auth.jwt_expiry_hours == 24

    def test_auth_defaults(self):
        """Test default auth configuration"""
        config = Config()
        assert config.auth.enable_auth is False
        assert config.auth.jwt_algorithm == "HS256"
        assert config.auth.jwt_expiry_hours == 24


class TestJWTTokens:
    """Test JWT token creation and verification"""

    @pytest.fixture
    def auth_manager(self, monkeypatch):
        """Create AuthManager with test config"""
        monkeypatch.setenv("AUTH_JWT_SECRET", "test_secret_key_12345")
        monkeypatch.setenv("AUTH_JWT_ALGORITHM", "HS256")
        monkeypatch.setenv("AUTH_JWT_EXPIRY_HOURS", "24")
        config = Config()
        return AuthManager(config)

    def test_create_jwt_token(self, auth_manager):
        """Test JWT token creation"""
        token = auth_manager.create_token("user_123", roles=["member"])

        assert token is not None
        assert isinstance(token, str)
        assert len(token) > 0
        assert "." in token  # JWT format has dots

    def test_verify_valid_token(self, auth_manager):
        """Test verifying valid JWT token"""
        token = auth_manager.create_token("user_123", roles=["member"])
        payload = auth_manager.verify_token(token)

        assert payload is not None
        assert payload["user_id"] == "user_123"
        assert "member" in payload["roles"]

    def test_verify_invalid_token(self, auth_manager):
        """Test verifying invalid JWT token"""
        payload = auth_manager.verify_token("invalid.token.here")

        assert payload is None

    def test_token_with_multiple_roles(self, auth_manager):
        """Test token with multiple roles"""
        roles = ["admin", "moderator"]
        token = auth_manager.create_token("user_456", roles=roles)
        payload = auth_manager.verify_token(token)

        assert payload is not None
        assert set(payload["roles"]) == set(roles)

    def test_token_has_expiry(self, auth_manager):
        """Test that token includes expiry"""
        token = auth_manager.create_token("user_123")
        payload = auth_manager.verify_token(token)

        assert "exp" in payload
        assert "iat" in payload
        assert payload["exp"] > payload["iat"]

    def test_missing_secret_raises_error(self, monkeypatch):
        """Test that missing secret raises error"""
        monkeypatch.setenv("AUTH_JWT_SECRET", "")
        config = Config()
        auth_manager = AuthManager(config)

        with pytest.raises(ValueError, match="JWT secret not configured"):
            auth_manager.create_token("user_123")

    def test_get_user_from_token(self, auth_manager):
        """Test extracting user ID from token"""
        token = auth_manager.create_token("user_789")
        user_id = auth_manager.get_user_from_token(token)

        assert user_id == "user_789"

    def test_get_user_from_invalid_token(self, auth_manager):
        """Test extracting user from invalid token"""
        user_id = auth_manager.get_user_from_token("invalid.token")

        assert user_id is None


class TestRolePermissions:
    """Test role-based permission system"""

    def test_admin_permissions(self):
        """Test admin role has all critical permissions"""
        assert RoleManager.has_permission(UserRole.ADMIN, "create_proposal")
        assert RoleManager.has_permission(UserRole.ADMIN, "manage_roles")
        assert RoleManager.has_permission(UserRole.ADMIN, "manage_treasury")
        assert RoleManager.has_permission(UserRole.ADMIN, "delete_proposal")

    def test_member_permissions(self):
        """Test member role has appropriate permissions"""
        assert RoleManager.has_permission(UserRole.MEMBER, "create_proposal")
        assert RoleManager.has_permission(UserRole.MEMBER, "vote")
        assert not RoleManager.has_permission(UserRole.MEMBER, "manage_roles")

    def test_guest_permissions(self):
        """Test guest role is limited"""
        assert RoleManager.has_permission(UserRole.GUEST, "view_proposals")
        assert RoleManager.has_permission(UserRole.GUEST, "view_treasury")
        assert not RoleManager.has_permission(UserRole.GUEST, "create_proposal")
        assert not RoleManager.has_permission(UserRole.GUEST, "vote")

    def test_moderator_permissions(self):
        """Test moderator role"""
        assert RoleManager.has_permission(UserRole.MODERATOR, "create_proposal")
        assert RoleManager.has_permission(UserRole.MODERATOR, "approve_proposals")
        assert not RoleManager.has_permission(UserRole.MODERATOR, "manage_roles")

    def test_get_all_permissions(self):
        """Test getting all permissions for a role"""
        admin_perms = set(RoleManager.get_permissions(UserRole.ADMIN))
        member_perms = set(RoleManager.get_permissions(UserRole.MEMBER))

        # Admin should have more permissions than member
        assert len(admin_perms) > len(member_perms)

    def test_has_any_permission(self):
        """Test checking for any permission"""
        actions = ["create_proposal", "delete_proposal"]
        assert RoleManager.has_any_permission(UserRole.MEMBER, actions)
        assert RoleManager.has_any_permission(UserRole.GUEST, ["view_proposals"])
        assert not RoleManager.has_any_permission(
            UserRole.GUEST, ["create_proposal", "vote"]
        )

    def test_has_all_permissions(self):
        """Test checking for all permissions"""
        actions = ["vote", "create_proposal"]
        assert RoleManager.has_all_permissions(UserRole.MEMBER, actions)
        assert not RoleManager.has_all_permissions(UserRole.GUEST, actions)


class TestRoleHierarchy:
    """Test role hierarchy and levels"""

    def test_role_hierarchy_levels(self):
        """Test role hierarchy ordering"""
        guest_level = RoleManager.get_role_hierarchy_level(UserRole.GUEST)
        member_level = RoleManager.get_role_hierarchy_level(UserRole.MEMBER)
        moderator_level = RoleManager.get_role_hierarchy_level(UserRole.MODERATOR)
        admin_level = RoleManager.get_role_hierarchy_level(UserRole.ADMIN)

        assert guest_level < member_level < moderator_level < admin_level

    def test_is_admin(self):
        """Test admin role check"""
        assert RoleManager.is_admin(UserRole.ADMIN)
        assert not RoleManager.is_admin(UserRole.MEMBER)

    def test_is_moderator(self):
        """Test moderator or higher check"""
        assert RoleManager.is_moderator(UserRole.MODERATOR)
        assert RoleManager.is_moderator(UserRole.ADMIN)
        assert not RoleManager.is_moderator(UserRole.MEMBER)

    def test_is_member(self):
        """Test member or higher check"""
        assert RoleManager.is_member(UserRole.MEMBER)
        assert RoleManager.is_member(UserRole.MODERATOR)
        assert RoleManager.is_member(UserRole.ADMIN)
        assert not RoleManager.is_member(UserRole.GUEST)


class TestAuthIntegration:
    """Test authentication manager with roles"""

    @pytest.fixture
    def auth_manager(self, monkeypatch):
        """Create AuthManager"""
        monkeypatch.setenv("AUTH_JWT_SECRET", "test_secret_key_12345")
        config = Config()
        return AuthManager(config)

    def test_token_role_checking(self, auth_manager):
        """Test role checking with tokens"""
        token = auth_manager.create_token("user_123", roles=["member"])

        assert auth_manager.has_role(token, "member")
        assert not auth_manager.has_role(token, "admin")

    def test_multiple_roles_in_token(self, auth_manager):
        """Test multiple roles in single token"""
        roles = ["member", "moderator"]
        token = auth_manager.create_token("user_123", roles=roles)

        assert auth_manager.has_role(token, "member")
        assert auth_manager.has_role(token, "moderator")
        assert not auth_manager.has_role(token, "admin")

    def test_token_expiry_flag(self, auth_manager):
        """Test token expiry check"""
        token = auth_manager.create_token("user_123")
        assert not auth_manager.is_token_expired(token)

        invalid_token = "expired.invalid.token"
        assert auth_manager.is_token_expired(invalid_token)
