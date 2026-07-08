from __future__ import annotations

import json
import os
import re
import urllib.request
import urllib.error
from pathlib import Path
from typing import Optional

from organs.identity.src.core.entities import Credential, SmokecheckResult
from organs.identity.src.core.ports import CredentialStorePort, SmokecheckPort


def _resolve_env_file() -> Path:
    """
    Resolve ~/.cynic-env path.
    Priority: CYNIC_ENV_FILE env var → default ~/.cynic-env
    """
    override = os.environ.get("CYNIC_ENV_FILE", "").strip()
    if override:
        return Path(override).expanduser().resolve()
    return Path.home() / ".cynic-env"


class EnvFileCredentialStore(CredentialStorePort):
    """
    Reads and writes credentials in ~/.cynic-env (or CYNIC_ENV_FILE).

    File format: shell-style KEY=VALUE lines.
    Comments (#) and blank lines are preserved on write.
    Existing non-credential lines are never touched.
    """

    def env_file_path(self) -> str:
        p = _resolve_env_file()
        # Redact home directory for safe logging
        try:
            return "~/" + str(p.relative_to(Path.home()))
        except ValueError:
            return str(p)

    def _read_raw_lines(self) -> list[str]:
        p = _resolve_env_file()
        if not p.exists():
            return []
        return p.read_text(encoding="utf-8").splitlines()

    def _write_raw_lines(self, lines: list[str]) -> None:
        p = _resolve_env_file()
        exists = p.exists()
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        if not exists:
            p.chmod(0o600)  # Restrict permissions only on initial creation

    def _parse_line(self, line: str) -> Optional[tuple[str, str]]:
        """Parse a KEY=VALUE line. Returns None for comments/blanks."""
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            return None
        if stripped.startswith("export "):
            stripped = stripped[len("export "):].strip()
        if "=" not in stripped:
            return None
        key, value = stripped.split("=", 1)
        return key.strip(), value.strip().strip("'\"")

    def read_key(self, key: str) -> Optional[Credential]:
        # Prefer live env first (already loaded in process)
        live = os.environ.get(key, "").strip()
        if live:
            return Credential(key=key, value=live)
        # Fall back to file
        for line in self._read_raw_lines():
            parsed = self._parse_line(line)
            if parsed and parsed[0] == key:
                return Credential(key=key, value=parsed[1])
        return None

    def list_keys(self) -> list[str]:
        keys = []
        seen = set()
        # From live env first
        for k, v in os.environ.items():
            if v.strip():
                keys.append(k)
                seen.add(k)
        # From file (keys not already in env)
        for line in self._read_raw_lines():
            parsed = self._parse_line(line)
            if parsed and parsed[0] not in seen:
                keys.append(parsed[0])
                seen.add(parsed[0])
        return sorted(keys)

    def write_key(self, credential: Credential) -> bool:
        lines = self._read_raw_lines()
        key = credential.key
        new_line = f"{key}={credential.value}"
        # Replace existing line if present
        replaced = False
        for i, line in enumerate(lines):
            parsed = self._parse_line(line)
            if parsed and parsed[0] == key:
                lines[i] = new_line
                replaced = True
                break
        if not replaced:
            lines.append(new_line)
        try:
            self._write_raw_lines(lines)
            # Also inject into current process env
            os.environ[key] = credential.value
            return True
        except OSError:
            return False

    def delete_key(self, key: str) -> bool:
        lines = self._read_raw_lines()
        new_lines = []
        found = False
        for line in lines:
            parsed = self._parse_line(line)
            if parsed and parsed[0] == key:
                found = True
                continue  # skip this line
            new_lines.append(line)
        if not found:
            return False
        try:
            self._write_raw_lines(new_lines)
            os.environ.pop(key, None)
            return True
        except OSError:
            return False


class ApiSmokecheckAdapter(SmokecheckPort):
    """Live liveness checks against provider APIs. Read-only, no mutations."""

    def check_porkbun(self, api_key: str, secret_key: str) -> SmokecheckResult:
        try:
            body = json.dumps({"apikey": api_key, "secretapikey": secret_key}).encode()
            req = urllib.request.Request(
                "https://api.porkbun.com/api/json/v3/ping",
                data=body,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read())
            ok = data.get("status") == "SUCCESS"
            return SmokecheckResult(
                provider="porkbun",
                ok=ok,
                message="Porkbun ping OK" if ok else f"Porkbun ping failed: {data.get('message')}",
                detail={"your_ip": data.get("yourIp")} if ok else {},
            )
        except Exception as exc:
            return SmokecheckResult(provider="porkbun", ok=False, message=str(exc))

    def check_cloudflare(self, token: str) -> SmokecheckResult:
        try:
            req = urllib.request.Request(
                "https://api.cloudflare.com/client/v4/user/tokens/verify",
                headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
                method="GET",
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read())
            ok = data.get("success", False)
            status = data.get("result", {}).get("status") if ok else None
            return SmokecheckResult(
                provider="cloudflare",
                ok=ok and status == "active",
                message=f"Cloudflare token {status}" if ok else "Cloudflare token invalid",
                detail={"status": status} if ok else {},
            )
        except Exception as exc:
            return SmokecheckResult(provider="cloudflare", ok=False, message=str(exc))
