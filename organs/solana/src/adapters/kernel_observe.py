"""Kernel observation transport for Solana library probes."""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from typing import Any


def post_observation(payload: dict[str, Any], *, timeout: int = 10) -> dict[str, Any]:
    rest_addr = os.getenv("CYNIC_REST_ADDR", "").strip()
    if not rest_addr:
        raise RuntimeError("CYNIC_REST_ADDR is not set")
    url = rest_addr if rest_addr.startswith(("http://", "https://")) else f"http://{rest_addr}"
    url = url.rstrip("/") + "/observe"
    headers = {"content-type": "application/json"}
    api_key = os.getenv("CYNIC_API_KEY", "").strip()
    if api_key:
        headers["authorization"] = f"Bearer {api_key}"
    observation = {
        "tool": "solana_pump_probe",
        "domain": payload.get("domain", "solana"),
        "target": payload.get("target", payload.get("mint", "unknown")),
        "status": "success" if payload.get("classification") != "probe_error" else "failed",
        "content": json.dumps(payload, sort_keys=True),
    }
    request = urllib.request.Request(
        url,
        data=json.dumps(observation).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"Kernel observe HTTP {exc.code}: {detail}") from exc
    return json.loads(body) if body else {"status": "empty"}
