"""
CYNIC CLI — Terminal status dashboard + human feedback loop (L3).

Usage:
  python -m cynic.cli            → full status (default)
  python -m cynic.cli status     → same
  python -m cynic.cli health     → quick health check only
  python -m cynic.cli lod        → LOD level only
  python -m cynic.cli loops      → 4 feedback loop completion matrix
  python -m cynic.cli review     → interactive accept/reject pending actions (L3)
  python -m cynic.cli watch      → live poll: notify when new actions arrive (L3)
  python -m cynic.cli feedback N → rate last judgment 1-5 (L3)
  python -m cynic.cli probes    → self-improvement proposals from L4 SelfProber
  python -m cynic.cli sdk [N]  → last N SDK sessions (L2 bidirectional loop)

Reads from (fastest path — no server needed):
  ~/.cynic/guidance.json        → last judgment verdict/Q/dogs
  ~/.cynic/session-latest.json  → session checkpoint info
  ~/.cynic/pending_actions.json → proposed action queue

Optionally queries (falls back gracefully if server is down):
  http://localhost:PORT/health
  http://localhost:PORT/lod
  http://localhost:PORT/act/telemetry
  http://localhost:PORT/actions
  http://localhost:PORT/feedback
  http://localhost:PORT/self-probes
  http://localhost:PORT/act/telemetry

φ-bound: confidence never shown above 61.8%.
"""
from __future__ import annotations

import io
import json
import os
import re
import sys
import time
import urllib.request
import urllib.error
from typing import Any, Dict, List, Optional

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
_CYNIC_DIR        = os.path.join(os.path.expanduser("~"), ".cynic")
_GUIDANCE         = os.path.join(_CYNIC_DIR, "guidance.json")
_CHECKPOINT       = os.path.join(_CYNIC_DIR, "session-latest.json")
_PENDING          = os.path.join(_CYNIC_DIR, "pending_actions.json")
_SDK_SESSIONS_LOG = os.path.join(_CYNIC_DIR, "sdk_sessions.jsonl")

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


def _api_post(path: str, body: Optional[Dict] = None) -> Optional[Dict[str, Any]]:
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
    except Exception:
        return None


def _pending_actions() -> tuple[List[Dict], bool]:
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
    except Exception:
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
    "L1 Machine→Actions":   (75, "⚠️  ActionProposer+queue done, no web UI"),
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
    print(_c("bold", f"┌{'─' * (w - 2)}┐"))
    label = f"  {title}"
    print(_c("bold", "│") + _c("cyan", label) + " " * (w - 2 - len(label)) + _c("bold", "│"))
    print(_c("bold", f"├{'─' * (w - 2)}┤"))
    for line in lines:
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


def _print_action(action: Dict, index: int, total: int) -> None:
    """Render one proposed action for the review screen."""
    action_id = action.get("action_id", "?")
    atype     = action.get("action_type", "?")
    verdict   = action.get("verdict", "?")
    priority  = action.get("priority", 3)
    desc      = action.get("description", "")
    prompt    = action.get("prompt", "")[:120]
    ts        = float(action.get("proposed_at", 0))

    atype_col    = _ATYPE_COLOR.get(atype, "white")
    priority_col = _PRIORITY_COLOR.get(priority, "white")
    v_col        = _VERDICT_COLOR.get(verdict, "white")

    print()
    print(_c("bold", f"  [{index}/{total}]  {action_id}  {_ago(ts)}"))
    print(
        f"  {_c(atype_col, atype)}"
        f"  priority={_c(priority_col, str(priority))}"
        f"  verdict={_c(v_col, verdict)}"
    )
    print(f"  {desc}")
    if prompt.strip():
        print(f"  {_c('gray', prompt[:100] + ('…' if len(prompt) >= 100 else ''))}")


