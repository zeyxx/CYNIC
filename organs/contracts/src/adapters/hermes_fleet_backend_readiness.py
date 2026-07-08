"""Probe CYNIC fleet inference backend readiness for Hermes Agent."""

from __future__ import annotations

from pathlib import Path
import json
import socket
from typing import Any
from urllib.parse import urlparse, urlunparse
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from organs.contracts.src.adapters.hermes_fleet_config_projection import (
    render_hermes_fleet_config_projection,
)
from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso


HERMES_FLEET_BACKEND_READINESS_SCHEMA = "cynic.hermes_fleet_backend_readiness.v1"


def probe_hermes_fleet_backend_readiness(
    *,
    backends_toml: Path,
    registry_json: Path | None = None,
    expected_backend_id: str = "qwen36-27b-gpu",
    timeout_secs: float = 5.0,
    observed_at: str | None = None,
) -> dict[str, Any]:
    projection = render_hermes_fleet_config_projection(
        backends_toml=backends_toml,
        registry_json=registry_json,
        expected_backend_id=expected_backend_id,
        observed_at=observed_at,
    )
    patch = projection["config_patch"]
    base_url = str(patch["model"]["base_url"]).rstrip("/")
    expected_model = str(patch["model"]["default"])
    url = f"{base_url}/models"
    endpoint_evidence = projection.get("endpoint_evidence", {})
    synapse_ip = endpoint_evidence.get("resolved_synapse_ip") if isinstance(endpoint_evidence, dict) else None
    evidence: dict[str, Any] = {
        "url": url,
        "expected_model": expected_model,
        "backend_id": expected_backend_id,
        "dns": _dns_evidence(base_url),
        "synapse_ip_probe": None,
    }
    violations: list[dict[str, Any]] = []
    model_ids: list[str] = []
    http_status: int | None = None
    requires_synapse_dns = str(evidence["dns"]["host"]).endswith(".synapse.cynic")
    if requires_synapse_dns and evidence["dns"]["status"] != "resolved":
        violations.append(
            {
                "kind": "synapse_dns_unresolved",
                "host": evidence["dns"]["host"],
                "error": evidence["dns"].get("error"),
            }
        )
    try:
        request = Request(url, headers={"Accept": "application/json"})
        with urlopen(request, timeout=timeout_secs) as response:
            http_status = int(response.status)
            body = response.read(2_000_000).decode("utf-8", errors="replace")
        payload = json.loads(body)
        model_ids = _model_ids(payload)
        if expected_model not in model_ids:
            violations.append(
                {
                    "kind": "expected_model_not_listed",
                    "expected": expected_model,
                    "observed_models": model_ids[:50],
                }
            )
    except HTTPError as exc:
        http_status = int(exc.code)
        violations.append({"kind": "http_error", "status": http_status})
    except (URLError, TimeoutError, OSError) as exc:
        violations.append({"kind": "connection_error", "error": str(exc)[:500]})
    except json.JSONDecodeError as exc:
        violations.append({"kind": "invalid_json", "error": str(exc)})

    if synapse_ip and requires_synapse_dns and evidence["dns"]["status"] != "resolved":
        ip_probe = _probe_models_url(_replace_url_host(base_url, str(synapse_ip)), expected_model, timeout_secs)
        evidence["synapse_ip_probe"] = ip_probe
        if ip_probe["status"] == "connection_error":
            violations.append(
                {
                    "kind": "synapse_mesh_route_unreachable",
                    "url": ip_probe["url"],
                    "error": ip_probe.get("error"),
                }
            )
        elif ip_probe["status"] == "ready":
            violations.append(
                {
                    "kind": "synapse_dns_required_but_ip_reachable",
                    "url": ip_probe["url"],
                }
            )
        elif ip_probe["status"] == "expected_model_not_listed":
            violations.append(
                {
                    "kind": "synapse_ip_expected_model_not_listed",
                    "url": ip_probe["url"],
                    "observed_models": ip_probe.get("observed_model_ids", [])[:50],
                }
            )
        elif ip_probe["status"] == "http_error":
            violations.append(
                {
                    "kind": "synapse_ip_http_error",
                    "url": ip_probe["url"],
                    "status": ip_probe.get("http_status"),
                }
            )
        elif ip_probe["status"] == "invalid_json":
            violations.append({"kind": "synapse_ip_invalid_json", "url": ip_probe["url"], "error": ip_probe.get("error")})

    return {
        "schema": HERMES_FLEET_BACKEND_READINESS_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "status": "ready" if not violations else "degraded",
        "mutates": False,
        "backend_id": expected_backend_id,
        "base_url": base_url,
        "models_url": url,
        "http_status": http_status,
        "expected_model": expected_model,
        "observed_model_ids": model_ids,
        "projection": projection,
        "violations": violations,
        "next_actions": _next_actions(violations),
        "evidence": evidence,
    }


