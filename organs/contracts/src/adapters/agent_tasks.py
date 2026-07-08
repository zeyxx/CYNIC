"""Shared Kernel agent-task HTTP read-back helpers."""

from __future__ import annotations

import urllib.error
import urllib.parse
import urllib.request
import json

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, normalize_kernel_base_url
from organs.contracts.src.surface_contracts import json_object_or_default


def agent_task_claim_path(task_id: str) -> str:
    return f"/agent-tasks/{urllib.parse.quote(str(task_id), safe='')}/claim"


def agent_task_claim_payload(agent_id: str) -> dict[str, str]:
    return {"agent_id": agent_id}


def agent_task_result_path(task_id: str) -> str:
    return f"/agent-tasks/{urllib.parse.quote(str(task_id), safe='')}/result"


def agent_task_result_payload(
    *,
    success: bool | None = None,
    result: object | None = None,
    error: object | None = None,
    kind: str | None = None,
    domain: str | None = None,
) -> dict:
    payload: dict[str, object] = {}
    if success is not None:
        payload["status"] = "completed" if success else "failed"
    if kind:
        payload["kind"] = kind
    if domain:
        payload["domain"] = domain
    if result is not None:
        payload["result"] = _result_value(result)
    if error is not None:
        payload["error"] = _result_value(error)
    return payload


def _result_value(value: object) -> object:
    if isinstance(value, (dict, list)):
        return json.dumps(value, separators=(",", ":"))
    return value


def task_id_in_agent_task_listings(task_id: str, listings: list[dict]) -> bool:
    if not task_id:
        return False
    return any(
        isinstance(task, dict) and task.get("id") == task_id
        for listing in listings
        if isinstance(listing, dict)
        for task in listing.get("tasks", []) or []
    )


def agent_task_items(payload: object) -> list[dict]:
    if isinstance(payload, list):
        return [item for item in payload if isinstance(item, dict)]
    if not isinstance(payload, dict):
        return []
    tasks = payload.get("tasks")
    if isinstance(tasks, list):
        return [item for item in tasks if isinstance(item, dict)]
    items = payload.get("items")
    if isinstance(items, list):
        return [item for item in items if isinstance(item, dict)]
    return []


def agent_task_listing_params(kind: str, domain: str | None = None, limit: int = 50) -> dict[str, object]:
    params: dict[str, object] = {"kind": kind}
    if domain:
        params["domain"] = domain
    params["limit"] = limit
    return params


def agent_task_listing_query(kind: str, domain: str | None = None, limit: int = 50) -> str:
    return urllib.parse.urlencode(agent_task_listing_params(kind, domain, limit))


def agent_task_collection_path(completed: bool = False) -> str:
    return "/agent-tasks/completed" if completed else "/agent-tasks"


def coord_register_path() -> str:
    return "/coord/register"


def coord_who_path() -> str:
    return "/coord/who"


def coord_claim_path() -> str:
    return "/coord/claim"


def coord_release_path() -> str:
    return "/coord/release"


def agent_task_visible(
    base_url: str,
    api_key: str,
    task_id: str,
    *,
    kind: str,
    domain: str,
    limit: int = 50,
    timeout: float = 10.0,
) -> bool:
    if not base_url or not task_id:
        return False

    base = normalize_kernel_base_url(base_url, allow_empty=True)
    if not base:
        return False

    query = agent_task_listing_query(kind, domain, limit)
    headers = kernel_auth_headers(api_key, content_type=False)

    for path in (agent_task_collection_path(False), agent_task_collection_path(True)):
        request = urllib.request.Request(
            f"{base}{path}?{query}",
            headers=headers,
            method="GET",
        )
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                if response.status >= 300:
                    continue
                payload = json_object_or_default(response.read(), {})
        except (urllib.error.URLError, urllib.error.HTTPError, OSError):
            continue

        if task_id_in_agent_task_listings(task_id, [payload]):
            return True

    return False
