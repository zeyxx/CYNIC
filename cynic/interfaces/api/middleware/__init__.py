"""API middleware for authentication, authorization, and request/response handling."""

from cynic.interfaces.api.middleware.auth import (
    APIAuthConfig,
    APIAuthManager,
    APIAuthMiddleware,
    get_client_cert_info,
    get_client_identity,
)
from cynic.interfaces.api.middleware.authz import (
    RBACAuthConfig,
    RBACAuthorizer,
    get_authorized_key_id,
    get_global_authorizer,
    require_authz,
)

__all__ = [
    "APIAuthConfig",
    "APIAuthManager",
    "APIAuthMiddleware",
    "get_client_identity",
    "get_client_cert_info",
    "RBACAuthConfig",
    "RBACAuthorizer",
    "require_authz",
    "get_authorized_key_id",
    "get_global_authorizer",
]
