"""Runtime observation helpers for the Synapse/WireGuard plane."""

from __future__ import annotations

import json
import shutil
import subprocess
from dataclasses import dataclass
from typing import Sequence


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    stdout: str
    stderr: str


def _run(command: Sequence[str], timeout: int = 3) -> CommandResult:
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
    """Observe local runtime state without mutating network configuration."""

    systemd_unit = f"wg-quick@{interface}"
    systemd = _run(["systemctl", "is-active", systemd_unit])
    ip_addr = _run(["ip", "-j", "addr", "show", "dev", interface])
    ip_route = _run(["ip", "route", "show", synapse_cidr])
    ip_route_get = _run(["ip", "route", "get", synapse_probe_ip])
    wg_dump = _run(["wg", "show", interface, "dump"]) if shutil.which("wg") else CommandResult(127, "", "wg binary not found")

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
        "source": "organ-wireguard local observe_network_runtime",
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
