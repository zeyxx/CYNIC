"""
Tier 1 EXPERIMENTAL: CHAOS baseline analysis of LLM compliance and behavior.

Research question: What does the organism's behavioral data actually show
                   across 6 compliance dimensions before we impose structure?
Success condition: Produces a baseline report with measured distributions,
                   not hardcoded categories. CHAOS before MATRIX.
Timeline: One-shot analysis, no pipeline.
Owned by: @T
Status: ACTIVE (2026-05-14)

Note: If not promoted to Tier 2 by 2026-06-14, delete.
"""

from __future__ import annotations

import json
import os
import sqlite3
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

CLAUDE_SESSIONS_DIR = Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC"
RTK_DB = Path.home() / ".local" / "share" / "rtk" / "history.db"
PROMPT_HISTORY = Path.home() / ".claude" / "history.jsonl"
GEMINI_CHATS_DIR = Path.home() / ".gemini" / "tmp" / "cynic" / "chats"
_raw_addr = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
KERNEL_ADDR = _raw_addr if _raw_addr.startswith("http") else f"http://{_raw_addr}"
KERNEL_KEY = os.environ.get("CYNIC_API_KEY", "")

REPORT_DIR = Path(__file__).parent / "reports"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _ts_to_dt(ts: str | int | float | None) -> datetime | None:
    if ts is None:
        return None
    if isinstance(ts, (int, float)):
        if ts > 1e12:
            ts = ts / 1000.0
        return datetime.fromtimestamp(ts, tz=timezone.utc)
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except (ValueError, AttributeError):
        return None


def _safe_div(a: float, b: float) -> float:
    return a / b if b else 0.0


def _percentile(values: list[float], p: float) -> float:
    if not values:
        return 0.0
    s = sorted(values)
    k = (len(s) - 1) * p
    f = int(k)
    c = f + 1
    if c >= len(s):
        return s[f]
    return s[f] + (k - f) * (s[c] - s[f])


def _kernel_get(path: str, params: str = "") -> Any:
    """GET from kernel REST API. Returns parsed JSON or None.
    Accepts 503 (degraded) as valid — kernel returns full health even when degraded."""
    import urllib.request
    import urllib.error

    url = f"{KERNEL_ADDR}{path}"
    if params:
        url += f"?{params}"
    req = urllib.request.Request(url)
    if KERNEL_KEY:
        req.add_header("Authorization", f"Bearer {KERNEL_KEY}")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        # 503 = degraded but still returns valid JSON body
        if e.code in (503, 200):
            try:
                return json.loads(e.read().decode())
            except (json.JSONDecodeError, AttributeError):
                pass
        print(f"  [warn] kernel GET {path} failed: {e}", file=sys.stderr)
        return None
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
        print(f"  [warn] kernel GET {path} failed: {e}", file=sys.stderr)
        return None


# ---------------------------------------------------------------------------
# 1. Claude Code Session JSONL Parsing
# ---------------------------------------------------------------------------

