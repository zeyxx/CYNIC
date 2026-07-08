"""MCP server runtime adapter for organ-browser."""

from __future__ import annotations

import json
import uuid
from functools import partial
from pathlib import Path
import os
import sys
from typing import Any, Callable

import requests
from organs.contracts.src.surface_contracts import json_loads_or_default
try:
    from organs.browser.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ORGAN_ROOT = bootstrap_repo_path() / "organs" / "browser" / "src"
if str(ORGAN_ROOT) not in sys.path:
    sys.path.insert(0, str(ORGAN_ROOT))

from organs.browser.src.adapters.hermes_agent_browser_adapter import HermesAgentBrowserAdapter
from organs.browser.src.core.entities import BrowserToolCall, HumanGate
from organs.browser.src.core.mcp_tools import HERMES_MUTATION_TOOLS
from organs.browser.src.core.mcp_tools import browser_policy_payload
from organs.browser.src.core.mcp_tools import hermes_tool_specs

DEFAULT_HUB_URL = "http://127.0.0.1:40770"
TIMEOUT_SECONDS = 5.0
SERVER_NAME = "cynic-browser"
SERVER_VERSION = "0.1.0"


def _hermes_adapter() -> HermesAgentBrowserAdapter | None:
    try:
        return HermesAgentBrowserAdapter()
    except Exception:
        return None


def _hub_url() -> str:
    return os.environ.get("ORGAN_BROWSER_HUB_URL", DEFAULT_HUB_URL).rstrip("/")


def _get(path: str, params: dict[str, Any] | None = None, timeout: float = TIMEOUT_SECONDS) -> dict[str, Any]:
    try:
        response = requests.get(f"{_hub_url()}{path}", params=params or {}, timeout=timeout)
        return {
            "ok": response.ok,
            "status_code": response.status_code,
            "hub_url": _hub_url(),
            "data": response.json(),
        }
    except Exception as exc:
        return {
            "ok": False,
            "status_code": None,
            "hub_url": _hub_url(),
            "error": str(exc),
            "diagnosis": "Browser Hub is not reachable from this Codex MCP process.",
        }


def browser_policy() -> dict[str, Any]:
    adapter = _hermes_adapter()
    hermes_capabilities = []
    if adapter is not None:
        for capability in adapter.capabilities():
            hermes_capabilities.append(
                {
                    "name": capability.name,
                    "policy_layer": capability.policy_layer,
                    "mutation_risk": capability.mutation_risk,
                    "requires_human_gate": capability.requires_human_gate,
                    "default_status": "enabled_mutation_requires_human_gate"
                    if capability.requires_human_gate
                    else "enabled_read_only",
                }
            )
    quotas = None
    if adapter is not None:
        quotas = {
            "max_tool_calls_per_task": adapter.max_tool_calls_per_task,
            "max_mutations_per_task": adapter.max_mutations_per_task,
        }
    return browser_policy_payload(
        hermes_capabilities=hermes_capabilities,
        hermes_quotas=quotas,
        hub_url=_hub_url(),
    )


def browser_hermes_tool(tool_name: str, **arguments: Any) -> dict[str, Any]:
    adapter = _hermes_adapter()
    if adapter is None:
        return {"ok": False, "status": "failed", "error": "Hermes adapter unavailable"}

    task_id = str(arguments.pop("task_id", str(uuid.uuid4())))
    requires_human_gate = bool(arguments.pop("human_gate", False))
    human_gate_reason = str(arguments.pop("human_gate_reason", ""))
    policy_layer = "mutate" if tool_name in HERMES_MUTATION_TOOLS else "interact"

    call = BrowserToolCall(
        task_id=task_id,
        tool_name=tool_name,
        arguments=arguments,
        policy_layer=policy_layer,
        human_gate=HumanGate(required=requires_human_gate, reason=human_gate_reason),
    )
    result = adapter.call_tool(call)
    return {
        "ok": result.status == "completed",
        "status": result.status,
        "task_id": result.task_id,
        "tool_name": result.tool_name,
        "observed_at": result.observed_at.isoformat(),
        "payload": result.payload,
        "evidence": [
            {"kind": item.kind, "value": item.value, "redaction": item.redaction} for item in result.evidence
        ],
        "mutations_performed": result.mutations_performed,
        "blockers": result.blockers,
    }


def browser_task_state(task_id: str) -> dict[str, Any]:
    adapter = _hermes_adapter()
    if adapter is None:
        return {"ok": False, "status": "failed", "error": "Hermes adapter unavailable"}
    if not task_id:
        return {"ok": False, "error": "task_id is required"}
    return {"ok": True, "task_id": task_id, "task_state": adapter.task_state(task_id)}


