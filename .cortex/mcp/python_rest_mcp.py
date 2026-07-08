import json
import os
import re
import sys
from urllib.parse import urlencode


def _tool(name, description, input_schema, handler):
    return {
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "handler": handler,
    }


def kernel_json_request(method: str, path: str, payload: dict | None = None) -> tuple[int, str, dict]:
    import os
    import json
    from urllib import request, error
    
    base_url = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030").rstrip("/")
    api_key = os.environ.get("CYNIC_API_KEY", "")
    
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
        
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    req = request.Request(f"{base_url}{path}", data=data, headers=headers, method=method)
    try:
        with request.urlopen(req, timeout=10) as resp:
            text = resp.read().decode("utf-8")
            try:
                parsed = json.loads(text)
            except Exception:
                parsed = {}
            return resp.status, text, parsed
    except error.HTTPError as exc:
        text = exc.read().decode("utf-8")
        try:
            parsed = json.loads(text)
        except Exception:
            parsed = {}
        return exc.code, text, parsed
    except Exception as exc:
        return 0, str(exc), {}


def _request(method: str, path: str, payload: dict | None = None) -> dict:
    status, text, parsed = kernel_json_request(method, path, payload)
    if status >= 400:
        raise RuntimeError(f"HTTP {status}: {text}")
    return parsed or {}


def build_lifecycle_profile_payloads(profile: str, domain: str = "organ-forge", agent_id: str = "cortex-fallback") -> dict:
    from organs.forge.src.adapters.kernel_adapter import build_lifecycle_profile_payloads as _build
    return _build(profile=profile, domain=domain, agent_id=agent_id)


def lifecycle_transition_contract() -> dict:
    from organs.forge.src.adapters.registry_adapter import lifecycle_transition_contract as _contract
    return _contract()


def lifecycle_blockers_payload() -> dict:
    from organs.forge.src.entrypoints.lifecycle_diagnostic_cli import lifecycle_blockers_payload as _blockers
    return _blockers()


def cynic_attention(agent_id: str, task_kind: str = "cortex", task_limit: int = 10, task_domain: str | None = None) -> dict:
    path = f"/attention?task_kind={task_kind}&task_limit={task_limit}"
    if task_domain:
        path += f"&task_domain={task_domain}"
    return _request("GET", path)


def cynic_claim_agent_task(task_id: str, agent_id: str) -> dict:
    return _request("POST", f"/agent-tasks/{task_id}/claim", {"agent_id": agent_id})


def cynic_coord_claim(agent_id: str, target: str, claim_type: str | None = None) -> dict:
    payload = {"agent_id": agent_id, "target": target}
    if claim_type:
        payload["claim_type"] = claim_type
    return _request("POST", "/coord/claim", payload)


def cynic_coord_claim_batch(agent_id: str, targets: list[str], claim_type: str | None = None) -> dict:
    payload = {"agent_id": agent_id, "targets": targets}
    if claim_type:
        payload["claim_type"] = claim_type
    return _request("POST", "/coord/claim-batch", payload)


def cynic_coord_register(agent_id: str, intent: str, agent_type: str) -> dict:
    return _request("POST", "/coord/register", {"agent_id": agent_id, "intent": intent, "agent_type": agent_type})


def cynic_coord_release(agent_id: str) -> dict:
    return _request("POST", "/coord/release", {"agent_id": agent_id})


def cynic_coord_who(limit: int | None = None) -> dict:
    path = "/coord/who"
    if limit is not None:
        path += f"?limit={limit}"
    return _request("GET", path)


def cynic_dispatch_agent_task(kind: str | None = None, domain: str | None = None, content: str | None = None, agent_id: str | None = None) -> dict:
    payload = {}
    if kind: payload["kind"] = kind
    if domain: payload["domain"] = domain
    if content: payload["content"] = content
    if agent_id: payload["agent_id"] = agent_id
    return _request("POST", "/agent-tasks", payload)


def cynic_health() -> dict:
    status, text, parsed = kernel_json_request("GET", "/health")
    return {
        "status": parsed.get("status", "unknown") if parsed else "unknown",
        "http_status": status,
    }


