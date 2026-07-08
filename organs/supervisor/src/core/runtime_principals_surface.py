"""Pure builders for runtime principal infrastructure surface reports."""

from __future__ import annotations


def infra_proof_requirements(report: dict[str, object], *, json_command: str) -> list[dict[str, object]]:
    command = str(report.get("command") or json_command.replace(" --json", ""))
    checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    requirements: list[dict[str, object]] = []

    if command.startswith("cynic bridge status"):
        for check, value in sorted(checks.items()):
            requirements.append(
                {
                    "check": str(check),
                    "status": "satisfied" if value is True else "missing",
                    "command": json_command,
                    "expected": f"bridge activation check {check}=true",
                    "resolver_organ": "organ-fleet",
                    "observed": value,
                }
            )
        if not requirements and report.get("status") != "active":
            requirements.append(
                {
                    "check": "bridge_active",
                    "status": "missing",
                    "command": json_command,
                    "expected": "lxc exec bridge activation status is active",
                    "resolver_organ": "organ-fleet",
                    "observed": report.get("status"),
                }
            )
    elif command.startswith("cynic fleet status"):
        summary = report.get("summary") if isinstance(report.get("summary"), dict) else {}
        requirements.append(
            {
                "check": "fleet_inventory_observed",
                "status": "satisfied" if summary.get("observed_resources", 0) else "missing",
                "command": json_command,
                "expected": "provider or fleet evidence observes declared runtime nodes",
                "resolver_organ": "organ-fleet",
                "observed": summary.get("observed_resources", 0),
            }
        )
        for item in report.get("anomalies", []):
            if not isinstance(item, dict):
                continue
            if item.get("severity") != "critical":
                continue
            requirements.append(
                {
                    "check": str(item.get("kind") or "fleet_anomaly"),
                    "status": "missing",
                    "command": "cynic activate --dry-run",
                    "expected": str(item.get("message") or "critical fleet anomaly is resolved"),
                    "resolver_organ": "organ-fleet",
                    "observed": item.get("target"),
                }
            )
    elif command.startswith("cynic mesh status"):
        requirements.append(
            {
                "check": "mesh_policy_ok",
                "status": "satisfied" if report.get("policy_status") == "ok" else "missing",
                "command": json_command,
                "expected": "Synapse mesh policy status is ok",
                "resolver_organ": "cynic-node",
                "observed": report.get("policy_status"),
            }
        )
        if report.get("dns_status") is not None:
            requirements.append(
                {
                    "check": "mesh_dns_ok",
                    "status": "satisfied" if report.get("dns_status") == "ok" else "missing",
                    "command": "cynic mesh status --resolve-dns --include-records --json",
                    "expected": "all rendered Synapse DNS names resolve to registry IPAM addresses",
                    "resolver_organ": "cynic-node",
                    "observed": report.get("dns_status"),
                }
            )
        if report.get("runtime_status") is not None:
            runtime_status = report.get("runtime_status")
            requirements.append(
                {
                    "check": "wireguard_runtime_observed",
                    "status": "satisfied" if runtime_status in {"ok", "mesh_active"} else "missing",
                    "command": "cynic activate --dry-run",
                    "expected": "wireguard runtime agent reports observed mesh state",
                    "resolver_organ": "cynic-node",
                    "observed": runtime_status,
                }
            )

    return requirements


