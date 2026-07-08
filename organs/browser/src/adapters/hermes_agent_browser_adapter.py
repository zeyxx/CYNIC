from __future__ import annotations

import datetime
import json
import os
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional

import httpx
from organs.contracts.src.surface_contracts import read_json_file_or_default

from ..core.entities import (
    BrowserBackendCapability,
    BrowserToolCall,
    BrowserToolResult,
    EvidenceItem,
)
from ..core.ports import HermesBrowserBackendPort


READ_ONLY_TOOLS = {
    "browser_navigate",
    "browser_snapshot",
    "browser_scroll",
    "browser_back",
    "browser_get_images",
    "browser_vision",
    "browser_console",
    "browser_cdp",
    "browser_dialog",
}

MUTATION_TOOLS = {
    "browser_click",
    "browser_type",
}


class HermesAgentBrowserAdapter(HermesBrowserBackendPort):
    """CYNIC policy adapter for Hermes-compatible browser tool calls."""

    def __init__(self, hub_url: str | None = None) -> None:
        self.hub_url = (hub_url or os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770")).rstrip("/")
        self.state_dir = Path(
            os.environ.get("ORGAN_BROWSER_STATE_DIR", str(Path.home() / ".cynic" / "organs" / "browser"))
        )
        self.task_state_dir = self.state_dir / "hermes_tasks"
        self.task_state_dir.mkdir(parents=True, exist_ok=True)
        self.call_registry_path = self.state_dir / "hermes_task_registry.jsonl"
        self.max_tool_calls_per_task = int(os.environ.get("HERMES_BROWSER_MAX_CALLS_PER_TASK", "120"))
        self.max_mutations_per_task = int(os.environ.get("HERMES_BROWSER_MAX_MUTATIONS_PER_TASK", "8"))

    def capabilities(self) -> List[BrowserBackendCapability]:
        capabilities = [
            BrowserBackendCapability(
                name=name,
                policy_layer="interact",
                mutation_risk="low",
                requires_human_gate=False,
            )
            for name in sorted(READ_ONLY_TOOLS)
        ]
        capabilities.extend(
            BrowserBackendCapability(
                name=name,
                policy_layer="mutate",
                mutation_risk="medium",
                requires_human_gate=True,
            )
            for name in sorted(MUTATION_TOOLS)
        )
        return capabilities

    def call_tool(self, call: BrowserToolCall) -> BrowserToolResult:
        call.task_id = (call.task_id or "").strip() or str(uuid.uuid4())
        task_state = self._load_task_state(call.task_id)

        if self._exceeds_call_budget(task_state):
            return self._blocked(call, f"Task budget exceeded: max {self.max_tool_calls_per_task} calls per task")

        if call.tool_name in MUTATION_TOOLS and not (call.human_gate and call.human_gate.required):
            self._append_registry_event(task_state, "blocked_no_human_gate", call.tool_name)
            self._store_task_state(task_state)
            return self._blocked(call, "Hermes-compatible mutation tool requires an explicit CYNIC human gate")

        if call.tool_name in MUTATION_TOOLS and self._exceeds_mutation_budget(task_state):
            self._append_registry_event(task_state, "blocked_mutation_budget", call.tool_name)
            self._store_task_state(task_state)
            return self._blocked(call, "Task mutation budget exceeded for this task session")

        if call.tool_name not in READ_ONLY_TOOLS and call.tool_name not in MUTATION_TOOLS:
            result = self._blocked(call, f"Unknown Hermes-compatible browser tool: {call.tool_name}")
            self._append_registry_event(task_state, "blocked_unknown_tool", call.tool_name, result.status)
            self._store_task_state(task_state)
            return result

        if call.tool_name == "browser_navigate":
            result = self._render(call)
        elif call.tool_name == "browser_snapshot":
            result = self._snapshot(call)
        elif call.tool_name in ("browser_click", "browser_type"):
            result = self._blocked(
                call,
                "Declared mutation tool is not yet implemented in the local hub path. Use read-only tools first.",
            )
        else:
            result = self._blocked(
                call,
                f"{call.tool_name} is declared as a Hermes-compatible target but is not adapted by the local Hub yet",
            )

        if result.status == "completed" and call.tool_name in MUTATION_TOOLS:
            task_state["mutation_calls"] = task_state.get("mutation_calls", 0) + 1
            result.mutations_performed = [call.tool_name]

        self._append_registry_event(
            task_state,
            "tool_executed",
            call.tool_name,
            status=result.status,
            mutations=result.mutations_performed,
        )
        self._store_task_state(task_state)
        return result

    def task_state(self, task_id: str) -> Dict[str, Any]:
        return self._load_task_state(task_id)

    def _render(self, call: BrowserToolCall) -> BrowserToolResult:
        url = str(call.arguments.get("url", ""))
        if not url.startswith(("http://", "https://")):
            return self._blocked(call, "browser_navigate requires an http(s) url argument")

        try:
            response = self._http_get("/render", params={"url": url}, timeout=30.0)
            response.raise_for_status()
            payload = self._read_json_payload(response)
        except Exception as exc:
            return self._failed(call, str(exc))

        evidence = [
            EvidenceItem(kind="rendered_title", value=str(payload.get("title", ""))),
            EvidenceItem(kind="rendered_markdown", value=str(payload.get("markdown", ""))),
        ]
        return BrowserToolResult(
            task_id=call.task_id,
            tool_name=call.tool_name,
            status="completed",
            observed_at=datetime.datetime.now(datetime.timezone.utc),
            payload=payload,
            evidence=evidence,
        )

    def _snapshot(self, call: BrowserToolCall) -> BrowserToolResult:
        try:
            tabs_response = self._http_get("/tabs", timeout=5.0)
            tabs_response.raise_for_status()
            status_response = self._http_get("/status", timeout=5.0)
            status_response.raise_for_status()
            payload = {
                "tabs": self._read_json_payload(tabs_response),
                "status": self._read_json_payload(status_response),
            }
        except Exception as exc:
            return self._failed(call, str(exc))

        return BrowserToolResult(
            task_id=call.task_id,
            tool_name=call.tool_name,
            status="completed",
            observed_at=datetime.datetime.now(datetime.timezone.utc),
            payload=payload,
            evidence=[
                EvidenceItem(kind="browser_tabs", value=str(len(payload.get("tabs", [])))),
                EvidenceItem(kind="browser_status", value=str(payload.get("status", {}).get("chrome", "unknown"))),
            ],
        )

    def _blocked(self, call: BrowserToolCall, reason: str) -> BrowserToolResult:
        return BrowserToolResult(
            task_id=call.task_id,
            tool_name=call.tool_name,
            status="blocked",
            observed_at=datetime.datetime.now(datetime.timezone.utc),
            blockers=[reason],
        )

    def _failed(self, call: BrowserToolCall, reason: str) -> BrowserToolResult:
        return BrowserToolResult(
            task_id=call.task_id,
            tool_name=call.tool_name,
            status="failed",
            observed_at=datetime.datetime.now(datetime.timezone.utc),
            blockers=[reason],
        )

    def _task_file(self, task_id: str) -> Path:
        return self.task_state_dir / f"{task_id}.json"

    def _load_task_state(self, task_id: str) -> Dict[str, Any]:
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        default_state = {
            "task_id": task_id,
            "created_at": now,
            "last_seen_at": now,
            "tool_calls": 0,
            "mutation_calls": 0,
            "last_tool": None,
            "last_status": None,
            "blocked_count": 0,
        }
        task_file = self._task_file(task_id)
        if not task_file.exists():
            return default_state
        try:
            state = read_json_file_or_default(task_file, {})
        except Exception:
            return default_state

        merged = default_state.copy()
        merged.update(state)
        merged["last_seen_at"] = now
        return merged

    def _store_task_state(self, task_state: Dict[str, Any]) -> None:
        task_state["last_seen_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        try:
            with open(self._task_file(task_state["task_id"]), "w", encoding="utf-8") as f:
                json.dump(task_state, f, indent=2, sort_keys=True)
        except Exception:
            pass

    def _append_registry_event(
        self,
        task_state: Dict[str, Any],
        event_type: str,
        tool_name: str,
        status: str | None = None,
        mutations: Optional[List[str]] = None,
    ) -> None:
        event = {
            "task_id": task_state["task_id"],
            "event": event_type,
            "tool_name": tool_name,
            "status": status,
            "mutations": mutations or [],
            "tool_calls": task_state.get("tool_calls", 0),
            "mutation_calls": task_state.get("mutation_calls", 0),
            "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        }
        try:
            with open(self.call_registry_path, "a", encoding="utf-8") as f:
                f.write(json.dumps(event) + "\n")
        except Exception:
            pass

        task_state["tool_calls"] = task_state.get("tool_calls", 0) + 1
        task_state["last_tool"] = tool_name
        task_state["last_status"] = status
        if status in {"blocked", "failed"}:
            task_state["blocked_count"] = task_state.get("blocked_count", 0) + 1

    def _exceeds_call_budget(self, task_state: Dict[str, Any]) -> bool:
        return task_state.get("tool_calls", 0) >= self.max_tool_calls_per_task

    def _exceeds_mutation_budget(self, task_state: Dict[str, Any]) -> bool:
        return task_state.get("mutation_calls", 0) >= self.max_mutations_per_task

    def _http_get(self, path: str, params: Dict[str, Any] | None = None, timeout: float = 30.0):
        response = httpx.get(f"{self.hub_url}{path}", params=params or {}, timeout=timeout)
        response.raise_for_status()
        return response

    def _read_json_payload(self, response: httpx.Response) -> Dict[str, Any]:
        payload = response.json()
        if isinstance(payload, dict):
            return payload
        return {"value": payload}