def cynic_lifecycle_dispatch(profile: str, agent_id: str = "cortex-fallback") -> dict:
    payloads_data = build_lifecycle_profile_payloads(
        profile=profile,
        domain="organ-forge",
        agent_id=agent_id,
    )
    payloads = payloads_data.get("payloads", [])
    dispatched_ids = []
    for payload in payloads:
        res = _request("POST", "/agent-tasks", payload)
        task_id = res.get("task_id")
        if task_id:
            dispatched_ids.append(task_id)
            
    visible_dispatched_ids = []
    missing_dispatched_ids = []
    
    for task_id in dispatched_ids:
        pending = _request("GET", f"/agent-tasks?kind=hermes&domain=organ-forge")
        pending_tasks = pending.get("tasks", []) if isinstance(pending, dict) else []
        completed = _request("GET", f"/agent-tasks/completed?kind=hermes&domain=organ-forge")
        completed_tasks = completed.get("tasks", []) if isinstance(completed, dict) else []
        
        all_tasks = pending_tasks + completed_tasks
        if any(t.get("task_id") == task_id for t in all_tasks):
            visible_dispatched_ids.append(task_id)
        else:
            missing_dispatched_ids.append(task_id)
            
    if missing_dispatched_ids:
        return {
            "status": "error",
            "visible_dispatched_ids": visible_dispatched_ids,
            "missing_dispatched_ids": missing_dispatched_ids,
        }
    return {
        "status": "dispatched",
        "visible_dispatched_ids": visible_dispatched_ids,
        "missing_dispatched_ids": [],
    }


def cynic_lifecycle_payloads(profile: str, agent_id: str = "cortex-fallback") -> dict:
    return build_lifecycle_profile_payloads(
        profile=profile,
        domain="organ-forge",
        agent_id=agent_id,
    )


def cynic_lifecycle_status() -> dict:
    from organs.forge.src.core.lifecycle_status import build_lifecycle_status_report
    
    contract = lifecycle_transition_contract()
    blockers = lifecycle_blockers_payload()
    return build_lifecycle_status_report(contract, blockers)


def cynic_list_completed_agent_tasks(kind: str | None = None, domain: str | None = None, limit: int | None = None) -> dict:
    path = "/agent-tasks/completed?"
    params = []
    if kind: params.append(f"kind={kind}")
    if domain: params.append(f"domain={domain}")
    if limit is not None: params.append(f"limit={limit}")
    return _request("GET", path + "&".join(params))


def cynic_list_pending_agent_tasks(kind: str | None = None, domain: str | None = None, limit: int | None = None) -> dict:
    path = "/agent-tasks?"
    params = []
    if kind: params.append(f"kind={kind}")
    if domain: params.append(f"domain={domain}")
    if limit is not None: params.append(f"limit={limit}")
    return _request("GET", path + "&".join(params))


def cynic_observe(agent_id: str | None = None, kind: str | None = None, domain: str | None = None, content: str | None = None, status: str | None = None) -> dict:
    payload = {}
    if agent_id: payload["agent_id"] = agent_id
    if kind: payload["kind"] = kind
    if domain: payload["domain"] = domain
    if content: payload["content"] = content
    if status: payload["status"] = status
    return _request("POST", "/observe", payload)


def cynic_library_query(
    domain: str | None = None,
    target: str | None = None,
    agent_id: str | None = None,
    project: str | None = None,
    workstream: str | None = None,
    thread_id: str | None = None,
    limit: int | None = None,
) -> dict:
    params = {}
    if domain: params["domain"] = domain
    if target: params["target"] = target
    if agent_id: params["agent_id"] = agent_id
    if project: params["project"] = project
    if workstream: params["workstream"] = workstream
    if thread_id: params["thread_id"] = thread_id
    if limit is not None: params["limit"] = min(limit, 100)
    query = f"?{urlencode(params)}" if params else ""
    return _request("GET", f"/observations{query}")


def cynic_update_agent_task_result(task_id: str, agent_id: str, status: str, result: str) -> dict:
    return _request("POST", f"/agent-tasks/{task_id}/result", {"agent_id": agent_id, "status": status, "result": result})



def cynic_watch_pending_agent_tasks(kind: str | None = None, domain: str | None = None) -> dict:
    # Fallback to list
    return cynic_list_pending_agent_tasks(kind=kind, domain=domain, limit=50)

