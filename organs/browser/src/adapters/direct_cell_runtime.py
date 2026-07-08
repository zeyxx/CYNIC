"""Shared helpers for direct L3 runtime cells."""

from __future__ import annotations

import os
import time
import urllib.request
from pathlib import Path


def read_pid(path: Path) -> int | None:
    try:
        return int(path.read_text(encoding="utf-8").strip())
    except Exception:
        return None


def pid_alive(pid: int | None) -> bool:
    if not pid:
        return False
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def wait_http_ready(url: str, timeout_seconds: float, *, accepted_status_ceiling: int = 500) -> bool:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if 200 <= int(response.status) < accepted_status_ceiling:
                    return True
        except Exception:
            time.sleep(0.2)
    return False
