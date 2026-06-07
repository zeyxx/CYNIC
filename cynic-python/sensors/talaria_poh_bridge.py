#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Talaria PoH Bridge — B&C API → CYNIC Talaria observations.

CYNIC owns Talaria observatory state. B&C is an autonomous chess organism that
exposes selected PoH/chess signals. This bridge observes B&C verification state
and posts raw Talaria events to CYNIC; it does not grant final human status.
"""

import logging
import os
import sys
from pathlib import Path

import requests

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from domains.talaria import blitzchill_poh_observed_event  # noqa: E402

AGENT_ID = "talaria-poh-bridge"
CYNIC_REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_API_KEY = os.getenv("CYNIC_API_KEY")
BC_API_URL = os.getenv("BC_API_URL", "https://blitzchill.space/api/verify")

logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] %(levelname)s | PoH Bridge — %(message)s",
)
logger = logging.getLogger("PoHBridge")


def _headers() -> dict[str, str]:
    return {
        "Authorization": f"Bearer {CYNIC_API_KEY}",
        "Content-Type": "application/json",
    }


def _post_observation(payload: dict) -> bool:
    if not CYNIC_API_KEY:
        logger.error("CYNIC_API_KEY is not set")
        return False

    resp = requests.post(
        f"{CYNIC_REST_ADDR.rstrip('/')}/observe",
        json=payload,
        headers=_headers(),
        timeout=10,
    )
    if resp.status_code != 200:
        logger.warning("kernel /observe returned HTTP %s: %s", resp.status_code, resp.text[:200])
        return False
    return True


def check_user_poh(wallet_address: str):
    """Observe B&C PoH status for a wallet and push a Talaria event to CYNIC."""
    source_url = f"{BC_API_URL}?wallet={wallet_address}"
    try:
        resp = requests.get(source_url, timeout=10)
        if resp.status_code == 404:
            return False
        if resp.status_code != 200:
            logger.warning("B&C verify returned HTTP %s for %s", resp.status_code, wallet_address)
            return None

        data = resp.json()
        verified = bool(data.get("verified"))
        event = blitzchill_poh_observed_event(
            wallet_address=wallet_address,
            verified=verified,
            source_url=source_url,
            raw=data,
        )
        payload = event.to_observe_payload(agent_id=AGENT_ID, status="verified" if verified else "not_verified")

        if not _post_observation(payload):
            return None

        logger.info("Observed B&C PoH wallet=%s verified=%s", wallet_address, verified)
        return verified
    except Exception as e:
        logger.error("Bridge error for %s: %s", wallet_address, e)
        return None


if __name__ == "__main__":
    logger.info("Talaria PoH Bridge ready: B&C signals -> CYNIC observations.")
