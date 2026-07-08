from __future__ import annotations

from typing import Any, Dict

import httpx

from organs.contracts.src.adapters.kernel_observe import post_kernel_observe
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from ..core.ports import ObservationPort
from .kernel_adapter_policy import normalize_kernel_base_url


class KernelObservationAdapter(ObservationPort):
    """HTTP adapter for kernel observation and read-back surfaces."""

    def __init__(self, kernel_base_url: str, api_key: str | None = None, timeout: float = 10.0):
        self.kernel_base_url = normalize_kernel_base_url(kernel_base_url)
        self.api_key = api_key
        self.timeout = timeout

    def _headers(self) -> Dict[str, str]:
        headers = kernel_auth_headers(self.api_key or "", content_type=False)
        headers["Accept"] = "application/json"
        return headers

    async def _request(self, method: str, path: str, payload: Dict[str, Any] | None = None, params: Dict[str, Any] | None = None) -> Dict[str, Any]:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.request(
                method,
                f"{self.kernel_base_url}{path}",
                json=payload,
                params=params,
                headers=self._headers(),
            )
            response.raise_for_status()
            data = response.json()
            return data if isinstance(data, dict) else {"items": data}

    async def observe(self, event: Dict[str, Any]) -> Dict[str, Any]:
        result = post_kernel_observe(
            request_post=httpx.post,
            kernel_url=self.kernel_base_url,
            auth_key=self.api_key or "",
            payload=event,
            timeout=self.timeout,
        )
        return {
            "posted": result.posted,
            "status_code": result.status_code,
            "error": result.error,
            "endpoint": f"{self.kernel_base_url}/observe",
        }

    async def audit_query(self, query: Dict[str, Any]) -> Dict[str, Any]:
        params = {key: value for key, value in query.items() if value is not None}
        path = "/observations"
        if params.pop("public", False):
            path = "/observations/public"
        if params.pop("state_history", False):
            path = "/state-history"
        return await self._request("GET", path, params=params)
