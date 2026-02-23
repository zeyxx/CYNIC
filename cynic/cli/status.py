"""
CYNIC CLI — `status` command (full judgment dashboard).
"""
from __future__ import annotations

import time

from cynic.cli.utils import (
    _GUIDANCE, _CHECKPOINT, _PENDING,
    _api_get, _read_json,
    _c, _bar, _ago, _format_s, _lod_str, _verdict_str, _section,
    _LOOPS,
)


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
