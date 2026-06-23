#!/usr/bin/env python3
"""Fleet headscale-alignment audit.

Output a single deterministic JSON object describing:
- expected fleet nodes from SSOT + registry
- observed tailscale peers
- role-target consistency for kernel/build roles
- optional kernel endpoint liveness

This is intended as an execution-time replacement for ad-hoc checks.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import urllib.error
import urllib.request
import re
import sys
import ssl
from urllib.parse import urlparse
from typing import Dict, List, Optional, Set, Tuple
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = ROOT / "infra/registry.json"
ENV_FILE = Path.home() / ".cynic-env"


def load_env_file() -> None:
    if not ENV_FILE.exists():
        return
    try:
        for line in ENV_FILE.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            if line.startswith("export "):
                line = line[len("export ") :]
            key, value = line.split("=", 1)
            key = key.strip()
            if not key or key in os.environ:
                continue
            value = value.strip().strip('"').strip("'")
            os.environ[key] = value
    except OSError:
        return


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def resolve_placeholders(text: str) -> Tuple[str, Set[str]]:
    unresolved: Set[str] = set()

    def repl(match: re.Match[str]) -> str:
        key = match.group(1)
        value = os.environ.get(key, "")
        if not value:
            unresolved.add(f"<{key}>")
            return match.group(0)
        return value

    return re.sub(r"<([A-Z0-9_]+)>", repl, text), unresolved


def shell(cmd: List[str]) -> str:
    out = subprocess.check_output(cmd, stderr=subprocess.STDOUT, text=True, timeout=10)
    return out.strip()


def normalize_headscale_url(raw_url: str) -> str:
    url = raw_url.strip()
    if not url:
        return ""
    if "://" not in url:
        url = f"https://{url}"
    return url


def http_get_json(url: str, headers: Dict[str, str]) -> dict:
    req = urllib.request.Request(url, headers=headers, method="GET")
    try:
        ctx = ssl.create_default_context()
        with urllib.request.urlopen(req, timeout=8, context=ctx) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError:
        ctx = ssl._create_unverified_context()
        with urllib.request.urlopen(req, timeout=8, context=ctx) as response:
            return json.loads(response.read().decode("utf-8"))


def proxmox_request(path: str) -> Tuple[Optional[dict], Optional[str]]:
    env_url = os.environ.get("CYNIC_PROXMOX_URL", "").rstrip("/")
    proxmox_host = os.environ.get("HEADSCALE_HOST_PROXMOX", "").strip()
    candidate_urls = []
    if env_url:
        candidate_urls.append(env_url)
    if proxmox_host:
        candidate_urls.append(f"https://{proxmox_host}:8006")

    if not candidate_urls:
        return None, "missing CYNIC_PROXMOX_URL and missing HEADSCALE_HOST_PROXMOX; set one of them"

    proxmox_token_id = os.environ.get("CYNIC_PROXMOX_TOKEN_ID", "")
    proxmox_secret = os.environ.get("CYNIC_PROXMOX_TOKEN_SECRET", os.environ.get("CYNIC_PROXMOX_SECRET", ""))
    if not proxmox_token_id or not proxmox_secret:
        return None, "missing proxmox env (CYNIC_PROXMOX_TOKEN_ID/TOKEN_SECRET)"

    attempted = []
    last_error = None
    seen = set()

    for raw_proxmox_url in candidate_urls:
        if not raw_proxmox_url:
            continue
        proxmox_url, unresolved = resolve_placeholders(raw_proxmox_url)
        if unresolved:
            last_error = f"unresolved placeholders in CYNIC_PROXMOX_URL: {', '.join(sorted(unresolved))}"
            continue

        if not proxmox_url:
            last_error = "empty CYNIC_PROXMOX_URL candidate after placeholder resolution"
            continue
        if not proxmox_url.startswith("http://") and not proxmox_url.startswith("https://"):
            proxmox_url = f"https://{proxmox_url}"

        if proxmox_url in seen:
            continue
        seen.add(proxmox_url)
        attempted.append(proxmox_url)

        cmd = [
            "curl",
            "-ks",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "-H",
            f"Authorization: PVEAPIToken={proxmox_token_id}={proxmox_secret}",
            f"{proxmox_url}{path}",
        ]
        try:
            out = shell(cmd)
        except Exception as exc:  # pragma: no cover - transport variance
            last_error = f"curl error for {proxmox_url}: {exc}"
            continue
        if not out:
            last_error = f"empty response for {proxmox_url}{path}"
            continue
        try:
            payload = json.loads(out)
        except json.JSONDecodeError as exc:
            last_error = f"invalid JSON for {proxmox_url}{path}: {exc}"
            continue
        if not isinstance(payload, dict) or "data" not in payload:
            last_error = f"unexpected payload for {proxmox_url}{path}: {out[:180]}"
            continue
        return payload, None

    if last_error is None:
        last_error = "no valid proxmox url candidates"
    return (
        None,
        f"{last_error}; tried endpoints: {', '.join(attempted) if attempted else 'none'}",
    )


def proxmox_inventory() -> dict:
    inventory = {
        "enabled": False,
        "error": None,
        "vms": [],
        "vm_by_id": {},
        "expected_by_name": [],
        "missing_by_name": [],
        "unexpected": [],
    }

    qemu_payload, qemu_err = proxmox_request("/api2/json/nodes/pve/qemu")
    lxc_payload, lxc_err = proxmox_request("/api2/json/nodes/pve/lxc")

    if qemu_err and lxc_err:
        inventory["error"] = qemu_err or lxc_err
        return inventory

    inventory["enabled"] = True

    for payload in [qemu_payload, lxc_payload]:
        if payload is None:
            continue
        for vm in payload.get("data", []) or []:
            vmid = vm.get("vmid")
            if vmid is None:
                continue
            vm_type = vm.get("type", "qemu")
            vm_id = str(vmid)
            record = {
                "id": vm_id,
                "type": vm_type,
                "name": vm.get("name", f"{vm_type}/{vm_id}"),
                "status": vm.get("status", "unknown"),
            }
            inventory["vms"].append(record)
            inventory["vm_by_id"][vm_id] = record

    return inventory


def tailscale_peers() -> Tuple[Set[str], List[str], Optional[str]]:
    try:
        return headscale_peers()
    except Exception:
        pass

    try:
        data = json.loads(shell(["headscale", "status", "--json"]))
    except subprocess.CalledProcessError as exc:
        detail = exc.stdout or exc.output or ""
        return set(), [], f"tailscale status failed: {detail.strip()}"
    except Exception as exc:  # pragma: no cover - defensive for unexpected runtime states
        return set(), [], f"tailscale status error: {exc}"
    names = []
    self_name = data.get("Self", {}).get("HostName")
    if self_name:
        names.append(self_name)
    for peer in data.get("Peer", {}).values():
        host = peer.get("HostName")
        if host:
            names.append(host)
    uniq = sorted(set(names))
    online = sorted(
        peer.get("HostName")
        for peer in data.get("Peer", {}).values()
        if peer.get("Online", False) and peer.get("HostName")
    )
    online = sorted(set([name for name in online if name is not None]))
    return set(uniq), online, None


def headscale_peers() -> Tuple[Set[str], List[str], Optional[str]]:
    base_url = normalize_headscale_url(
        next(
            (v for v in (
                os.environ.get("CYNIC_HEADSCALE_URL"),
                os.environ.get("CYNIC_HEADSCALE_API_URL"),
                os.environ.get("HEADSCALE_URL"),
                os.environ.get("HEADSCALE_API_URL"),
                os.environ.get("HEADSCALE_BASE_URL"),
            ) if isinstance(v, str) and v.strip()),
            "",
        )
    )
    token = next(
        (v for v in (
            os.environ.get("CYNIC_HEADSCALE_API_KEY"),
            os.environ.get("CYNIC_HEADSCALE_KEY"),
            os.environ.get("CYNIC_HEADSCALE_TOKEN"),
            os.environ.get("HEADSCALE_API_KEY"),
            os.environ.get("HEADSCALE_KEY"),
            os.environ.get("HEADSCALE_TOKEN"),
        ) if isinstance(v, str) and v.strip()),
        "",
    )

    if not base_url:
        raise RuntimeError("missing-headscale-url")
    if not token:
        raise RuntimeError("missing-headscale-token")

    headers = {"Authorization": f"Bearer {token}"}
    paths = ["/api/v1/node", "/api/v1/nodes", "/api/v1/node/list", "/api/v1/nodes/list"]

    for path in paths:
        try:
            payload = http_get_json(f"{base_url.rstrip('/')}{path}", headers)
        except Exception:
            continue

        nodes: List[dict] = []
        if isinstance(payload, list):
            nodes = payload
        elif isinstance(payload, dict):
            for key in ("nodes", "data", "result", "machines"):
                raw = payload.get(key)
                if isinstance(raw, list):
                    nodes = [node for node in raw if isinstance(node, dict)]
                    break

        if not nodes:
            continue

        names: Set[str] = set()
        online: Set[str] = set()
        for node in nodes:
            raw_name = None
            for key in ("name", "Name", "machine_name", "MachineName", "hostname", "HostName", "given_name", "givenName", "node_name"):
                value = node.get(key)
                if isinstance(value, str) and value.strip():
                    raw_name = value.strip()
                    break
            if raw_name is None:
                continue
            names.add(raw_name)

            online_flag = node.get("online")
            if isinstance(online_flag, bool) and online_flag:
                online.add(raw_name)
                continue
            if isinstance(online_flag, str):
                status = online_flag.strip().lower()
                if status in {"online", "on", "ready", "up"}:
                    online.add(raw_name)
                    continue

            status_flag = node.get("status")
            if isinstance(status_flag, str) and status_flag.strip().lower() in {"online", "on", "ready", "up"}:
                online.add(raw_name)

        if names:
            return names, sorted(online), None

    raise RuntimeError("headscale-unreachable-or-empty")


def collect_tailscale_env_requirements(registry: dict) -> Dict[str, List[str]]:
    required: Dict[str, Set[str]] = {}
    for node in registry.get("nodes", []):
        name = node.get("name", "")
        tailscale_value = node.get("network", {}).get("headscale")
        if not isinstance(tailscale_value, str):
            continue
        for match in re.findall(r"<([A-Z0-9_]+)>", tailscale_value):
            required.setdefault(match, set()).add(name)
    return {name: sorted(nodes) for name, nodes in sorted(required.items())}


def is_loopback_endpoint(addr: str) -> bool:
    normalized = (addr or "").strip()
    if not normalized:
        return False
    url = urlparse(normalized if "://" in normalized else f"http://{normalized}")
    host = (url.hostname or "").lower()
    return host in {"127.0.0.1", "localhost", "::1"}


def resolve_kernel_endpoint(registry: dict) -> Tuple[str, Optional[str]]:
    kernel_endpoint = ""
    for node in registry.get("nodes", []):
        if node.get("role") != "kernel_primary":
            continue
        tailscale_ref = str(node.get("network", {}).get("headscale", ""))
        matches = re.findall(r"<([A-Z0-9_]+)>", tailscale_ref)
        if matches:
            kernel_endpoint = f"<{matches[0]}>:3030"
            break
    if not kernel_endpoint:
        return "", "no kernel_primary tailscale placeholder in registry"

    configured_addr = os.environ.get("CYNIC_REST_ADDR", "").strip()
    if configured_addr:
        configured_addr, unresolved = resolve_placeholders(configured_addr)
        if unresolved:
            return "", f"unresolved placeholders in CYNIC_REST_ADDR: {', '.join(sorted(unresolved))}"
        if is_loopback_endpoint(configured_addr) and os.environ.get("CYNIC_ALLOW_LOCAL_KERNEL") != "1":
            return "", "CYNIC_REST_ADDR resolves to loopback; set a fleet endpoint (e.g. <HEADSCALE_CORE>:3030) or CYNIC_ALLOW_LOCAL_KERNEL=1"
        return configured_addr, None

    configured_addr = kernel_endpoint

    configured_addr, unresolved = resolve_placeholders(configured_addr)
    if unresolved:
        return "", f"unresolved placeholders in kernel endpoint: {', '.join(sorted(unresolved))}"
    if is_loopback_endpoint(configured_addr) and os.environ.get("CYNIC_ALLOW_LOCAL_KERNEL") != "1":
        return "", "desired kernel endpoint resolves to loopback; set a non-local fleet endpoint"
    return configured_addr, None


def resolve_kernel_health(addr: str, endpoint_error: Optional[str] = None) -> dict:
    health = {"configured": False, "ok": False, "status_code": None, "error": None}
    if endpoint_error:
        health["error"] = endpoint_error
        return health
    if not addr:
        health["error"] = "kernel endpoint is empty"
        return health

    endpoint = addr if addr.startswith("http://") or addr.startswith("https://") else f"http://{addr}"
    if endpoint.rstrip("/").endswith("/"):
        endpoint = endpoint.rstrip("/")
    health_url = f"{endpoint}/health"

    health["configured"] = True
    try:
        with urllib.request.urlopen(health_url, timeout=5) as response:
            health["status_code"] = response.getcode()
            health["ok"] = 200 <= response.status <= 503
    except urllib.error.URLError as exc:
        health["error"] = str(exc)
    except Exception as exc:  # pragma: no cover - safety for unexpected transport states
        health["error"] = str(exc)
    return health


def is_truthy_env(name: str) -> bool:
    return os.environ.get(name, "").strip().lower() in {"1", "true", "yes", "on"}


def collect() -> dict:
    load_env_file()

    if not REGISTRY_PATH.exists():
        raise FileNotFoundError(f"Missing registry file: {REGISTRY_PATH}")

    registry = load_json(REGISTRY_PATH)

    expected_nodes = set()
    expected_nodes.update(
        node.get("name")
        for node in registry.get("nodes", [])
        if node.get("name") and node.get("role") != "hypervisor"
    )

    observed, online, tailscale_error = tailscale_peers()
    proxmox = proxmox_inventory()
    tailscale_env_required = collect_tailscale_env_requirements(registry)
    tailscale_env_present = {
        name: name in os.environ and bool(os.environ.get(name))
        for name in tailscale_env_required
    }
    tailscale_env_missing = sorted([name for name, present in tailscale_env_present.items() if not present])
    node_aliases: Dict[str, Set[str]] = {}
    for node in registry.get("nodes", []):
        aliases: Set[str] = set()
        raw_aliases = node.get("aliases", [])
        if isinstance(raw_aliases, str):
            aliases.add(raw_aliases)
        elif isinstance(raw_aliases, list):
            aliases.update([str(x) for x in raw_aliases if isinstance(x, str)])
        aliases.discard("")
        aliases.add(str(node.get("name", "")))
        aliases = {alias for alias in aliases if alias}
        node_name = node.get("name")
        if node_name:
            node_aliases[node_name] = aliases

    observed_names = set(observed)

    expected_by_role = {
        "kernel_primary": [
            node.get("name")
            for node in registry.get("nodes", [])
            if node.get("role") == "kernel_primary" and node.get("name")
        ],
        "build_runner": [
            node.get("name")
            for node in registry.get("nodes", [])
            if node.get("role") == "build_runner" and node.get("name")
        ],
        "inference_node": [
            node.get("name")
            for node in registry.get("nodes", [])
            if node.get("role") == "inference_node" and node.get("name")
        ],
        "cortex_host": [
            node.get("name")
            for node in registry.get("nodes", [])
            if node.get("role") == "cortex_host" and node.get("name")
        ],
        "mesh_control": [
            node.get("name")
            for node in registry.get("nodes", [])
            if node.get("role") == "mesh_control" and node.get("name")
        ],
    }

    expected_roles = {role: sorted(nodes) for role, nodes in expected_by_role.items()}
    proxmox_hosted_roles = {
        role: sorted(
            node_name
            for node_name in nodes
            if any(
                n.get("name") == node_name and n.get("host") == "cynic-host-proxmox"
                for n in registry.get("nodes", [])
            )
        )
        for role, nodes in expected_roles.items()
    }

    proxmox_expected_ids: Set[str] = set()
    proxmox_expected_name: Set[str] = set()
    proxmox_expected_name_aliases: Dict[str, Set[str]] = {}
    proxmox_expected_alias_set: Set[str] = set()
    for node in registry.get("nodes", []):
        if node.get("role") == "hypervisor":
            for vm in node.get("vms", []):
                vm_id = str(vm.get("id", "").split("/")[-1])
                vm_name = vm.get("name", "")
                vm_aliases: Set[str] = set()
                raw_vm_aliases = vm.get("aliases", [])
                if isinstance(raw_vm_aliases, str):
                    vm_aliases.add(raw_vm_aliases)
                elif isinstance(raw_vm_aliases, list):
                    vm_aliases.update([str(x) for x in raw_vm_aliases if isinstance(x, str)])
                vm_aliases.discard("")
                vm_aliases.add(str(vm_name))
                vm_aliases = {alias for alias in vm_aliases if alias}
                if vm_id:
                    proxmox_expected_ids.add(vm_id)
                if vm_name:
                    proxmox_expected_name.add(vm_name)
                if vm_name:
                    proxmox_expected_name_aliases[str(vm_name)] = vm_aliases
                proxmox_expected_alias_set.update(vm_aliases)
        elif node.get("host") == "cynic-host-proxmox":
            vm_name = node.get("name")
            if vm_name:
                vm_name = str(vm_name)
                proxmox_expected_name.add(vm_name)
                aliases = proxmox_expected_name_aliases.setdefault(vm_name, set())
                aliases.update({vm_name})
                raw_aliases = node.get("aliases", [])
                if isinstance(raw_aliases, str):
                    aliases.add(raw_aliases)
                elif isinstance(raw_aliases, list):
                    aliases.update(str(x) for x in raw_aliases if isinstance(x, str))
                proxmox_expected_name_aliases[vm_name] = aliases
                proxmox_expected_alias_set.add(vm_name)

    proxmox_observed_ids = set(proxmox["vm_by_id"].keys())
    proxmox_observed_names = {item["name"] for item in proxmox["vms"]}
    proxmox_missing_ids = sorted(proxmox_expected_ids - proxmox_observed_ids)
    proxmox_present_name = [
        vm_name
        for vm_name, aliases in proxmox_expected_name_aliases.items()
        if aliases & proxmox_observed_names
    ]
    proxmox_missing_names = sorted(proxmox_expected_name - set(proxmox_present_name))
    proxmox_present_ids = sorted(proxmox_expected_ids & proxmox_observed_ids)
    proxmox_unexpected = sorted(
        {item["name"] for item in proxmox["vms"]} - proxmox_expected_alias_set
    )
    proxmox_by_role = {}
    for role, names in proxmox_hosted_roles.items():
        if not names:
            continue
        present_names = []
        missing_names = []
        role_status = {}
        for name in names:
            aliases = node_aliases.get(name, {name})
            observed_alias = sorted(aliases & proxmox_observed_names)
            if observed_alias:
                present_names.append(name)
                vm_id = next(
                    (k for k, v in proxmox["vm_by_id"].items() if v["name"] in observed_alias),
                    "",
                )
                if vm_id:
                    role_status[name] = proxmox["vm_by_id"][vm_id].get("status", "unknown")
            else:
                missing_names.append(name)
        proxmox_by_role[role] = {
            "names": names,
            "present": present_names,
            "missing": missing_names,
            "vm_status": role_status,
        }

    expected_set = set(expected_nodes)
    present = sorted(
        node
        for node, names in node_aliases.items()
        if node in expected_set and names & observed_names
    )
    missing = sorted(expected_set - set(present))
    expected_fleet_aliases = {alias for aliases in node_aliases.values() for alias in aliases}
    extra = sorted(observed - expected_fleet_aliases)

    expected_online = sorted(
        node
        for role_nodes in expected_by_role.values()
        for node in role_nodes
    )
    role_misses = {
        role: sorted(
            set(nodes)
            - {
                name
                for name in nodes
                if name in node_aliases and node_aliases[name] & set(online)
            }
        )
        for role, nodes in expected_by_role.items()
        if nodes
    }

    kernel_addr, kernel_endpoint_error = resolve_kernel_endpoint(registry)
    kernel = resolve_kernel_health(kernel_addr, kernel_endpoint_error)

    return {
        "fleet": {
            "expected_nodes": sorted(expected_set),
            "observed_nodes": sorted(observed),
            "online_nodes": sorted(online),
            "node_aliases": {k: sorted(v) for k, v in node_aliases.items()},
            "tailscale_status_error": tailscale_error,
            "tailscale_env_required": tailscale_env_required,
            "tailscale_env_missing": tailscale_env_missing,
            "tailscale_env_present": tailscale_env_present,
            "present": present,
            "missing": missing,
            "unexpected": extra,
            "expected_online_roles": expected_roles,
            "offline_by_role": role_misses,
            "proxmox": {
                "enabled": proxmox["enabled"],
                "error": proxmox["error"],
                "expected_vms_by_id": sorted(proxmox_expected_ids),
                "observed_vms_by_id": sorted(proxmox_observed_ids),
                "present_vms_by_id": proxmox_present_ids,
                "missing_vms_by_id": proxmox_missing_ids,
                "missing_vms_by_name": proxmox_missing_names,
                "unexpected_vms": proxmox_unexpected,
                "observed_vms": proxmox["vms"],
                "role_presence": proxmox_by_role,
            },
        },
        "kernel": {
            "endpoint": kernel_addr,
            "health": kernel,
        },
        "probes": {
            "expected_online_targets": expected_online,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="Print compact JSON output")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit non-zero when missing role nodes or endpoint is not healthy",
    )
    args = parser.parse_args()

    try:
        report = collect()
    except FileNotFoundError as exc:
        print(json.dumps({
            "error": str(exc),
            "fleet": {
                "expected_nodes": [],
                "observed_nodes": [],
                "online_nodes": [],
                "tailscale_status_error": str(exc),
                "missing": [],
                "unexpected": [],
                "expected_online_roles": {},
                "offline_by_role": {},
                "proxmox": {
                    "enabled": False,
                    "error": str(exc),
                },
            },
            "kernel": {
                "endpoint": os.environ.get("CYNIC_REST_ADDR", ""),
                "health": {
                    "configured": False,
                    "ok": False,
                    "status_code": None,
                    "error": str(exc),
                },
            },
            "probes": {"expected_online_targets": []},
        }, indent=2, sort_keys=True))
        return 2
    except Exception as exc:  # pragma: no cover - runtime exception path
        print(json.dumps({
            "error": str(exc),
            "fleet": {
                "expected_nodes": [],
                "observed_nodes": [],
                "online_nodes": [],
                "tailscale_status_error": str(exc),
                "missing": [],
                "unexpected": [],
                "expected_online_roles": {},
                "offline_by_role": {},
                "proxmox": {
                    "enabled": False,
                    "error": str(exc),
                },
            },
            "kernel": {
                "endpoint": os.environ.get("CYNIC_REST_ADDR", ""),
                "health": {
                    "configured": False,
                    "ok": False,
                    "status_code": None,
                    "error": str(exc),
                },
            },
            "probes": {"expected_online_targets": []},
        }, indent=2, sort_keys=True))
        return 3

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print("fleet observed:", ", ".join(report["fleet"]["observed_nodes"]))
        print("expected:", ", ".join(report["fleet"]["expected_nodes"]))
        print("missing:", ", ".join(report["fleet"]["missing"]) or "<none>")
        print("unexpected:", ", ".join(report["fleet"]["unexpected"]) or "<none>")
        print("kernel_health:", report["kernel"]["health"])

    if not args.strict:
        return 0

    skip_tailscale = is_truthy_env("CYNIC_AUDIT_SKIP_TAILSCALE")
    skip_proxmox = is_truthy_env("CYNIC_AUDIT_SKIP_PROXMOX")
    skip_kernel = is_truthy_env("CYNIC_AUDIT_SKIP_KERNEL")

    if report["fleet"]["tailscale_env_missing"]:
        if not skip_tailscale:
            return 1
        print("INFO: strict tailscale checks skipped", file=sys.stderr)

    if report["fleet"]["tailscale_status_error"] and not skip_tailscale:
        return 1

    if report["fleet"]["missing"] and not skip_tailscale:
        return 1

    if report["fleet"]["proxmox"]["enabled"] and report["fleet"]["proxmox"]["missing_vms_by_id"] and not skip_proxmox:
        return 1
    if (
        report["fleet"]["proxmox"]["error"]
        and "missing proxmox env" not in report["fleet"]["proxmox"]["error"]
        and not skip_proxmox
    ):
        return 1
    if (
        report["kernel"]["health"]["error"] is not None
        and not skip_kernel
    ):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
