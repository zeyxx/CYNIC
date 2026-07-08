"""Status report builders for organ-wireguard."""

from __future__ import annotations

import json
import socket
from typing import Callable


Resolver = Callable[[str, object], list]


def dns_records_from_hosts(rendered_hosts: str) -> list[dict]:
    records = []
    for line in rendered_hosts.splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) < 2:
            continue
        ip, names = parts[0], parts[1:]
        fqdn_names = sorted(name for name in names if name.endswith(".synapse.cynic"))
        if not fqdn_names:
            continue
        primary_name = fqdn_names[0].removesuffix(".synapse.cynic")
        records.append({"node": primary_name, "ip": ip, "names": fqdn_names})
    return sorted(records, key=lambda item: item["node"])


def dns_check(record: dict, resolver: Resolver = socket.getaddrinfo) -> dict:
    expected_ip = record["ip"]
    checks = []
    for name in record["names"]:
        try:
            resolved_ips = sorted({item[4][0] for item in resolver(name, None)})
            error = None
        except socket.gaierror as exc:
            resolved_ips = []
            error = str(exc)
        status = (
            "resolved_expected_ip"
            if expected_ip in resolved_ips
            else "unresolved"
            if not resolved_ips
            else "resolved_unexpected_ip"
        )
        check = {
            "name": name,
            "expected_ip": expected_ip,
            "resolved_ips": resolved_ips,
            "status": status,
        }
        if error:
            check["error"] = error
        checks.append(check)
    status = (
        "ok"
        if checks and all(check["status"] == "resolved_expected_ip" for check in checks)
        else "no_fqdn_names"
        if not checks
        else "violation"
    )
    return {
        "node": record["node"],
        "ip": expected_ip,
        "status": status,
        "checks": checks,
    }


def mesh_status_payload(
    *,
    registry_ref: str,
    records: list[dict],
    resolve_dns: bool,
    include_records: bool,
    registry_policy: dict | None = None,
    runtime_observation: dict | None = None,
) -> dict:
    registry_policy = registry_policy or {}
    dns_checks = [dns_check(record) for record in records] if resolve_dns else []
    dns_status = "not_checked"
    if resolve_dns:
        dns_status = "ok" if all(check["status"] == "ok" for check in dns_checks) else "violation"
    missing = [
        {"kind": "synapse_dns_resolution", "detail": check}
        for check in dns_checks
        if check["status"] != "ok"
    ] if resolve_dns else []
    drift = []
    legacy_mesh_controller_status = (
        registry_policy.get("legacy_mesh_controller_status")
        or registry_policy.get("organ_wireguard_status")
    )
    if legacy_mesh_controller_status and legacy_mesh_controller_status != "active":
        drift.append(
            {
                "kind": "controller_not_active",
                "expected": "active",
                "observed": legacy_mesh_controller_status,
                "reason": "registry does not declare the legacy Synapse mesh implementation as an active controller",
            }
        )
    runtime_status = "not_observed"
    blocking = []
    if runtime_observation:
        observed_status = runtime_observation.get("status", "unknown")
        runtime_status = "mesh_active" if observed_status == "active" else observed_status
        normalized_drift = runtime_observation.get("drift")
        if isinstance(normalized_drift, list):
            drift.extend(item for item in normalized_drift if isinstance(item, dict))
        else:
            for missing_item in runtime_observation.get("missing", []):
                drift.append(
                    {
                        "kind": "runtime_missing",
                        "missing": missing_item,
                        "reason": "local WireGuard/Synapse runtime observation did not satisfy the declared mesh",
                    }
                )
        normalized_blocking = runtime_observation.get("blocking")
        if isinstance(normalized_blocking, list):
            blocking = [item for item in normalized_blocking if isinstance(item, dict)]
        elif runtime_status in {"blocked", "unknown"}:
            blocking = list(drift)
    elif legacy_mesh_controller_status and legacy_mesh_controller_status != "active":
        runtime_status = "not_active"
    elif runtime_status in {"blocked", "unknown"}:
        blocking = list(drift)
    return {
        "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
        "zone": "synapse.cynic",
        "registry_ref": registry_ref,
        "canonical_service_plane": registry_policy.get("canonical_service_plane", "synapse"),
        "synapse_cidr": registry_policy.get("synapse_cidr"),
        "provider_fallback_planes": registry_policy.get("provider_fallback_planes", []),
        "controller_status": legacy_mesh_controller_status,
        "policy_status": "ok",
        "runtime_status": runtime_status,
        "resolve_dns": resolve_dns,
        "peers": len(records),
        "dns_status": dns_status,
        "records": records if include_records else [],
        "dns_checks": dns_checks if resolve_dns else [],
        "runtime_observation": runtime_observation or {},
        "drift": drift,
        "blocking": blocking,
        "missing": missing,
        "offline": [],
    }


def mesh_status_json(
    *,
    registry_ref: str,
    records: list[dict],
    resolve_dns: bool,
    include_records: bool,
    registry_policy: dict | None = None,
    runtime_observation: dict | None = None,
) -> str:
    return json.dumps(
        mesh_status_payload(
            registry_ref=registry_ref,
            records=records,
            resolve_dns=resolve_dns,
            include_records=include_records,
            registry_policy=registry_policy,
            runtime_observation=runtime_observation,
        )
    )
