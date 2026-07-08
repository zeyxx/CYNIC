"""Shared environment loading adapter for organism runtime membranes."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Iterable


def load_env_file(path: Path, *, allowed_keys: Iterable[str] | None = None) -> list[str]:
    """Load shell-style KEY=VALUE lines without overriding existing variables."""

    allowed = set(allowed_keys) if allowed_keys is not None else None
    loaded: list[str] = []
    if not path.exists():
        return loaded
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[len("export ") :].strip()
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        if not key or (allowed is not None and key not in allowed):
            continue
        if os.environ.get(key, "").strip():
            continue
        os.environ[key] = value.strip().strip("'\"")
        loaded.append(key)
    return loaded


def load_required_env_from_default_file(
    required: tuple[str, ...],
    *,
    env_file_var: str = "CYNIC_ENV_FILE",
    default_name: str = ".cynic-env",
    disable_var: str = "CYNIC_LOAD_ENV_FILE",
) -> dict[str, object]:
    """Load required env keys from the organism env file and return redacted evidence."""

    before = {key: bool(os.environ.get(key, "").strip()) for key in required}
    if all(before.values()):
        return {
            "source": "process",
            "loaded": False,
            "loaded_keys": [],
            "present": before,
        }
    if os.environ.get(disable_var, "1").lower() in {"0", "false", "no"}:
        return {
            "source": "disabled",
            "loaded": False,
            "loaded_keys": [],
            "present": before,
        }

    path = Path(os.environ.get(env_file_var, str(Path.home() / default_name))).expanduser()
    loaded_keys = load_env_file(path, allowed_keys=required)
    after = {key: bool(os.environ.get(key, "").strip()) for key in required}
    return {
        "source": str(path),
        "loaded": bool(loaded_keys),
        "loaded_keys": loaded_keys,
        "present": after,
    }
