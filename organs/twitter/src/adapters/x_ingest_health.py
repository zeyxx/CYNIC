"""Runtime health adapter for the X ingest daemon."""

from __future__ import annotations

import logging
import socket
import time
from pathlib import Path

import requests

from organs.twitter.src.adapters.x_ingest_store import detect_auth_failure
from organs.twitter.src.adapters.x_kernel_transport import build_heartbeat_payload


logger = logging.getLogger("x-ingest-health")


def post_heartbeat(
    *,
    transport,
    organ_name: str,
    account_id: str,
    dataset: Path,
    sent: int,
    errors: int,
) -> bool:
    """POST organ heartbeat to /observe and report local capture health."""
    try:
        size = dataset.stat().st_size
        mtime = dataset.stat().st_mtime
        age_secs = time.time() - mtime
        dataset_growing = age_secs < 300
    except OSError:
        size = 0
        age_secs = -1
        dataset_growing = False

    proxy_up = _tcp_port_open("127.0.0.1", 8888)
    cdp_up = _chrome_cdp_up()
    auth_failed = not dataset_growing and detect_auth_failure(dataset)

    status = "ok"
    failure_reason = None
    if not proxy_up:
        status = "critical"
        failure_reason = "proxy_down"
    elif not cdp_up:
        status = "degraded"
        failure_reason = "cdp_down"
    elif auth_failed:
        status = "critical"
        failure_reason = "x_auth_expired"
    elif not dataset_growing:
        status = "degraded"
        failure_reason = "dataset_stale"

    context = (
        f"sent={sent} errors={errors} "
        f"dataset_bytes={size} dataset_age_secs={int(age_secs)} "
        f"dataset_growing={dataset_growing} "
        f"proxy_up={proxy_up} cdp_up={cdp_up} auth_failed={auth_failed}"
    )
    payload = build_heartbeat_payload(organ_name, account_id, status, context, failure_reason)

    try:
        if transport.post_observe(payload, timeout=5):
            logger.debug("heartbeat: %s (%s)", status, context)
            return True
        logger.warning("heartbeat POST failed")
        return False
    except requests.RequestException as exc:
        logger.warning("heartbeat failed: %s", exc)
        return False


def _tcp_port_open(host: str, port: int) -> bool:
    try:
        with socket.create_connection((host, port), timeout=1):
            return True
    except (ConnectionRefusedError, OSError):
        return False


def _chrome_cdp_up() -> bool:
    try:
        resp = requests.get("http://127.0.0.1:40769/json/version", timeout=2)
        return resp.status_code == 200
    except requests.RequestException:
        return False
