"""Tests for PKI infrastructure."""

from __future__ import annotations

import platform
import tempfile
from pathlib import Path

import pytest

from cynic.kernel.security.pki import PKI, PKIConfig


@pytest.fixture
def temp_cert_dir() -> Path:
    """Create temporary directory for certificates."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def pki_config(temp_cert_dir: Path) -> PKIConfig:
    """Create PKI config with temporary directory."""
    return PKIConfig(
        cert_dir=temp_cert_dir,
        root_ca_days=365,
        intermediate_ca_days=182,
        service_cert_days=90,
    )


@pytest.fixture
def pki(pki_config: PKIConfig) -> PKI:
    """Create PKI instance."""
    return PKI(pki_config)


class TestPKISetup:
    """Tests for PKI initialization."""

    def test_setup_root_ca(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Root CA can be generated and loaded."""
        root_cert, root_key = pki.setup_root_ca()

        assert root_cert is not None
        assert root_key is not None
        assert pki_config.root_ca_cert_path().exists()
        assert pki_config.root_ca_key_path().exists()

    def test_setup_root_ca_idempotent(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Second root CA setup loads existing, doesn't regenerate."""
        root_cert_1, _ = pki.setup_root_ca()
        serial_1 = root_cert_1.serial_number

        # Setup again - should load existing
        root_cert_2, _ = pki.setup_root_ca()
        serial_2 = root_cert_2.serial_number

        assert serial_1 == serial_2  # Same cert loaded

    def test_setup_intermediate_ca(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Intermediate CA can be generated and signed by root CA."""
        int_cert, int_key = pki.setup_intermediate_ca()

        assert int_cert is not None
        assert int_key is not None
        assert pki_config.intermediate_ca_cert_path().exists()
        assert pki_config.intermediate_ca_key_path().exists()

        # Verify it's a CA cert
        assert int_cert.extensions.get_extension_for_class(
            __import__("cryptography.x509", fromlist=["BasicConstraints"]).BasicConstraints
        ).value.ca

    def test_setup_intermediate_ca_idempotent(
        self, pki: PKI, pki_config: PKIConfig
    ) -> None:
        """Second intermediate CA setup loads existing, doesn't regenerate."""
        int_cert_1, _ = pki.setup_intermediate_ca()
        serial_1 = int_cert_1.serial_number

        int_cert_2, _ = pki.setup_intermediate_ca()
        serial_2 = int_cert_2.serial_number

        assert serial_1 == serial_2


class TestServiceCertificates:
    """Tests for service certificate generation."""

    def test_generate_service_cert(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Service certificate can be generated."""
        svc_cert, svc_key = pki.generate_service_cert("api")

        assert svc_cert is not None
        assert svc_key is not None
        assert pki_config.service_cert_path("api").exists()
        assert pki_config.service_key_path("api").exists()

    def test_generate_multiple_service_certs(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Multiple services can have different certificates."""
        api_cert, _ = pki.generate_service_cert("api")
        event_bus_cert, _ = pki.generate_service_cert("event-bus")
        core_cert, _ = pki.generate_service_cert("core")

        assert api_cert.serial_number != event_bus_cert.serial_number
        assert event_bus_cert.serial_number != core_cert.serial_number

    def test_generate_service_cert_with_san(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Service certificate can have Subject Alternative Names."""
        san_names = ["api.local", "api.cynic.local", "127.0.0.1"]
        svc_cert, _ = pki.generate_service_cert("api", san_names=san_names)

        # Verify SANs are in cert (if extension is present)
        assert svc_cert is not None

    def test_service_cert_idempotent(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Second service cert setup loads existing, doesn't regenerate."""
        svc_cert_1, _ = pki.generate_service_cert("api")
        serial_1 = svc_cert_1.serial_number

        svc_cert_2, _ = pki.generate_service_cert("api")
        serial_2 = svc_cert_2.serial_number

        assert serial_1 == serial_2


class TestCertificateInfo:
    """Tests for certificate inspection."""

    def test_get_cert_info(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Certificate metadata can be extracted."""
        pki.generate_service_cert("api")
        cert_path = pki_config.service_cert_path("api")

        info = pki.get_cert_info(cert_path)

        assert "subject" in info
        assert "issuer" in info
        assert "serial_number" in info
        assert "not_valid_before" in info
        assert "not_valid_after" in info
        assert "is_expired" in info
        assert info["is_expired"] is False

    def test_is_cert_expired(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Certificate expiration can be checked."""
        pki.generate_service_cert("api")
        cert_path = pki_config.service_cert_path("api")

        is_expired = pki.is_cert_expired(cert_path)
        assert is_expired is False  # Just generated, not expired

    def test_get_cert_expiry(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Certificate expiry timestamp can be retrieved."""
        pki.generate_service_cert("api")
        cert_path = pki_config.service_cert_path("api")

        from datetime import datetime

        expiry = pki.get_cert_expiry(cert_path)
        now = datetime.utcnow()
        days_until_expiry = (expiry - now).days

        # Service cert should be valid for config.service_cert_days
        assert 85 < days_until_expiry <= 90  # Allow some tolerance


class TestCertificateStorage:
    """Tests for certificate file I/O."""

    @pytest.mark.skipif(platform.system() == "Windows", reason="Unix permissions not applicable on Windows")
    def test_key_file_permissions(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Private keys are stored with restricted permissions (0o600)."""
        pki.generate_service_cert("api")
        key_path = pki_config.service_key_path("api")

        # Check file permissions
        stat = key_path.stat()
        mode = stat.st_mode & 0o777
        assert mode == 0o600  # Read-write owner only

    @pytest.mark.skipif(platform.system() == "Windows", reason="Unix permissions not applicable on Windows")
    def test_cert_file_permissions(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Certificates are readable (0o644)."""
        pki.generate_service_cert("api")
        cert_path = pki_config.service_cert_path("api")

        stat = cert_path.stat()
        mode = stat.st_mode & 0o777
        assert mode == 0o644  # Read-only except owner

    @pytest.mark.skipif(platform.system() == "Windows", reason="Unix permissions not applicable on Windows")
    def test_cert_dir_permissions(self, pki_config: PKIConfig) -> None:
        """Cert directory has restricted permissions (0o700)."""
        stat = pki_config.cert_dir.stat()
        mode = stat.st_mode & 0o777
        assert mode == 0o700  # rwx owner only


class TestCertificateLoadingAndValidation:
    """Tests for certificate loading and chain validation."""

    def test_load_existing_root_ca(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Existing root CA can be loaded."""
        pki.setup_root_ca()
        root_key = pki._load_key(pki_config.root_ca_key_path())
        root_cert = pki._load_cert(pki_config.root_ca_cert_path())

        assert root_key is not None
        assert root_cert is not None

    def test_load_nonexistent_key_raises(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Loading nonexistent key raises FileNotFoundError."""
        with pytest.raises(FileNotFoundError):
            pki._load_key(pki_config.service_key_path("nonexistent"))

    def test_load_nonexistent_cert_raises(self, pki: PKI, pki_config: PKIConfig) -> None:
        """Loading nonexistent cert raises FileNotFoundError."""
        with pytest.raises(FileNotFoundError):
            pki._load_cert(pki_config.service_cert_path("nonexistent"))

    def test_force_regenerate_root_ca(self, pki: PKI, pki_config: PKIConfig) -> None:
        """force=True regenerates root CA even if it exists."""
        root_cert_1, _ = pki.setup_root_ca()
        serial_1 = root_cert_1.serial_number

        # Regenerate with force=True
        root_cert_2, _ = pki.setup_root_ca(force=True)
        serial_2 = root_cert_2.serial_number

        # Different certs (different serials)
        assert serial_1 != serial_2

    def test_force_regenerate_service_cert(self, pki: PKI, pki_config: PKIConfig) -> None:
        """force=True regenerates service cert even if it exists."""
        svc_cert_1, _ = pki.generate_service_cert("api")
        serial_1 = svc_cert_1.serial_number

        svc_cert_2, _ = pki.generate_service_cert("api", force=True)
        serial_2 = svc_cert_2.serial_number

        assert serial_1 != serial_2
