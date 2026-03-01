"""
Shared helpers for the CYNIC CLI — colors, HTTP, file I/O, display primitives.
"""
from __future__ import annotations

import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from typing import Any

# ── Windows UTF-8 fix ──────────────────────────────────────────────────────
# Windows terminals default to CP1252 which can't render box-drawing chars
# or emoji (█, ░, 🟢, etc.). Force UTF-8 output so the dashboard renders.
if sys.platform == "win32":
    try:
        # Only wrap if not already wrapped and buffer is actually available
        if hasattr(sys.stdout, 'buffer') and not isinstance(sys.stdout, io.TextIOWrapper):
            sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
        if hasattr(sys.stderr, 'buffer') and not isinstance(sys.stderr, io.TextIOWrapper):
            sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
    except (AttributeError, ValueError, OSError):
        pass  # Already wrapped or buffer unavailable

# ── Paths ── (portable across host/container via CYNIC_STATE_DIR env var) ──
# Use CYNIC_STATE_DIR env var if available (Docker), otherwise fall back to ~/.cynic
_CYNIC_DIR = os.environ.get("CYNIC_STATE_DIR", os.path.join(os.path.expanduser("~"), ".cynic"))
_GUIDANCE         = os.path.join(_CYNIC_DIR, "guidance.json")
_CHECKPOINT       = os.path.join(_CYNIC_DIR, "session-latest.json")
_PENDING          = os.path.join(_CYNIC_DIR, "pending_actions.json")
_SDK_SESSIONS_LOG = os.path.join(_CYNIC_DIR, "sdk_sessions.jsonl")
_CONSCIOUSNESS    = os.path.join(_CYNIC_DIR, "consciousness.json")

# Server (default port from CynicConfig — single source of truth)
from cynic.kernel.core.config import CynicConfig as _CynicConfig

_PORT = _CynicConfig.from_env().port
_API  = f"http://localhost:{_PORT}"
_API_TIMEOUT = 2.0   # never block the CLI for more than 2s


# ── Colors (ANSI) ──────────────────────────────────────────────────────────
_C = {
    "reset":  "\033[0m",
    "bold":   "\033[1m",
    "dim":    "\033[2m",
    "red":    "\033[91m",
    "yellow": "\033[93m",
    "green":  "\033[92m",
    "cyan":   "\033[96m",
    "orange": "\033[33m",
    "white":  "\033[97m",
    "gray":   "\033[90m",
}


def _c(color: str, text: str) -> str:
    """Wrap text in ANSI color. Disable if not a TTY."""
    if not sys.stdout.isatty():
        return text
    return f"{_C.get(color, '')}{text}{_C['reset']}"


# ── File helpers ───────────────────────────────────────────────────────────

def _read_json(path: str) -> Optional[dict[str, Any]]:
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except OSError:
        return None


def _api_get(path: str) -> Optional[dict[str, Any]]:
    try:
        req = urllib.request.Request(
            f"{_API}{path}",
            headers={"User-Agent": "CYNIC-CLI/2.0"},
        )
        with urllib.request.urlopen(req, timeout=_API_TIMEOUT) as resp:
            return json.loads(resp.read().decode())
    except json.JSONDecodeError:
        return None