def cmd_review() -> None:
    """
    Interactive review of pending proposed actions.

    Shows each PENDING action one at a time.
    Reads from API if server is running; falls back to direct file read.
    Writes accept/reject via API if available; falls back to file mode.

    Keys: [a]ccept  [r]eject  [s]kip  [q]uit
    """
    pending, api_available = _pending_actions()

    if not pending:
        print()
        print(_c("green", "  *tail wag* No pending actions — queue is empty."))
        print()
        return

    # Sort by priority (1=critical first), then by age
    pending.sort(key=lambda a: (a.get("priority", 3), a.get("proposed_at", 0)))

    print()
    print(_c("bold", f"  CYNIC ACTION REVIEW — {len(pending)} pending"))
    mode_note = "API mode" if api_available else _c("orange", "file mode (server offline)")
    print(f"  {_c('dim', mode_note)}  [a]ccept  [r]eject  [s]kip  [q]uit")

    accepted = rejected = skipped = 0

    for i, action in enumerate(pending, 1):
        action_id = action.get("action_id", "?")
        _print_action(action, i, len(pending))
        print()

        try:
            raw = input("  choice > ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print()
            break

        if raw == "q":
            break
        elif raw == "a":
            if api_available:
                result = _api_post(f"/actions/{action_id}/accept")
                ok = result is not None and result.get("accepted")
            else:
                ok = _file_set_status(action_id, "ACCEPTED")
            if ok:
                print(_c("green", "  *tail wag* Accepted"))
                accepted += 1
            else:
                print(_c("red", "  *GROWL* Failed to accept — check server"))
        elif raw == "r":
            if api_available:
                result = _api_post(f"/actions/{action_id}/reject")
                ok = result is not None and result.get("rejected")
            else:
                ok = _file_set_status(action_id, "REJECTED")
            if ok:
                print(_c("orange", "  *head tilt* Rejected"))
                rejected += 1
            else:
                print(_c("red", "  *GROWL* Failed to reject — check server"))
        else:
            print(_c("dim", "  skipped"))
            skipped += 1

    print()
    print(
        f"  *yawn* Done.  accepted={_c('green', str(accepted))}"
        f"  rejected={_c('orange', str(rejected))}"
        f"  skipped={_c('dim', str(skipped))}"
    )
    print()


def cmd_watch() -> None:
    """
    Poll for new pending actions every N seconds.
    Notifies when the queue grows. Ctrl+C to stop.

    Usage: python -m cynic.cli watch [interval_seconds]
    Default interval: 10s.
    """
    args = sys.argv[2:]
    try:
        interval = int(args[0]) if args else 10
    except ValueError:
        interval = 10

    print()
    print(_c("bold", f"  *sniff* CYNIC WATCH — polling every {interval}s  (Ctrl+C to stop)"))
    print(_c("dim",  f"  Run 'python -m cynic.cli review' when actions appear"))
    print()

    last_count = -1
    last_ids: set = set()

    while True:
        try:
            pending, api_ok = _pending_actions()
        except Exception:
            pending, api_ok = [], False

        current_ids = {a.get("action_id") for a in pending}
        new_ids     = current_ids - last_ids
        count       = len(pending)

        if new_ids and last_ids:
            # New actions arrived
            for action_id in new_ids:
                action = next((a for a in pending if a.get("action_id") == action_id), {})
                atype  = action.get("action_type", "?")
                desc   = action.get("description", "")[:60]
                col    = _ATYPE_COLOR.get(atype, "white")
                print(
                    f"  *ears perk* [{_c(col, atype)}] {action_id}  {_c('dim', desc)}"
                )
            print(f"  → {count} pending  run 'cynic.cli review' to process")
        elif count != last_count:
            if count == 0 and last_count > 0:
                print(f"  *tail wag* Queue cleared — all actions processed")
            elif last_count >= 0:
                status_col = "green" if count == 0 else "yellow"
                print(f"  {_c(status_col, str(count))} pending actions  {_c('dim', _ago(time.time()))}")

        last_count = count
        last_ids   = current_ids

        try:
            time.sleep(interval)
        except KeyboardInterrupt:
            print()
            print(_c("dim", "  *yawn* Watch stopped."))
            print()
            break


def cmd_feedback() -> None:
    """
    Rate the last kernel judgment (1=bad … 5=good).

    Usage: python -m cynic.cli feedback [1-5]
    Requires server to be running.

    Example:
      python -m cynic.cli feedback 4   → reward 0.70 → QTable update
    """
    args = sys.argv[2:]
    if not args:
        print()
        print(_c("bold", "  Usage: python -m cynic.cli feedback [1-5]"))
        print()
        print("  Rates the last judgment seen by the kernel.")
        print("  Reward mapping (φ-aligned):")
        for rating, reward in [(1, 0.10), (2, 0.30), (3, 0.50), (4, 0.70), (5, 0.90)]:
            bar = _bar(reward * 100, max_score=100.0, width=8)
            print(f"    {rating}/5 → reward {reward:.2f}  {bar}")
        print()
        sys.exit(1)

    try:
        rating = int(args[0])
    except ValueError:
        print(_c("red", f"*GROWL* Rating must be 1-5, got: {args[0]}"))
        sys.exit(1)

    if not (1 <= rating <= 5):
        print(_c("red", f"*GROWL* Rating out of range: {rating} (must be 1-5)"))
        sys.exit(1)

    result = _api_post("/feedback", {"rating": rating})
    if result is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)

    msg     = result.get("message", "")
    q_value = result.get("q_value", 0.0)
    reward  = result.get("reward", 0.0)
    action  = result.get("action", "?")
    sk      = result.get("state_key", "")[:40]

    reward_col = "green" if reward >= 0.6 else ("cyan" if reward >= 0.4 else "orange")
    print()
    print(_c("bold", f"  *tail wag* Feedback: {rating}/5"))
    print(f"  reward={_c(reward_col, f'{reward:.2f}')}  Q[{action}]={q_value:.4f}")
    print(f"  {_c('dim', sk)}")
    if msg:
        print(f"  {_c('dim', msg[:80])}")
    print()


