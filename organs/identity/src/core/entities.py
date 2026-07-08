from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

# Commitment algorithm — post-quantum preparation.
# SHA3-512 (Keccak family) is NIST-standardised and quantum-resistant.
_COMMITMENT_ALG = "sha3_512"
_COMMITMENT_SCHEMA = "cynic.identity.commitment.v2"


class CredentialShape(str, Enum):
    """Known shape patterns for common credentials."""
    PORKBUN_API_KEY       = "pk1_"
    PORKBUN_SECRET_KEY    = "sk1_"
    CLOUDFLARE_TOKEN      = "cf_"      # heuristic — CF tokens vary
    GENERIC               = ""


@dataclass
class Credential:
    """
    A credential entry as stored in the env file.
    The value is held only in memory during the operation — never logged.
    """
    key: str
    value: str          # ephemeral — never serialized to disk beyond ~/.cynic-env

    @property
    def shape(self) -> str:
        """First 4 chars + stars — enough to confirm format without exposing value."""
        if len(self.value) <= 4:
            return "****"
        return self.value[:4] + "*" * min(8, len(self.value) - 4)

    @property
    def commitment(self) -> str:
        """SHA3-512 of the value — post-quantum proof of possession without value exposure."""
        return hashlib.sha3_512(self.value.encode()).hexdigest()

    @property
    def commitment_schema(self) -> str:
        return _COMMITMENT_SCHEMA

    @property
    def length(self) -> int:
        return len(self.value)


@dataclass
class CredentialAttestation:
    """
    What an agent is allowed to see about a credential.
    Values are never included — only presence, shape, length, and commitment.
    """
    key: str
    present: bool
    shape: Optional[str] = None
    length: Optional[int] = None
    commitment: Optional[str] = None        # SHA3-512 hex digest of value
    commitment_schema: Optional[str] = None # e.g. cynic.identity.commitment.v2
    source: Optional[str] = None            # redacted path


@dataclass
class SmokecheckResult:
    """Result of a live API liveness check for a credential group."""
    provider: str                       # e.g. "porkbun", "cloudflare"
    ok: bool
    message: str
    detail: dict = field(default_factory=dict)


@dataclass
class CredentialGroup:
    """A logical group of credentials belonging to the same provider."""
    name: str
    keys: list[str]
    smokecheck_endpoint: Optional[str] = None
