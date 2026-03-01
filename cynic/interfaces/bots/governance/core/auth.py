"""
Authentication and Authorization Management

Provides:
- JWT token creation and verification
- Role-based access control
- Token expiry and validation
- User authentication flow
"""

import jwt
from datetime import datetime, timedelta
from typing import Dict, Optional
from . import config


class AuthManager:
    """Manage JWT authentication and token lifecycle"""

    def __init__(self, config: Config):
        """
        Initialize AuthManager with configuration.

        Args:
            config: Configuration object containing auth settings
        """
        self.config = config
        self.secret = config.auth.jwt_secret
        self.algorithm = config.auth.jwt_algorithm
        self.expiry_hours = config.auth.jwt_expiry_hours

    def create_token(self, user_id: str, roles: list = None) -> str:
        """
        Create JWT token for user.

        Args:
            user_id: Unique user identifier
            roles: List of user roles

        Returns:
            Encoded JWT token string

        Raises:
            ValueError: If JWT secret is not configured
        """
        if not self.secret:
            raise ValueError("JWT secret not configured")

        payload = {
            "user_id": user_id,
            "roles": roles or [],
            "exp": datetime.utcnow() + timedelta(hours=self.expiry_hours),
            "iat": datetime.utcnow()
        }

        return jwt.encode(payload, self.secret, algorithm=self.algorithm)

    def verify_token(self, token: str) -> Optional[Dict]:
        """
        Verify and decode JWT token.

        Args:
            token: JWT token to verify

        Returns:
            Decoded token payload or None if invalid
        """
        try:
            payload = jwt.decode(token, self.secret, algorithms=[self.algorithm])
            return payload
        except jwt.InvalidTokenError:
            return None

    def has_role(self, token: str, required_role: str) -> bool:
        """
        Check if token contains required role.

        Args:
            token: JWT token to check
            required_role: Required role name

        Returns:
            True if token contains role, False otherwise
        """
        payload = self.verify_token(token)
        if not payload:
            return False

        return required_role in payload.get("roles", [])

    def get_user_from_token(self, token: str) -> Optional[str]:
        """
        Extract user ID from token.

        Args:
            token: JWT token

        Returns:
            User ID or None if token invalid
        """
        payload = self.verify_token(token)
        if not payload:
            return None

        return payload.get("user_id")

    def is_token_expired(self, token: str) -> bool:
        """
        Check if token is expired.

        Args:
            token: JWT token to check

        Returns:
            True if expired, False if valid
        """
        payload = self.verify_token(token)
        return payload is None