# Probe dimension → color
_PROBE_DIM_COLOR = {
    "QTABLE":   "orange",
    "ESCORE":   "yellow",
    "CONFIG":   "cyan",
    "RESIDUAL": "dim",
}


def cmd_probes() -> None:
    """
    Show self-improvement proposals generated by the L4 SelfProber.

    SelfProber analyzes QTable, EScore, and Residual patterns on
    EMERGENCE_DETECTED events and produces concrete improvement recommendations.

    Usage: python -m cynic.cli probes [status]
      status: PENDING (default) | all | APPLIED | DISMISSED
    """
    args = sys.argv[2:]
    filter_status = (args[0].upper() if args else "PENDING")

    # Try API first
    path = f"/self-probes" + ("" if filter_status == "PENDING" else f"?status={filter_status}")
    data = _api_get(path)

    if data is None:
        # File fallback: read ~/.cynic/self_proposals.json directly
        _SELF_PROPOSALS = os.path.join(_CYNIC_DIR, "self_proposals.json")
        raw = _read_json(_SELF_PROPOSALS)
        if not isinstance(raw, list):
            print()
            print(_c("gray", "  *sniff* No self-improvement proposals yet."))
            print(_c("dim", "  Run server and trigger a judgment to generate probes."))
            print()
            return
        all_p = raw
        proposals = (
            [p for p in all_p if p.get("status") == filter_status]
            if filter_status != "ALL"
            else all_p
        )
        api_ok = False
    else:
        proposals = data.get("proposals", [])
        api_ok = True

    if not proposals:
        print()
        print(_c("green", f"  *tail wag* No {filter_status.lower()} proposals."))
        print(_c("dim",  f"  {'API' if api_ok else 'file'} mode — probes generated on EMERGENCE_DETECTED"))
        print()
        return

    print()
    mode_note = "API mode" if api_ok else _c("orange", "file mode (server offline)")
    print(_c("bold", f"  CYNIC SELF-PROBES — {len(proposals)} {filter_status.lower()}"))
    print(f"  {_c('dim', mode_note)}")
    print()

    for p in proposals:
        dim     = p.get("dimension", "?")
        target  = p.get("target", "?")
        rec     = p.get("recommendation", "")
        cur     = float(p.get("current_value", 0))
        sug     = float(p.get("suggested_value", 0))
        sev     = float(p.get("severity", 0))
        ts      = float(p.get("proposed_at", 0))
        pid     = p.get("probe_id", "?")
        pattern = p.get("pattern_type", "?")
        status  = p.get("status", "PENDING")

        dim_col  = _PROBE_DIM_COLOR.get(dim, "white")
        sev_col  = "red" if sev >= 0.7 else ("orange" if sev >= 0.4 else "yellow")
        stat_col = "green" if status == "APPLIED" else ("dim" if status == "DISMISSED" else "yellow")

        sev_bar = _bar(sev * 100, max_score=100.0, width=8)

        print(
            f"  {_c(dim_col, f'[{dim}]')}"
            f"  {pid}"
            f"  {_ago(ts)}"
            f"  pattern={_c('dim', pattern)}"
        )
        print(
            f"  target={_c('bold', target[:50])}"
            f"  severity={_c(sev_col, sev_bar)}"
            f"  status={_c(stat_col, status)}"
        )
        print(f"  {rec[:100]}")
        if cur != sug:
            print(f"  {_c('dim', f'current={cur:.4f} → suggested={sug:.4f}')}")
        print()

    # Summary stats
    if data and "stats" in data:
        s = data["stats"]
        print(
            _c("dim",
               f"  total={s.get('proposed_total',0)}"
               f"  pending={s.get('pending',0)}"
               f"  applied={s.get('applied',0)}"
               f"  dismissed={s.get('dismissed',0)}"
            )
        )
    print()