TOOLS = {
    "cynic_attention": _tool("cynic_attention", "Get attention pressure", {}, cynic_attention),
    "cynic_claim_agent_task": _tool("cynic_claim_agent_task", "Claim task", {}, cynic_claim_agent_task),
    "cynic_coord_claim": _tool("cynic_coord_claim", "Claim coordination target", {}, cynic_coord_claim),
    "cynic_coord_claim_batch": _tool("cynic_coord_claim_batch", "Claim multiple targets", {}, cynic_coord_claim_batch),
    "cynic_coord_register": _tool("cynic_coord_register", "Register agent session", {}, cynic_coord_register),
    "cynic_coord_release": _tool("cynic_coord_release", "Release agent claims", {}, cynic_coord_release),
    "cynic_coord_who": _tool("cynic_coord_who", "List active agents and claims", {}, cynic_coord_who),
    "cynic_dispatch_agent_task": _tool("cynic_dispatch_agent_task", "Dispatch task", {}, cynic_dispatch_agent_task),
    "cynic_health": _tool("cynic_health", "Read kernel health", {}, cynic_health),
    "cynic_lifecycle_dispatch": _tool("cynic_lifecycle_dispatch", "Dispatch release profile", {}, cynic_lifecycle_dispatch),
    "cynic_lifecycle_payloads": _tool("cynic_lifecycle_payloads", "Dry-run release payloads", {}, cynic_lifecycle_payloads),
    "cynic_lifecycle_status": _tool("cynic_lifecycle_status", "Read release validation status", {}, cynic_lifecycle_status),
    "cynic_list_completed_agent_tasks": _tool("cynic_list_completed_agent_tasks", "List completed tasks", {}, cynic_list_completed_agent_tasks),
    "cynic_watch_pending_agent_tasks": _tool("cynic_watch_pending_agent_tasks", "Watch pending tasks", {}, cynic_watch_pending_agent_tasks),
    "cynic_list_pending_agent_tasks": _tool("cynic_list_pending_agent_tasks", "List pending tasks", {}, cynic_list_pending_agent_tasks),
    "cynic_observe": _tool("cynic_observe", "Publish structured observation", {}, cynic_observe),
    "cynic_library_query": _tool("cynic_library_query", "Query CYNIC dynamic library observations", {}, cynic_library_query),
    "cynic_update_agent_task_result": _tool("cynic_update_agent_task_result", "Complete task with result", {}, cynic_update_agent_task_result),
}


def _handle(request: dict) -> dict:
    method = request.get("method")
    req_id = request.get("id")
    if method == "initialize":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "serverInfo": {
                    "name": "cynic-python-rest-mcp",
                    "version": "1.0.0",
                },
                "capabilities": {
                    "tools": {},
                },
                "protocolVersion": "2024-11-05",
            }
        }
    if method == "notifications/initialized":
        return {}
    if method == "tools/list":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {
                        "name": name,
                        "description": t["description"],
                        "inputSchema": t["inputSchema"] or {"type": "object", "properties": {}},
                    }
                    for name, t in TOOLS.items()
                ]
            }
        }
    if method == "tools/call":
        params = request.get("params", {})
        name = params.get("name")
        arguments = params.get("arguments", {})
        if name in TOOLS:
            try:
                res = TOOLS[name]["handler"](**arguments)
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(res) if isinstance(res, (dict, list)) else str(res),
                            }
                        ]
                    }
                }
            except Exception as e:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32603,
                        "message": str(e),
                    }
                }
        else:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32602,
                    "message": f"Method not found: {name}",
                }
            }
    return {
        "jsonrpc": "2.0",
        "id": req_id,
        "error": {
            "code": -32601,
            "message": f"Method not found: {method}",
        }
    }


def _read_message(stream) -> dict | None:
    line = stream.readline()
    if not line:
        return None

    stripped = line.strip()
    if not stripped:
        return _read_message(stream)

    if stripped.startswith(b"{"):
        return json.loads(stripped.decode("utf-8"))

    headers = {}
    while stripped:
        key, _, value = stripped.decode("ascii").partition(":")
        headers[key.lower()] = value.strip()
        line = stream.readline()
        if not line:
            return None
        stripped = line.strip()

    length = int(headers.get("content-length", "0"))
    if length <= 0:
        return None
    body = stream.read(length)
    if not body:
        return None
    return json.loads(body.decode("utf-8"))


def _write_message(stream, response: dict) -> None:
    if not response:
        return
    body = json.dumps(response, separators=(",", ":")).encode("utf-8")
    stream.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
    stream.write(body)
    stream.flush()


def main() -> int:
    while True:
        request = _read_message(sys.stdin.buffer)
        if request is None:
            return 0
        response = _handle(request)
        _write_message(sys.stdout.buffer, response)


if __name__ == "__main__":
    raise SystemExit(main())
