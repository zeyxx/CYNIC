from __future__ import annotations

import json
from typing import Any, Dict

import httpx

from organs.contracts.src.adapters.agent_tasks import (
    agent_task_collection_path,
    task_id_in_agent_task_listings,
)
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from organs.contracts.src.surface_contracts import json_object_or_default

from ..core.ports import LifecycleDispatchPort
from .kernel_adapter_policy import (
    build_lifecycle_profile_payloads,
    build_lifecycle_task_payload,
    load_registry,
    normalize_kernel_base_url,
)


class KernelLifecycleDispatchAdapter(LifecycleDispatchPort):
    """HTTP adapter that builds lifecycle payloads and dispatches them to the kernel queue."""

    def __init__(self, kernel_base_url: str, agent_id: str, api_key: str | None = None, timeout: float = 10.0):
        self.kernel_base_url = normalize_kernel_base_url(kernel_base_url)
        self.agent_id = agent_id
        self.api_key = api_key
        self.timeout = timeout
        self.registry = load_registry()

    def _headers(self) -> Dict[str, str]:
        headers = kernel_auth_headers(self.api_key or "")
        headers["Accept"] = "application/json"
        return headers

    async def _post(self, path: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.post(
                f"{self.kernel_base_url}{path}",
                content=json.dumps(payload),
                headers=self._headers(),
            )
            response.raise_for_status()
            data = response.json()
            return data if isinstance(data, dict) else {"items": data}

    async def _get(self, path: str, params: Dict[str, Any]) -> Dict[str, Any]:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.get(
                f"{self.kernel_base_url}{path}",
                params=params,
                headers=self._headers(),
            )
            response.raise_for_status()
            data = response.json()
            return data if isinstance(data, dict) else {"items": data}

    async def dispatch(
        self,
        profile: str,
        intent: str,
        objective: str,
        payload: Dict[str, Any] | None = None,
    ) -> Dict[str, Any]:
        lifecycle = self.registry.get("lifecycle", {}).get("kernel_release", {})
        domain = (payload or {}).get("domain", lifecycle.get("task_domain", "repo-lifecycle"))
        task_payload = build_lifecycle_task_payload(
            intent=intent,
            domain=domain,
            agent_id=self.agent_id,
            registry=self.registry,
        )
        content = json_object_or_default(task_payload["content"], {})
        content["objective"] = objective
        content["profile"] = profile
        if payload:
            content.update(payload)
        request_payload = {
            "kind": lifecycle.get("task_kind", task_payload["kind"]),
            "domain": domain,
            "agent_id": self.agent_id,
            "content": json.dumps(content, separators=(",", ":")),
        }
        result = await self._post(agent_task_collection_path(False), request_payload)
        task_id = str(result.get("task_id") or result.get("id") or "")
        if task_id:
            visible = await self._agent_task_visible(
                task_id,
                kind=str(request_payload.get("kind") or ""),
                domain=str(request_payload.get("domain") or ""),
            )
            if not visible:
                raise RuntimeError(
                    "lifecycle dispatch read-back failed: "
                    f"task_id={task_id} kind={request_payload.get('kind')} domain={request_payload.get('domain')}"
                )
            result["read_back"] = "pending or completed Kernel task visible after dispatch"
        return result

    async def _agent_task_visible(self, task_id: str, *, kind: str, domain: str) -> bool:
        if not task_id or not kind or not domain:
            return False
        params = {"kind": kind, "domain": domain, "limit": 50}
        listings = [
            await self._get(agent_task_collection_path(False), params),
            await self._get(agent_task_collection_path(True), params),
        ]
        return task_id_in_agent_task_listings(task_id, listings)

    async def payloads(self, profile: str) -> Dict[str, Any]:
        lifecycle = self.registry.get("lifecycle", {}).get("kernel_release", {})
        domain = lifecycle.get("task_domain", "repo-lifecycle")
        result = build_lifecycle_profile_payloads(
            profile=profile,
            domain=domain,
            agent_id=self.agent_id,
            registry=self.registry,
        )
        return {
            "profile": profile,
            "domain": domain,
            "agent_id": self.agent_id,
            "payloads": result,
        }
