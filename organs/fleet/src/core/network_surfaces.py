from __future__ import annotations

import json
import shutil
import socket
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Callable
from typing import Sequence


Resolver = Callable[[str, object], list]


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    stdout: str
    stderr: str


def dns_provider_policy() -> dict:
    return {
        "canonical_provider": "coredns",
        "canonical_binary": "coredns",
        "selection_source": "organ-fleet Synapse DNS activation contract",
        "service_contract": "*.synapse.cynic",
        "approved_sources": [
            "distribution_package",
            "approved_internal_package_mirror",
        ],
        "known_noncanonical_candidates": [
            "dnsmasq",
            "unbound",
            "bind9",
            "systemd-resolved",
        ],
        "promotion_rule": (
            "A candidate provider becomes usable only after a dedicated adapter contract, "
            "bundle renderer, service template, and activation-status checks are added."
        ),
    }


def dns_activation_plan() -> dict:
    return {
        "schema": "cynic.organ_fleet.dns_activation_plan.v1",
        "mutates": False,
        "target": {
            "zone": "synapse.cynic",
            "hosts": "/etc/coredns/synapse.hosts",
            "corefile": "/etc/coredns/Corefile",
            "service": "/etc/systemd/system/coredns-synapse.service",
            "resolved_dropin": "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
        },
        "required_owner": "root:root",
        "required_modes": {
            "hosts": "0644",
            "corefile": "0644",
            "service": "0644",
            "resolved_dropin": "0644",
        },
        "provider_policy": dns_provider_policy(),
        "package_preconditions": [
            {
                "name": "coredns",
                "binary": "coredns",
                "allowed_sources": [
                    "distribution_package",
                    "approved_internal_package_mirror",
                ],
                "forbidden_sources": [
                    "curl_pipe_shell",
                    "wget_pipe_shell",
                    "unverified_upstream_binary",
                ],
            }
        ],
        "activation_checks": [
            "install CoreDNS from a distribution package or approved internal package mirror",
            "command -v coredns",
            "install -o root -g root -m 0644 synapse.hosts /etc/coredns/synapse.hosts",
            "install -o root -g root -m 0644 Corefile /etc/coredns/Corefile",
            "install -o root -g root -m 0644 coredns-synapse.service /etc/systemd/system/coredns-synapse.service",
            "install -o root -g root -m 0644 cynic-synapse-dns.conf /etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
            "systemctl daemon-reload",
            "systemctl enable --now coredns-synapse.service",
            "systemctl restart systemd-resolved.service",
            "getent hosts organ-surrealdb.synapse.cynic resolves 10.20.0.3",
        ],
        "forbidden": [
            "manual /etc/hosts edits as canonical DNS",
            "192.168.* service endpoint",
            "remote installer piped to shell",
            "wget installer piped to shell",
            "unverified upstream CoreDNS binary",
            "automatic fallback to a noncanonical DNS provider without an adapter contract",
        ],
    }


def coredns_corefile() -> str:
    return """.:1053 {
    errors
    hosts /etc/coredns/synapse.hosts {
        fallthrough
    }
}
"""


def coredns_systemd_unit() -> str:
    return """[Unit]
Description=CYNIC Synapse CoreDNS
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/bin/env coredns -conf /etc/coredns/Corefile
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"""


def resolved_dropin() -> str:
    return """[Resolve]
DNS=127.0.0.1:1053
Domains=~synapse.cynic
"""


def dns_activation_readme() -> str:
    return """# organ-fleet DNS activation bundle

This bundle is non-mutating. Install it only through an approved activation organ
or operator path. Cortex sessions must not install these files directly as root.

CoreDNS binary policy:

- Allowed: distribution package or approved internal package mirror.
- Forbidden: remote installer piped to shell, unverified upstream binary drops.
- Noncanonical providers such as dnsmasq, unbound or bind9 require their own
  adapter contract before they can activate Synapse DNS.

After installation, verify:

1. `systemctl is-active coredns-synapse.service`
2. `getent hosts organ-surrealdb.synapse.cynic`
3. `python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records`
"""