def parse_claude_sessions() -> dict[str, Any]:
    """Parse all Claude Code session JSONL files."""
    print("  Parsing Claude Code sessions...")

    sessions: dict[str, list[dict]] = defaultdict(list)
    jsonl_files = sorted(CLAUDE_SESSIONS_DIR.glob("*.jsonl"))

    total_records = 0
    parse_errors = 0

    for fpath in jsonl_files:
        session_id = fpath.stem
        with open(fpath, encoding="utf-8") as f:
            for line in f:
                total_records += 1
                try:
                    record = json.loads(line)
                    sessions[session_id].append(record)
                except json.JSONDecodeError:
                    parse_errors += 1

    # --- Per-session metrics ---
    session_metrics: list[dict[str, Any]] = []

    for sid, records in sessions.items():
        turns = [r for r in records if r.get("type") in ("user", "assistant")]
        assistant_turns = [r for r in turns if r["type"] == "assistant"]
        user_turns = [r for r in turns if r["type"] == "user"]

        # Timestamps
        timestamps = [_ts_to_dt(r.get("timestamp")) for r in turns]
        timestamps = [t for t in timestamps if t is not None]
        if len(timestamps) < 2:
            duration_min = 0.0
        else:
            duration_min = (max(timestamps) - min(timestamps)).total_seconds() / 60.0

        start_time = min(timestamps) if timestamps else None

        # Token accounting
        total_input = 0
        total_output = 0
        total_cache_create = 0
        total_cache_read = 0
        model_turns: Counter[str] = Counter()

        for r in assistant_turns:
            msg = r.get("message", {})
            usage = msg.get("usage", {})
            total_input += usage.get("input_tokens", 0)
            total_output += usage.get("output_tokens", 0)
            total_cache_create += usage.get("cache_creation_input_tokens", 0)
            total_cache_read += usage.get("cache_read_input_tokens", 0)
            model = msg.get("model", "unknown")
            model_turns[model] += 1

        # Tool call extraction
        tool_calls: Counter[str] = Counter()
        tool_sequence: list[tuple[str, int]] = []  # (tool_name, turn_index)

        for idx, r in enumerate(turns):
            if r.get("type") == "assistant":
                for content in r.get("message", {}).get("content", []):
                    if content.get("type") == "tool_use":
                        name = content.get("name", "unknown")
                        tool_calls[name] += 1
                        tool_sequence.append((name, idx))

        # Branch tracking
        branches = set()
        for r in turns:
            branch = r.get("gitBranch")
            if branch:
                branches.add(branch)

        edits_on_main = 0
        first_edit_branch = None
        for r in turns:
            if r.get("type") == "assistant":
                for content in r.get("message", {}).get("content", []):
                    if content.get("type") == "tool_use" and content.get("name") in ("Edit", "Write"):
                        branch = r.get("gitBranch", "")
                        if first_edit_branch is None:
                            first_edit_branch = branch
                        if branch in ("main", "master"):
                            edits_on_main += 1

        # Read-before-Edit compliance
        read_files: set[str] = set()
        edit_without_read = 0
        total_edits = 0

        for r in turns:
            if r.get("type") == "assistant":
                for content in r.get("message", {}).get("content", []):
                    if content.get("type") != "tool_use":
                        continue
                    name = content.get("name", "")
                    inp = content.get("input", {})
                    if name == "Read":
                        fp = inp.get("file_path", "")
                        if fp:
                            read_files.add(fp)
                    elif name in ("Edit", "Write"):
                        total_edits += 1
                        fp = inp.get("file_path", "")
                        if fp and fp not in read_files:
                            edit_without_read += 1

        # Sidechain (subagent) analysis
        sidechain_turns = sum(1 for r in turns if r.get("isSidechain"))

        session_metrics.append({
            "session_id": sid,
            "start_time": start_time.isoformat() if start_time else None,
            "duration_min": round(duration_min, 1),
            "total_turns": len(turns),
            "user_turns": len(user_turns),
            "assistant_turns": len(assistant_turns),
            "sidechain_turns": sidechain_turns,
            "input_tokens": total_input,
            "output_tokens": total_output,
            "cache_create_tokens": total_cache_create,
            "cache_read_tokens": total_cache_read,
            "models": dict(model_turns),
            "tool_calls": dict(tool_calls),
            "tool_call_count": sum(tool_calls.values()),
            "branches": sorted(branches),
            "edits_on_main": edits_on_main,
            "first_edit_branch": first_edit_branch,
            "total_edits": total_edits,
            "edit_without_read": edit_without_read,
            "read_before_edit_ratio": round(
                _safe_div(total_edits - edit_without_read, total_edits), 3
            ),
        })

    # --- Aggregates ---
    all_tool_calls: Counter[str] = Counter()
    all_models: Counter[str] = Counter()
    for sm in session_metrics:
        all_tool_calls.update(sm["tool_calls"])
        all_models.update(sm["models"])

    total_input_all = sum(s["input_tokens"] for s in session_metrics)
    total_output_all = sum(s["output_tokens"] for s in session_metrics)
    total_cache_create_all = sum(s["cache_create_tokens"] for s in session_metrics)
    total_cache_read_all = sum(s["cache_read_tokens"] for s in session_metrics)
    durations = [s["duration_min"] for s in session_metrics if s["duration_min"] > 0]

    return {
        "source": "claude_code_sessions",
        "total_sessions": len(sessions),
        "total_records": total_records,
        "parse_errors": parse_errors,
        "aggregate": {
            "total_input_tokens": total_input_all,
            "total_output_tokens": total_output_all,
            "total_cache_create_tokens": total_cache_create_all,
            "total_cache_read_tokens": total_cache_read_all,
            "cache_hit_ratio": round(
                _safe_div(total_cache_read_all, total_cache_read_all + total_cache_create_all), 4
            ),
            "output_input_ratio": round(_safe_div(total_output_all, total_input_all), 4),
            "models": dict(all_models.most_common()),
            "tool_calls": dict(all_tool_calls.most_common(30)),
            "session_duration_p50": round(_percentile(durations, 0.5), 1),
            "session_duration_p90": round(_percentile(durations, 0.9), 1),
            "session_duration_max": round(max(durations) if durations else 0, 1),
        },
        "compliance": {
            "sessions_with_edit_on_main": sum(
                1 for s in session_metrics if s["edits_on_main"] > 0
            ),
            "total_edits_on_main": sum(s["edits_on_main"] for s in session_metrics),
            "avg_read_before_edit_ratio": round(
                _safe_div(
                    sum(s["read_before_edit_ratio"] for s in session_metrics if s["total_edits"] > 0),
                    sum(1 for s in session_metrics if s["total_edits"] > 0),
                ),
                3,
            ),
            "sessions_branched_before_first_edit": sum(
                1
                for s in session_metrics
                if s["first_edit_branch"] and s["first_edit_branch"] not in ("main", "master")
            ),
            "sessions_with_edits": sum(1 for s in session_metrics if s["total_edits"] > 0),
        },
        "sessions": session_metrics,
    }


