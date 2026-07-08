"""Kernel agent-task REST client for organ-forge."""

from __future__ import annotations

from urllib import request
from urllib.parse import urlparse

from organs.contracts.src.adapters.agent_tasks import (
    agent_task_claim_path,
    agent_task_claim_payload,
    agent_task_collection_path,
    agent_task_listing_query,
    agent_task_result_path,
    agent_task_result_payload,
    task_id_in_agent_task_listings,
)
from organs.contracts.src.adapters.kernel_http import kernel_json_request


class KernelTaskClient:
    def __init__(self, base_url: str, api_key: str):
        self.base_url = base_url
        self.api_key = api_key

    def post_agent_task(self, payload: dict) -> dict:
        status, text, payload_obj = kernel_json_request(
            "POST",
            self.base_url,
            agent_task_collection_path(False),
            self.api_key,
            payload=payload,
            timeout=10,
        )
        if status in (200, 201):
            return payload_obj if isinstance(payload_obj, dict) else {}
        if status is None:
            raise RuntimeError(f"agent task dispatch failed: {text}{self._endpoint_hint()}")
        raise RuntimeError(f"agent task dispatch failed: HTTP {status} {text[:200]}")

    def list_agent_tasks(
        self,
        *,
        kind: str,
        domain: str,
        completed: bool = False,
        limit: int = 50,
    ) -> dict:
        path = agent_task_collection_path(completed)
        query = agent_task_listing_query(kind, domain, limit)
        status, text, payload_obj = kernel_json_request(
            "GET",
            self.base_url,
            f"{path}?{query}",
            self.api_key,
            timeout=10,
        )
        if status in (200, 201):
            return payload_obj if isinstance(payload_obj, dict) else {}
        if status is None:
            raise RuntimeError(f"agent task read-back failed: {text}{self._endpoint_hint()}")
        raise RuntimeError(f"agent task read-back failed: HTTP {status} {text[:200]}")

    def _endpoint_hint(self) -> str:
        parsed = urlparse(self.base_url)
        host = parsed.hostname or ""
        if host == "localhost" or host.startswith("127."):
            port = f":{parsed.port}" if parsed.port else ""
            return (
                f"; endpoint={parsed.scheme or 'http'}://{host}{port}; "
                "CYNIC_REST_ADDR points to loopback, source the runtime env with the registry kernel endpoint"
            )
        return ""

    def assert_agent_task_visible(self, task_id: str, *, kind: str, domain: str) -> None:
        listings = [
            self.list_agent_tasks(kind=kind, domain=domain, completed=False),
            self.list_agent_tasks(kind=kind, domain=domain, completed=True),
        ]
        if not task_id_in_agent_task_listings(task_id, listings):
            raise RuntimeError(
                f"agent task dispatch read-back incomplete: task_id={task_id} "
                f"kind={kind} domain={domain}"
            )

    def claim_agent_task(self, task_id: str, agent_id: str) -> bool:
        status, text, _ = kernel_json_request(
            "POST",
            self.base_url,
            agent_task_claim_path(task_id),
            self.api_key,
            payload=agent_task_claim_payload(agent_id),
            timeout=10,
        )
        if status in (200, 201):
            return True
        if status is None:
            raise RuntimeError(f"agent task claim failed: {text}")
        raise RuntimeError(f"agent task claim failed: HTTP {status} {text[:200]}")

    def submit_agent_task_result(
        self,
        task_id: str,
        *,
        success: bool,
        result: dict | None = None,
        error: object | None = None,
        kind: str | None = None,
        domain: str | None = None,
    ) -> bool:
        status, text, _ = kernel_json_request(
            "POST",
            self.base_url,
            agent_task_result_path(task_id),
            self.api_key,
            payload=agent_task_result_payload(
                success=success,
                result=result,
                error=error,
                kind=kind,
                domain=domain,
            ),
            timeout=10,
        )
        if status in (200, 201):
            return True
        if status is None:
            raise RuntimeError(f"agent task result submit failed: {text}")
        raise RuntimeError(f"agent task result submit failed: HTTP {status} {text[:200]}")
