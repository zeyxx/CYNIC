"""HTTP adapter for Freebox OS API calls."""

from __future__ import annotations

import requests

from ..core.ports import FreeboxProvider


class FreeboxHttpAdapter(FreeboxProvider):
    """Concrete HTTP adapter for Freebox OS."""

    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")

    def request_authorization(self, payload: dict[str, str]) -> dict:
        response = requests.post(f"{self.base_url}/login/authorize/", json=payload, timeout=30)
        response.raise_for_status()
        return self._as_dict(response, "authorization")

    def read_authorization_status(self, track_id: str) -> str:
        response = requests.get(f"{self.base_url}/login/authorize/{track_id}", timeout=30)
        response.raise_for_status()
        payload = self._as_dict(response, "authorization_status")
        result = payload.get("result")
        status = None if not isinstance(result, dict) else result.get("status")
        return str(status) if isinstance(status, str) else "unknown"

    def get_challenge(self) -> str:
        response = requests.get(f"{self.base_url}/login/", timeout=30)
        response.raise_for_status()
        payload = self._as_dict(response, "challenge")
        result = payload.get("result")
        challenge = None if not isinstance(result, dict) else result.get("challenge")
        if not isinstance(challenge, str):
            raise RuntimeError(f"Unexpected challenge payload: {payload!r}")
        return challenge

    def request_session_token(self, app_id: str, password: str) -> str:
        response = requests.post(
            f"{self.base_url}/login/session/",
            json={"app_id": app_id, "password": password},
            timeout=30,
        )
        response.raise_for_status()
        payload = self._as_dict(response, "session")
        result = payload.get("result")
        if not isinstance(result, dict):
            raise RuntimeError(f"Unexpected session payload: {payload!r}")
        token = result.get("session_token")
        if not isinstance(token, str):
            raise RuntimeError(f"Invalid session token in payload: {payload!r}")
        return token

    def request_port_open(self, session_token: str, payload: dict[str, object]) -> dict:
        response = requests.post(
            f"{self.base_url}/fw/redir/",
            json=payload,
            headers={"X-Fbx-App-Auth": session_token},
            timeout=30,
        )
        response.raise_for_status()
        return self._as_dict(response, "open_port")

    def _as_dict(self, response: requests.Response, operation: str) -> dict:
        try:
            payload = response.json()
        except ValueError as exc:  # pragma: no cover - adapter-level parsing guard
            raise RuntimeError(f"Malformed JSON for {operation}: {response.text}") from exc
        if not isinstance(payload, dict):
            raise RuntimeError(f"Non-dict payload for {operation}: {payload!r}")
        return payload