# ---------------------------------------------------------------------------
# 2. RTK Metabolic Data
# ---------------------------------------------------------------------------

def parse_rtk() -> dict[str, Any]:
    """Parse RTK SQLite database for metabolic analysis."""
    print("  Parsing RTK history...")

    if not RTK_DB.exists():
        return {"source": "rtk", "error": "database not found"}

    conn = sqlite3.connect(str(RTK_DB))
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()

    # Total stats
    cur.execute("SELECT COUNT(*) as cnt FROM commands")
    total = cur.fetchone()["cnt"]

    cur.execute("""
        SELECT MIN(timestamp) as first_ts, MAX(timestamp) as last_ts,
               SUM(input_tokens) as total_in, SUM(output_tokens) as total_out,
               SUM(saved_tokens) as total_saved,
               AVG(savings_pct) as avg_savings,
               AVG(exec_time_ms) as avg_exec_ms
        FROM commands
    """)
    stats = dict(cur.fetchone())

    # Top commands by frequency (anti-pattern detection)
    cur.execute("""
        SELECT original_cmd, COUNT(*) as cnt,
               AVG(savings_pct) as avg_savings,
               SUM(saved_tokens) as total_saved
        FROM commands
        GROUP BY original_cmd
        ORDER BY cnt DESC
        LIMIT 20
    """)
    top_commands = [dict(r) for r in cur.fetchall()]

    # Consecutive identical commands (KC2 detection)
    cur.execute("""
        SELECT original_cmd, timestamp
        FROM commands
        ORDER BY timestamp
    """)
    rows = cur.fetchall()
    max_consecutive = 0
    current_streak = 1
    streak_cmd = ""
    consecutive_patterns: Counter[str] = Counter()

    for i in range(1, len(rows)):
        if rows[i]["original_cmd"] == rows[i - 1]["original_cmd"]:
            current_streak += 1
        else:
            if current_streak >= 3:
                consecutive_patterns[rows[i - 1]["original_cmd"]] += 1
            if current_streak > max_consecutive:
                max_consecutive = current_streak
                streak_cmd = rows[i - 1]["original_cmd"]
            current_streak = 1

    # Release builds (KC3)
    cur.execute("""
        SELECT COUNT(*) as cnt FROM commands
        WHERE original_cmd LIKE '%--release%'
    """)
    release_builds = cur.fetchone()["cnt"]

    # CYNIC project specific
    cur.execute("""
        SELECT COUNT(*) as cnt, SUM(saved_tokens) as saved
        FROM commands
        WHERE project_path LIKE '%CYNIC%'
    """)
    cynic_stats = dict(cur.fetchone())

    conn.close()

    return {
        "source": "rtk",
        "total_commands": total,
        "date_range": {"first": stats["first_ts"], "last": stats["last_ts"]},
        "tokens": {
            "total_input": stats["total_in"],
            "total_output": stats["total_out"],
            "total_saved": stats["total_saved"],
            "avg_savings_pct": round(stats["avg_savings"] or 0, 2),
        },
        "avg_exec_time_ms": round(stats["avg_exec_ms"] or 0, 1),
        "anti_patterns": {
            "release_builds_KC3": release_builds,
            "max_consecutive_identical_KC2": max_consecutive,
            "max_consecutive_cmd": streak_cmd[:80],
            "commands_with_3plus_repeats": sum(consecutive_patterns.values()),
        },
        "cynic_project": cynic_stats,
        "top_commands": top_commands[:15],
    }


