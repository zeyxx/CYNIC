"""Shared Kernel /observe HTTP transport."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Callable, Any

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, kernel_url as build_kernel_url


@dataclass(frozen=True)
class KernelObserveResult:
    posted: bool
    status_code: int | None = None
    error: str | None = None


def post_kernel_observe(
    *,
    request_post: Callable[..., Any],
    kernel_url: str,
    auth_key: str,
    payload: dict,
    timeout: float = 15,
) -> KernelObserveResult:
    if not kernel_url:
        return KernelObserveResult(posted=False, error="kernel_url_missing")
    url = build_kernel_url(kernel_url, "/observe", allow_empty=True)
    if not url:
        return KernelObserveResult(posted=False, error="kernel_url_missing")
    try:
        response = request_post(
            url,
            headers=kernel_auth_headers(auth_key),
            data=json.dumps(payload),
            timeout=timeout,
        )
        status_code = int(getattr(response, "status_code", 0))
        return KernelObserveResult(posted=status_code == 200, status_code=status_code)
    except Exception as exc:
        return KernelObserveResult(posted=False, error=str(exc))
