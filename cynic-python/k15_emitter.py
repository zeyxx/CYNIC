#!/usr/bin/env python3
"""
K15 Emitter — Real-Time Data Flow to Kernel

Provides `emit_observation()` to send domain verdicts, wallet corpus, and other
structured data to the kernel's `/observe` endpoint in real-time, enabling
immediate K15 consumer consumption → agent-tasks dispatch → organism learning.

This is the K15 producer layer: emit → consume → act.
"""

import json
import logging
import os
from typing import Dict, Any, Optional
from datetime import datetime
from pathlib import Path

try:
    import requests
except ImportError:
    requests = None

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] k15_emitter: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


def get_kernel_address() -> str:
    """Get kernel REST address from environment or default."""
    return os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")


def get_api_key() -> Optional[str]:
    """Get kernel API key from environment or ~/.cynic-env.

    Secrets are read from secure location (env var or user home dir config),
    never hardcoded or logged. This is safe.
    """
    # Secure: environment variable (not logged)
    api_key = os.environ.get("CYNIC_API_KEY")
    if api_key:
        return api_key

    # Secure: user home directory config file (not in repo)
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        with open(env_file) as f:
            for line in f:
                if line.startswith("CYNIC_API_KEY="):
                    return line.split("=", 1)[1].strip()

    return None


def emit_observation(
    tool: str,
    domain: str,
    context: str,
    target: Optional[str] = None,
    status: str = "success",
    agent_id: str = "k15-producer",
    session_id: Optional[str] = None,
) -> bool:
    """Emit a data observation to kernel for K15 consumption.

    Args:
        tool: Tool name (e.g., "domain_verdict_builder", "wallet_corpus_builder")
        domain: Domain hint (e.g., "token-analysis", "wallet-judgment")
        context: Structured data (JSON string, ≤200 chars truncated by kernel)
        target: File path or target identifier
        status: "success", "error", etc. (default "success")
        agent_id: Reporting agent (default "k15-producer")
        session_id: Optional session identifier

    Returns:
        True if emit successful (202), False otherwise
    """
    if requests is None:
        logger.warning("requests library not available; observation not emitted")
        return False

    kernel_addr = get_kernel_address()
    api_key = get_api_key()

    payload = {
        "tool": tool,
        "domain": domain,
        "context": context,
        "status": status,
        "agent_id": agent_id,
    }

    if target:
        payload["target"] = target

    if session_id:
        payload["session_id"] = session_id

    url = f"{kernel_addr}/observe"
    headers = {
        "Content-Type": "application/json",
    }

    # Add auth if available
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    try:
        resp = requests.post(url, json=payload, headers=headers, timeout=5)

        if resp.status_code == 202:
            logger.info(f"✓ Emitted observation: {tool} @ {domain}")
            return True
        else:
            logger.warning(f"Kernel returned {resp.status_code}: {resp.text}")
            return False

    except requests.RequestException as e:
        logger.error(f"Failed to emit: {e} (kernel at {kernel_addr}?)")
        return False


def emit_domain_verdicts(records: list, count: int = 0) -> bool:
    """Emit domain verdict batch to kernel.

    Args:
        records: List of TokenDomainVerdictRecord dicts
        count: Optional count suffix for logging

    Returns:
        True if emit successful
    """
    if not records:
        logger.warning("No records to emit")
        return False

    # Aggregate verdict distribution
    verdicts = {}
    for r in records:
        v = r.get("domain_verdict", "unknown")
        verdicts[v] = verdicts.get(v, 0) + 1

    context = json.dumps({
        "records_count": len(records),
        "verdicts": verdicts,
        "timestamp": datetime.utcnow().isoformat(),
    })

    return emit_observation(
        tool="domain_verdict_builder",
        domain="token-analysis",
        context=context,
        target="cynic-python/domain_verdicts_personalized.json",
    )


def emit_wallet_corpus(corpus: list, count: int = 0) -> bool:
    """Emit wallet corpus batch to kernel.

    Args:
        corpus: List of wallet profile dicts
        count: Optional count suffix for logging

    Returns:
        True if emit successful
    """
    if not corpus:
        logger.warning("No corpus to emit")
        return False

    # Aggregate human/sybil split
    humans = sum(1 for c in corpus if c.get("is_human"))
    sybils = len(corpus) - humans

    context = json.dumps({
        "corpus_count": len(corpus),
        "humans": humans,
        "sybils": sybils,
        "timestamp": datetime.utcnow().isoformat(),
    })

    return emit_observation(
        tool="wallet_corpus_builder",
        domain="wallet-judgment",
        context=context,
        target="cynic-python/validation_corpus.json",
    )


if __name__ == "__main__":
    # Test emission
    kernel = get_kernel_address()
    api_key = "***" if get_api_key() else "none"

    print(f"K15 Emitter Configuration")
    print(f"  Kernel: {kernel}")
    print(f"  API Key: {api_key}")
    print()
    print("To use:")
    print("  from k15_emitter import emit_domain_verdicts, emit_wallet_corpus")
    print("  emit_domain_verdicts(records)")
    print("  emit_wallet_corpus(corpus)")