# ---------------------------------------------------------------------------
# 3. Kernel Observations, Verdicts, Crystals
# ---------------------------------------------------------------------------

def parse_kernel() -> dict[str, Any]:
    """Query kernel REST API for observations, verdicts, crystals."""
    print("  Querying kernel...")

    result: dict[str, Any] = {"source": "kernel"}

    # Health
    health = _kernel_get("/health")
    if health is None:
        result["error"] = "kernel unreachable"
        return result

    metabolism = health.get("metabolism", {})
    result["health"] = {
        "status": health.get("status"),
        "uptime_seconds": health.get("uptime_seconds"),
        "total_requests": health.get("total_requests"),
        "total_tokens_processed": health.get("total_tokens_processed"),
    }
    result["metabolism"] = {
        "observations_ingested": metabolism.get("observations_ingested", 0),
        "observations_digested": metabolism.get("observations_digested", 0),
        "digestion_ratio": round(
            _safe_div(
                metabolism.get("observations_digested", 0),
                metabolism.get("observations_ingested", 0),
            ),
            4,
        ),
        "crystal_observations": metabolism.get("crystal_observations", 0),
        "verdicts_created": metabolism.get("verdicts_created", 0),
        "verdicts_served": metabolism.get("verdicts_served", 0),
        "rtk_savings_pct": metabolism.get("rtk_savings_pct"),
    }

    # Dogs
    dogs = health.get("dogs", [])
    result["dogs"] = {
        "count": len(dogs),
        "details": [
            {
                "id": d.get("id"),
                "kind": d.get("kind"),
                "circuit": d.get("circuit_state"),
                "calls": d.get("calls"),
                "json_valid_rate": d.get("json_valid_rate"),
                "mean_latency_ms": d.get("mean_latency_ms"),
            }
            for d in dogs
        ],
    }

    # Alerts
    alerts = health.get("alerts", [])
    result["alerts"] = [
        {"severity": a.get("severity"), "message": a.get("message", "")[:100]}
        for a in alerts
    ]

    # Observations (last 100 — API cap)
    obs_data = _kernel_get("/observations", "limit=100")
    observations = []
    if obs_data:
        if isinstance(obs_data, dict) and "observations" in obs_data:
            observations = obs_data["observations"]
        elif isinstance(obs_data, list):
            observations = obs_data

    domain_counts: Counter[str] = Counter()
    tool_counts: Counter[str] = Counter()
    agent_counts: Counter[str] = Counter()
    tag_counts: Counter[str] = Counter()
    for o in observations:
        domain_counts[o.get("domain", "unknown")] += 1
        tool_counts[o.get("tool", "unknown")] += 1
        agent_counts[o.get("agent_id", "unknown")] += 1
        for t in o.get("tags", []):
            tag_counts[t] += 1

    result["observations_sample"] = {
        "returned": len(observations),
        "domains": dict(domain_counts.most_common()),
        "tools": dict(tool_counts.most_common()),
        "agents": dict(agent_counts.most_common()),
        "tags": dict(tag_counts.most_common()),
    }

    # Verdicts (last 20 — API cap)
    verdicts_data = _kernel_get("/verdicts", "limit=20")
    verdicts = []
    if verdicts_data:
        if isinstance(verdicts_data, dict) and "verdicts" in verdicts_data:
            verdicts = verdicts_data["verdicts"]
        elif isinstance(verdicts_data, list):
            verdicts = verdicts_data

    if verdicts:
        q_scores: list[float] = []
        for v in verdicts:
            qs = v.get("q_score")
            if isinstance(qs, dict):
                total = qs.get("total")
                if total is not None:
                    q_scores.append(float(total))
            elif isinstance(qs, (int, float)):
                q_scores.append(float(qs))
        verdict_types: Counter[str] = Counter(v.get("verdict", "unknown") for v in verdicts)

        # Per-axiom analysis — scores may be in "scores", "q_score", or top-level
        axiom_names = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]
        axiom_stats = {}
        for ax in axiom_names:
            vals = []
            for v in verdicts:
                # Try multiple locations where axiom scores might live
                score = None
                for src in (v.get("scores", {}), v.get("q_score", {}), v):
                    if isinstance(src, dict) and ax in src:
                        candidate = src[ax]
                        if isinstance(candidate, (int, float)):
                            score = float(candidate)
                            break
                if score is not None:
                    vals.append(score)
            if vals:
                axiom_stats[ax] = {
                    "mean": round(sum(vals) / len(vals), 3),
                    "min": round(min(vals), 3),
                    "max": round(max(vals), 3),
                    "p50": round(_percentile(vals, 0.5), 3),
                }

        result["verdicts"] = {
            "returned": len(verdicts),
            "total_all_time": metabolism.get("verdicts_created", 0),
            "q_score_mean": round(sum(q_scores) / len(q_scores), 3) if q_scores else 0,
            "q_score_p50": round(_percentile(q_scores, 0.5), 3),
            "q_score_max": round(max(q_scores), 3) if q_scores else 0,
            "verdict_distribution": dict(verdict_types),
            "axiom_stats": axiom_stats,
        }
    else:
        result["verdicts"] = {"returned": 0, "total_all_time": metabolism.get("verdicts_created", 0)}

    # Crystals
    crystals_data = _kernel_get("/crystals", "limit=100")
    crystals = []
    if crystals_data:
        if isinstance(crystals_data, dict) and "crystals" in crystals_data:
            crystals = crystals_data["crystals"]
        elif isinstance(crystals_data, list):
            crystals = crystals_data

    if crystals:
        state_counts: Counter[str] = Counter(c.get("state", "unknown") for c in crystals)
        domain_crystal_counts: Counter[str] = Counter(c.get("domain", "unknown") for c in crystals)
        certainties = [c.get("certainty", 0) for c in crystals if c.get("certainty") is not None]

        result["crystals"] = {
            "total": len(crystals),
            "states": dict(state_counts),
            "domains": dict(domain_crystal_counts.most_common()),
            "certainty_mean": round(sum(certainties) / len(certainties), 3) if certainties else 0,
            "certainty_p50": round(_percentile(certainties, 0.5), 3),
            "canonical_count": state_counts.get("canonical", 0),
        }
    else:
        result["crystals"] = {"total": 0}

    return result


