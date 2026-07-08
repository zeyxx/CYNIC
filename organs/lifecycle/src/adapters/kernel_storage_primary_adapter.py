from __future__ import annotations

from pathlib import Path
from typing import Any, Dict

import httpx

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from ..core.ports import StoragePrimaryPort
from .kernel_adapter_policy import normalize_kernel_base_url


class KernelStoragePrimaryAdapter(StoragePrimaryPort):
    """HTTP adapter for storage-primary health and backup surfaces."""

    def __init__(self, kernel_base_url: str, api_key: str | None = None, timeout: float = 30.0):
        self.kernel_base_url = normalize_kernel_base_url(kernel_base_url)
        self.api_key = api_key
        self.timeout = timeout

    def _headers(self) -> Dict[str, str]:
        headers = kernel_auth_headers(self.api_key or "", content_type=False)
        headers["Accept"] = "application/json"
        return headers

    async def _get_json(self, path: str) -> Dict[str, Any]:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.get(f"{self.kernel_base_url}{path}", headers=self._headers())
            response.raise_for_status()
            data = response.json()
            return data if isinstance(data, dict) else {"items": data}

    async def status(self) -> Dict[str, Any]:
        payload = await self._get_json("/health")
        return {
            "status": payload.get("storage", "unknown"),
            "readiness": payload.get("readiness", {}),
            "storage_namespace": payload.get("storage_namespace"),
            "storage_database": payload.get("storage_database"),
            "storage_metrics": payload.get("storage_metrics", {}),
        }

    async def readiness(self) -> Dict[str, Any]:
        payload = await self._get_json("/health")
        readiness = payload.get("readiness", {})
        return {
            "storage_ok": readiness.get("storage_ok", False),
            "healthy": readiness.get("healthy", False),
            "status": readiness.get("status", "unknown"),
            "causes": readiness.get("causes", []),
        }

    async def bundle(self, output_dir: str) -> Dict[str, Any]:
        output_path = Path(output_dir).expanduser().resolve()
        output_path.mkdir(parents=True, exist_ok=True)
        bundle_path = output_path / "cynic_backup.surql"
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.get(f"{self.kernel_base_url}/lifecycle/backup", headers=self._headers())
            response.raise_for_status()
            bundle_path.write_bytes(response.content)
        return {
            "status": "bundled",
            "path": str(bundle_path),
            "bytes": bundle_path.stat().st_size,
        }
