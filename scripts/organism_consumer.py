#!/usr/bin/env python3
"""
Organism Consumer v0.1.0 — Minimal Viable Verdict Consumer

CHAOS->MATRIX: Emits raw observations per consumed verdict.
No schema design. Structure emerges from accumulated data.

Reads verdicts from kernel, classifies ripeness, emits observations
back to /observe with the verdict's source domain (Option C:
domain follows source, not hardcoded "organism").

K15: Consumer that ACTS — classifies ripeness, emits decision observation.
"""

__version__ = "0.1.0"

import json
import logging
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    import requests
except ImportError:
    print("FATAL: requests library required", file=sys.stderr)
    sys.exit(1)

# --- Constants ---

PHI = 1.6180339887
PHI_INV = 1.0 / PHI        # 0.618 — max confidence
PHI_INV2 = PHI_INV ** 2    # 0.382 — decay threshold
HOWL_THRESHOLD = PHI_INV2 + (PHI_INV ** 4)  # 0.528

CONSUMED_LOG = Path.home() / ".cynic" / "organism" / "consumed_verdicts.jsonl"
DECISION_LOG = Path.home() / ".cynic" / "organism" / "decisions.jsonl"

# --- Logging ---

logger = logging.getLogger("organism-consumer")
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] organism: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)

# --- Kernel Communication ---


def get_kernel_address() -> str:
    """Get kernel REST address from environment."""
    addr = os.environ.get("CYNIC_REST_ADDR", "localhost:3030")
    if not addr.startswith(("http://", "https://")):
        addr = f"http://{addr}"
    return addr


def get_api_key() -> Optional[str]:
    """Get kernel API key from environment or ~/.cynic-env."""
    api_key = os.environ.get("CYNIC_API_KEY")
    if api_key:
        return api_key
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        with open(env_file) as f:
            for line in f:
                if line.startswith("CYNIC_API_KEY="):
                    return line.split("=", 1)[1].strip()
    return None


def kernel_get(path: str) -> Optional[Any]:
    """GET from kernel. Returns parsed JSON or None."""
    addr = get_kernel_address()
    api_key = get_api_key()
    headers: Dict[str, str] = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    try:
        resp = requests.get(f"{addr}{path}", headers=headers, timeout=10)
        if resp.status_code == 200:
            return resp.json()
        logger.warning("GET %s returned %d", path, resp.status_code)
        return None
    except requests.RequestException as e:
        logger.error("GET %s failed: %s", path, e)
        return None


def kernel_post_observe(payload: Dict[str, Any]) -> bool:
    """POST to /observe. Returns True on success."""
    addr = get_kernel_address()
    api_key = get_api_key()
    headers: Dict[str, str] = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    try:
        resp = requests.post(
            f"{addr}/observe", json=payload, headers=headers, timeout=5
        )
        if resp.status_code in (200, 202):
            return True
        logger.warning("POST /observe returned %d: %s", resp.status_code, resp.text)
        return False
    except requests.RequestException as e:
        logger.error("POST /observe failed: %s", e)
        return False


# --- State ---


def load_consumed_ids() -> set:
    """Load set of already-consumed verdict IDs from log."""
    consumed = set()
    if CONSUMED_LOG.exists():
        with open(CONSUMED_LOG) as f:
            for line in f:
                try:
                    entry = json.loads(line)
                    consumed.add(entry.get("verdict_id", ""))
                except json.JSONDecodeError:
                    pass
    return consumed


def log_consumed(verdict_id: str, decision: Dict[str, Any]) -> None:
    """Append consumed verdict to log."""
    CONSUMED_LOG.parent.mkdir(parents=True, exist_ok=True)
    entry = {
        "verdict_id": verdict_id,
        "consumed_at": datetime.utcnow().isoformat() + "Z",
        **decision,
    }
    with open(CONSUMED_LOG, "a") as f:
        f.write(json.dumps(entry) + "\n")


def log_decision(decision: Dict[str, Any]) -> None:
    """Append decision to organism decision log."""
    DECISION_LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(DECISION_LOG, "a") as f:
        f.write(json.dumps(decision) + "\n")


# --- Ripeness Classification ---


def extract_q_total(verdict: Dict[str, Any]) -> float:
    """Extract total q_score from verdict (handles dict or float)."""
    q = verdict.get("q_score", 0.0)
    if isinstance(q, dict):
        return float(q.get("total", 0.0))
    if isinstance(q, (int, float)):
        return float(q)
    try:
        return float(q)
    except (ValueError, TypeError):
        return 0.0


