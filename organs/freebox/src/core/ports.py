"""Ports for Freebox runtime interactions."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Protocol, runtime_checkable

from organs.contracts.src.surface_contracts import read_json_file_or_default, write_json_file


@runtime_checkable
class FreeboxProvider(Protocol):
    """Protocol for Freebox HTTP operations used by the domain layer."""

    def request_authorization(self, payload: dict[str, str]) -> dict[str, Any]:
        """Request authorization from Freebox and return the API response."""

    def read_authorization_status(self, track_id: str) -> str:
        """Return the authorization status string for a given track id."""

    def get_challenge(self) -> str:
        """Return a challenge string required for login."""

    def request_session_token(self, app_id: str, password: str) -> str:
        """Return a session token from Freebox login endpoint."""

    def request_port_open(self, session_token: str, payload: dict[str, Any]) -> dict[str, Any]:
        """Open a port redirection and return the API response."""


@runtime_checkable
class TokenStore(Protocol):
    """Protocol to persist and load Freebox app tokens."""

    def read_token(self) -> str | None:
        """Return the saved app token, if any."""

    def write_token(self, token: str) -> None:
        """Persist a token for subsequent login attempts."""


class FileTokenStore(TokenStore):
    """Filesystem-backed token store."""

    def __init__(self, token_file: str | Path) -> None:
        self.token_file = Path(token_file).expanduser()

    def read_token(self) -> str | None:
        if not self.token_file.exists():
            return None
        payload = read_json_file_or_default(self.token_file, {})
        if not isinstance(payload, dict):
            return None
        token = payload.get("app_token")
        return str(token) if isinstance(token, str) else None

    def write_token(self, token: str) -> None:
        write_json_file(self.token_file, {"app_token": token})
