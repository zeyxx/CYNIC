#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Daily telemetry digest — aggregates 12 organism metrics into one kernel observation.

K15 Consumer: kernel /observations (domain=telemetry), session-init reads for context injection
Systemd: cynic-telemetry-digest.timer (daily 07:00 UTC)
Promotion date: 2026-05-18 (built as Tier 2 from inception — clear consumer from day 1)

Metrics collected:
  1. kernel_alive         — /health HTTP status
  2. dogs_active          — count + circuit states from /health
  3. observations_24h     — count from /observations
  4. verdicts_24h         — count + verdict distribution from /verdicts
  5. hermes_cron_status   — systemd timer success/fail from journalctl
  6. hermes_agent_ids     — distinct agent_ids in recent observations
  7. obs_volume_by_agent  — observation count per agent_id
  8. watchdog_restarts    — restart count from journalctl
  9. crystal_state        — total/forming/crystallized from /health
 10. usage_cost           — token consumption from /usage
 11. storage_state        — connected/namespace from /health
 12. alerts_active        — count + severities from /health

Failure mode: If kernel unreachable, logs error and exits 1 (systemd captures).
"""
from __future__ import annotations

__version__ = "1.0.0"

import json
import logging
import os
import subprocess
import sys
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from typing import Any

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stderr)],
)
log = logging.getLogger("telemetry-digest")


def env_required(name: str) -> str:
    val = os.environ.get(name, "")
    if not val:
        log.error("Missing required env var: %s", name)
        sys.exit(1)
    return val


KERNEL_ADDR = env_required("CYNIC_REST_ADDR")
API_KEY = env_required("CYNIC_API_KEY")
BASE = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"


def api_get(path: str, timeout: int = 10) -> Any:
    """GET from kernel with Bearer auth. Accepts 503 (degraded) as valid JSON."""
    url = f"{BASE}{path}"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {API_KEY}"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        # /health returns 503 for degraded/critical — still valid JSON
        if e.code == 503:
            try:
                return json.loads(e.read().decode())
            except (json.JSONDecodeError, Exception):
                pass
        log.warning("GET %s failed: HTTP %d", path, e.code)
        return None
    except (urllib.error.URLError, TimeoutError) as e:
        log.warning("GET %s failed: %s", path, e)
        return None


def api_post(path: str, body: dict[str, Any], timeout: int = 10) -> bool:
    """POST JSON to kernel with Bearer auth."""
    url = f"{BASE}{path}"
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        url,
        data=data,
        headers={
            "Authorization": f"Bearer {API_KEY}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status in (200, 201, 202)
    except (urllib.error.URLError, TimeoutError) as e:
        log.warning("POST %s failed: %s", path, e)
        return False


def journalctl_count(unit: str, since: str, grep_pattern: str | None = None) -> int:
    """Count journal entries for a unit since a timestamp."""
    cmd = ["journalctl", "--user", "-u", unit, f"--since={since}", "--no-pager", "-q"]
    if grep_pattern:
        cmd.extend(["--grep", grep_pattern])
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=15,
        )
        lines = [l for l in result.stdout.strip().split("\n") if l.strip()]
        return len(lines)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return -1


def systemd_timer_status(unit: str) -> dict[str, Any]:
    """Check if a systemd timer's service ran successfully today."""
    try:
        result = subprocess.run(
            ["systemctl", "--user", "show", unit, "--property=ActiveState,Result,ExecMainStartTimestamp"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        props: dict[str, str] = {}
        for line in result.stdout.strip().split("\n"):
            if "=" in line:
                k, v = line.split("=", 1)
                props[k.strip()] = v.strip()
        return {
            "active": props.get("ActiveState", "unknown"),
            "result": props.get("Result", "unknown"),
            "last_start": props.get("ExecMainStartTimestamp", ""),
        }
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return {"active": "error", "result": "error", "last_start": ""}


def collect_health() -> dict[str, Any]:
    """Metric 1 (kernel_alive), 2 (dogs), 9 (crystals), 11 (storage), 12 (alerts)."""
    health = api_get("/health")
    if health is None:
        return {
            "kernel_alive": False,
            "dogs": [],
            "crystals": {},
            "storage": "unreachable",
            "alerts": [],
            "uptime_seconds": 0,
        }
    dogs = health.get("dogs", [])
    return {
        "kernel_alive": True,
        "status": health.get("status", "unknown"),
        "dogs_active": sum(1 for d in dogs if d.get("circuit") != "open"),
        "dogs_total": len(dogs),
        "dogs": [
            {"id": d.get("id", "?"), "circuit": d.get("circuit", "?"), "failures": d.get("failures", 0)}
            for d in dogs
        ],
        "crystals": health.get("crystals", {}),
        "storage": health.get("storage", "unknown"),
        "storage_namespace": health.get("storage_namespace", ""),
        "alerts_count": len(health.get("alerts", [])),
        "alerts_critical": sum(1 for a in health.get("alerts", []) if a.get("severity") == "critical"),
        "alerts_warning": sum(1 for a in health.get("alerts", []) if a.get("severity") == "warning"),
        "alerts": [
            {"kind": a.get("kind", "?"), "severity": a.get("severity", "?")}
            for a in health.get("alerts", [])
        ],
        "uptime_seconds": health.get("uptime_seconds", 0),
        "background_tasks": [
            {"name": t.get("name", "?"), "status": t.get("status", "?")}
            for t in health.get("background_tasks", [])
        ],
    }


def collect_verdicts() -> dict[str, Any]:
    """Metric 4: verdict count + distribution (last 24h approximation via limit)."""
    verdicts = api_get("/verdicts?limit=200")
    if not verdicts:
        return {"total": 0, "distribution": {}, "domains": {}}

    now = datetime.now(timezone.utc)
    cutoff = now - timedelta(hours=24)

    today: list[dict[str, Any]] = []
    for v in verdicts:
        ts_str = v.get("timestamp", "")
        try:
            ts = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
            if ts >= cutoff:
                today.append(v)
        except (ValueError, TypeError):
            continue

    dist: dict[str, int] = {}
    domains: dict[str, int] = {}
    for v in today:
        verdict = v.get("verdict", "unknown")
        dist[verdict] = dist.get(verdict, 0) + 1
        domain = v.get("domain", "unknown")
        domains[domain] = domains.get(domain, 0) + 1

    return {"total_24h": len(today), "distribution": dist, "domains": domains}


def collect_observations() -> dict[str, Any]:
    """Metric 3, 6, 7: observation count, agent_ids, volume by agent."""
    obs = api_get("/observations?limit=100")
    if not obs:
        return {"total": 0, "by_agent": {}, "by_domain": {}}

    now = datetime.now(timezone.utc)
    cutoff = now - timedelta(hours=24)

    by_agent: dict[str, int] = {}
    by_domain: dict[str, int] = {}
    count_24h = 0

    for o in obs:
        ts_str = o.get("created_at", "")
        try:
            ts = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
            if ts < cutoff:
                continue
        except (ValueError, TypeError):
            continue

        count_24h += 1
        agent = o.get("agent_id", "unknown")
        by_agent[agent] = by_agent.get(agent, 0) + 1
        domain = o.get("domain", "general")
        by_domain[domain] = by_domain.get(domain, 0) + 1

    capped = len(obs) >= 100  # API max is 100 — true count may be higher
    return {"total_24h": count_24h, "capped": capped, "by_agent": by_agent, "by_domain": by_domain}


def collect_usage() -> dict[str, Any]:
    """Metric 10: token consumption + cost."""
    usage = api_get("/usage")
    if not usage:
        return {"total_tokens": 0, "total_requests": 0, "estimated_cost_usd": 0.0}
    return {
        "total_tokens": usage.get("total_tokens", 0),
        "total_requests": usage.get("total_requests", 0),
        "estimated_cost_usd": usage.get("estimated_cost_usd", 0.0),
    }


def collect_hermes_crons() -> dict[str, Any]:
    """Metric 5: Hermes cron status from systemd."""
    timers = [
        "hermes-curation",
        "hermes-search-generator",
        "hermes-feedback-loop",
        "hermes-k15-consumer",
        "hermes-gemini-briefing",
        "hermes-data-organism",
    ]
    results: dict[str, dict[str, Any]] = {}
    for t in timers:
        results[t] = systemd_timer_status(f"{t}.service")

    ok = sum(1 for r in results.values() if r.get("result") == "success")
    return {"total": len(timers), "success": ok, "failed": len(timers) - ok, "detail": results}


def collect_watchdog() -> dict[str, Any]:
    """Metric 8: watchdog restart count (last 24h)."""
    since = (datetime.now(timezone.utc) - timedelta(hours=24)).strftime("%Y-%m-%d %H:%M:%S")
    total_runs = journalctl_count("surreal-watchdog.service", since, "Starting")
    # Actual restarts log "FAILED — restarting"
    actual_restarts = journalctl_count("surreal-watchdog.service", since, "FAILED")
    return {"runs_24h": total_runs, "restarts_24h": actual_restarts}


def build_digest() -> dict[str, Any]:
    """Assemble all 12 metrics into one digest."""
    now = datetime.now(timezone.utc)
    log.info("Collecting telemetry digest at %s", now.isoformat())

    health = collect_health()
    verdicts = collect_verdicts()
    observations = collect_observations()
    usage = collect_usage()
    hermes = collect_hermes_crons()
    watchdog = collect_watchdog()

    digest = {
        "timestamp": now.isoformat(),
        "version": __version__,
        # 1. Kernel alive
        "kernel_alive": health.get("kernel_alive", False),
        "kernel_status": health.get("status", "unknown"),
        "uptime_seconds": health.get("uptime_seconds", 0),
        # 2. Dogs active
        "dogs_active": health.get("dogs_active", 0),
        "dogs_total": health.get("dogs_total", 0),
        "dogs": health.get("dogs", []),
        # 3. Observations (24h)
        "observations_24h": observations.get("total_24h", 0),
        "observations_capped": observations.get("capped", False),
        "obs_by_agent": observations.get("by_agent", {}),
        "obs_by_domain": observations.get("by_domain", {}),
        # 4. Verdicts (24h)
        "verdicts_24h": verdicts.get("total_24h", 0),
        "verdict_distribution": verdicts.get("distribution", {}),
        "verdict_domains": verdicts.get("domains", {}),
        # 5. Hermes cron status
        "hermes_crons_ok": hermes.get("success", 0),
        "hermes_crons_total": hermes.get("total", 0),
        "hermes_crons_failed": hermes.get("failed", 0),
        "hermes_crons": hermes.get("detail", {}),
        # 8. Watchdog restarts
        "watchdog_runs_24h": watchdog.get("runs_24h", 0),
        "watchdog_restarts_24h": watchdog.get("restarts_24h", 0),
        # 9. Crystal state
        "crystals": health.get("crystals", {}),
        # 10. Usage / cost
        "usage": usage,
        # 11. Storage state
        "storage": health.get("storage", "unknown"),
        # 12. Alerts
        "alerts_count": health.get("alerts_count", 0),
        "alerts_critical": health.get("alerts_critical", 0),
        "alerts_warning": health.get("alerts_warning", 0),
        "alerts": health.get("alerts", []),
        # Background tasks
        "background_tasks": health.get("background_tasks", []),
    }
    return digest


def post_digest(digest: dict[str, Any]) -> bool:
    """Push digest to kernel as a telemetry observation."""
    # Compact the digest for the context field (200 char limit on context)
    summary = (
        f"K:{1 if digest['kernel_alive'] else 0} "
        f"D:{digest['dogs_active']}/{digest['dogs_total']} "
        f"V:{digest['verdicts_24h']} "
        f"O:{digest['observations_24h']} "
        f"H:{digest['hermes_crons_ok']}/{digest['hermes_crons_total']} "
        f"Wr:{digest['watchdog_runs_24h']}/{digest['watchdog_restarts_24h']}r "
        f"A:{digest['alerts_critical']}c/{digest['alerts_warning']}w"
    )

    body = {
        "tool": "telemetry_digest",
        "target": "daily-digest",
        "domain": "telemetry",
        "status": "success" if digest["kernel_alive"] else "error",
        "context": summary[:200],
        "agent_id": "cron-telemetry-digest",
        "tags": ["telemetry", "daily-digest", "cron"],
    }

    ok = api_post("/observe", body)
    if ok:
        log.info("Digest posted to kernel: %s", summary)
    else:
        log.error("Failed to post digest to kernel")
    return ok


def main() -> None:
    log.info("daily_telemetry_digest v%s starting", __version__)

    digest = build_digest()

    # Always print to stdout (captured by journalctl)
    print(json.dumps(digest, indent=2, default=str))

    # Post to kernel
    if not post_digest(digest):
        log.error("Digest posting failed — kernel may be unreachable")
        sys.exit(1)

    log.info("Done. %d metrics collected.", 12)


if __name__ == "__main__":
    main()