def dns_provider_status() -> dict:
    policy = dns_provider_policy()
    binary_names = [policy["canonical_binary"], "dnsmasq", "unbound", "named", "resolvectl"]
    observed = {name: shutil.which(name) for name in binary_names}
    canonical_available = observed[policy["canonical_binary"]] is not None
    noncanonical_available = [
        name for name, path in observed.items()
        if name != policy["canonical_binary"] and path is not None
    ]
    return {
        "canonical_provider": policy["canonical_provider"],
        "canonical_binary": policy["canonical_binary"],
        "canonical_available": canonical_available,
        "usable_provider": policy["canonical_provider"] if canonical_available else None,
        "observed_binaries": observed,
        "noncanonical_available": noncanonical_available,
        "blocked_reason": None if canonical_available else "canonical_dns_provider_missing",
    }


def file_contains(path: Path, needle: str) -> bool:
    if not path.exists():
        return False
    try:
        return needle in path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return needle in path.read_text(encoding="utf-8", errors="ignore")


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
    elif isinstance(runtime_observation, dict):
        critical_hosts_resolve = (
            runtime_observation.get("dns_projection", {})
            .get("checks", {})
            .get("critical_hosts_resolve")
        )
        if isinstance(critical_hosts_resolve, bool):
            dns_status = "ok" if critical_hosts_resolve else "violation"
    missing = [
        {"kind": "synapse_dns_resolution", "detail": check}
        for check in dns_checks
        if check["status"] != "ok"
    ] if resolve_dns else []
    drift = []
    blocking = []
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


def render_provider_fallback_hosts(records: list[dict]) -> str:
    lines = [
        "# TEMPORARY CYNIC PROVIDER FALLBACK HOSTS",
        "# Generated by organ-fleet from infra/registry.json network.local values.",
        "# Not canonical. Remove when Synapse/WireGuard DNS is active.",
        "",
    ]
    for record in records:
        names = " ".join(record["names"])
        lines.append(f"{record['ip']}\t{names}")
    return "\n".join(lines) + "\n"


def run_text(command: list[str], timeout: int = 3) -> tuple[int, str, str]:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return completed.returncode, completed.stdout.strip(), completed.stderr.strip()
    except (FileNotFoundError, subprocess.TimeoutExpired) as exc:
        return 127, "", str(exc)