def probe_hermes_fleet_backend_readiness_from_args(args: list[str]) -> dict[str, Any]:
    backends_toml = Path("backends.toml")
    registry_json: Path | None = Path("infra/registry.json")
    expected_backend = "qwen36-27b-gpu"
    timeout_secs = 5.0
    index = 0
    while index < len(args):
        if args[index] == "--backends-toml" and index + 1 < len(args):
            backends_toml = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--registry" and index + 1 < len(args):
            registry_json = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--expected-backend" and index + 1 < len(args):
            expected_backend = args[index + 1]
            index += 2
            continue
        if args[index] == "--timeout-secs" and index + 1 < len(args):
            timeout_secs = float(args[index + 1])
            index += 2
            continue
        raise RuntimeError(
            "usage: hermes-fleet-backend-readiness [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"
        )
    return probe_hermes_fleet_backend_readiness(
        backends_toml=backends_toml,
        registry_json=registry_json if registry_json and registry_json.exists() else None,
        expected_backend_id=expected_backend,
        timeout_secs=timeout_secs,
    )


def _model_ids(payload: Any) -> list[str]:
    if not isinstance(payload, dict):
        return []
    data = payload.get("data")
    if not isinstance(data, list):
        return []
    ids: list[str] = []
    for item in data:
        if isinstance(item, dict) and item.get("id"):
            ids.append(str(item["id"]))
    return ids


def _dns_evidence(base_url: str) -> dict[str, Any]:
    parsed = urlparse(base_url)
    host = parsed.hostname or ""
    if not host:
        return {"host": host, "status": "missing_host", "resolved_ips": []}
    try:
        infos = socket.getaddrinfo(host, None)
        ips = sorted({item[4][0] for item in infos if item and item[4]})
    except OSError as exc:
        return {"host": host, "status": "unresolved", "resolved_ips": [], "error": str(exc)[:500]}
    return {"host": host, "status": "resolved" if ips else "unresolved", "resolved_ips": ips}


def _replace_url_host(base_url: str, host: str) -> str:
    parsed = urlparse(base_url)
    netloc = host
    if parsed.port:
        netloc = f"{host}:{parsed.port}"
    if parsed.username:
        credentials = parsed.username
        if parsed.password:
            credentials = f"{credentials}:{parsed.password}"
        netloc = f"{credentials}@{netloc}"
    return urlunparse((parsed.scheme, netloc, parsed.path, parsed.params, parsed.query, parsed.fragment)).rstrip("/")


def _probe_models_url(base_url: str, expected_model: str, timeout_secs: float) -> dict[str, Any]:
    url = f"{base_url.rstrip('/')}/models"
    try:
        request = Request(url, headers={"Accept": "application/json"})
        with urlopen(request, timeout=timeout_secs) as response:
            http_status = int(response.status)
            body = response.read(2_000_000).decode("utf-8", errors="replace")
        payload = json.loads(body)
        model_ids = _model_ids(payload)
        if expected_model not in model_ids:
            return {
                "status": "expected_model_not_listed",
                "url": url,
                "http_status": http_status,
                "observed_model_ids": model_ids,
            }
        return {"status": "ready", "url": url, "http_status": http_status, "observed_model_ids": model_ids}
    except HTTPError as exc:
        return {"status": "http_error", "url": url, "http_status": int(exc.code)}
    except (URLError, TimeoutError, OSError) as exc:
        return {"status": "connection_error", "url": url, "error": str(exc)[:500]}
    except json.JSONDecodeError as exc:
        return {"status": "invalid_json", "url": url, "error": str(exc)}


def _next_actions(violations: list[dict[str, Any]]) -> list[dict[str, str]]:
    kinds = {str(item.get("kind")) for item in violations}
    actions: list[dict[str, str]] = []
    if "synapse_dns_unresolved" in kinds:
        actions.append(
            {
                "owner": "organ-fleet",
                "action": "activate_synapse_dns",
                "command": "python3 organs/fleet/src/entrypoints/cli.py dns plan",
                "done_when": "cynic-host-gpu.synapse.cynic resolves to the registry Synapse IP",
            }
        )
    if "synapse_mesh_route_unreachable" in kinds:
        actions.append(
            {
                "owner": "organ-fleet",
                "action": "restore_synapse_mesh_route",
                "command": "python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records",
                "done_when": "10.20.0.7:8080 is reachable from the Hermes executor host",
            }
        )
    if "expected_model_not_listed" in kinds or "synapse_ip_expected_model_not_listed" in kinds:
        actions.append(
            {
                "owner": "cynic-host-gpu",
                "action": "restart_expected_llama_server_model",
                "command": "schtasks /Run /TN llama-server-27b",
                "done_when": "/v1/models lists Qwen3.6-27B-IQ3_XXS.gguf",
            }
        )
    if "connection_error" in kinds and "synapse_mesh_route_unreachable" not in kinds:
        actions.append(
            {
                "owner": "cynic-host-gpu",
                "action": "check_inference_service_socket",
                "command": "curl -fsS http://cynic-host-gpu.synapse.cynic:8080/v1/models",
                "done_when": "OpenAI-compatible models endpoint returns HTTP 200",
            }
        )
    return actions
