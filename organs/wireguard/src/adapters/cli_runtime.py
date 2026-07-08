"""Runtime functions for the organ-wireguard CLI surface."""

from __future__ import annotations

import asyncio
import argparse
import json
import logging
import os
import shutil
import socket
import subprocess
from pathlib import Path

from organs.contracts.src.surface_contracts import (
    ReportSurfaceSpec,
    SurfaceRegistry,
    attach_surface_contract,
    json_object_or_default,
)
from organs.fleet.src.core import cli_runtime as fleet_cli_runtime
from organs.wireguard.src.adapters.coredns_adapter import CoreDNSAdapter
from organs.wireguard.src.adapters.registry_adapter import RegistryAdapter
from organs.wireguard.src.core.dns_activation import coredns_corefile as _coredns_corefile
from organs.wireguard.src.core.dns_activation import coredns_systemd_unit as _coredns_systemd_unit
from organs.wireguard.src.core.dns_activation import dns_activation_plan as _dns_activation_plan
from organs.wireguard.src.core.dns_activation import dns_activation_readme as _dns_activation_readme
from organs.wireguard.src.core.dns_activation import dns_provider_policy as _dns_provider_policy
from organs.wireguard.src.core.dns_activation import resolved_dropin as _resolved_dropin
from organs.wireguard.src.core.status_reports import dns_records_from_hosts as _dns_records_from_hosts
from organs.wireguard.src.core.status_reports import mesh_status_json as _mesh_status_json
from organs.wireguard.src.core.status_reports import mesh_status_payload as _mesh_status_payload
from organs.wireguard.src.core.runtime_observation import observe_network_runtime as _observe_network_runtime


logger = logging.getLogger("wireguard-cli")

WIREGUARD_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.infrastructure_mcp.network_mesh_status.v1",
    command="cynic status",
)
WIREGUARD_DNS_PLAN_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_plan.v1",
    command="cynic dns plan",
)
WIREGUARD_DNS_BUNDLE_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_bundle.v1",
    command="cynic dns bundle",
)
WIREGUARD_DNS_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_status.v1",
    command="cynic dns activation-status",
)
WIREGUARD_PROVIDER_FALLBACK_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.provider_fallback_hosts.v1",
    command="cynic fallback-hosts",
)
WIREGUARD_PROVIDER_FALLBACK_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.provider_fallback_status.v1",
    command="cynic fallback-status",
)
WIREGUARD_SURFACE_REGISTRY = SurfaceRegistry(
    reports=(
        WIREGUARD_DNS_BUNDLE_SURFACE,
        WIREGUARD_DNS_PLAN_SURFACE,
        WIREGUARD_DNS_STATUS_SURFACE,
        WIREGUARD_PROVIDER_FALLBACK_SURFACE,
        WIREGUARD_PROVIDER_FALLBACK_STATUS_SURFACE,
        WIREGUARD_STATUS_SURFACE,
    ),
)


def run_sync_dns(registry: str, state_dir: str) -> int:
    nodes = RegistryAdapter(registry).fetch_static_topology()
    CoreDNSAdapter(state_dir=state_dir).write_synapse_hosts(nodes)
    print(f"wrote {len(nodes)} synapse DNS records to {Path(state_dir) / 'synapse.hosts'}")
    return 0


def run_validate(registry: str) -> int:
    nodes = RegistryAdapter(registry).fetch_static_topology()
    print(f"network policy ok: {len(nodes)} synapse DNS records")
    return 0


def run_status(registry: str, resolve_dns: bool, include_records: bool, observe_runtime: bool = False) -> int:
    return fleet_cli_runtime.mesh_status(
        argparse.Namespace(
            registry=Path(registry),
            resolve_dns=resolve_dns,
            include_records=include_records,
            observe_runtime=observe_runtime,
        )
    )


def run_dns_plan() -> int:
    return fleet_cli_runtime.dns_plan(argparse.Namespace())


def run_dns_bundle(registry: str, output_dir: str) -> int:
    return fleet_cli_runtime.dns_bundle(
        argparse.Namespace(
            registry=Path(registry),
            output_dir=Path(output_dir),
        )
    )


def run_provider_fallback_hosts(registry: str, output_dir: str) -> int:
    return fleet_cli_runtime.provider_fallback_hosts(
        argparse.Namespace(
            registry=Path(registry),
            output_dir=Path(output_dir),
        )
    )


def run_provider_fallback_status(
    registry: str,
    hosts_path: str,
    check_connectivity: bool,
) -> int:
    return fleet_cli_runtime.provider_fallback_status(
        argparse.Namespace(
            registry=Path(registry),
            hosts_path=Path(hosts_path),
            check_connectivity=check_connectivity,
        )
    )