def run_command(command: Sequence[str], timeout: int = 3) -> CommandResult:
    try:
        completed = subprocess.run(
            list(command),
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return CommandResult(completed.returncode, completed.stdout.strip(), completed.stderr.strip())
    except FileNotFoundError as exc:
        return CommandResult(127, "", str(exc))
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout.decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        stderr = exc.stderr.decode() if isinstance(exc.stderr, bytes) else (exc.stderr or "")
        return CommandResult(124, stdout.strip(), stderr.strip() or "command timed out")


def observe_network_runtime(
    *,
    interface: str = "wg0",
    synapse_cidr: str = "10.20.0.0/24",
    synapse_probe_ip: str = "10.20.0.1",
) -> dict:
    systemd_unit = f"wg-quick@{interface}"
    systemd = run_command(["systemctl", "is-active", systemd_unit])
    ip_addr = run_command(["ip", "-j", "addr", "show", "dev", interface])
    ip_route = run_command(["ip", "route", "show", synapse_cidr])
    ip_route_get = run_command(["ip", "route", "get", synapse_probe_ip])
    wg_dump = run_command(["wg", "show", interface, "dump"]) if shutil.which("wg") else CommandResult(127, "", "wg binary not found")

    interface_addresses = []
    if ip_addr.returncode == 0 and ip_addr.stdout:
        try:
            parsed = json.loads(ip_addr.stdout)
            for item in parsed:
                for addr in item.get("addr_info", []):
                    local = addr.get("local")
                    prefix = addr.get("prefixlen")
                    if local and prefix is not None:
                        interface_addresses.append(f"{local}/{prefix}")
        except json.JSONDecodeError:
            interface_addresses = []

    interface_exists = ip_addr.returncode == 0
    systemd_active = systemd.returncode == 0 and systemd.stdout == "active"
    wg_active = wg_dump.returncode == 0
    route_uses_interface = f" dev {interface}" in f" {ip_route.stdout} " or f" dev {interface}" in f" {ip_route_get.stdout} "
    status = "mesh_active" if interface_exists and systemd_active and wg_active and route_uses_interface else "blocked"

    missing = []
    if not interface_exists:
        missing.append("wireguard_interface")
    if not systemd_active:
        missing.append("wg_quick_systemd_active")
    if not wg_active:
        missing.append("wg_runtime_state")
    if not route_uses_interface:
        missing.append("synapse_route_via_wireguard_interface")

    drift = [
        {
            "kind": "runtime_missing",
            "missing": missing_item,
            "reason": "local WireGuard/Synapse runtime observation did not satisfy the declared mesh",
        }
        for missing_item in missing
    ]
    return {
        "schema": "cynic.network_mesh.runtime_observation.v1",
        "source": "organ-fleet local observe_network_runtime",
        "interface": interface,
        "synapse_cidr": synapse_cidr,
        "synapse_probe_ip": synapse_probe_ip,
        "status": status,
        "severity": "clear" if status == "mesh_active" else "critical",
        "blockers": missing,
        "missing": missing,
        "drift": drift,
        "blocking": drift if status == "blocked" else [],
        "checks": {
            "interface_exists": interface_exists,
            "systemd_active": systemd_active,
            "wg_binary_available": shutil.which("wg") is not None,
            "wg_active": wg_active,
            "route_uses_interface": route_uses_interface,
        },
        "observed": {
            "systemd_unit": systemd_unit,
            "systemd_status": systemd.stdout or systemd.stderr,
            "interface_addresses": interface_addresses,
            "synapse_route": ip_route.stdout,
            "synapse_route_get": ip_route_get.stdout,
            "wg_peer_lines": len([line for line in wg_dump.stdout.splitlines() if line.strip()]) if wg_active else 0,
        },
        "errors": {
            "ip_addr": ip_addr.stderr if ip_addr.returncode != 0 else None,
            "ip_route": ip_route.stderr if ip_route.returncode != 0 else None,
            "ip_route_get": ip_route_get.stderr if ip_route_get.returncode != 0 else None,
            "wg_show": wg_dump.stderr if wg_dump.returncode != 0 else None,
        },
    }


def provider_route_status(route: dict) -> dict:
    cidr = route.get("cidr")
    gateway = route.get("gateway")
    _, active_stdout, active_stderr = run_text(["ip", "route", "show", str(cidr)])
    _, active_connections, nm_active_stderr = run_text(["nmcli", "-t", "-f", "NAME", "connection", "show", "--active"])
    nm_outputs = []
    nm_errors = []
    for connection_name in [line for line in active_connections.splitlines() if line.strip()]:
        _, routes_stdout, routes_stderr = run_text(
            ["nmcli", "-g", "ipv4.routes", "connection", "show", connection_name]
        )
        if routes_stdout:
            nm_outputs.append(routes_stdout)
        if routes_stderr:
            nm_errors.append(f"{connection_name}: {routes_stderr}")
    nm_stdout = "\n".join(nm_outputs)
    expected = f"{cidr} {gateway}"
    active = bool(cidr and gateway and str(cidr) in active_stdout and str(gateway) in active_stdout)
    persistent = bool(cidr and gateway and expected in nm_stdout)
    return {
        **route,
        "active": active,
        "persistent": persistent,
        "observed_route": active_stdout,
        "observed_nmcli_routes": nm_stdout,
        "errors": {
            "ip_route": active_stderr or None,
            "nmcli": "\n".join([nm_active_stderr, *nm_errors]).strip() or None,
        },
    }


def provider_connectivity_status(service_checks: list[dict]) -> list[dict]:
    results = []
    for check in service_checks:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(2)
        try:
            sock.connect((check["host"], int(check["port"])))
            status = "open"
            network_status = "reachable"
            error = None
        except OSError as exc:
            status = "closed"
            network_status = "blocked" if "timed out" in str(exc).lower() else "reachable"
            error = str(exc)
        finally:
            sock.close()
        results.append({**check, "status": status, "network_status": network_status, "error": error})
    return results
