from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Optional

from .entities import Credential, CredentialAttestation, SmokecheckResult


class CredentialStorePort(ABC):
    """
    Read and write credentials in the env file.
    The file path is resolved at runtime from CYNIC_ENV_FILE or ~/.cynic-env.
    """

    @abstractmethod
    def read_key(self, key: str) -> Optional[Credential]:
        """Read a single credential by key. Returns None if absent."""

    @abstractmethod
    def list_keys(self) -> list[str]:
        """List all keys present in the store. Never returns values."""

    @abstractmethod
    def write_key(self, credential: Credential) -> bool:
        """
        Write or overwrite a credential in the env file.
        Mutating — must be called only from approved CLI paths.
        """

    @abstractmethod
    def delete_key(self, key: str) -> bool:
        """Remove a credential from the env file."""

    @abstractmethod
    def env_file_path(self) -> str:
        """Return the resolved env file path (redacted for logging)."""


class SmokecheckPort(ABC):
    """Live liveness checks for credential groups against their provider APIs."""

    @abstractmethod
    def check_porkbun(self, api_key: str, secret_key: str) -> SmokecheckResult:
        """Non-mutating ping against Porkbun API to verify credential validity."""

    @abstractmethod
    def check_cloudflare(self, token: str) -> SmokecheckResult:
        """Non-mutating token verify against Cloudflare API."""
