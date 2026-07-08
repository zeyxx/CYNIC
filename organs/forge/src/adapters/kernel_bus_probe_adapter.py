"""Kernel bus health probing for organ-forge."""

from __future__ import annotations

import socket
from urllib import error, request
from urllib.parse import urlparse

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from organs.contracts.src.surface_contracts import json_loads_or_default

from .fleet_evidence_adapter import FleetEvidenceAdapter


class KernelBusProbeAdapter:
    def __init__(self, fleet_evidence: FleetEvidenceAdapter | None = None):
        self.fleet_evidence = fleet_evidence or FleetEvidenceAdapter()

    def probe(
        self,
        *,
        base_url: str,
        api_key: str,
        probe_config: dict,
    ) -> dict:
        if not probe_config:
            return {
                "kernel_probe": None,
                "fleet_inventory_probe": None,
            }
        path = probe_config.get("path", "/health")
        timeout = int(probe_config.get("timeout_seconds", 10))
        accepted = set(probe_config.get("accepted_status", [200]))
        headers = {}
        if probe_config.get("requires_bearer_auth", True):
            if not api_key:
                raise RuntimeError("CYNIC_API_KEY is required for kernel bus probe")
            headers = kernel_auth_headers(api_key, content_type=False)
        req = request.Request(
            f"{base_url}{path}",
            method="GET",
            headers=headers,
        )
        kernel_probe = {
            "enabled": True,
            "path": path,
            "accepted_status": sorted(accepted),
            "status": "pending",
        }
        health_payload = None
        try:
            with request.urlopen(req, timeout=timeout) as resp:
                status = resp.status
                raw = resp.read().decode("utf-8", errors="replace")
            health_payload = json_loads_or_default(raw, None)
            if not isinstance(health_payload, dict):
                kernel_probe.update({"status_error": "health_payload_invalid_json"})
        except error.HTTPError as exc:
            status = exc.code
            try:
                raw = exc.read().decode("utf-8", errors="replace")
            except Exception:
                raw = ""
            health_payload = json_loads_or_default(raw, None)
            if not isinstance(health_payload, dict):
                kernel_probe.update({"status_error": "health_payload_invalid_json"})
        except OSError as exc:
            diagnosis = diagnose_kernel_http_failure(
                base_url=base_url,
                path=path,
                timeout_seconds=min(timeout, 3),
            )
            kernel_probe.update(
                {
                    "status": "error",
                    "error": type(exc).__name__,
                    "endpoint": _redacted_endpoint(base_url),
                    "diagnosis": diagnosis,
                }
            )
            raise RuntimeError(
                "kernel bus probe failed: "
                f"{type(exc).__name__}; endpoint={_redacted_endpoint(base_url)}"
                f"; classification={diagnosis['failure_class']}"
                f"{_diagnosis_detail_suffix(diagnosis)}"
                f"{_loopback_hint(base_url)}"
            )

        kernel_probe.update(
            {"status": "ok" if status in accepted else "rejected", "http_status": status}
        )
        if health_payload is not None:
            fleet_inventory_probe = self.fleet_evidence.extract_inventory_from_health(
                health_payload
            )
        else:
            fleet_inventory_probe = {"enabled": True, "status": "missing_body"}
        if status not in accepted:
            raise RuntimeError(
                f"kernel bus probe rejected HTTP {status}; accepted={sorted(accepted)}"
            )
        return {
            "kernel_probe": kernel_probe,
            "fleet_inventory_probe": fleet_inventory_probe,
        }


def diagnose_kernel_http_failure(
    *,
    base_url: str,
    path: str = "/health",
    timeout_seconds: int = 2,
) -> dict[str, object]:
    parsed = urlparse(base_url if "://" in base_url else f"http://{base_url}")
    host = parsed.hostname or ""
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    diagnosis: dict[str, object] = {
        "endpoint": _redacted_endpoint(base_url),
        "path": path,
        "tcp_reachable": False,
        "tcp_error": None,
        "unauthenticated_http_status": None,
        "unauthenticated_http_error": None,
        "failure_class": "unknown",
    }

    try:
        with socket.create_connection((host, port), timeout=timeout_seconds):
            diagnosis["tcp_reachable"] = True
    except OSError as exc:
        diagnosis["tcp_error"] = type(exc).__name__
        diagnosis["failure_class"] = "network_unreachable"
        return diagnosis

    req = request.Request(f"{base_url}{path}", method="GET")
    try:
        with request.urlopen(req, timeout=timeout_seconds) as resp:
            diagnosis["unauthenticated_http_status"] = resp.status
    except error.HTTPError as exc:
        diagnosis["unauthenticated_http_status"] = exc.code
    except OSError as exc:
        diagnosis["unauthenticated_http_error"] = type(exc).__name__
        diagnosis["failure_class"] = "http_path_unreachable"
        return diagnosis

    status = diagnosis["unauthenticated_http_status"]
    if status in {401, 403}:
        diagnosis["failure_class"] = "authenticated_path_stalled"
    elif isinstance(status, int):
        diagnosis["failure_class"] = "auth_probe_failed_after_http_reachability"
    else:
        diagnosis["failure_class"] = "http_path_unreachable"
    return diagnosis


def _redacted_endpoint(base_url: str) -> str:
    parsed = urlparse(base_url)
    host = parsed.hostname or ""
    port = f":{parsed.port}" if parsed.port else ""
    if host in {"localhost"} or host.startswith("127."):
        redacted_host = host
    elif host.startswith("10."):
        redacted_host = "10.x.x.x"
    elif host.startswith("100."):
        redacted_host = "100.x.x.x"
    else:
        redacted_host = "<redacted-host>"
    return f"{parsed.scheme or 'http'}://{redacted_host}{port}"


def _loopback_hint(base_url: str) -> str:
    host = urlparse(base_url).hostname or ""
    if host == "localhost" or host.startswith("127."):
        return "; CYNIC_REST_ADDR points to loopback, source the runtime env with the registry kernel endpoint"
    return ""


def _diagnosis_detail_suffix(diagnosis: dict[str, object]) -> str:
    details: list[str] = []
    if diagnosis.get("tcp_reachable") is True:
        details.append("tcp=reachable")
    unauth_status = diagnosis.get("unauthenticated_http_status")
    if isinstance(unauth_status, int):
        details.append(f"unauth_http_status={unauth_status}")
    unauth_error = diagnosis.get("unauthenticated_http_error")
    if unauth_error:
        details.append(f"unauth_http_error={unauth_error}")
    if not details:
        return ""
    return "; " + "; ".join(details)
