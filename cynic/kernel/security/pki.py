"""PKI infrastructure for mTLS service-to-service authentication.

Handles certificate generation, storage, and validation for mutual TLS.
Implements 3-tier hierarchy: Root CA  Intermediate CA  Leaf Certs (per service).

Design:
- Root CA: Generated once, stored securely (should be in Vault by Task 1.2)
- Intermediate CA: Generated once, represents "CYNIC cluster authority"
- Leaf Certs: Generated per service (API, EventBus, Core, etc.), short-lived (1-2 years)
- CRL/OCSP: Future addition for revocation (not in MVP)

Security notes:
- All private keys stored with restricted permissions (0o600)
- Root CA private key should be offline (Vault) by Task 1.2
- Intermediate CA private key in Vault or encrypted local storage
- Service certs rotated periodically
"""

from __future__ import annotations

import logging
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from cryptography import x509
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID

logger = logging.getLogger(__name__)


class PKIConfig:
    """Configuration for PKI operations."""

    def __init__(
        self,
        cert_dir: str | Path = "./certs",
        key_size: int = 2048,
        root_ca_days: int = 3650,  # 10 years
        intermediate_ca_days: int = 1825,  # 5 years
        service_cert_days: int = 730,  # 2 years
    ):
        """Initialize PKI configuration.

        Args:
            cert_dir: Directory for certificate storage
            key_size: RSA key size (2048 or 4096)
            root_ca_days: Root CA certificate validity period
            intermediate_ca_days: Intermediate CA validity period
            service_cert_days: Service certificate validity period
        """
        self.cert_dir = Path(cert_dir)
        self.key_size = key_size
        self.root_ca_days = root_ca_days
        self.intermediate_ca_days = intermediate_ca_days
        self.service_cert_days = service_cert_days

        # Ensure cert directory exists with restricted permissions
        self.cert_dir.mkdir(parents=True, exist_ok=True)
        self.cert_dir.chmod(0o700)

    def root_ca_cert_path(self) -> Path:
        return self.cert_dir / "root-ca.crt"

    def root_ca_key_path(self) -> Path:
        return self.cert_dir / "root-ca.key"

    def intermediate_ca_cert_path(self) -> Path:
        return self.cert_dir / "intermediate-ca.crt"

    def intermediate_ca_key_path(self) -> Path:
        return self.cert_dir / "intermediate-ca.key"

    def service_cert_path(self, service_name: str) -> Path:
        return self.cert_dir / f"{service_name}.crt"

    def service_key_path(self, service_name: str) -> Path:
        return self.cert_dir / f"{service_name}.key"

    def service_chain_path(self, service_name: str) -> Path:
        """Path to cert chain file (service cert + intermediate CA)."""
        return self.cert_dir / f"{service_name}-chain.crt"