def infra_next_checks(report: dict[str, object], *, json_command: str) -> list[str]:
    command = str(report.get("command") or json_command.replace(" --json", ""))
    missing = {str(item) for item in report.get("missing", [])}
    checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    failed = {str(key) for key, value in checks.items() if value is False}
    anomalies = report.get("anomalies", [])
    anomaly_kinds = {
        str(item.get("kind"))
        for item in anomalies
        if isinstance(item, dict) and item.get("kind")
    }
    next_checks: list[str] = []
    if command.startswith("cynic storage status"):
        if "bridge_active" in missing or "bridge_active" in failed:
            next_checks.append("cynic bridge status")
        if "fleet_observes_node_running" in missing or "fleet_observes_node_running" in failed:
            next_checks.append("cynic fleet status")
        if "mesh_reaches_storage" in missing or "mesh_reaches_storage" in failed:
            next_checks.append("cynic mesh status --resolve-dns --include-records")
        if "bundle_complete" in missing or "bundle_complete" in failed:
            next_checks.append("cynic storage bundle --output-dir <dir>")
        if "service_health_healthy" in missing or "service_health_healthy" in failed:
            next_checks.append("cynic organism")
    elif command.startswith("cynic bridge status"):
        if missing or failed:
            next_checks.append("cynic bridge plan")
            next_checks.append("cynic activate --dry-run")
    elif command.startswith("cynic fleet status"):
        if "active_without_runtime_proof" in anomaly_kinds:
            next_checks.append("cynic activate --dry-run")
            next_checks.append("cynic actions")
    elif command.startswith("cynic mesh status"):
        if report.get("dns_status") == "not_checked":
            next_checks.append("cynic mesh status --resolve-dns --include-records")
        if report.get("runtime_status") == "not_observed":
            next_checks.append("cynic activate --dry-run")

    deduped: list[str] = []
    for item in next_checks:
        if item not in deduped:
            deduped.append(item)
    return deduped


def infra_report_text(report: dict[str, object], *, title: str, json_command: str) -> str:
    status_value = report.get("status")
    if status_value is None and report.get("anomalies"):
        status_value = "degraded"
    elif status_value is None:
        status_value = "unknown"
    lines = [
        title,
        f"Status: {status_value}",
        f"Command: {report.get('command', json_command.replace(' --json', ''))}",
        f"Mutates mempool: {report.get('mutates_mempool', False)}",
    ]
    for label, key in (
        ("Policy", "policy_status"),
        ("DNS", "dns_status"),
        ("Runtime", "runtime_status"),
        ("Peers", "peers"),
        ("Zone", "zone"),
    ):
        value = report.get(key)
        if value is not None:
            lines.append(f"{label}: {value}")
    target = report.get("target")
    if isinstance(target, dict):
        node = target.get("node")
        service = target.get("service")
        endpoint = target.get("endpoint")
        if node:
            lines.append(f"Node: {node}")
        if service:
            lines.append(f"Service: {service}")
        if endpoint:
            lines.append(f"Endpoint: {endpoint}")
    missing = report.get("missing", [])
    if missing:
        lines.append("Missing:")
        for item in missing[:12]:
            lines.append(f"  - {item}")
        if len(missing) > 12:
            lines.append(f"  - ... {len(missing) - 12} more")
    checks = report.get("checks")
    if isinstance(checks, dict):
        failed = [key for key, value in checks.items() if value is False]
        if failed:
            lines.append("Failed checks:")
            for item in failed[:12]:
                lines.append(f"  - {item}")
            if len(failed) > 12:
                lines.append(f"  - ... {len(failed) - 12} more")
    anomalies = report.get("anomalies", [])
    if anomalies:
        lines.append("Anomalies:")
        for item in anomalies[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('severity', 'unknown')}: {item.get('kind')} {item.get('target')}")
        if len(anomalies) > 8:
            lines.append(f"  - ... {len(anomalies) - 8} more")
    next_checks = report.get("next_checks")
    if not isinstance(next_checks, list):
        next_checks = infra_next_checks(report, json_command=json_command)
    if next_checks:
        lines.append("Next checks:")
        for command in next_checks:
            lines.append(f"  - {command}")
    proof_requirements = report.get("proof_requirements")
    if isinstance(proof_requirements, list) and proof_requirements:
        lines.append("Proof:")
        for item in proof_requirements[:12]:
            if not isinstance(item, dict):
                continue
            lines.append(
                f"  - {item.get('check')}: {item.get('command')} -> {item.get('expected')}"
            )
        if len(proof_requirements) > 12:
            lines.append(f"  - ... {len(proof_requirements) - 12} more")
    lines.append(f"Machine output: {json_command}")
    return "\n".join(lines) + "\n"