def browser_status() -> dict[str, Any]:
    return _get("/status", timeout=2.0)


def browser_tabs() -> dict[str, Any]:
    return _get("/tabs", timeout=2.0)


def browser_render_public_url(url: str, selector: str | None = None, wait_ms: int = 5000) -> dict[str, Any]:
    if not url.startswith(("http://", "https://")):
        return {"ok": False, "error": "url must start with http:// or https://"}
    if wait_ms < 0 or wait_ms > 20000:
        return {"ok": False, "error": "wait_ms must be between 0 and 20000"}

    params: dict[str, Any] = {"url": url, "wait": str(wait_ms)}
    if selector:
        params["selector"] = selector
    return _get("/render", params=params, timeout=max(5.0, wait_ms / 1000 + 5.0))


TOOLS: dict[str, dict[str, Any]] = {
    "browser_policy": {
        "description": "Return the browser organ safety policy exposed to Codex.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        "handler": browser_policy,
    },
    "browser_status": {
        "description": "Check whether CYNIC Browser Hub is reachable and return its status.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        "handler": browser_status,
    },
    "browser_tabs": {
        "description": "List Browser Hub tabs and ownership metadata.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        "handler": browser_tabs,
    },
    "browser_render_public_url": {
        "description": (
            "Render a public URL through Browser Hub's rendering endpoint. "
            "Do not pass authenticated dashboards, personal pages, wallet pages, or URLs containing secrets."
        ),
        "inputSchema": {
            "type": "object",
            "additionalProperties": False,
            "required": ["url"],
            "properties": {
                "url": {"type": "string", "description": "Public http(s) URL to render."},
                "selector": {"type": "string", "description": "Optional CSS selector to focus extraction."},
                "wait_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 20000,
                    "default": 5000,
                },
            },
        },
        "handler": browser_render_public_url,
    },
    "browser_task_state": {
        "description": "Read Hermes task state (budgets, last tool, counters).",
        "inputSchema": {
            "type": "object",
            "required": ["task_id"],
            "additionalProperties": False,
            "properties": {"task_id": {"type": "string"}},
        },
        "handler": browser_task_state,
    },
}


def _hermes_tools() -> dict[str, dict[str, Any]]:
    adapter = _hermes_adapter()
    if adapter is None:
        return {}
    available = {cap.name for cap in adapter.capabilities()}
    tools: dict[str, dict[str, Any]] = {}
    for tool_name, spec in hermes_tool_specs().items():
        if tool_name not in available:
            continue
        tools[tool_name] = {
            "description": spec["description"],
            "inputSchema": spec["schema"],
            "handler": partial(browser_hermes_tool, tool_name),
        }
    return tools


_dynamically_resolved_hermes_tools = _hermes_tools()
TOOLS.update(_dynamically_resolved_hermes_tools)


def _tool_list() -> list[dict[str, Any]]:
    return [
        {
            "name": name,
            "description": spec["description"],
            "inputSchema": spec["inputSchema"],
        }
        for name, spec in TOOLS.items()
    ]


def _result(request_id: Any, result: Any) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}


def _content(payload: Any) -> dict[str, Any]:
    return {
        "content": [
            {
                "type": "text",
                "text": json.dumps(payload, sort_keys=True),
            }
        ],
        "isError": False,
    }


def _handle(message: dict[str, Any]) -> dict[str, Any] | None:
    method = message.get("method")
    request_id = message.get("id")

    if method == "notifications/initialized":
        return None

    if method == "initialize":
        return _result(
            request_id,
            {
                "protocolVersion": message.get("params", {}).get("protocolVersion", "2024-11-05"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
                "instructions": (
                    "Policy-wrapped access to CYNIC Browser Hub. Read-only by default; "
                    "no submit/post/wallet/sign/trade/stake/deposit/secret storage tools are exposed."
                ),
            },
        )

    if method == "tools/list":
        return _result(request_id, {"tools": _tool_list()})

    if method == "tools/call":
        params = message.get("params", {})
        name = params.get("name")
        arguments = params.get("arguments") or {}
        spec = TOOLS.get(name)
        if not spec:
            return _error(request_id, -32602, f"unknown tool: {name}")
        handler: Callable[..., Any] = spec["handler"]
        try:
            return _result(request_id, _content(handler(**arguments)))
        except TypeError as exc:
            return _error(request_id, -32602, str(exc))
        except Exception as exc:
            return _result(request_id, {"content": [{"type": "text", "text": str(exc)}], "isError": True})

    return _error(request_id, -32601, f"method not found: {method}")


def main() -> int:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            message = json_loads_or_default(line, {})
            response = _handle(message)
        except Exception as exc:
            response = _error(None, -32700, str(exc))
        if response is not None:
            sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
            sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
