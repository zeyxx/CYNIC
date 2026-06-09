"""
Tier 1 EXPERIMENTAL: Outcome feedback loop for token-analysis verdicts.

Research question: Do Dog Q-scores correlate with actual token outcomes?
Success condition: rho(q_score, outcome_label) > 0.3 after 30 days data
Timeline: 30 days from 2026-05-23
Death date: 2026-06-22 — delete if no acting K15 consumer wired by then
Promotion condition: A calibration script reads outcome observations, computes rho,
  proposes Dog weight changes. That script becomes the K15 consumer -> Tier 2.

K15 Exception (conscious):
  Producer: this script
  Consumer: NONE — explicit Tier 1 exception per python-lifecycle.md

Note: outcome_collector.py in this directory is a separate Tier 2 module
(GeckoTerminal/file-based). This module targets the kernel /verdicts feed
and stores results via /observe for future calibration.
"""
__version__ = "0.1.0"

import json
import logging
import os
import sys
from datetime import datetime, timezone, timedelta
from typing import Optional

import requests

logger = logging.getLogger(__name__)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s"
)

CYNIC_REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_API_KEY = os.environ.get("CYNIC_API_KEY", "")
_HEADERS: dict[str, str] = {}
if CYNIC_API_KEY:
    _HEADERS["Authorization"] = f"Bearer {CYNIC_API_KEY}"


def classify_outcome(
    price_delta_pct: float,
    holder_delta_pct: float,
    liquidity_delta_pct: float,
) -> str:
    """
    Derive outcome label from delta metrics.
    Epistemic status: heuristic — not ground truth.

    Input contract:
      price_delta_pct: percentage change in price (e.g. -85.0 = -85%)
      holder_delta_pct: percentage change in holder count
      liquidity_delta_pct: percentage change in liquidity (USD)
    Output: one of "RUG", "DECLINE", "GROWTH", "STABLE"
    Failure mode: any float input is valid; no exception is raised
    Valid domains: token-analysis post-judgment outcomes
    """
    if price_delta_pct <= -80.0 and (
        holder_delta_pct <= -50.0 or liquidity_delta_pct <= -90.0
    ):
        return "RUG"
    if price_delta_pct < -30.0 or holder_delta_pct < -20.0:
        return "DECLINE"
    if price_delta_pct > 30.0 and holder_delta_pct > 10.0:
        return "GROWTH"
    return "STABLE"


def fetch_dexscreener(mint: str) -> dict:
    """Fetch current price/volume/liquidity from DexScreener (free, no credits)."""
    try:
        url = f"https://api.dexscreener.com/latest/dex/tokens/{mint}"
        r = requests.get(url, timeout=10)
        r.raise_for_status()
        pairs = r.json().get("pairs") or []
        if not pairs:
            return {}
        p = pairs[0]
        return {
            "price_usd": float(p.get("priceUsd") or 0),
            "volume_24h_usd": float((p.get("volume") or {}).get("h24") or 0),
            "liquidity_usd": float((p.get("liquidity") or {}).get("usd") or 0),
        }
    except Exception as e:
        logger.warning("DexScreener fetch failed for %s: %s", mint, e)
        return {}


def _get_verdicts_in_window(window_hours: int) -> list:
    """Query kernel for token-analysis verdicts aged ~window_hours."""
    try:
        r = requests.get(
            f"{CYNIC_REST_ADDR}/verdicts",
            headers=_HEADERS,
            params={"domain": "token-analysis", "limit": 200},
            timeout=15,
        )
        r.raise_for_status()
        verdicts = r.json()
    except Exception as e:
        raise RuntimeError(f"Kernel /verdicts query failed: {e}") from e

    now = datetime.now(timezone.utc)
    lo = now - timedelta(hours=window_hours + 1)
    hi = now - timedelta(hours=window_hours - 1)

    result = []
    for v in verdicts:
        created_str = v.get("created_at", "")
        if not created_str:
            continue
        try:
            judged_at = datetime.fromisoformat(created_str.replace("Z", "+00:00"))
        except ValueError:
            continue
        if lo <= judged_at <= hi:
            result.append(v)
    return result


def collect_outcomes(window_hours: int) -> int:
    """Collect outcome data for verdicts at window_hours after judgment."""
    verdicts = _get_verdicts_in_window(window_hours)
    logger.info("Found %d token-analysis verdicts in window=%dh", len(verdicts), window_hours)

    tool = "outcome_7d" if window_hours >= 168 else "outcome_24h"
    stored = 0

    for v in verdicts:
        mint = (v.get("content") or "").strip()
        if len(mint) < 32 or len(mint) > 44:
            continue

        current = fetch_dexscreener(mint)
        if not current:
            logger.warning("No DexScreener data for %s — skipping", mint)
            continue

        ctx = {
            "verdict_id": v.get("id", ""),
            "original_qscore": v.get("q_score", {}).get("total", 0.0),
            "original_verdict": v.get("kind", "Unknown"),
            "current_price_usd": current.get("price_usd"),
            "current_volume_24h_usd": current.get("volume_24h_usd"),
            "current_liquidity_usd": current.get("liquidity_usd"),
            "schema_version": 1,
        }

        try:
            obs_r = requests.post(
                f"{CYNIC_REST_ADDR}/observe",
                headers=_HEADERS,
                json={
                    "tool": tool,
                    "target": mint,
                    "domain": "token-analysis",
                    "context": json.dumps(ctx),
                    "agent_id": "outcome_collector",
                    "tags": ["outcome", "7d" if window_hours >= 168 else "24h"],
                },
                timeout=10,
            )
            obs_r.raise_for_status()
            stored += 1
            logger.info("Stored %s for mint=%s q=%.3f", tool, mint, ctx["original_qscore"])
        except Exception as e:
            logger.warning("Failed to store outcome for %s: %s", mint, e)

    logger.info("outcome_collector done: %d/%d outcomes stored", stored, len(verdicts))
    return stored


if __name__ == "__main__":
    wh = int(sys.argv[1]) if len(sys.argv) > 1 else 24
    n = collect_outcomes(wh)
    sys.exit(0 if n >= 0 else 1)
