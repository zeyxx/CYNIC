"""
CYNIC CLI — Terminal status dashboard for the human who needs to see everything.

Usage:
  python -m cynic.cli           → full status (default)
  python -m cynic.cli status    → same
  python -m cynic.cli health    → quick health check only
  python -m cynic.cli lod       → LOD level only
  python -m cynic.cli loops     → 4 feedback loop completion matrix

Reads from (fastest path — no server needed):
  ~/.cynic/guidance.json        → last judgment verdict/Q/dogs
  ~/.cynic/session-latest.json  → session checkpoint info
  ~/.cynic/pending_actions.json → proposed action queue (P5, future)

Optionally queries (falls back gracefully if server is down):
  http://localhost:PORT/health
  http://localhost:PORT/lod
  http://localhost:PORT/act/telemetry

φ-bound: confidence never shown above 61.8%.
"""
from __future__ import annotations

import io
import json
import os
import sys
import time
import urllib.request
from typing import Any, Dict, Optional

# ── Windows UTF-8 fix ──────────────────────────────────────────────────────
# Windows terminals default to CP1252 which can't render box-drawing chars
# or emoji (█, ░, 🟢, etc.). Force UTF-8 output so the dashboard renders.
if sys.platform == "win32":
    try:
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
    except AttributeError:
        pass  # Already wrapped (e.g. pytest captures stdout)

# ── Paths ──────────────────────────────────────────────────────────────────
_CYNIC_DIR = os.path.join(os.path.expanduser("~"), ".cynic")
_GUIDANCE   = os.path.join(_CYNIC_DIR, "guidance.json")
_CHECKPOINT = os.path.join(_CYNIC_DIR, "session-latest.json")
_PENDING    = os.path.join(_CYNIC_DIR, "pending_actions.json")

# Server (default port; overridable via CYNIC_PORT env)
_PORT = int(os.getenv("CYNIC_PORT", "8765"))
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

def _read_json(path: str) -> Optional[Dict[str, Any]]:
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except Exception:
        return None


def _api_get(path: str) -> Optional[Dict[str, Any]]:
    try:
        req = urllib.request.Request(
            f"{_API}{path}",
            headers={"User-Agent": "CYNIC-CLI/2.0"},
        )
        with urllib.request.urlopen(req, timeout=_API_TIMEOUT) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return None


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
    "L1 Machine→Actions": (42, "❌ ActionProposer missing"),
    "L2 CYNIC↔Claude Code": (68, "⚠️  ACT result→QTable (P6 done)"),
    "L3 Human→CYNIC→Human": (62, "⚠️  /feedback exists, no UI yet"),
    "L4 CYNIC→CYNIC Self": (62, "⚠️  code analysis missing"),
}


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
    print(_c("bold", f"┌{'─' * (w - 2)}┐"))
    label = f"  {title}"
    print(_c("bold", "│") + _c("cyan", label) + " " * (w - 2 - len(label)) + _c("bold", "│"))
    print(_c("bold", f"├{'─' * (w - 2)}┤"))
    for line in lines:
        # Strip ANSI for length calculation
        import re
        plain = re.sub(r"\033\[[0-9;]*m", "", line)
        padding = max(0, w - 2 - len(plain))
        print(_c("bold", "│") + " " + line + " " * padding + _c("bold", "│"))
    print(_c("bold", f"└{'─' * (w - 2)}┘"))
    print()


def _divider() -> None:
    print()


# ── Commands ───────────────────────────────────────────────────────────────