# ---------------------------------------------------------------------------
# 4. Gemini CLI Data
# ---------------------------------------------------------------------------

def parse_gemini() -> dict[str, Any]:
    """Parse Gemini CLI session-*.jsonl chat files."""
    print("  Parsing Gemini data...")

    result: dict[str, Any] = {"source": "gemini_cli"}

    if not GEMINI_CHATS_DIR.exists():
        result["error"] = "chats directory not found"
        return result

    session_files = sorted(GEMINI_CHATS_DIR.glob("session-*.jsonl"))
    total_sessions = len(session_files)
    substantive = 0
    total_user_turns = 0
    total_assistant_turns = 0
    total_tool_calls = 0
    model_counts: Counter[str] = Counter()
    tool_counts: Counter[str] = Counter()
    durations: list[float] = []
    all_timestamps: list[datetime] = []

    for sf in session_files:
        turns: list[dict] = []
        with open(sf, encoding="utf-8") as f:
            for line in f:
                try:
                    r = json.loads(line)
                    # Skip metadata lines ($set updates, session headers without 'type')
                    if "type" not in r:
                        continue
                    turns.append(r)
                except json.JSONDecodeError:
                    pass

        if len(turns) < 2:
            continue

        substantive += 1
        user_count = 0
        assistant_count = 0
        session_tools = 0
        timestamps: list[datetime] = []

        for t in turns:
            ts = _ts_to_dt(t.get("timestamp"))
            if ts:
                timestamps.append(ts)
                all_timestamps.append(ts)

            if t.get("type") == "user":
                user_count += 1
            elif t.get("type") == "assistant":
                assistant_count += 1
                model = t.get("model", "")
                if model:
                    model_counts[model] += 1
                # Count tool calls in content
                for c in t.get("content", []):
                    if isinstance(c, dict) and c.get("type") == "tool_use":
                        tool_counts[c.get("name", "unknown")] += 1
                        session_tools += 1
                # Also check toolCalls field (Gemini format)
                for tc in t.get("toolCalls", []):
                    tool_counts[tc.get("name", "unknown")] += 1
                    session_tools += 1

        total_user_turns += user_count
        total_assistant_turns += assistant_count
        total_tool_calls += session_tools

        if len(timestamps) >= 2:
            dur = (max(timestamps) - min(timestamps)).total_seconds() / 60.0
            durations.append(dur)

    result["sessions"] = {
        "total_files": total_sessions,
        "substantive": substantive,
        "total_user_turns": total_user_turns,
        "total_assistant_turns": total_assistant_turns,
        "total_tool_calls": total_tool_calls,
        "session_duration_p50_min": round(_percentile(durations, 0.5), 1),
        "session_duration_p90_min": round(_percentile(durations, 0.9), 1),
        "models": dict(model_counts.most_common(10)),
        "tool_calls": dict(tool_counts.most_common(20)),
    }

    if all_timestamps:
        result["sessions"]["date_range"] = {
            "first": min(all_timestamps).isoformat(),
            "last": max(all_timestamps).isoformat(),
        }

    return result


