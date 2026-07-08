"""Shared Kernel HTTP membrane helpers."""

from __future__ import annotations

import json
import os
from typing import Any
from urllib import error, request
from urllib.parse import urlparse

from organs.contracts.src.surface_contracts import json_object_or_default


def normalize_kernel_base_url(
    raw: str,
    *,
    allow_empty: bool = False,
    reject_loopback: bool = True,
    allow_loopback_env: str = "CYNIC_ALLOW_LOCAL_KERNEL",
) -> str:
    value = (raw or "").strip()
    if not value:
        if allow_empty:
            return ""
        raise RuntimeError("CYNIC_REST_ADDR is required (SSOT kernel endpoint).")

    if not value.startswith(("http://", "https://")):
        value = f"http://{value}"

    parsed = urlparse(value)
    host = parsed.hostname or ""
    is_loopback = host == "localhost" or host.startswith("127.")
    if reject_loopback and is_loopback and os.getenv(allow_loopback_env, "0") != "1":
        if allow_empty:
            return ""
        raise RuntimeError("Loopback kernel endpoint rejected by SSOT policy.")

    return value.rstrip("/")


def kernel_auth_headers(api_key: str, *, content_type: bool = True) -> dict[str, str]:
    headers: dict[str, str] = {}
    if content_type:
        headers["Content-Type"] = "application/json"
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    return headers


def kernel_url(base_url: str, path: str, *, allow_empty: bool = False) -> str:
    base = normalize_kernel_base_url(base_url, allow_empty=allow_empty)
    if not base:
        return ""
    return f"{base}{path if path.startswith('/') else '/' + path}"


def kernel_json_request(
    method: str,
    base_url: str,
    path: str,
    api_key: str,
    *,
    payload: dict[str, Any] | None = None,
    timeout: float = 10.0,
) -> tuple[int | None, str, Any]:
    url = kernel_url(base_url, path, allow_empty=True)
    if not url:
        return None, "kernel_url_missing", None
    body = json.dumps(payload).encode("utf-8") if payload is not None else None
    req = request.Request(
        url,
        data=body,
        method=method,
        headers=kernel_auth_headers(api_key, content_type=payload is not None),
    )
    try:
        with request.urlopen(req, timeout=timeout) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            status = getattr(resp, "status", 200)
            return status, text, json_object_or_default(text, {})
    except error.HTTPError as exc:
        text = exc.read().decode("utf-8", errors="replace")
        return exc.code, text, json_object_or_default(text, {})
    except OSError as exc:
        return None, str(exc), None
