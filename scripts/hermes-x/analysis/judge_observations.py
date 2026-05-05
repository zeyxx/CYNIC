#!/usr/bin/env python3
"""
Judge Observations — K15 Seam 2 feedback loop.

Reads observations from organ-local observations/ directory.
Sends each to kernel /judge endpoint to get Dogs' verdict on agent findings.
Stores verdicts in observation-verdicts/ for Gemini learning.

This closes the feedback loop:
  Hermes 9B posts observation
    ↓
  Kernel judges observation (verdict on agent accuracy/insight)
    ↓
  Verdicts stored organ-local
    ↓
  Gemini reads verdicts, extracts patterns, updates SKILL.md

Usage:
    python3 judge_observations.py --organ-dir ~/.cynic/organs/hermes/x
    python3 judge_observations.py --organ-dir ~/.cynic/organs/hermes/x --limit 5

Environment:
    CYNIC_REST_ADDR  — kernel address (required)
    CYNIC_API_KEY    — kernel auth token (required)
"""

import argparse
import json
import logging
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional

import requests

logger = logging.getLogger("judge-observations")

KERNEL_ADDR = ""
API_KEY = ""


def load_env():
    global KERNEL_ADDR, API_KEY
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")

    if KERNEL_ADDR and API_KEY:
        return

    env_file = Path.home() / ".cynic-env"
    if not env_file.exists():
        return

    for line in env_file.read_text().splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        if line.startswith("export "):
            line = line[7:]
        key, _, val = line.partition("=")
        val = val.strip().strip('"').strip("'")
        if key.strip() == "CYNIC_REST_ADDR" and not KERNEL_ADDR:
            KERNEL_ADDR = val
        elif key.strip() == "CYNIC_API_KEY" and not API_KEY:
            API_KEY = val


def _kernel_url() -> str:
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    return addr


def _headers() -> dict:
    return {"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"}


def load_observations(organ_dir: str, limit: int = 10) -> list[dict]:
    """Load observations from organ-local observations/ directory.

    Returns list of observation dicts with added _file path.
    """
    obs_dir = Path(organ_dir) / "observations"
    if not obs_dir.exists():
        logger.warning("observations directory not found: %s", obs_dir)
        return []

    observations = []
    try:
        for obs_file in sorted(obs_dir.glob("*.json"))[-limit:]:
            try:
                with open(obs_file) as f:
                    obs = json.load(f)
                    obs["_file"] = str(obs_file)
                    # Generate observation ID from finding + timestamp
                    obs_id = f"{obs.get('finding', 'unknown').replace(' ', '_')}_{obs.get('timestamp', '')}"
                    obs["_id"] = obs_id
                    observations.append(obs)
            except json.JSONDecodeError as e:
                logger.warning("Failed to parse observation file %s: %s", obs_file, e)
                continue
    except OSError as e:
        logger.warning("Failed to scan observations directory: %s", e)

    return observations


def judge_observation(observation: dict, organ_dir: str) -> Optional[dict]:
    """Send observation to /judge endpoint.

    Returns verdict dict or None on error.
    """
    # Build judgment request from observation
    content = f"Agent finding: {observation.get('finding', 'unknown')}\n\nDetails: {observation.get('details', '')}"
    domain = observation.get("domain", "twitter")

    # Add narratives as context hint
    narratives = observation.get("narratives", [])
    context = f"Narratives: {', '.join(narratives)}\nSignal score: {observation.get('signal_score', 0)}"

    payload = {
        "content": content,
        "context": context,
        "domain": f"observation_{domain}",  # distinguish from regular tweets
        "crystals": False,  # don't create crystals yet
    }

    try:
        resp = requests.post(
            f"{_kernel_url()}/judge",
            json=payload,
            headers=_headers(),
            timeout=30,
        )
        if resp.status_code == 200:
            verdict_data = resp.json()
            # Handle both wrapped and unwrapped responses
            verdict = verdict_data.get("verdict")
            if isinstance(verdict, dict):
                verdict_type = verdict.get("verdict", "?")
            else:
                verdict_type = str(verdict)
            logger.info(
                "judged observation %s: verdict=%s",
                observation.get("_id"),
                verdict_type,
            )
            return verdict
        else:
            logger.warning(
                "judge failed for observation %s: %d %s",
                observation.get("_id"),
                resp.status_code,
                resp.text[:100],
            )
            return None
    except requests.RequestException as e:
        logger.warning("judge request failed for observation %s: %s", observation.get("_id"), e)
        return None


def store_verdict(observation: dict, verdict: dict, organ_dir: str) -> bool:
    """Store verdict in observation-verdicts/ directory.

    File structure mirrors verdict storage for consistency.
    """
    verdict_dir = Path(organ_dir) / "observation-verdicts"
    verdict_dir.mkdir(parents=True, exist_ok=True)

    obs_id = observation.get("_id", "unknown")
    verdict_file = verdict_dir / f"{obs_id}.json"

    verdict_with_meta = {
        "observation_id": obs_id,
        "observation_finding": observation.get("finding", ""),
        "observation_domain": observation.get("domain", ""),
        "observation_narratives": observation.get("narratives", []),
        "observation_signal_score": observation.get("signal_score", 0),
        "verdict": verdict,
        "stored_at": datetime.utcnow().isoformat() + "Z",
    }

    try:
        with open(verdict_file, "w") as f:
            json.dump(verdict_with_meta, f, indent=2)
        logger.info("stored verdict: %s", verdict_file.name)
        return True
    except OSError as e:
        logger.warning("Failed to store verdict for observation %s: %s", obs_id, e)
        return False


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()

    parser = argparse.ArgumentParser(description="Judge observations and store verdicts")
    parser.add_argument("--organ-dir", required=True, help="Organ directory containing observations/")
    parser.add_argument("--limit", type=int, default=10, help="Max observations to judge")
    args = parser.parse_args()

    organ_dir = str(Path(args.organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error("Organ directory not found: %s", organ_dir)
        sys.exit(1)

    if not KERNEL_ADDR or not API_KEY:
        logger.error("CYNIC_REST_ADDR and CYNIC_API_KEY environment variables required")
        sys.exit(1)

    # Load observations
    observations = load_observations(organ_dir, limit=args.limit)
    if not observations:
        logger.info("no observations to judge")
        return

    logger.info("judging %d observation(s)", len(observations))

    # Judge each observation
    judged_count = 0
    for obs in observations:
        verdict = judge_observation(obs, organ_dir)
        if verdict:
            if store_verdict(obs, verdict, organ_dir):
                judged_count += 1

    logger.info("judged and stored %d verdict(s)", judged_count)


if __name__ == "__main__":
    main()