def run_dns_activation_status(
    hosts_path: str,
    corefile_path: str,
    service_path: str,
    resolved_dropin_path: str,
    resolve_dns: bool,
) -> int:
    return fleet_cli_runtime.dns_activation_status(
        argparse.Namespace(
            registry=Path("infra/registry.json"),
            hosts_path=Path(hosts_path),
            corefile_path=Path(corefile_path),
            service_path=Path(service_path),
            resolved_dropin_path=Path(resolved_dropin_path),
            resolve_dns=resolve_dns,
        )
    )


def _dns_provider_status() -> dict:
    policy = _dns_provider_policy()
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


def _file_contains(path: Path, needle: str) -> bool:
    if not path.exists():
        return False
    try:
        return needle in path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return needle in path.read_text(encoding="utf-8", errors="ignore")


def _capture_status(resolve_dns: bool, include_records: bool) -> str:
    registry_adapter = RegistryAdapter("infra/registry.json")
    nodes = registry_adapter.fetch_static_topology()
    records = _dns_records(nodes)
    return _mesh_status_json(
        registry_ref="infra/registry.json",
        records=records,
        resolve_dns=resolve_dns,
        include_records=include_records,
        registry_policy=registry_adapter.network_policy_summary(),
    )


def _dns_records(nodes: list) -> list[dict]:
    rendered = CoreDNSAdapter(state_dir="/tmp").render_synapse_hosts(nodes)
    return _dns_records_from_hosts(rendered)


def _render_provider_fallback_hosts(records: list[dict]) -> str:
    lines = [
        "# TEMPORARY CYNIC PROVIDER FALLBACK HOSTS",
        "# Generated by organ-wireguard from infra/registry.json network.local values.",
        "# Not canonical. Remove when Synapse/WireGuard DNS is active.",
        "",
    ]
    for record in records:
        names = " ".join(record["names"])
        lines.append(f"{record['ip']}\t{names}")
    return "\n".join(lines) + "\n"


def _run_text(command: list[str], timeout: int = 3) -> tuple[int, str, str]:
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


def _provider_route_status(route: dict) -> dict:
    cidr = route.get("cidr")
    gateway = route.get("gateway")
    _, active_stdout, active_stderr = _run_text(["ip", "route", "show", str(cidr)])
    _, active_connections, nm_active_stderr = _run_text(["nmcli", "-t", "-f", "NAME", "connection", "show", "--active"])
    nm_outputs = []
    nm_errors = []
    for connection_name in [line for line in active_connections.splitlines() if line.strip()]:
        _, routes_stdout, routes_stderr = _run_text(
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


def _provider_connectivity_status(service_checks: list[dict]) -> list[dict]:
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


def run_daemon() -> int:
    if os.geteuid() != 0:
        logger.warning("Daemon is not running as root. Netlink commands will likely fail.")

    from organs.wireguard.src.adapters.netlink_adapter import NetlinkWireguardAdapter
    from organs.wireguard.src.adapters.surreal_adapter import SurrealMeshAdapter
    from organs.wireguard.src.controllers.mesh_agent import MeshAgent

    node_name = os.environ.get("CYNIC_NODE_NAME") or socket.gethostname()
    interface = os.environ.get("CYNIC_WG_INTERFACE", "wg0")
    stun_host = os.environ.get("CYNIC_STUN_HOST", "stun.l.google.com")
    stun_port = int(os.environ.get("CYNIC_STUN_PORT", 19302))
    surreal_url = os.environ.get("SURREALDB_URL", "http://localhost:8000")
    surreal_ns = os.environ.get("SURREALDB_NS", "cynic")
    surreal_db = os.environ.get("SURREALDB_DB", "main")
    surreal_user = os.environ.get("SURREALDB_USER", "root")
    surreal_pass = os.environ.get("SURREALDB_PASS", "")

    if not surreal_pass:
        logger.warning("SURREALDB_PASS is not set.")

    agent = MeshAgent(
        node_name=node_name,
        interface=interface,
        stun_host=stun_host,
        stun_port=stun_port,
        state_port=SurrealMeshAdapter(
            url=surreal_url,
            ns=surreal_ns,
            db=surreal_db,
            user=surreal_user,
            password=surreal_pass,
        ),
        net_port=NetlinkWireguardAdapter(),
        dns_port=CoreDNSAdapter(state_dir="/etc/coredns"),
    )

    try:
        asyncio.run(agent.start())
    except KeyboardInterrupt:
        logger.info("Exiting organ-wireguard daemon.")
        return 0
    return 0