class PKI:
    """PKI operations for mTLS infrastructure."""

    def __init__(self, config: PKIConfig):
        self.config = config
        self.backend = default_backend()

    # ==================== Key Generation ====================

    def _generate_rsa_key(self) -> rsa.RSAPrivateKey:
        """Generate RSA private key."""
        return rsa.generate_private_key(
            public_exponent=65537,
            key_size=self.config.key_size,
            backend=self.backend,
        )

    def _save_key(self, key: rsa.RSAPrivateKey, path: Path) -> None:
        """Save private key to file with restricted permissions."""
        pem = key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.TraditionalOpenSSL,
            encryption_algorithm=serialization.NoEncryption(),
        )
        path.write_bytes(pem)
        path.chmod(0o600)  # Read-write for owner only

    def _load_key(self, path: Path) -> rsa.RSAPrivateKey:
        """Load private key from file."""
        if not path.exists():
            raise FileNotFoundError(f"Key file not found: {path}")
        pem = path.read_bytes()
        return serialization.load_pem_private_key(
            pem,
            password=None,
            backend=self.backend,
        )

    # ==================== Certificate Generation ====================

    def _generate_self_signed_cert(
        self,
        private_key: rsa.RSAPrivateKey,
        common_name: str,
        organization: str = "CYNIC",
        days: int = 365,
    ) -> x509.Certificate:
        """Generate a self-signed certificate."""
        subject = issuer = x509.Name(
            [
                x509.NameAttribute(NameOID.ORGANIZATION_NAME, organization),
                x509.NameAttribute(NameOID.COMMON_NAME, common_name),
            ]
        )

        cert = (
            x509.CertificateBuilder()
            .subject_name(subject)
            .issuer_name(issuer)
            .public_key(private_key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(datetime.now(timezone.utc))
            .not_valid_after(datetime.now(timezone.utc) + timedelta(days=days))
            .add_extension(
                x509.BasicConstraints(ca=True, path_length=None),
                critical=True,
            )
            .add_extension(
                x509.KeyUsage(
                    digital_signature=True,
                    key_cert_sign=True,
                    crl_sign=True,
                    content_commitment=False,
                    key_encipherment=False,
                    data_encipherment=False,
                    key_agreement=False,
                    encipher_only=False,
                    decipher_only=False,
                ),
                critical=True,
            )
            .sign(private_key, hashes.SHA256(), self.backend)
        )
        return cert

    def _generate_signed_cert(
        self,
        csr: x509.CertificateSigningRequest,
        ca_cert: x509.Certificate,
        ca_key: rsa.RSAPrivateKey,
        days: int = 365,
        is_ca: bool = False,
    ) -> x509.Certificate:
        """Generate a certificate signed by CA."""
        cert = (
            x509.CertificateBuilder()
            .subject_name(csr.subject)
            .issuer_name(ca_cert.issuer)
            .public_key(csr.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(datetime.now(timezone.utc))
            .not_valid_after(datetime.now(timezone.utc) + timedelta(days=days))
            .add_extension(
                x509.BasicConstraints(ca=is_ca, path_length=None),
                critical=True,
            )
            .add_extension(
                x509.KeyUsage(
                    digital_signature=True,
                    key_cert_sign=is_ca,
                    crl_sign=is_ca,
                    content_commitment=False,
                    key_encipherment=True,
                    data_encipherment=False,
                    key_agreement=False,
                    encipher_only=False,
                    decipher_only=False,
                ),
                critical=True,
            )
            .add_extension(
                x509.SubjectAlternativeName(
                    [x509.DNSName(name.value) for name in csr.extensions[0].value]
                    if csr.extensions
                    else [],
                ),
                critical=False,
            )
            .sign(ca_key, hashes.SHA256(), self.backend)
        )
        return cert

    def _generate_csr(
        self,
        private_key: rsa.RSAPrivateKey,
        common_name: str,
        organization: str = "CYNIC",
        san_names: list[str] | None = None,
    ) -> x509.CertificateSigningRequest:
        """Generate a certificate signing request (CSR)."""
        subject = x509.Name(
            [
                x509.NameAttribute(NameOID.ORGANIZATION_NAME, organization),
                x509.NameAttribute(NameOID.COMMON_NAME, common_name),
            ]
        )

        builder = x509.CertificateSigningRequestBuilder().subject_name(subject)

        if san_names:
            builder = builder.add_extension(
                x509.SubjectAlternativeName([x509.DNSName(name) for name in san_names]),
                critical=False,
            )

        csr = builder.sign(private_key, hashes.SHA256(), self.backend)
        return csr

    # ==================== Certificate I/O ====================

    def _save_cert(self, cert: x509.Certificate, path: Path) -> None:
        """Save certificate to PEM file."""
        pem = cert.public_bytes(serialization.Encoding.PEM)
        path.write_bytes(pem)
        path.chmod(0o644)

    def _load_cert(self, path: Path) -> x509.Certificate:
        """Load certificate from PEM file."""
        if not path.exists():
            raise FileNotFoundError(f"Certificate file not found: {path}")
        pem = path.read_bytes()
        return x509.load_pem_x509_certificate(pem, self.backend)

    # ==================== Public API ====================

    def setup_root_ca(
        self, force: bool = False
    ) -> tuple[x509.Certificate, rsa.RSAPrivateKey]:
        """Set up root CA (one-time operation).

        Args:
            force: If True, regenerate even if it exists

        Returns:
            Tuple of (root_ca_cert, root_ca_key)
        """
        cert_path = self.config.root_ca_cert_path()
        key_path = self.config.root_ca_key_path()

        if cert_path.exists() and key_path.exists() and not force:
            logger.info("Root CA already exists, loading...")
            return self._load_cert(cert_path), self._load_key(key_path)

        logger.info("Generating root CA...")
        key = self._generate_rsa_key()
        cert = self._generate_self_signed_cert(
            key,
            common_name="CYNIC Root CA",
            days=self.config.root_ca_days,
        )

        self._save_key(key, key_path)
        self._save_cert(cert, cert_path)

        logger.info(f"Root CA created: {cert_path}")
        return cert, key

    def setup_intermediate_ca(
        self, force: bool = False
    ) -> tuple[x509.Certificate, rsa.RSAPrivateKey]:
        """Set up intermediate CA (one-time operation).

        Args:
            force: If True, regenerate even if it exists

        Returns:
            Tuple of (intermediate_ca_cert, intermediate_ca_key)
        """
        cert_path = self.config.intermediate_ca_cert_path()
        key_path = self.config.intermediate_ca_key_path()

        if cert_path.exists() and key_path.exists() and not force:
            logger.info("Intermediate CA already exists, loading...")
            return self._load_cert(cert_path), self._load_key(key_path)

        logger.info("Generating intermediate CA...")
        root_cert, root_key = self.setup_root_ca()

        # Generate intermediate CA key and CSR
        int_key = self._generate_rsa_key()
        int_csr = self._generate_csr(
            int_key,
            common_name="CYNIC Intermediate CA",
        )

        # Sign CSR with root CA
        int_cert = self._generate_signed_cert(
            int_csr,
            root_cert,
            root_key,
            days=self.config.intermediate_ca_days,
            is_ca=True,
        )

        self._save_key(int_key, key_path)
        self._save_cert(int_cert, cert_path)

        logger.info(f"Intermediate CA created: {cert_path}")
        return int_cert, int_key

    def generate_service_cert(
        self,
        service_name: str,
        san_names: list[str] | None = None,
        force: bool = False,
    ) -> tuple[x509.Certificate, rsa.RSAPrivateKey]:
        """Generate a service certificate.

        Args:
            service_name: Service identifier (e.g., "api", "event-bus", "core")
            san_names: Subject Alternative Names (DNS names for the service)
            force: If True, regenerate even if it exists

        Returns:
            Tuple of (service_cert, service_key)
        """
        cert_path = self.config.service_cert_path(service_name)
        key_path = self.config.service_key_path(service_name)

        if cert_path.exists() and key_path.exists() and not force:
            logger.info(f"Certificate for {service_name} already exists, loading...")
            return self._load_cert(cert_path), self._load_key(key_path)

        logger.info(f"Generating certificate for {service_name}...")
        int_cert, int_key = self.setup_intermediate_ca()

        # Generate service key and CSR
        svc_key = self._generate_rsa_key()
        svc_csr = self._generate_csr(
            svc_key,
            common_name=service_name,
            san_names=san_names or [service_name, f"{service_name}.local"],
        )

        # Sign CSR with intermediate CA
        svc_cert = self._generate_signed_cert(
            svc_csr,
            int_cert,
            int_key,
            days=self.config.service_cert_days,
            is_ca=False,
        )

        self._save_key(svc_key, key_path)
        self._save_cert(svc_cert, cert_path)

        logger.info(f"Service certificate created for {service_name}: {cert_path}")
        return svc_cert, svc_key

    def get_service_cert(self, service_name: str) -> x509.Certificate:
        """Load an existing service certificate."""
        cert_path = self.config.service_cert_path(service_name)
        return self._load_cert(cert_path)

    def get_service_key(self, service_name: str) -> rsa.RSAPrivateKey:
        """Load an existing service private key."""
        key_path = self.config.service_key_path(service_name)
        return self._load_key(key_path)

    # ==================== Certificate Validation ====================

    def verify_cert_chain(self, cert_path: Path, ca_cert_path: Path) -> bool:
        """Verify that a certificate is signed by a CA.

        Args:
            cert_path: Path to certificate to verify
            ca_cert_path: Path to CA certificate

        Returns:
            True if certificate is valid and signed by CA
        """
        try:
            cert = self._load_cert(cert_path)
            ca_cert = self._load_cert(ca_cert_path)

            # Verify the signature
            ca_cert.public_key().verify(
                cert.signature,
                cert.tbs_certificate_bytes,
                padding=cert.signature_algorithm_oid._signature_algorithm,
                algorithm=hashes.SHA256(),
            )
            return True
        except Exception as e:
            logger.warning(f"Certificate verification failed: {e}")
            return False

    def get_cert_expiry(self, cert_path: Path) -> datetime:
        """Get certificate expiration timestamp."""
        cert = self._load_cert(cert_path)
        return cert.not_valid_after

    def is_cert_expired(self, cert_path: Path) -> bool:
        """Check if certificate is expired."""
        expiry = self.get_cert_expiry(cert_path)
        return datetime.utcnow() > expiry

    def get_cert_info(self, cert_path: Path) -> dict[str, Any]:
        """Get certificate metadata."""
        cert = self._load_cert(cert_path)
        return {
            "subject": cert.subject.rfc4514_string(),
            "issuer": cert.issuer.rfc4514_string(),
            "serial_number": str(cert.serial_number),
            "not_valid_before": cert.not_valid_before,
            "not_valid_after": cert.not_valid_after,
            "is_expired": datetime.utcnow() > cert.not_valid_after,
        }
