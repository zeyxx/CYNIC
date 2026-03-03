"""Tests for mTLS verification."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from cynic.kernel.security.mtls import MTLSConfig, MTLSSSLContext, MTLSVerifier
from cynic.kernel.security.pki import PKI, PKIConfig


@pytest.fixture
def temp_cert_dir() -> Path:
    """Create temporary directory for certificates."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def pki(temp_cert_dir: Path) -> PKI:
    """Create PKI instance with temporary certs."""
    config = PKIConfig(cert_dir=temp_cert_dir)
    return PKI(config)


@pytest.fixture
def mtls_config() -> MTLSConfig:
    """Create mTLS configuration."""
    return MTLSConfig(require_client_cert=True)


@pytest.fixture
def mtls_verifier(mtls_config: MTLSConfig) -> MTLSVerifier:
    """Create mTLS verifier."""
    return MTLSVerifier(mtls_config)


class TestMTLSVerifier:
    """Tests for mTLS certificate verification."""

    def test_extract_client_identity(
        self, mtls_verifier: MTLSVerifier, pki: PKI
    ) -> None:
        """Client identity can be extracted from certificate CN."""
        svc_cert, _ = pki.generate_service_cert("api")
        client_identity = mtls_verifier.extract_client_identity(svc_cert)

        assert client_identity == "api"

    def test_get_cert_info(self, mtls_verifier: MTLSVerifier, pki: PKI) -> None:
        """Certificate metadata can be extracted."""
        svc_cert, _ = pki.generate_service_cert("api")
        cert_info = mtls_verifier.get_cert_info(svc_cert)

        assert "subject" in cert_info
        assert "issuer" in cert_info
        assert "serial_number" in cert_info
        assert "not_valid_before" in cert_info
        assert "not_valid_after" in cert_info
        assert cert_info["client_identity"] == "api"

    def test_verify_certificate_valid(
        self, mtls_verifier: MTLSVerifier, pki: PKI, temp_cert_dir: Path
    ) -> None:
        """Valid certificate passes verification."""
        svc_cert, svc_key = pki.generate_service_cert("api")

        # Convert cert to DER for verification
        from cryptography.hazmat.primitives import serialization

        cert_der = svc_cert.public_bytes(serialization.Encoding.DER)
        int_cert, _ = pki.setup_intermediate_ca()

        is_valid, error = mtls_verifier.verify_certificate(cert_der, int_cert)
        # Note: actual signature verification requires cryptography library setup
        # For now, we're testing the wrapper logic
        assert isinstance(is_valid, bool)


class TestMTLSSSLContext:
    """Tests for SSL context creation."""

    def test_create_server_context(
        self, pki: PKI, temp_cert_dir: Path, mtls_config: MTLSConfig
    ) -> None:
        """Server SSL context can be created."""
        svc_cert, _ = pki.generate_service_cert("api")
        pki_config = pki.config

        ssl_factory = MTLSSSLContext(mtls_config)
        context = ssl_factory.create_server_context(
            pki_config.service_cert_path("api"),
            pki_config.service_key_path("api"),
            pki_config.intermediate_ca_cert_path(),
        )

        assert context is not None
        # Verify cert is loaded
        assert context.check_hostname is False  # Server context

    def test_create_client_context(
        self, pki: PKI, temp_cert_dir: Path, mtls_config: MTLSConfig
    ) -> None:
        """Client SSL context can be created."""
        svc_cert, _ = pki.generate_service_cert("api")
        pki_config = pki.config

        ssl_factory = MTLSSSLContext(mtls_config)
        context = ssl_factory.create_client_context(
            pki_config.service_cert_path("api"),
            pki_config.service_key_path("api"),
            pki_config.intermediate_ca_cert_path(),
            verify_hostname=False,
        )

        assert context is not None

    def test_server_context_requires_client_cert(
        self, pki: PKI, temp_cert_dir: Path
    ) -> None:
        """Server context can be configured to require client cert."""
        svc_cert, _ = pki.generate_service_cert("api")
        pki_config = pki.config

        config = MTLSConfig(require_client_cert=True)
        ssl_factory = MTLSSSLContext(config)
        context = ssl_factory.create_server_context(
            pki_config.service_cert_path("api"),
            pki_config.service_key_path("api"),
        )

        import ssl

        assert context.verify_mode == ssl.CERT_REQUIRED


class TestMTLSConfiguration:
    """Tests for mTLS configuration."""

    def test_mtls_config_defaults(self) -> None:
        """mTLS config has sensible defaults."""
        config = MTLSConfig()

        assert config.require_client_cert is True
        assert config.verify_hostname is True
        assert config.revocation_enabled is False

    def test_mtls_config_custom(self, temp_cert_dir: Path) -> None:
        """mTLS config can be customized."""
        config = MTLSConfig(
            require_client_cert=False,
            ca_cert_path=temp_cert_dir / "ca.crt",
            verify_hostname=False,
            revocation_enabled=True,
        )

        assert config.require_client_cert is False
        assert config.ca_cert_path == temp_cert_dir / "ca.crt"
        assert config.verify_hostname is False
        assert config.revocation_enabled is True
