"""
Role-Based Access Control (RBAC)

Provides:
- User role definitions
- Permission management per role
- Role hierarchy
- Permission checking utilities
"""

from enum import Enum


class UserRole(str, Enum):
    """User roles in governance system"""

    ADMIN = "admin"
    MODERATOR = "moderator"
    MEMBER = "member"
    GUEST = "guest"


class RoleManager:
    """Manage user roles and their permissions"""

    # Permission matrix: role -> list of allowed actions
    PERMISSIONS = {
        UserRole.ADMIN: [
            "create_proposal",
            "modify_proposal",
            "delete_proposal",
            "manage_roles",
            "manage_treasury",
            "execute_decision",
            "view_treasury",
            "vote",
            "view_proposals",
            "moderate_comments",
            "view_analytics",
        ],
        UserRole.MODERATOR: [
            "create_proposal",
            "modify_proposal",
            "manage_members",
            "approve_proposals",
            "view_treasury",
            "vote",
            "view_proposals",
            "moderate_comments",
            "view_analytics",
        ],
        UserRole.MEMBER: [
            "create_proposal",
            "vote",
            "view_treasury",
            "view_proposals",
            "comment",
        ],
        UserRole.GUEST: [
            "view_proposals",
            "view_treasury",
        ],
    }

    @classmethod
    def has_permission(cls, role: UserRole, action: str) -> bool:
        """
        Check if role has permission for action.

        Args:
            role: User role to check
            action: Action to verify

        Returns:
            True if role has permission, False otherwise
        """
        return action in cls.PERMISSIONS.get(role, [])

    @classmethod
    def get_permissions(cls, role: UserRole) -> list[str]:
        """
        Get all permissions for a role.

        Args:
            role: User role

        Returns:
            List of allowed actions for role
        """
        return cls.PERMISSIONS.get(role, [])

    @classmethod
    def get_user_roles(cls, user_id: str, guild_id: int) -> list[UserRole]:
        """
        Get user roles from Discord guild.

        This is a placeholder for Discord integration.
        In production, this would fetch from Discord API.

        Args:
            user_id: Discord user ID
            guild_id: Discord guild (server) ID

        Returns:
            List of user roles in guild
        """
        # Placeholder: would fetch from Discord in real implementation
        return [UserRole.MEMBER]

    @classmethod
    def has_any_permission(cls, role: UserRole, actions: list[str]) -> bool:
        """
        Check if role has any of the listed permissions.

        Args:
            role: User role
            actions: List of actions to check

        Returns:
            True if role has any permission, False otherwise
        """
        role_perms = set(cls.get_permissions(role))
        return bool(role_perms.intersection(set(actions)))

    @classmethod
    def has_all_permissions(cls, role: UserRole, actions: list[str]) -> bool:
        """
        Check if role has all listed permissions.

        Args:
            role: User role
            actions: List of required actions

        Returns:
            True if role has all permissions, False otherwise
        """
        role_perms = set(cls.get_permissions(role))
        required = set(actions)
        return required.issubset(role_perms)

    @classmethod
    def get_role_hierarchy_level(cls, role: UserRole) -> int:
        """
        Get hierarchy level of role (higher = more permissions).

        Args:
            role: User role

        Returns:
            Hierarchy level (0-3, higher is more privileged)
        """
        hierarchy = {
            UserRole.GUEST: 0,
            UserRole.MEMBER: 1,
            UserRole.MODERATOR: 2,
            UserRole.ADMIN: 3,
        }
        return hierarchy.get(role, 0)

    @classmethod
    def is_admin(cls, role: UserRole) -> bool:
        """Check if role is admin."""
        return role == UserRole.ADMIN

    @classmethod
    def is_moderator(cls, role: UserRole) -> bool:
        """Check if role is moderator or higher."""
        level = cls.get_role_hierarchy_level(role)
        return level >= cls.get_role_hierarchy_level(UserRole.MODERATOR)

    @classmethod
    def is_member(cls, role: UserRole) -> bool:
        """Check if role is member or higher."""
        level = cls.get_role_hierarchy_level(role)
        return level >= cls.get_role_hierarchy_level(UserRole.MEMBER)