def cmd_sdk() -> None:
    """
    Show recent Claude Code SDK sessions from JSONL persistence (L2).

    Sessions are written to ~/.cynic/sdk_sessions.jsonl after each SDK task.
    This command shows them without needing the server to be running.

    Usage: python -m cynic.cli sdk [N]
      N: number of recent sessions to show (default 10)
    """
    args = sys.argv[2:]
    n = int(args[0]) if args and args[0].isdigit() else 10

    # Try API first (live view)
    data = _api_get(f"/act/telemetry?n={n}")

    if data is not None:
        sessions = data.get("sessions", [])
        stats = data.get("stats", {})
        api_ok = True
    else:
        # File fallback: read JSONL
        sessions = []
        try:
            with open(_SDK_SESSIONS_LOG, encoding="utf-8") as fh:
                all_lines = fh.readlines()
            for line in all_lines[-n:]:
                line = line.strip()
                if line:
                    try:
                        sessions.append(json.loads(line))
                    except Exception:
                        pass
        except FileNotFoundError:
            pass
        stats = {}
        api_ok = False

    print()
    mode_note = "API mode" if api_ok else _c("orange", "file mode (server offline)")
    print(_c("bold", f"  CYNIC SDK SESSIONS — last {len(sessions)}"))
    print(f"  {_c('dim', mode_note)}")
    print()

    if not sessions:
        print(_c("gray", "  *sniff* No SDK sessions recorded yet."))
        print(_c("dim", "  Sessions appear after running /act/execute."))
        print()
        return

    _VERDICT_COLOR = {"BARK": "red", "GROWL": "orange", "WAG": "yellow", "HOWL": "green"}

    for s in sessions:
        verdict   = s.get("output_verdict", "?")
        q_score   = float(s.get("output_q_score", 0))
        task      = (s.get("task") or "")[:60]
        t_type    = s.get("task_type", "?")
        complexity = s.get("complexity", "?")
        cost      = float(s.get("total_cost_usd", 0))
        is_error  = s.get("is_error", False)
        ts        = float(s.get("timestamp", 0))
        reward    = float(s.get("reward", 0))

        vc = _VERDICT_COLOR.get(verdict, "white")
        ec = "red" if is_error else "green"

        q_bar = _bar(q_score, max_score=100.0, width=8)

        print(
            f"  {_c(vc, verdict):<8}"
            f"  Q={_c(vc, q_bar)}"
            f"  {_c(ec, 'ERR' if is_error else 'OK ')}"
            f"  {t_type:<8}"
            f"  {complexity:<8}"
            f"  ${cost:.4f}"
            f"  r={reward:.3f}"
            f"  {_ago(ts)}"
        )
        if task:
            print(f"    {_c('dim', task)}")
    print()

    if stats:
        count = stats.get("count", 0)
        err_r = stats.get("error_rate", 0)
        mean_q = stats.get("mean_q_score", 0)
        total_c = stats.get("total_cost_usd", 0)
        print(
            _c("dim",
               f"  sessions={count}"
               f"  error_rate={err_r:.1%}"
               f"  mean_Q={mean_q:.1f}"
               f"  total_cost=${total_c:.4f}"
            )
        )
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
        "status":   cmd_status,
        "health":   cmd_health,
        "lod":      cmd_lod,
        "loops":    cmd_loops,
        "review":   cmd_review,
        "watch":    cmd_watch,
        "feedback": cmd_feedback,
        "probes":   cmd_probes,
        "sdk":      cmd_sdk,
    }

    fn = dispatch.get(cmd)
    if fn is None:
        print(f"*head tilt* Unknown command: {cmd}")
        print(f"  Available: {', '.join(dispatch)}")
        sys.exit(1)

    fn()


if __name__ == "__main__":
    main()