def cmd_status() -> None:
    now_str = time.strftime("%Y-%m-%d %H:%M:%S")
    print()
    print(_c("bold", f"  ╔══ CYNIC STATUS — {now_str} ══╗"))
    print()

    # ── 1. Guidance (last judgment from files) ─────────────────────────────
    g = _read_json(_GUIDANCE)
    if g:
        verdict  = g.get("verdict", "?")
        q_score  = float(g.get("q_score", 0.0))
        conf     = float(g.get("confidence", 0.0))
        reality  = g.get("reality", "?")
        sk       = g.get("state_key", "")
        ts       = float(g.get("timestamp", 0.0))
        dog_votes = g.get("dog_votes", {})

        v_str  = _verdict_str(verdict, q_score)
        ago_s  = _ago(ts)

        lines = [
            f"{v_str}  conf={conf*100:.0f}%  {_c('gray', ago_s)}",
            f"  {_c('dim', reality + '×JUDGE')}  state={_c('dim', sk[:40])}",
        ]

        if dog_votes:
            bar_parts = []
            for dog, score in list(dog_votes.items())[:6]:
                bar = _bar(float(score), max_score=100.0, width=6)
                bar_parts.append(f"{_c('dim', dog[:6])} {bar}")
            lines.append("  " + "  ".join(bar_parts))

        _section("LAST JUDGMENT", lines)
    else:
        _section("LAST JUDGMENT", [_c("gray", "  No guidance.json found — server not started?")])

    # ── 2. LOD from API ────────────────────────────────────────────────────
    lod_data = _api_get("/lod")
    if lod_data:
        lod_val  = lod_data.get("current_lod", 0)
        forced   = lod_data.get("forced", False)
        streak   = lod_data.get("healthy_streak", 0)
        uptime   = lod_data.get("uptime_s", 0)
        trans    = lod_data.get("total_transitions", 0)
        desc     = lod_data.get("description", "")
        lod_line = _lod_str(lod_val)
        if forced:
            lod_line += _c("orange", " [FORCED]")
        lines = [
            lod_line,
            f"  {_c('dim', desc)}",
            f"  uptime={_format_s(uptime)}  transitions={trans}  healthy_streak={streak}",
        ]
        # Show most recent transition if any
        recent = lod_data.get("recent_transitions", [])
        if recent:
            last_t = recent[-1]
            lines.append(
                f"  last: {_c('dim', last_t.get('from','?'))} → {_c('dim', last_t.get('to','?'))}"
                f"  err={last_t.get('error_rate',0):.2f}"
                f"  lat={last_t.get('latency_ms',0):.0f}ms"
            )
        _section("SURVIVAL LOD", lines)
    else:
        # Fallback: derive LOD from guidance or disk
        _section("SURVIVAL LOD", [_c("gray", "  API not reachable — start server to see LOD")])

    # ── 3. Session checkpoint ──────────────────────────────────────────────
    ck = _read_json(_CHECKPOINT)
    if ck:
        saved_at = float(ck.get("saved_at", 0.0))
        chunks   = len(ck.get("chunks", []))
        age_s    = _ago(saved_at)
        lines = [
            f"checkpoint {_c('green', age_s)}  chunks={_c('cyan', str(chunks))}",
        ]
    else:
        lines = [_c("gray", "  No session checkpoint yet")]
    pending = _read_json(_PENDING)
    pending_count = len(pending) if isinstance(pending, list) else 0
    lines.append(f"pending_actions={_c('yellow' if pending_count else 'dim', str(pending_count))}")
    _section("SESSION", lines)

    # ── 4. Learning from API ───────────────────────────────────────────────
    health_data = _api_get("/health")
    if health_data:
        learn    = health_data.get("learning", {})
        states   = learn.get("states", 0)
        updates  = learn.get("total_updates", 0)
        active   = learn.get("active", False)
        dogs     = health_data.get("dogs", [])
        llms     = health_data.get("llm_adapters", [])
        sched    = health_data.get("scheduler", {})
        judged   = health_data.get("judgments_total", 0)
        lines = [
            f"QTable: {_c('cyan', str(states))} states  {_c('cyan', str(updates))} updates"
            f"  loop={'✅' if active else '❌'}",
            f"Dogs: {len(dogs)}  LLMs: {len(llms)}  judgments: {judged}",
        ]
        queued = sched.get("queued", 0)
        if queued:
            lines.append(f"scheduler queue: {_c('yellow', str(queued))} pending")
        _section("LEARNING", lines)
    else:
        _section("LEARNING", [_c("gray", "  API not reachable — QTable stats unavailable")])

    # ── 5. Telemetry (SDK sessions) ────────────────────────────────────────
    tel_data = _api_get("/act/telemetry")
    if tel_data and tel_data.get("stats", {}).get("count", 0) > 0:
        s = tel_data["stats"]
        lines = [
            f"sessions={s.get('count',0)}"
            f"  error_rate={s.get('error_rate',0):.1%}"
            f"  mean_reward={s.get('mean_reward',0):.3f}",
            f"verdicts={s.get('verdicts',{})}",
        ]
        _section("SDK TELEMETRY", lines)

    # ── 6. Feedback loops matrix ───────────────────────────────────────────
    lines = []
    for loop_name, (pct, note) in _LOOPS.items():
        bar = _bar(pct, max_score=100.0, width=8)
        col = "green" if pct >= 80 else ("cyan" if pct >= 61 else ("yellow" if pct >= 42 else "red"))
        lines.append(f"{_c(col, bar)} {pct}%  {loop_name}")
        lines.append(f"   {_c('dim', note)}")
    _section("FEEDBACK LOOPS", lines)

    print(_c("dim", f"  *sniff* Confidence: 55% (φ⁻¹ limit)"))
    print()


def cmd_health() -> None:
    data = _api_get("/health")
    if data is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)
    status = data.get("status", "?")
    uptime = data.get("uptime_s", 0)
    col = "green" if status == "alive" else "orange"
    print(f"*sniff* {_c(col, status.upper())}  uptime={_format_s(uptime)}  dogs={len(data.get('dogs',[]))}")


def cmd_lod() -> None:
    data = _api_get("/lod")
    if data is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)
    lod_val = data.get("current_lod", 0)
    print(_lod_str(lod_val) + f"  {_c('dim', data.get('description',''))}")


def cmd_loops() -> None:
    print()
    print(_c("bold", "  CYNIC FEEDBACK LOOPS — completion matrix"))
    print()
    for loop_name, (pct, note) in _LOOPS.items():
        bar = _bar(pct, max_score=100.0, width=12)
        col = "green" if pct >= 80 else ("cyan" if pct >= 61 else ("yellow" if pct >= 42 else "red"))
        print(f"  {_c(col, bar)} {pct:3d}%  {_c('bold', loop_name)}")
        print(f"         {_c('dim', note)}")
    print()


def _format_s(s: float) -> str:
    if s < 60:
        return f"{s:.0f}s"
    if s < 3600:
        return f"{s/60:.1f}m"
    return f"{s/3600:.1f}h"


# ── Entry point ────────────────────────────────────────────────────────────

def main() -> None:
    args = sys.argv[1:]
    cmd  = args[0] if args else "status"

    dispatch = {
        "status": cmd_status,
        "health": cmd_health,
        "lod":    cmd_lod,
        "loops":  cmd_loops,
    }

    fn = dispatch.get(cmd)
    if fn is None:
        print(f"*head tilt* Unknown command: {cmd}")
        print(f"  Available: {', '.join(dispatch)}")
        sys.exit(1)

    fn()


if __name__ == "__main__":
    main()
