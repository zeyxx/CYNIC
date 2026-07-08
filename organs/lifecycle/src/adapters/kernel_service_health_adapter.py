from __future__ import annotations

from typing import Any, Dict

import httpx

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from ..core.ports import ServiceHealthPort
from .kernel_adapter_policy import normalize_kernel_base_url


class KernelServiceHealthAdapter(ServiceHealthPort):
    """HTTP adapter for the kernel service registry and health surface."""

    def __init__(self, kernel_base_url: str, api_key: str | None = None, timeout: float = 10.0):
        self.kernel_base_url = normalize_kernel_base_url(kernel_base_url)
        self.api_key = api_key
        self.timeout = timeout

    def _headers(self) -> Dict[str, str]:
        headers = kernel_auth_headers(self.api_key or "", content_type=False)
        headers["Accept"] = "application/json"
        return headers

    async def _get(self, path: str) -> Any:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.get(f"{self.kernel_base_url}{path}", headers=self._headers())
            response.raise_for_status()
            return response.json()

    async def check_service_health(self, service_name: str) -> Dict[str, Any]:
        services = await self._get("/services")
        for service in services if isinstance(services, list) else []:
            if service.get("service_id") == service_name or service.get("node") == service_name:
                return {"service_name": service_name, "status": "ok", "service": service}
        return {"service_name": service_name, "status": "missing"}

    async def service_status(self, service_name: str) -> Dict[str, Any]:
        health = await self.check_service_health(service_name)
        if health.get("status") != "ok":
            return health
        health_payload = await self._get("/health")
        return {
            "service_name": service_name,
            "status": health_payload.get("status", "unknown") if isinstance(health_payload, dict) else "unknown",
            "service": health.get("service"),
            "kernel": health_payload if isinstance(health_payload, dict) else {"payload": health_payload},
        }