# ---------------------------------------------------------------------------
# 5. Hermes Agent Data
# ---------------------------------------------------------------------------

def parse_hermes() -> dict[str, Any]:
    """Parse Hermes organ behavioral data."""
    print("  Parsing Hermes data...")

    result: dict[str, Any] = {"source": "hermes"}
    hermes_dir = Path.home() / ".cynic" / "organs" / "hermes"

    # Verdicts
    verdicts_dir = hermes_dir / "x" / "verdicts"
    if verdicts_dir.exists():
        verdict_files = list(verdicts_dir.glob("*.json"))
        q_scores: list[float] = []
        verdict_types: Counter[str] = Counter()
        domains: Counter[str] = Counter()

        for vf in verdict_files:
            try:
                with open(vf) as f:
                    v = json.load(f)
                vd = v.get("verdict", {})
                q = vd.get("q_score", {}).get("total")
                if q is not None:
                    q_scores.append(q)
                verdict_types[vd.get("verdict", "unknown")] += 1
                domains[vd.get("domain", "unknown")] += 1
            except (json.JSONDecodeError, KeyError):
                pass

        result["tweet_verdicts"] = {
            "total": len(verdict_files),
            "q_score_mean": round(sum(q_scores) / len(q_scores), 3) if q_scores else 0,
            "q_score_p50": round(_percentile(q_scores, 0.5), 3),
            "q_score_p90": round(_percentile(q_scores, 0.9), 3),
            "verdict_distribution": dict(verdict_types.most_common()),
            "domains": dict(domains.most_common()),
        }

    # Dataset
    dataset_file = hermes_dir / "x" / "datasets" / "cynic" / "dataset.v2.jsonl"
    if dataset_file.exists():
        tweet_count = 0
        with open(dataset_file) as f:
            for _ in f:
                tweet_count += 1
        result["tweet_dataset"] = {"total_tweets": tweet_count}

    # Farming log
    farming_log = hermes_dir / "x" / "farming_log.jsonl"
    if farming_log.exists():
        cycles = 0
        domains_farmed: Counter[str] = Counter()
        with open(farming_log) as f:
            for line in f:
                cycles += 1
                try:
                    r = json.loads(line)
                    for s in r.get("searches", []):
                        domains_farmed[s.get("domain", "unknown")] += 1
                except json.JSONDecodeError:
                    pass
        result["farming"] = {
            "total_cycles": cycles,
            "domains_farmed": dict(domains_farmed.most_common()),
        }

    # Behavior log
    behavior_log = hermes_dir / "behavior" / "behavior_log.jsonl"
    if behavior_log.exists():
        total_lines = 0
        action_types: Counter[str] = Counter()
        with open(behavior_log) as f:
            for line in f:
                total_lines += 1
                try:
                    r = json.loads(line)
                    if "type" in r:
                        action_types[r["type"]] += 1
                    elif "action" in r:
                        action_types[r["action"]] += 1
                except json.JSONDecodeError:
                    pass
        result["behavior_log"] = {
            "total_entries": total_lines,
            "action_types": dict(action_types.most_common(15)),
        }

    return result


