#!/usr/bin/env bash
# CYNIC fleet SSOT env bootstrap helper.
#
# Use cases:
#   scripts/fleet-bootstrap-env.sh --print
#   scripts/fleet-bootstrap-env.sh --check
#   scripts/fleet-bootstrap-env.sh --resolve-tailscale
#   scripts/fleet-bootstrap-env.sh --resolve-headscale

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
MODE="--print"
RESOLVE_TAILSCALE="0"
RESOLVE_HEADSCALE="0"

if [[ $# -gt 0 ]]; then
  MODE="$1"
fi

if [[ "$MODE" == "--resolve-tailscale" ]]; then
  RESOLVE_TAILSCALE="1"
  MODE="--print"
fi
if [[ "$MODE" == "--resolve-headscale" ]]; then
  RESOLVE_HEADSCALE="1"
  RESOLVE_TAILSCALE="1"
  MODE="--print"
fi

if [[ "$MODE" != "--print" && "$MODE" != "--check" ]]; then
  echo "Usage: $0 [--print|--check|--resolve-tailscale|--resolve-headscale]" >&2
  exit 2
fi

python3 - "$REPO_ROOT" "$MODE" "$RESOLVE_TAILSCALE" "$RESOLVE_HEADSCALE" <<'PY'
from __future__ import annotations

import json
import os
import re
import ssl
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Dict, Iterable, List, Set, Tuple


def subprocess_output(cmd: List[str]) -> str:
    return subprocess.check_output(cmd, stderr=subprocess.STDOUT, text=True, timeout=10).strip()


def collect_tailscale_observed() -> Set[str]:
    try:
        output = subprocess_output(["headscale", "status", "--json"])
    except Exception:
        return set()

    if not output:
        return set()

    try:
        data = json.loads(output)
    except json.JSONDecodeError:
        return set()

    names = set()
    self_node = data.get("Self", {}).get("HostName")
    if self_node:
        names.add(self_node)
    for peer in data.get("Peer", {}).values():
        host = peer.get("HostName")
        if host:
            names.add(host)
    return names


def first_non_empty(*names: str) -> str:
    for name in names:
        value = os.environ.get(name, "").strip()
        if value:
            return value
    return ""


def normalize_headscale_url(raw_url: str) -> str:
    url = raw_url.strip()
    if not url:
        return ""
    if "://" not in url:
        url = f"https://{url}"
    return url


def parse_api_json_bytes(payload: bytes) -> Any:
    return json.loads(payload.decode("utf-8"))


def http_get_json(url: str, headers: Dict[str, str]) -> Any:
    req = urllib.request.Request(url, headers=headers, method="GET")
    try:
        context = ssl.create_default_context()
        with urllib.request.urlopen(req, timeout=10, context=context) as response:
            return parse_api_json_bytes(response.read())
    except urllib.error.URLError:
        context = ssl._create_unverified_context()
        with urllib.request.urlopen(req, timeout=10, context=context) as response:
            return parse_api_json_bytes(response.read())


def collect_headscale_nodes(base_url: str, api_key: str) -> Set[str]:
    if not base_url or not api_key:
        return set()

    base = base_url.rstrip("/")
    headers = {"Authorization": f"Bearer {api_key}"}
    paths = [
        "/api/v1/node",
        "/api/v1/nodes",
        "/api/v1/node/list",
        "/api/v1/nodes/list",
        "/api/v1/node/list?preauth=false",
    ]

    for path in paths:
        try:
            payload = http_get_json(f"{base}{path}", headers)
        except Exception:
            continue

        candidate_nodes: List[dict[str, object]] = []
        if isinstance(payload, list):
            candidate_nodes = [n for n in payload if isinstance(n, dict)]
        elif isinstance(payload, dict):
            for key in ("nodes", "data", "result", "machines"):
                raw_nodes = payload.get(key)
                if isinstance(raw_nodes, list):
                    candidate_nodes = [n for n in raw_nodes if isinstance(n, dict)]
                    break

        names: Set[str] = set()
        for node in candidate_nodes:
            for key in (
                "name",
                "Name",
                "machine_name",
                "MachineName",
                "hostname",
                "HostName",
                "given_name",
                "givenName",
                "node_name",
            ):
                raw_name = node.get(key)
                if isinstance(raw_name, str) and raw_name.strip():
                    names.add(raw_name.strip())

        if names:
            return names

    return set()


def collect_headscale_observed(base_url: str, api_key: str) -> tuple[Set[str], str]:
    if not base_url:
        return set(), "missing-headscale-url"
    if not api_key:
        return set(), "missing-headscale-token"
    names = collect_headscale_nodes(base_url, api_key)
    if names:
        return names, "ok"
    return set(), "headscale-unreachable-or-empty"


def find_net_match(aliases: Iterable[str], observed: Set[str]) -> Set[str]:
    aliases_set = {str(alias) for alias in aliases}
    direct = {name for name in observed if name in aliases_set}
    if direct:
        return direct

    normalized_aliases = {alias.lower() for alias in aliases_set}
    partial = set()
    for alias in normalized_aliases:
        for name in observed:
            normalized_name = name.lower()
            if alias in normalized_name or normalized_name in alias:
                partial.add(name)
    return partial


def collect_placeholders(payload: dict, include_non_tailscale: bool = True) -> dict:
    placeholders: Dict[str, Dict[str, object]] = {}

    for node in payload.get("nodes", []):
        name = node.get("name", "<unnamed>")
        tailscale_ref = node.get("network", {}).get("headscale", "")
        for key in re.findall(r"<([A-Z0-9_]+)>", str(tailscale_ref)):
            placeholders.setdefault(key, {
                "kind": "headscale",
                "nodes": [],
                "aliases": set(),
            })
            placeholders[key]["nodes"].append(name)
            aliases = []
            raw_aliases = node.get("aliases", [])
            if isinstance(raw_aliases, str):
                aliases.append(raw_aliases)
            elif isinstance(raw_aliases, list):
                aliases.extend([str(x) for x in raw_aliases if isinstance(x, str)])
            aliases.append(node.get("name", ""))
            placeholders[key]["aliases"].update([x for x in aliases if x])

    if include_non_tailscale:
        for key in re.findall(r"<([A-Z0-9_]+)>", str(payload)):
            placeholders.setdefault(key, {
                "kind": "runtime",
                "nodes": [],
                "aliases": set(),
            })

    for info in placeholders.values():
        if isinstance(info.get("aliases"), set):
            info["aliases"] = sorted(info["aliases"])  # type: ignore[index]
    return placeholders


def resolve_candidate(
    info: Dict[str, object],
    current: str,
    resolve_tailscale: bool,
    resolve_headscale: bool,
    observed_tailscale: Set[str],
    observed_headscale: Set[str],
) -> Tuple[str, str]:
    kind = str(info.get("kind", ""))
    aliases = set(info.get("aliases", [])) if isinstance(info.get("aliases"), Iterable) else set()
    if current:
        return current, "set"
    if kind != "headscale":
        return "", "missing"

    if resolve_headscale and observed_headscale:
        matches = sorted(find_net_match(aliases, observed_headscale))
        if len(matches) == 1:
            return matches[0], "resolved_headscale"

    if resolve_tailscale and observed_tailscale:
        matches = sorted(find_net_match(aliases, observed_tailscale))
        if len(matches) == 1:
            return matches[0], "resolved_tailscale"

    return "", "missing"


repo_root = Path(sys.argv[1]).resolve()
mode = sys.argv[2]
resolve_tailscale = bool(int(sys.argv[3]))
resolve_headscale = bool(int(sys.argv[4]))

registry_path = repo_root / "infra" / "registry.json"
if not registry_path.exists():
    raise FileNotFoundError(f"Missing SSOT path: {registry_path}")

registry = json.loads(registry_path.read_text(encoding="utf-8"))

placeholders = collect_placeholders(registry)

observed_tailscale: Set[str] = collect_tailscale_observed() if resolve_tailscale else set()
headscale_api_url = first_non_empty(
    "CYNIC_HEADSCALE_URL",
    "CYNIC_HEADSCALE_API_URL",
    "HEADSCALE_URL",
    "HEADSCALE_API_URL",
    "HEADSCALE_BASE_URL",
)
headscale_api_url = normalize_headscale_url(headscale_api_url)
headscale_api_key = first_non_empty(
    "CYNIC_HEADSCALE_API_KEY",
    "CYNIC_HEADSCALE_KEY",
    "CYNIC_HEADSCALE_TOKEN",
    "HEADSCALE_API_KEY",
    "HEADSCALE_KEY",
    "HEADSCALE_TOKEN",
)
observed_headscale, headscale_status = collect_headscale_observed(headscale_api_url, headscale_api_key) if resolve_headscale else (set(), "skip")

missing = []
optional_missing = []

for var, info in sorted(placeholders.items()):
    current = os.environ.get(var, "")
    aliases: List[str] = list(info.get("aliases") or [])
    source_node = ", ".join(info.get("nodes") or ["_"])
    kind = str(info.get("kind", ""))

    value, status_kind = resolve_candidate(
        info,
        current,
        resolve_tailscale=resolve_tailscale,
        resolve_headscale=resolve_headscale,
        observed_tailscale=observed_tailscale,
        observed_headscale=observed_headscale,
    )

    if not value:
        candidate = aliases[0] if aliases else ""
        status = f"MISSING (suggest: {candidate}) # source:{source_node} ({kind})"
        if kind == "runtime":
            optional_missing.append(var)
        else:
            missing.append(var)
    else:
        if status_kind == "resolved_headscale":
            status = f"RESOLVED {value: <38} # source:{source_node} ({kind}:headscale)"
        elif status_kind == "resolved_tailscale":
            status = f"RESOLVED {value: <38} # source:{source_node} ({kind}:tailscale)"
        elif status_kind == "set":
            status = f"{value: <38} # source:{source_node} ({kind}:explicit)"
        else:
            status = f"{value: <38} # source:{source_node} ({kind})"

    if mode == "--check":
        if value:
            print(f"OK {var}={status}")
        else:
            print(f"{'REQ' if kind != 'runtime' else 'OPT'} MISSING {var}={status}")
    else:
        if kind == "runtime" and not value:
            continue
        if value:
            if status_kind == "resolved_headscale":
                print(f'export {var}="{value}"  # resolved from headscale reference: {source_node}')
            elif status_kind == "resolved_tailscale":
                print(f'export {var}="{value}"  # resolved from local tailscale peers: {source_node}')
            elif status_kind == "set":
                print(f'export {var}="{value}"  # already-set: {source_node}')
            else:
                print(f'export {var}="{value}"')
        elif kind == "headscale" and status_kind == "missing":
            default = candidate if candidate else "<UNSET>"
            if resolve_headscale:
                print(f'export {var}="{default}"  # resolve from headscale not possible ({headscale_status})')
            elif resolve_tailscale:
                print(f'export {var}="{default}"  # resolve from tailscale not possible')
            else:
                print(f'export {var}="{default}"  # suggest from SSOT alias: {",".join(aliases)}')
        else:
            print(f'export {var}="{candidate if candidate else "<UNSET>"}"  # set this from SSOT alias: {",".join(aliases)}')

if mode == "--check":
    if missing:
        print(f"MISSING_COUNT={len(missing)}")
        print("MISSING=" + ",".join(missing))
        raise SystemExit(1)
    if optional_missing:
        print("OPTIONAL_MISSING=" + ",".join(optional_missing))
    print("MISSING_COUNT=0")

if mode != "--check":
    if optional_missing:
        print(f"# optional_placeholders={','.join(sorted(optional_missing))}")

    current_kernel = os.environ.get("CYNIC_REST_ADDR", "")
    if not current_kernel:
        core = os.environ.get("HEADSCALE_CORE", "")
        if core:
            print('export CYNIC_REST_ADDR=' + f'"http://{core}:3030"' + '  # from SSOT desired kernel endpoint')
        else:
            print('export CYNIC_REST_ADDR="http://<HEADSCALE_CORE>:3030"  # define HEADSCALE_CORE before sourcing this value')

    current_proxmox = os.environ.get("CYNIC_PROXMOX_URL", "")
    if not current_proxmox:
        proxmox_host = os.environ.get("HEADSCALE_HOST_PROXMOX", "")
        if proxmox_host:
            print('export CYNIC_PROXMOX_URL=' + f'"https://{proxmox_host}:8006"')
        else:
            print('export CYNIC_PROXMOX_URL="https://<HEADSCALE_HOST_PROXMOX>:8006"')
PY
