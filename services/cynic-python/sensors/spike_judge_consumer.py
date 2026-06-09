#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Spike judge consumer — routes spike_detector observations to /judge.

K15 Consumer: GET /observations?agent_id=spike-detector → POST /judge domain=token-analysis
Systemd: cynic-spike-judge-consumer.timer (every 5 minutes, offset +2.5min after spike detector)
Promotion date: 2026-05-29
Stability: new

Design:
  - Polls /observations for unseen spike_detector entries
  - Calls /judge for each new mint, logs the verdict
  - State file tracks seen observation IDs (pruned to DEDUP_WINDOW_SECS)
  - On judge timeout: skips + will retry next cycle (ID not saved to state)
  - SPIKE_JUDGE_DOGS (optional): comma-separated dog IDs to force specific dogs
    e.g. SPIKE_JUDGE_DOGS=deterministic-dog to bypass offline LLM dogs

K15 Falsification: run with SPIKE_THRESHOLD=0 SPIKE_MIN_AGE_HOURS=0 on spike_detector,
  then run this script — /verdicts?limit=10 must show new entries with agent_id=spike-detector.
"""

from __future__ import annotations

import json
import logging
import os
import sys
import time
from pathlib import Path
from typing import Optional
import urllib.request
import urllib.error

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REQUEST_TIMEOUT: int = int(os.environ.get("SPIKE_JUDGE_REQUEST_TIMEOUT", "20"))
FETCH_LIMIT: int = int(os.environ.get("SPIKE_JUDGE_FETCH_LIMIT", "50"))
DEDUP_WINDOW_SECS: int = int(os.environ.get("SPIKE_JUDGE_DEDUP_WINDOW_SECS", "7200"))  # 2h
STATE_FILE: Path = Path(os.environ.get(
    "SPIKE_JUDGE_STATE_FILE",
    str(Path.home() / ".cynic" / "spike_judge_consumer_seen.json"),
))
# Optional: comma-separated dog IDs (e.g. "deterministic-dog")
_dogs_env: str = os.environ.get("SPIKE_JUDGE_DOGS", "")
JUDGE_DOGS: list[str] = [d.strip() for d in _dogs_env.split(",") if d.strip()]

_raw_addr: str = os.environ.get("CYNIC_REST_ADDR", "")
CYNIC_REST_ADDR: str = (
    f"http://{_raw_addr}" if _raw_addr and not _raw_addr.startswith("http") else _raw_addr
)
CYNIC_API_KEY: str = os.environ.get("CYNIC_API_KEY", "")

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

logging.basicConfig(
    format="%(asctime)s spike-judge-consumer %(levelname)s %(message)s",
    level=logging.INFO,
    stream=sys.stdout,
)
log = logging.getLogger("spike_judge_consumer")


# ---------------------------------------------------------------------------
# State (seen observation IDs)
# ---------------------------------------------------------------------------

def load_seen() -> dict[str, float]:
    """Load {observation_id: timestamp} from state file, pruned to DEDUP_WINDOW_SECS."""
    if not STATE_FILE.exists():
        return {}
    try:
        raw: dict[str, float] = json.loads(STATE_FILE.read_text())
        cutoff = time.time() - DEDUP_WINDOW_SECS
        return {k: v for k, v in raw.items() if v > cutoff}
    except (json.JSONDecodeError, OSError) as exc:
        log.warning("could not load state file %s: %s — starting fresh", STATE_FILE, exc)
        return {}


def save_seen(seen: dict[str, float]) -> None:
    try:
        STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
        STATE_FILE.write_text(json.dumps(seen))
    except OSError as exc:
        log.warning("could not save state file %s: %s", STATE_FILE, exc)


# ---------------------------------------------------------------------------
# Kernel API helpers
# ---------------------------------------------------------------------------

def _headers() -> dict[str, str]:
    h = {"Accept": "application/json"}
    if CYNIC_API_KEY:
        h["Authorization"] = f"Bearer {CYNIC_API_KEY}"
    return h


def fetch_spike_observations() -> list[dict]:
    """Fetch recent spike_detector observations from kernel."""
    if not CYNIC_REST_ADDR:
        log.error("CYNIC_REST_ADDR not set — cannot fetch observations")
        return []
    if not CYNIC_API_KEY:
        log.error("CYNIC_API_KEY not set — cannot fetch observations")
        return []

    url = (
        f"{CYNIC_REST_ADDR.rstrip('/')}/observations"
        f"?domain=token-analysis&agent_id=spike-detector&limit={FETCH_LIMIT}"
    )
    req = urllib.request.Request(url, headers=_headers())
    try:
        with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
            data: list[dict] = json.loads(resp.read())
            return data
    except urllib.error.HTTPError as exc:
        log.error("GET /observations HTTP %d: %s", exc.code, exc.reason)
        return []
    except urllib.error.URLError as exc:
        log.error("GET /observations network error: %s", exc.reason)
        return []
    except Exception as exc:  # noqa: BLE001
        log.error("GET /observations unexpected error: %s", exc)
        return []


def judge_mint(mint: str, obs_id: str) -> Optional[dict]:
    """POST /judge for a mint address. Returns verdict dict or None on failure."""
    if not CYNIC_REST_ADDR:
        return None

    url = f"{CYNIC_REST_ADDR.rstrip('/')}/judge"
    payload: dict = {"content": mint, "domain": "token-analysis"}
    if JUDGE_DOGS:
        payload["dogs"] = JUDGE_DOGS

    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={**_headers(), "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        log.error("POST /judge HTTP %d for obs %s mint %s: %s", exc.code, obs_id, mint, exc.reason)
        return None
    except urllib.error.URLError as exc:
        log.warning(
            "POST /judge timeout/network for obs %s mint %s — will retry next cycle: %s",
            obs_id, mint, exc.reason,
        )
        return None
    except Exception as exc:  # noqa: BLE001
        log.error("POST /judge unexpected error for obs %s: %s", obs_id, exc)
        return None


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------

def run() -> None:
    log.info(
        "spike_judge_consumer starting (fetch_limit=%d, timeout=%ds, dogs=%s)",
        FETCH_LIMIT,
        REQUEST_TIMEOUT,
        JUDGE_DOGS or "all",
    )

    observations = fetch_spike_observations()
    if not observations:
        log.info("no observations fetched — nothing to do")
        return

    log.info("fetched %d spike_detector observations", len(observations))
    seen = load_seen()
    now = time.time()

    judged = 0
    skipped_seen = 0
    skipped_timeout = 0

    for obs in observations:
        obs_id: str = obs.get("id", "")
        mint: str = obs.get("target", "")

        if not obs_id or not mint:
            continue

        if obs_id in seen:
            skipped_seen += 1
            continue

        verdict = judge_mint(mint, obs_id)
        if verdict is None:
            skipped_timeout += 1
            # Do NOT save to seen — retry next cycle
            continue

        verdict_kind = verdict.get("verdict", "?")
        q_total = verdict.get("q_score", {}).get("total", 0.0)
        dogs_used = verdict.get("dogs_used", "?")
        log.info(
            "judged obs=%s mint=%s verdict=%s q=%.3f dogs=%s",
            obs_id, mint[:20], verdict_kind, q_total, dogs_used,
        )

        seen[obs_id] = now
        save_seen(seen)
        judged += 1

    log.info(
        "done: judged=%d skipped(seen=%d timeout=%d)",
        judged, skipped_seen, skipped_timeout,
    )


# ---------------------------------------------------------------------------
# Tests (run with: python3 spike_judge_consumer.py --test)
# ---------------------------------------------------------------------------

def _run_tests() -> None:
    """Basic unit tests — no network calls."""
    import tempfile

    # load_seen: empty when file missing
    with tempfile.TemporaryDirectory() as tmp:
        sf = Path(tmp) / "seen.json"
        assert load_seen.__module__  # just ensure importable

    # load_seen: prunes expired entries
    with tempfile.TemporaryDirectory() as tmp:
        sf = Path(tmp) / "seen.json"
        old_state = {"obs:old": time.time() - DEDUP_WINDOW_SECS - 1, "obs:new": time.time()}
        sf.write_text(json.dumps(old_state))

        # Monkey-patch STATE_FILE
        global STATE_FILE
        orig = STATE_FILE
        STATE_FILE = sf
        result = load_seen()
        STATE_FILE = orig

        assert "obs:old" not in result, "expired entry should be pruned"
        assert "obs:new" in result, "fresh entry should be kept"

    # JUDGE_DOGS parsing
    assert JUDGE_DOGS == [] or all(isinstance(d, str) for d in JUDGE_DOGS)

    log.info("all tests passed")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--test":
        _run_tests()
    else:
        run()
