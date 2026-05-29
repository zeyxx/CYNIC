#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Spike detector — GeckoTerminal trending Solana pools → CYNIC /observe.

K15 Consumer: POST /observe domain=token-analysis → pipeline → /judge
Systemd: cynic-spike-detector.timer (every 5 minutes)
Promotion date: 2026-05-29 (ported from CultScreener fix #3 analysis)
Stability: new

Spike score formula (ported from CultScreener tokens.js):
    spike_score = min(vol_mcap_ratio × 30, 40)
                + min(abs(price_change_pct_5m) / 2, 30)
                + min(m5_txns / 100, 20)
    Max = 90. Default threshold = 30.

Filters applied before emitting:
    - Token pool age > MIN_AGE_HOURS (default 24h) — skip freshly minted rugs
    - spike_score >= SPIKE_THRESHOLD (default 30)
    - Mint not seen within DEDUP_WINDOW_SECS (default 1800s = 30min)

Source: https://api.geckoterminal.com/api/v2/networks/solana/trending_pools
Free tier: 30 req/min, no API key required.
"""

from __future__ import annotations

import json
import logging
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional
import urllib.request
import urllib.error

# ---------------------------------------------------------------------------
# Configuration (overridable via environment variables)
# ---------------------------------------------------------------------------

GECKO_URL = "https://api.geckoterminal.com/api/v2/networks/solana/trending_pools?page=1&duration=5m"
SPIKE_THRESHOLD: float = float(os.environ.get("SPIKE_THRESHOLD", "30"))
MIN_AGE_HOURS: float = float(os.environ.get("SPIKE_MIN_AGE_HOURS", "24"))
DEDUP_WINDOW_SECS: int = int(os.environ.get("SPIKE_DEDUP_WINDOW_SECS", "1800"))
REQUEST_TIMEOUT: int = int(os.environ.get("SPIKE_REQUEST_TIMEOUT", "15"))
STATE_FILE: Path = Path(os.environ.get(
    "SPIKE_STATE_FILE",
    str(Path.home() / ".cynic" / "spike_detector_seen.json"),
))

_raw_addr: str = os.environ.get("CYNIC_REST_ADDR", "")
CYNIC_REST_ADDR: str = (
    f"http://{_raw_addr}" if _raw_addr and not _raw_addr.startswith("http") else _raw_addr
)
CYNIC_API_KEY: str = os.environ.get("CYNIC_API_KEY", "")

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

logging.basicConfig(
    format="%(asctime)s spike-detector %(levelname)s %(message)s",
    level=logging.INFO,
    stream=sys.stdout,
)
log = logging.getLogger("spike_detector")


# ---------------------------------------------------------------------------
# Pure functions (testable without I/O)
# ---------------------------------------------------------------------------

def compute_spike_score(
    vol_usd_m5: float,
    fdv_usd: float,
    price_change_pct_m5: float,
    txns_m5: int,
) -> float:
    """Compute spike score from 5-minute market data.

    Ported from CultScreener backend/src/routes/tokens.js::spikeScore.
    All three components are capped to prevent a single signal dominating.

    Args:
        vol_usd_m5: 5-minute trading volume in USD
        fdv_usd: Fully-diluted valuation in USD (proxy for market cap)
        price_change_pct_m5: 5-minute price change as percentage (e.g. 12.5 = +12.5%)
        txns_m5: Number of transactions in the last 5 minutes

    Returns:
        Spike score in [0, 90]. Higher = more anomalous market activity.
    """
    if fdv_usd <= 0:
        vol_mcap_ratio = 0.0
    else:
        vol_mcap_ratio = (vol_usd_m5 / fdv_usd) * 100.0

    component_volume = min(vol_mcap_ratio * 30, 40)
    component_price = min(abs(price_change_pct_m5) / 2, 30)
    component_txns = min(txns_m5 / 100, 20)
    return component_volume + component_price + component_txns


def extract_mint(pool: dict) -> Optional[str]:  # type: ignore[type-arg]
    """Extract the base token mint address from a GeckoTerminal pool object."""
    try:
        token_id: str = (
            pool["relationships"]["base_token"]["data"]["id"]
        )
        # Format: "solana_<mint_address>"
        if token_id.startswith("solana_"):
            return token_id[len("solana_"):]
    except (KeyError, TypeError):
        pass
    return None


def pool_age_hours(pool: dict) -> float:  # type: ignore[type-arg]
    """Return the pool age in hours. Returns 0 on parse error (conservative — will be filtered)."""
    created_at: Optional[str] = pool.get("attributes", {}).get("pool_created_at")
    if not created_at:
        return 0.0
    try:
        dt = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
        age = datetime.now(timezone.utc) - dt
        return age.total_seconds() / 3600.0
    except (ValueError, TypeError):
        return 0.0


# ---------------------------------------------------------------------------
# State management (dedup)
# ---------------------------------------------------------------------------

def load_seen() -> dict[str, float]:
    """Load seen mints → timestamp from state file."""
    if not STATE_FILE.exists():
        return {}
    try:
        data = json.loads(STATE_FILE.read_text())
        if isinstance(data, dict):
            return {k: float(v) for k, v in data.items()}
    except (json.JSONDecodeError, OSError):
        pass
    return {}


def save_seen(seen: dict[str, float]) -> None:
    """Persist seen mints, dropping entries older than DEDUP_WINDOW_SECS."""
    now = time.time()
    fresh = {k: v for k, v in seen.items() if now - v < DEDUP_WINDOW_SECS}
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(fresh))


# ---------------------------------------------------------------------------
# GeckoTerminal fetch
# ---------------------------------------------------------------------------

def fetch_trending_pools() -> list[dict]:  # type: ignore[type-arg]
    """Fetch trending Solana pools from GeckoTerminal (free tier).

    Returns list of pool objects, or raises on network/parse error.
    """
    req = urllib.request.Request(
        GECKO_URL,
        headers={"Accept": "application/json", "User-Agent": "cynic-spike-detector/1.0"},
    )
    with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
        body = json.loads(resp.read().decode())
    pools: list[dict] = body.get("data", [])  # type: ignore[type-arg]
    return pools


# ---------------------------------------------------------------------------
# CYNIC observation POST
# ---------------------------------------------------------------------------

def post_observation(mint: str, spike_score: float, extra: dict) -> bool:  # type: ignore[type-arg]
    """POST a token-analysis observation to CYNIC kernel.

    Returns True on success, False on failure.
    """
    if not CYNIC_REST_ADDR:
        log.error("CYNIC_REST_ADDR not set — cannot POST observation")
        return False
    if not CYNIC_API_KEY:
        log.error("CYNIC_API_KEY not set — cannot POST observation")
        return False

    payload = json.dumps({
        "tool": "spike_detector",
        "target": mint,
        "domain": "token-analysis",
        "context": mint,
        "agent_id": "spike-detector",
        "tags": ["spike", "geckoterminal"],
        "metadata": {
            "spike_score": round(spike_score, 2),
            **extra,
        },
    }).encode()

    url = f"{CYNIC_REST_ADDR.rstrip('/')}/observe"
    req = urllib.request.Request(
        url,
        data=payload,
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {CYNIC_API_KEY}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
            status = resp.getcode()
            if 200 <= status < 300:
                return True
            log.warning("POST /observe returned HTTP %d for mint %s", status, mint)
            return False
    except urllib.error.HTTPError as e:
        # 4xx = permanent error (bad token, auth); 5xx = transient (retry on next cycle)
        if e.code < 500:
            log.error("POST /observe permanent error HTTP %d for mint %s", e.code, mint)
        else:
            log.warning("POST /observe transient error HTTP %d for mint %s — will retry", e.code, mint)
        return False
    except urllib.error.URLError as e:
        log.warning("POST /observe network error for mint %s: %s", mint, e.reason)
        return False


# ---------------------------------------------------------------------------
# Main detection loop
# ---------------------------------------------------------------------------

def run() -> None:
    """Fetch trending pools, score, filter, and emit observations."""
    log.info("spike_detector starting (threshold=%.0f, min_age=%.0fh, dedup=%ds)",
             SPIKE_THRESHOLD, MIN_AGE_HOURS, DEDUP_WINDOW_SECS)

    try:
        pools = fetch_trending_pools()
    except Exception as exc:
        log.error("GeckoTerminal fetch failed: %s", exc)
        sys.exit(1)

    log.info("fetched %d trending pools", len(pools))
    seen = load_seen()
    now = time.time()

    emitted = 0
    skipped_age = 0
    skipped_score = 0
    skipped_dedup = 0

    for pool in pools:
        mint = extract_mint(pool)
        if not mint:
            continue

        age_h = pool_age_hours(pool)
        if age_h < MIN_AGE_HOURS:
            skipped_age += 1
            log.debug("skip %s — age %.1fh < %.0fh", mint, age_h, MIN_AGE_HOURS)
            continue

        attrs = pool.get("attributes", {})
        try:
            vol_m5 = float(attrs.get("volume_usd", {}).get("m5") or 0)
            fdv = float(attrs.get("fdv_usd") or 0)
            price_chg = float(attrs.get("price_change_percentage", {}).get("m5") or 0)
            txns_m5_data = attrs.get("transactions", {}).get("m5", {})
            txns_m5 = int(txns_m5_data.get("buys", 0)) + int(txns_m5_data.get("sells", 0))
        except (TypeError, ValueError):
            log.debug("skip %s — could not parse attributes", mint)
            continue

        score = compute_spike_score(vol_m5, fdv, price_chg, txns_m5)

        if score < SPIKE_THRESHOLD:
            skipped_score += 1
            log.debug("skip %s — score %.1f < threshold %.0f", mint, score, SPIKE_THRESHOLD)
            continue

        if mint in seen and (now - seen[mint]) < DEDUP_WINDOW_SECS:
            skipped_dedup += 1
            log.debug("skip %s — seen %.0fs ago (dedup window %ds)",
                      mint, now - seen[mint], DEDUP_WINDOW_SECS)
            continue

        extra = {
            "vol_usd_m5": round(vol_m5, 2),
            "fdv_usd": round(fdv, 2),
            "price_change_pct_m5": round(price_chg, 2),
            "txns_m5": txns_m5,
            "pool_age_hours": round(age_h, 1),
        }
        log.info("spike detected mint=%s score=%.1f vol_m5=%.0f price_chg=%.1f%% txns=%d age=%.0fh",
                 mint, score, vol_m5, price_chg, txns_m5, age_h)

        if post_observation(mint, score, extra):
            seen[mint] = now
            emitted += 1

    save_seen(seen)
    log.info("done: emitted=%d skipped(age=%d score=%d dedup=%d)",
             emitted, skipped_age, skipped_score, skipped_dedup)


# ---------------------------------------------------------------------------
# Tests (run with: python3 spike_detector.py --test)
# ---------------------------------------------------------------------------

def run_tests() -> None:
    """Offline unit tests — no I/O required."""
    errors: list[str] = []

    # compute_spike_score: all-zero inputs
    s = compute_spike_score(0, 0, 0, 0)
    if s != 0.0:
        errors.append(f"zero inputs: expected 0.0, got {s}")

    # compute_spike_score: max caps
    s = compute_spike_score(1e9, 1.0, 200.0, 10000)
    if s != 90.0:
        errors.append(f"max caps: expected 90.0, got {s}")

    # compute_spike_score: realistic spike
    # vol=50k, fdv=500k → ratio=10% → component=min(300,40)=40
    # price_chg=20% → component=min(10,30)=10
    # txns=200 → component=min(2,20)=2
    s = compute_spike_score(50_000, 500_000, 20.0, 200)
    expected = 40.0 + 10.0 + 2.0
    if abs(s - expected) > 0.001:
        errors.append(f"realistic spike: expected {expected}, got {s}")

    # compute_spike_score: below threshold
    s = compute_spike_score(100, 1_000_000, 1.0, 5)
    if s >= SPIKE_THRESHOLD:
        errors.append(f"low activity should be below threshold, got {s}")

    # extract_mint: valid pool
    pool = {"relationships": {"base_token": {"data": {"id": "solana_So11111111111111111111111111111111111111112"}}}}
    m = extract_mint(pool)
    expected_mint = "So11111111111111111111111111111111111111112"
    if m != expected_mint:
        errors.append(f"extract_mint: expected {expected_mint!r}, got {m!r}")

    # extract_mint: missing relationships
    if extract_mint({}) is not None:
        errors.append("extract_mint on empty dict should return None")

    # extract_mint: non-solana prefix
    pool2 = {"relationships": {"base_token": {"data": {"id": "ethereum_0xabc"}}}}
    if extract_mint(pool2) is not None:
        errors.append("extract_mint with non-solana prefix should return None")

    if errors:
        for e in errors:
            print(f"FAIL: {e}")
        sys.exit(1)
    else:
        print(f"OK: 7 tests passed")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--test":
        run_tests()
    else:
        run()
