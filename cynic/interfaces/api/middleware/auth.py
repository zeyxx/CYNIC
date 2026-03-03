"""API gateway authentication middleware for mTLS verification.

Integrates mTLS certificate verification into the FastAPI application.
Provides dependency injection for checking client identity in route handlers.

Integration:
- Lifespan event: Initialize PKI and load certificates
- ASGI middleware: Verify client cert on every request
- Route dependencies: Extract client identity for authorization (Task 1.5)
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import Any

from fastapi import Depends, HTTPException, Request
from starlette.middleware.base import BaseHTTPMiddleware

from cynic.kernel.security.mtls import MTLSConfig, MTLSSSLContext, MTLSVerifier
from cynic.kernel.security.pki import PKI, PKIConfig

logger = logging.getLogger(__name__)


class APIAuthConfig:
    """Configuration for API authentication."""

    def __init__(
        self,
        cert_dir: str | Path = "./certs",
        require_mtls: bool = True,
        service_name: str = "api",
    ):
        """Initialize API auth config.

        Args:
            cert_dir: Directory for certificates
            require_mtls: Whether to require mTLS
            service_name: This service's name (for cert loading)
        """
        self.cert_dir = Path(cert_dir)
        self.require_mtls = require_mtls
        self.service_name = service_name
        self.pki_config = PKIConfig(cert_dir=cert_dir)
        self.mtls_config = MTLSConfig(
            require_client_cert=require_mtls,
            ca_cert_path=self.pki_config.intermediate_ca_cert_path(),
        )


class APIAuthMiddleware(BaseHTTPMiddleware):
    """Middleware for API authentication via mTLS.

    On each request:
    1. Extract client certificate from ASGI scope (if using mTLS socket)
    2. Verify certificate is signed by trusted CA
    3. Extract client service identity from cert CN
    4. Add to request scope for route handlers
    """

    def __init__(self, app: Any, config: APIAuthConfig):
        super().__init__(app)
        self.config = config
        self.verifier = MTLSVerifier(config.mtls_config)

    async def dispatch(self, request: Request, call_next: Any) -> Any:
        """Process request with mTLS verification."""
        # Extract client cert from TLS connection
        # In production, this comes from the SSL socket (via ASGI server)
        # For now, we check for client cert in request scope
        client_cert = None

        if hasattr(request, "scope"):
            # Try to get cert from ASGI scope (set by uvicorn/hypercorn with ssl)
            client_cert = request.scope.get("client_cert")

            # For testing/development, check for cert in header
            if not client_cert:
                cert_header = request.headers.get("X-Client-Cert")
                if cert_header:
                    import base64

                    try:
                        client_cert = base64.b64decode(cert_header)
                    except Exception as e:
                        logger.warning(f"Failed to decode X-Client-Cert header: {e}")

        # Verify client certificate if mTLS is required
        if self.config.require_mtls:
            if not client_cert:
                logger.warning(
                    f"{request.method} {request.url.path} - No client certificate"
                )
                raise HTTPException(status_code=401, detail="Client certificate required")

            # Verify the cert (would check against CA in production)
            is_valid, error = self.verifier.verify_certificate(
                client_cert,
                self.verifier.load_ca_cert(self.config.mtls_config.ca_cert_path)
                if self.config.mtls_config.ca_cert_path
                else None,
            )
            if not is_valid:
                logger.warning(
                    f"{request.method} {request.url.path} - "
                    f"Invalid client certificate: {error}"
                )
                raise HTTPException(status_code=401, detail=f"Invalid certificate: {error}")

        # Extract client identity and add to scope
        if client_cert:
            try:
                from cryptography import x509

                from cryptography.hazmat.backends import default_backend

                cert = x509.load_der_x509_certificate(client_cert, default_backend())
                client_identity = self.verifier.extract_client_identity(cert)
                request.scope["client_identity"] = client_identity
                request.scope["client_cert_info"] = self.verifier.get_cert_info(cert)

                logger.debug(
                    f"{request.method} {request.url.path} - "
                    f"Client: {client_identity}"
                )
            except Exception as e:
                logger.error(f"Failed to parse client certificate: {e}")
                if self.config.require_mtls:
                    raise HTTPException(status_code=401, detail="Invalid certificate format")

        # Call the next middleware/route
        response = await call_next(request)
        return response


# ==================== Dependency Injection ====================


async def get_client_identity(request: Request) -> str:
    """Extract authenticated client identity from request.

    Use in route handlers:
        @app.get("/")
        async def my_route(client_id: str = Depends(get_client_identity)):
            ...
    """
    client_id = request.scope.get("client_identity")
    if not client_id:
        raise HTTPException(status_code=401, detail="Client identity not found")
    return client_id


async def get_client_cert_info(request: Request) -> dict[str, Any]:
    """Extract client certificate metadata from request.

    Use in route handlers:
        @app.get("/")
        async def my_route(cert_info: dict = Depends(get_client_cert_info)):
            ...
    """
    cert_info = request.scope.get("client_cert_info")
    if not cert_info:
        raise HTTPException(status_code=401, detail="Certificate info not found")
    return cert_info


# ==================== Lifecycle Management ====================


class APIAuthManager:
    """Manages API authentication lifecycle (startup/shutdown)."""

    def __init__(self, config: APIAuthConfig | None = None):
        self.config = config or APIAuthConfig()
        self.pki: PKI | None = None
        self.ssl_context: MTLSSSLContext | None = None

    async def startup(self) -> None:
        """Initialize PKI and generate certificates if needed."""
        logger.info("Starting API authentication manager...")

        # Initialize PKI
        self.pki = PKI(self.config.pki_config)

        # Setup certificate chain (idempotent)
        try:
            root_cert, root_key = self.pki.setup_root_ca()
            logger.info(f"✓ Root CA: {self.config.pki_config.root_ca_cert_path()}")

            int_cert, int_key = self.pki.setup_intermediate_ca()
            logger.info(f"✓ Intermediate CA: {self.config.pki_config.intermediate_ca_cert_path()}")

            # Generate service certificate for this API instance
            svc_cert, svc_key = self.pki.generate_service_cert(
                self.config.service_name,
                san_names=[
                    "api",
                    "api.local",
                    "api.cynic.local",
                    "127.0.0.1",
                ],
            )
            logger.info(
                f"✓ Service cert ({self.config.service_name}): "
                f"{self.config.pki_config.service_cert_path(self.config.service_name)}"
            )

            # Create SSL context for server
            self.ssl_context = MTLSSSLContext(self.config.mtls_config)
            logger.info("✓ SSL context configured")

            logger.info("API authentication manager started")
        except Exception as e:
            logger.error(f"Failed to initialize API authentication: {e}")
            raise

    async def shutdown(self) -> None:
        """Clean up resources."""
        logger.info("Stopping API authentication manager...")
        # Cleanup if needed
        logger.info("API authentication manager stopped")

    def get_ssl_context(self) -> MTLSSSLContext | None:
        """Get SSL context for HTTPS server."""
        return self.ssl_context