def _api_post(path: str, body: Optional[dict] = None) -> Optional[dict[str, Any]]:
    """POST to the API. Returns parsed JSON or None on any error."""
    try:
        data = json.dumps(body or {}).encode()
        req = urllib.request.Request(
            f"{_API}{path}",
            data=data,
            headers={"User-Agent": "CYNIC-CLI/2.0", "Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=_API_TIMEOUT) as resp:
            return json.loads(resp.read().decode())
    except json.JSONDecodeError:
        return None


def _pending_actions() -> tuple[list[dict], bool]:
    """
    Return (pending_actions_list, api_available).
    Tries API first; falls back to direct file read.
    """
    data = _api_get("/actions")
    if data is not None:
        return data.get("actions", []), True
    # File fallback
    raw = _read_json(_PENDING)
    if isinstance(raw, list):
        return [a for a in raw if a.get("status") == "PENDING"], False
    return [], False


def _file_set_status(action_id: str, status: str) -> bool:
    """
    Directly update pending_actions.json without the server.
    Used when API is unreachable. Returns True on success.
    """
    try:
        raw = _read_json(_PENDING)
        if not isinstance(raw, list):
            return False
        for action in raw:
            if action.get("action_id") == action_id:
                action["status"] = status
                break
        else:
            return False
        os.makedirs(_CYNIC_DIR, exist_ok=True)
        with open(_PENDING, "w", encoding="utf-8") as fh:
            json.dump(raw, fh, indent=2)
        return True
    except OSError:
        return False


# ── Verdict → color ────────────────────────────────────────────────────────

_VERDICT_COLOR = {
    "HOWL":  "green",
    "WAG":   "cyan",
    "GROWL": "orange",
    "BARK":  "red",
}

_VERDICT_SYMBOL = {
    "HOWL":  "🟢",
    "WAG":   "🟡",
    "GROWL": "🟠",
    "BARK":  "🔴",
}


def _verdict_str(verdict: str, q: float = 0.0) -> str:
    sym  = _VERDICT_SYMBOL.get(verdict, "⚪")
    col  = _VERDICT_COLOR.get(verdict, "white")
    return f"{sym} {_c(col, verdict)} Q={q:.1f}"


# ── Progress bar ───────────────────────────────────────────────────────────

def _bar(score: float, max_score: float = 100.0, width: int = 10) -> str:
    """Render a φ-styled bar: [██████░░░░] (10 chars)."""
    filled = int(round(min(score / max_score, 1.0) * width))
    return f"[{'█' * filled}{'░' * (width - filled)}]"


# ── LOD display ────────────────────────────────────────────────────────────

_LOD_COLOR = {0: "green", 1: "cyan", 2: "orange", 3: "red"}
_LOD_NAMES = {0: "FULL", 1: "REDUCED", 2: "EMERGENCY", 3: "MINIMAL"}


def _lod_str(lod_val: int) -> str:
    name = _LOD_NAMES.get(lod_val, str(lod_val))
    col  = _LOD_COLOR.get(lod_val, "white")
    return _c(col, f"LOD {lod_val} {name}")


# ── Loop completion ────────────────────────────────────────────────────────

_LOOPS = {
    "L1 Machine→Actions":   (82, "✅  accept→ACT_REQUESTED, reject→QTable, auto_executed linked"),
    "L2 CYNIC↔Claude Code": (82, "✅  prompt enrichment + JSONL persist + L2→L1 cross-feed"),
    "L3 Human→CYNIC→Human": (82, "✅  review/watch/feedback commands live"),
    "L4 CYNIC→CYNIC Self":  (82, "✅  SelfProber: QTable/EScore/Residual analysis live"),
}

# Action type → display color
_ATYPE_COLOR = {
    "INVESTIGATE": "red",
    "REFACTOR":    "orange",
    "ALERT":       "yellow",
    "MONITOR":     "dim",
}

# Priority → color
_PRIORITY_COLOR = {1: "red", 2: "orange", 3: "cyan", 4: "dim"}


# ── Time ago ───────────────────────────────────────────────────────────────

def _ago(ts: float) -> str:
    if ts <= 0:
        return "never"
    delta = time.time() - ts
    if delta < 60:
        return f"{delta:.0f}s ago"
    if delta < 3600:
        return f"{delta/60:.1f}m ago"
    return f"{delta/3600:.1f}h ago"


# ── Disk bar ───────────────────────────────────────────────────────────────

def _disk_bar(used_pct: float) -> str:
    if used_pct >= 0.90:
        col = "red"
    elif used_pct >= 0.764:
        col = "orange"
    elif used_pct >= 0.618:
        col = "yellow"
    else:
        col = "green"
    bar = _bar(used_pct * 100, max_score=100.0)
    return f"{_c(col, bar)} {used_pct * 100:.1f}%"


# ── Sections ───────────────────────────────────────────────────────────────

def _section(title: str, lines: list) -> None:
    w = 68
    for line in lines:
        plain = re.sub(r"\033\[[0-9;]*m", "", line)
        max(0, w - 2 - len(plain))


def _divider() -> None:
    pass


# ── Time formatter ─────────────────────────────────────────────────────────

def _format_s(s: float) -> str:
    if s < 60:
        return f"{s:.0f}s"
    if s < 3600:
        return f"{s/60:.1f}m"
    return f"{s/3600:.1f}h"
