"""Role-Based Access Control (RBAC) authorization middleware for governance endpoints.

Protects critical governance endpoints with API key authentication and RBAC checks.
Integrates with AccessController (Task 1.5) and AuditLog (Task 1.3).

Protection strategy:
- API key extracted from Authorization header (Bearer <key_id>:<key_secret>)
- AccessController validates key and checks permissions
- AuditLog records all authorization decisions (granted/denied)
- Governance endpoints require OPERATOR or ADMIN role

Integration:
- Applied as dependency injection to protected route handlers
- Can be added to any endpoint: @app.post("/path", dependencies=[Depends(require_authz(...))])
- Logs to audit trail for compliance and forensics
"""

from __future__ import annotations

import base64
import logging
from typing import Any

from fastapi import Depends, HTTPException, Request

from cynic.kernel.security.rbac import AccessController, Permission, Resource, Role
from cynic.kernel.security.audit_log import AuditLogger, AuditAction, AuditResult

logger = logging.getLogger(__name__)


class RBACAuthConfig:
    """Configuration for RBAC authorization."""

    def __init__(
        self,
        require_auth: bool = True,
        audit_log_dir: str | None = None,
        default_role: Role = Role.VIEWER,
    ):
        """Initialize RBAC config.

        Args:
            require_auth: Whether authentication is required
            audit_log_dir: Directory for audit log files
            default_role: Default role if not specified
        """
        self.require_auth = require_auth
        self.audit_log_dir = audit_log_dir
        self.default_role = default_role


class RBACAuthorizer:
    """Validates API key authentication and RBAC authorization.

    Extracts API key from request, validates it with AccessController,
    and logs all authorization decisions to AuditLog.
    """

    def __init__(
        self,
        access_controller: AccessController | None = None,
        audit_logger: AuditLogger | None = None,
        config: RBACAuthConfig | None = None,
    ):
        """Initialize authorizer.

        Args:
            access_controller: AccessController instance (will create if None)
            audit_logger: AuditLogger instance (will create if None)
            config: RBACAuthConfig (will create if None)
        """
        self.access_controller = access_controller or AccessController()
        self.audit_logger = audit_logger or AuditLogger("api-authz")
        self.config = config or RBACAuthConfig()

    async def authorize(
        self,
        request: Request,
        resource: Resource,
        permission: Permission,
    ) -> tuple[bool, Role | None, str]:
        """Authorize request with RBAC.

        Args:
            request: FastAPI request object
            resource: Resource being accessed
            permission: Permission required

        Returns:
            (is_authorized, role, error_message)
        """
        # Extract API key from Authorization header
        # Format: Authorization: Bearer <key_id>:<key_secret>
        auth_header = request.headers.get("Authorization", "")
        if not auth_header.startswith("Bearer "):
            error = "Missing or invalid Authorization header"
            await self._log_denial(
                request,
                resource,
                error,
                actor="unknown",
            )
            logger.warning(f"RBAC: {error}")
            return False, None, error

        try:
            credentials = auth_header[7:]  # Remove "Bearer "
            key_id, key_secret = credentials.split(":", 1)
        except ValueError:
            error = "Invalid API key format. Expected key_id:key_secret"
            await self._log_denial(
                request,
                resource,
                error,
                actor="unknown",
            )
            logger.warning(f"RBAC: {error}")
            return False, None, error

        # Validate API key and check permissions using AccessController
        is_authorized, role, authz_error = await self.access_controller.validate_request(
            key_id,
            key_secret,
            resource,
            permission,
        )

        if is_authorized:
            await self._log_grant(request, resource, key_id, role)
            logger.debug(
                f"RBAC: Granted {role.value} access to {resource.value}:{permission.value}"
            )
        else:
            await self._log_denial(request, resource, authz_error, actor=key_id)
            logger.warning(
                f"RBAC: Denied access to {resource.value}:{permission.value} - {authz_error}"
            )

        return is_authorized, role, authz_error

    async def _log_grant(
        self,
        request: Request,
        resource: Resource,
        key_id: str,
        role: Role,
    ) -> None:
        """Log successful authorization."""
        await self.audit_logger.log(
            action=AuditAction.AUTHZ_CHECK_GRANTED,
            resource=f"{request.method} {request.url.path}",
            result=AuditResult.SUCCESS,
            actor=key_id,
            metadata={
                "role": role.value,
                "resource": resource.value,
                "remote_addr": request.client.host if request.client else "unknown",
                "correlation_id": request.state.correlation_id
                if hasattr(request.state, "correlation_id")
                else "unknown",
            },
        )

    async def _log_denial(
        self,
        request: Request,
        resource: Resource,
        reason: str,
        actor: str = "unknown",
    ) -> None:
        """Log failed authorization."""
        await self.audit_logger.log(
            action=AuditAction.AUTHZ_CHECK_DENIED,
            resource=f"{request.method} {request.url.path}",
            result=AuditResult.DENIED,
            actor=actor,
            metadata={
                "resource": resource.value,
                "reason": reason,
                "remote_addr": request.client.host if request.client else "unknown",
                "correlation_id": request.state.correlation_id
                if hasattr(request.state, "correlation_id")
                else "unknown",
            },
        )


# Global authorizer instance (created once per process)
_global_authorizer: RBACAuthorizer | None = None


async def get_global_authorizer(config: RBACAuthConfig | None = None) -> RBACAuthorizer:
    """Get or create the global authorizer instance.

    Uses singleton pattern to avoid creating multiple AccessController/AuditLogger instances.
    """
    global _global_authorizer
    if _global_authorizer is None:
        _global_authorizer = RBACAuthorizer(config=config)
    return _global_authorizer


def require_authz(
    resource: Resource,
    permission: Permission = Permission.WRITE,
) -> Any:
    """Create a dependency that enforces RBAC authorization.

    Use in route handlers:
        @app.post("/api/proposals")
        async def submit_proposal(
            req: ProposalRequest,
            authz: RBACAuthorizer = Depends(require_authz(Resource.GOVERNANCE, Permission.WRITE)),
        ):
            ...

    Args:
        resource: Resource being protected
        permission: Permission required (defaults to WRITE)

    Returns:
        Async dependency function
    """

    async def authz_dependency(request: Request) -> RBACAuthorizer:
        """Dependency that checks authorization and returns authorizer."""
        authorizer = await get_global_authorizer()

        # Check authorization
        is_authorized, role, error = await authorizer.authorize(
            request,
            resource,
            permission,
        )

        if not is_authorized:
            raise HTTPException(
                status_code=403,
                detail=error,
            )

        # Return authorizer so route handler can access authenticated user info
        return authorizer

    return Depends(authz_dependency)


async def get_authorized_key_id(request: Request) -> str:
    """Extract authenticated API key ID from request.

    Use in route handlers to access the authenticated key ID:
        @app.get("/protected")
        async def protected_endpoint(key_id: str = Depends(get_authorized_key_id)):
            ...

    Requires authorization middleware to have already validated the key.
    """
    auth_header = request.headers.get("Authorization", "")
    if not auth_header.startswith("Bearer "):
        raise HTTPException(status_code=401, detail="Unauthorized")

    try:
        credentials = auth_header[7:]
        key_id, _ = credentials.split(":", 1)
        return key_id
    except ValueError:
        raise HTTPException(status_code=401, detail="Invalid credentials")