# ---------------------------------------------------------------------------
# 6. Cross-Agent Synthesis
# ---------------------------------------------------------------------------

def synthesize(
    claude: dict, rtk: dict, kernel: dict, gemini: dict, hermes: dict
) -> dict[str, Any]:
    """Compute cross-cutting compliance dimensions from all sources."""
    print("  Synthesizing compliance dimensions...")

    sessions = claude.get("sessions", [])
    sessions_with_edits = [s for s in sessions if s["total_edits"] > 0]

    # --- 1. Constitutional Compliance ---
    constitutional = {
        "read_before_edit_avg": claude.get("compliance", {}).get(
            "avg_read_before_edit_ratio", 0
        ),
        "branch_before_edit_pct": round(
            _safe_div(
                claude.get("compliance", {}).get("sessions_branched_before_first_edit", 0),
                claude.get("compliance", {}).get("sessions_with_edits", 1),
            ),
            3,
        ),
        "edits_on_main_total": claude.get("compliance", {}).get("total_edits_on_main", 0),
        "sessions_editing_main": claude.get("compliance", {}).get(
            "sessions_with_edit_on_main", 0
        ),
    }

    # --- 2. Metabolic Compliance ---
    durations = [s["duration_min"] for s in sessions if s["duration_min"] > 0]
    tool_counts = [s["tool_call_count"] for s in sessions if s["tool_call_count"] > 0]
    output_per_session = [s["output_tokens"] for s in sessions if s["output_tokens"] > 0]

    metabolic = {
        "total_sessions": len(sessions),
        "tokens_per_session_p50": round(_percentile(output_per_session, 0.5)),
        "tokens_per_session_p90": round(_percentile(output_per_session, 0.9)),
        "cache_hit_ratio": claude.get("aggregate", {}).get("cache_hit_ratio", 0),
        "rtk_avg_savings_pct": rtk.get("tokens", {}).get("avg_savings_pct", 0),
        "rtk_release_builds_KC3": rtk.get("anti_patterns", {}).get("release_builds_KC3", 0),
        "rtk_max_consecutive_KC2": rtk.get("anti_patterns", {}).get(
            "max_consecutive_identical_KC2", 0
        ),
        "tools_per_session_p50": round(_percentile(tool_counts, 0.5)),
        "tools_per_session_p90": round(_percentile(tool_counts, 0.9)),
    }

    # --- 3. Axiomatique (from kernel verdicts) ---
    axiomatique = kernel.get("verdicts", {})

    # --- 4. Temporal ---
    short_sessions = sum(1 for d in durations if d < 10)
    long_sessions = sum(1 for d in durations if d > 120)
    temporal = {
        "session_duration_p50_min": round(_percentile(durations, 0.5), 1),
        "session_duration_p90_min": round(_percentile(durations, 0.9), 1),
        "short_sessions_under_10min": short_sessions,
        "long_sessions_over_2h": long_sessions,
        "digestion_ratio": kernel.get("metabolism", {}).get("digestion_ratio", 0),
    }

    # --- 5. Learning ---
    learning = {
        "crystals_total": kernel.get("crystals", {}).get("total", 0),
        "crystals_canonical": kernel.get("crystals", {}).get("canonical_count", 0),
        "crystals_certainty_mean": kernel.get("crystals", {}).get("certainty_mean", 0),
        "observations_ingested": kernel.get("metabolism", {}).get("observations_ingested", 0),
        "observations_digested": kernel.get("metabolism", {}).get("observations_digested", 0),
        "verdicts_created": kernel.get("metabolism", {}).get("verdicts_created", 0),
        "verdicts_served": kernel.get("metabolism", {}).get("verdicts_served", 0),
        "verdict_serve_ratio": round(
            _safe_div(
                kernel.get("metabolism", {}).get("verdicts_served", 0),
                kernel.get("metabolism", {}).get("verdicts_created", 0),
            ),
            4,
        ),
    }

    # --- 6. Cross-Agent ---
    cross_agent = {
        "claude_sessions": claude.get("total_sessions", 0),
        "gemini_sessions": gemini.get("sessions", {}).get("total_files", 0),
        "gemini_substantive": gemini.get("sessions", {}).get("substantive", 0),
        "hermes_tweet_verdicts": hermes.get("tweet_verdicts", {}).get("total", 0),
        "hermes_farming_cycles": hermes.get("farming", {}).get("total_cycles", 0),
        "hermes_behavior_entries": hermes.get("behavior_log", {}).get("total_entries", 0),
    }

    return {
        "constitutional": constitutional,
        "metabolic": metabolic,
        "axiomatique": axiomatique,
        "temporal": temporal,
        "learning": learning,
        "cross_agent": cross_agent,
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 60)
    print("CYNIC CHAOS Baseline Analysis — 2026-05-14")
    print("=" * 60)
    print()

    # Source env
    global KERNEL_ADDR, KERNEL_KEY
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        with open(env_file) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    if line.startswith("export "):
                        line = line[7:]
                    key, _, val = line.partition("=")
                    val = val.strip("'\"")
                    os.environ[key.strip()] = val
        raw_addr = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
        KERNEL_ADDR = raw_addr if raw_addr.startswith("http") else f"http://{raw_addr}"
        KERNEL_KEY = os.environ.get("CYNIC_API_KEY", "")

    # Parse all sources
    claude = parse_claude_sessions()
    rtk = parse_rtk()
    kernel = parse_kernel()
    gemini = parse_gemini()
    hermes = parse_hermes()

    # Synthesize
    compliance = synthesize(claude, rtk, kernel, gemini, hermes)

    # Build report
    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "epistemic_status": "observed — all values probed from live data, no inference",
        "compliance_dimensions": compliance,
        "sources": {
            "claude_code": {
                k: v for k, v in claude.items() if k != "sessions"
            },
            "rtk": rtk,
            "kernel": kernel,
            "gemini": gemini,
            "hermes": hermes,
        },
    }

    # Write report
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    report_path = REPORT_DIR / "chaos_baseline_2026_05_14.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)

    print()
    print("=" * 60)
    print("COMPLIANCE DIMENSIONS (CHAOS — raw measurements)")
    print("=" * 60)

    for dim_name, dim_data in compliance.items():
        print(f"\n--- {dim_name.upper()} ---")
        if isinstance(dim_data, dict):
            for k, v in dim_data.items():
                if isinstance(v, dict):
                    print(f"  {k}:")
                    for kk, vv in v.items():
                        print(f"    {kk}: {vv}")
                else:
                    print(f"  {k}: {v}")

    print(f"\nReport written to: {report_path}")
    print(f"Report size: {report_path.stat().st_size / 1024:.1f} KB")


if __name__ == "__main__":
    main()