def assess_ripeness(verdict: Dict[str, Any]) -> Dict[str, Any]:
    """Assess whether a verdict is ripe for organism action.

    Returns raw decision dict — no schema, CHAOS->MATRIX.
    """
    q_total = extract_q_total(verdict)
    # Verdict type comes from kernel (already classified)
    verdict_type = verdict.get("verdict", "unknown").upper()
    domain = verdict.get("domain", "unknown")
    target = verdict.get("target", "unknown")
    dog_count = verdict.get("voter_count", 0)
    max_disagree = verdict.get("max_disagreement", 0.0)
    anomaly = verdict.get("anomaly_detected", False)

    # Ripeness gate:
    # - HOWL or WAG with >= 2 dogs = ripe
    # - max_disagreement > phi^-2 (0.382) = NOT ripe (Kairos signal)
    # - anomaly_detected = NOT ripe (Dogs disagree on something fundamental)
    kairos_blocked = max_disagree > PHI_INV2
    ripe = (
        verdict_type in ("HOWL", "WAG")
        and dog_count >= 2
        and not kairos_blocked
        and not anomaly
    )

    reason_parts = [
        f"{verdict_type} q={q_total:.3f} dogs={dog_count}",
        f"disagree={max_disagree:.3f}",
    ]
    if kairos_blocked:
        reason_parts.append("KAIROS_BLOCKED")
    if anomaly:
        reason_parts.append("ANOMALY")
    reason_parts.append("RIPE" if ripe else "NOT_YET")

    return {
        "verdict_id": verdict.get("verdict_id", "unknown"),
        "domain": domain,
        "target": target,
        "q_total": q_total,
        "verdict_type": verdict_type,
        "dog_count": dog_count,
        "max_disagreement": max_disagree,
        "anomaly_detected": anomaly,
        "kairos_blocked": kairos_blocked,
        "ripe": ripe,
        "reason": " | ".join(reason_parts),
        "assessed_at": datetime.utcnow().isoformat() + "Z",
    }


# --- Main Loop ---


def consume_verdicts() -> int:
    """Fetch verdicts, classify ripeness, emit observations.

    Returns count of newly consumed verdicts.
    """
    # 1. Fetch recent verdicts
    verdicts_data = kernel_get("/verdicts")
    if verdicts_data is None:
        logger.warning("Could not fetch verdicts from kernel")
        return 0

    # Handle both list and dict-with-list responses
    if isinstance(verdicts_data, dict):
        verdicts: List[Dict[str, Any]] = verdicts_data.get("verdicts", [])
    elif isinstance(verdicts_data, list):
        verdicts = verdicts_data
    else:
        logger.warning("Unexpected verdicts response type: %s", type(verdicts_data))
        return 0

    if not verdicts:
        logger.info("No verdicts available")
        return 0

    # 2. Filter already consumed
    consumed_ids = load_consumed_ids()
    new_verdicts = [
        v for v in verdicts
        if v.get("verdict_id", "") not in consumed_ids
    ]

    if not new_verdicts:
        logger.info("All %d verdicts already consumed", len(verdicts))
        return 0

    logger.info("Found %d new verdicts (of %d total)", len(new_verdicts), len(verdicts))

    # 3. Assess and emit
    consumed_count = 0
    for verdict in new_verdicts:
        verdict_id = verdict.get("verdict_id", "unknown")
        decision = assess_ripeness(verdict)

        # Emit observation back to kernel (Option C: domain follows source)
        payload = {
            "tool": "organism_consumer",
            "domain": decision["domain"],
            "target": verdict_id,
            "context": json.dumps({
                "ripeness": decision["ripe"],
                "verdict_type": decision["verdict_type"],
                "q_total": decision["q_total"],
                "dog_count": decision["dog_count"],
                "max_disagreement": decision["max_disagreement"],
                "kairos_blocked": decision["kairos_blocked"],
                "anomaly_detected": decision["anomaly_detected"],
                "reason": decision["reason"],
            }),
            "status": "ripe" if decision["ripe"] else "immature",
            "agent_id": "organism-consumer",
            "confidence": "observed",
            "maturity": decision["q_total"],
        }

        emitted = kernel_post_observe(payload)
        if emitted:
            logger.info("Consumed %s: %s", verdict_id, decision["reason"])
        else:
            logger.warning("Failed to emit for %s (logged locally)", verdict_id)

        # Always log locally (even if kernel unreachable)
        log_consumed(verdict_id, decision)
        log_decision(decision)
        consumed_count += 1

    return consumed_count


def main() -> None:
    """Entry point."""
    logger.info("Organism Consumer v%s starting", __version__)
    logger.info("Kernel: %s", get_kernel_address())
    logger.info("API key: %s", "***" if get_api_key() else "MISSING")
    logger.info("Consumed log: %s", CONSUMED_LOG)

    count = consume_verdicts()

    logger.info("Done. Consumed %d new verdicts.", count)

    # Summary
    if CONSUMED_LOG.exists():
        total = sum(1 for _ in open(CONSUMED_LOG))
        logger.info("Total consumed (all time): %d", total)

    if DECISION_LOG.exists():
        ripe_count = 0
        with open(DECISION_LOG) as f:
            for line in f:
                try:
                    d = json.loads(line)
                    if d.get("ripe"):
                        ripe_count += 1
                except json.JSONDecodeError:
                    pass
        logger.info("Ripe verdicts (all time): %d", ripe_count)


if __name__ == "__main__":
    main()
