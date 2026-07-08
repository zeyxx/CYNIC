"""Shared Kernel /observe transport for organ-twitter adapters."""

from __future__ import annotations

import os
from dataclasses import dataclass

import requests

from organs.contracts.src.adapters.kernel_http import kernel_url
from organs.contracts.src.adapters.kernel_observe import post_kernel_observe


@dataclass(frozen=True)
class ObservePostResult:
    posted: bool
    skipped: bool
    status: int | None = None
    error: str | None = None


def kernel_observe_url(addr: str) -> str:
    return kernel_url(addr, "/observe", allow_empty=True)


def post_observe_payload(payload: dict, *, timeout: float = 10) -> ObservePostResult:
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    url = kernel_observe_url(addr)
    if not url or not key:
        return ObservePostResult(posted=False, skipped=True, error="kernel env not set")
    result = post_kernel_observe(
        request_post=requests.post,
        kernel_url=addr,
        auth_key=key,
        payload=payload,
        timeout=timeout,
    )
    return ObservePostResult(
        posted=result.posted,
        skipped=False,
        status=result.status_code,
        error=result.error,
    )
