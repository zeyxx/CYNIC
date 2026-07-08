from __future__ import annotations

from typing import Any, Dict, List

import httpx

from organs.contracts.src.adapters.agent_tasks import (
    agent_task_claim_path,
    agent_task_claim_payload,
    agent_task_collection_path,
    agent_task_items,
    agent_task_listing_params,
    agent_task_result_path,
    agent_task_result_payload,
    coord_claim_path,
    coord_register_path,
    coord_release_path,
    coord_who_path,
)
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from ..core.ports import CoordinationPort
from .kernel_adapter_policy import normalize_kernel_base_url


class KernelCoordinationAdapter(CoordinationPort):
    """HTTP adapter for kernel coordination and task queue surfaces."""

    def __init__(self, kernel_base_url: str, api_key: str | None = None, timeout: float = 10.0):
        self.kernel_base_url = normalize_kernel_base_url(kernel_base_url)
        self.api_key = api_key
        self.timeout = timeout

    def _headers(self) -> Dict[str, str]:
        headers = kernel_auth_headers(self.api_key or "", content_type=False)
        headers["Accept"] = "application/json"
        return headers

    async def _request(
        self,
        method: str,
        path: str,
        payload: Dict[str, Any] | None = None,
        params: Dict[str, Any] | None = None,
    ) -> Dict[str, Any]:
        url = f"{self.kernel_base_url}{path}"
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.request(
                method,
                url,
                json=payload,
                params=params,
                headers=self._headers(),
            )
            response.raise_for_status()
            data = response.json()
            if isinstance(data, dict):
                return data
            return {"items": data}

    async def register_agent(self, agent_id: str, intent: str, agent_type: str) -> Dict[str, Any]:
        return await self._request(
            "POST",
            coord_register_path(),
            {"agent_id": agent_id, "intent": intent, "agent_type": agent_type},
        )

    async def who(self) -> Dict[str, Any]:
        return await self._request("GET", coord_who_path())

    async def claim(self, agent_id: str, target: str) -> Dict[str, Any]:
        return await self._request(
            "POST",
            coord_claim_path(),
            {"agent_id": agent_id, "target": target, "claim_type": "file"},
        )

    async def release(self, agent_id: str) -> Dict[str, Any]:
        return await self._request("POST", coord_release_path(), {"agent_id": agent_id})

    async def list_pending_tasks(self, kind: str, domain: str | None = None) -> List[Dict[str, Any]]:
        params: Dict[str, Any] = agent_task_listing_params(kind, domain)
        params["status"] = "pending"
        data = await self._request("GET", agent_task_collection_path(False), params=params)
        return agent_task_items(data)

    async def list_completed_tasks(self, kind: str, domain: str | None = None) -> List[Dict[str, Any]]:
        params: Dict[str, Any] = agent_task_listing_params(kind, domain)
        data = await self._request("GET", agent_task_collection_path(True), params=params)
        return agent_task_items(data)

    async def claim_agent_task(self, task_id: str, agent_id: str) -> Dict[str, Any]:
        return await self._request("POST", agent_task_claim_path(task_id), agent_task_claim_payload(agent_id))

    async def update_agent_task_result(
        self,
        task_id: str,
        kind: str,
        domain: str,
        result: Dict[str, Any],
    ) -> Dict[str, Any]:
        payload = agent_task_result_payload(kind=kind, domain=domain, result=result)
        return await self._request("POST", agent_task_result_path(task_id), payload)
