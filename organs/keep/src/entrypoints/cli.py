#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import asdict
from pathlib import Path

def _resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate

    return Path(__file__).resolve().parent


ROOT = _resolve_project_root()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.contracts.src.surface_contracts import json_object_or_raise
from organs.contracts.src.surface_contracts import read_json_file_or_default
from organs.contracts.src.surface_contracts import read_json_file_or_raise

from organs.fleet.src.adapters.proxmox_adapter import ProxmoxListTextAdapter
from organs.fleet.src.adapters.registry_adapter import RegistryAdapter
from organs.fleet.src.core.service import FleetTruthService
from organs.keep.src.core.storage_activation import StoragePrimaryActivation


def main() -> int:
    parser = argparse.ArgumentParser(description="organ-keep storage activation CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)
    storage = subparsers.add_parser("storage-primary")
    storage_sub = storage.add_subparsers(dest="storage_command", required=True)
    plan = storage_sub.add_parser("plan")
    plan.add_argument("--registry", default="infra/registry.json")
    bundle = storage_sub.add_parser("bundle")
    bundle.add_argument("--registry", default="infra/registry.json")
    bundle.add_argument("--schema", default="infra/surrealdb/schema.surql")
    bundle.add_argument("--output-dir", required=True)
    status = storage_sub.add_parser("status")
    status.add_argument("--registry", default="infra/registry.json")
    status.add_argument("--bundle-dir")
    status.add_argument("--bridge-status-json")
    status.add_argument("--fleet-report-json")
    status.add_argument("--proxmox-list-text")
    status.add_argument("--mesh-status-json")
    status.add_argument("--mesh-status-stdin", action="store_true")
    status.add_argument("--health-json")
    status.add_argument("--evidence-stdin", action="store_true")
    args = parser.parse_args()

    registry = read_json_file_or_raise(ROOT / args.registry, RuntimeError("registry JSON is invalid"))
    activation = StoragePrimaryActivation(registry)
    if args.storage_command == "plan":
        print(json.dumps(activation.manifest(), indent=2, sort_keys=True))
        return 0
    if args.storage_command == "bundle":
        payload = activation.write_bundle(Path(args.output_dir).resolve(), ROOT / args.schema)
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    if args.storage_command == "status":
        if args.evidence_stdin and args.mesh_status_stdin:
            parser.error("--evidence-stdin and --mesh-status-stdin are mutually exclusive")
        if args.mesh_status_json and args.mesh_status_stdin:
            parser.error("--mesh-status-json and --mesh-status-stdin are mutually exclusive")
        if args.evidence_stdin and args.proxmox_list_text == "-":
            parser.error("--evidence-stdin and --proxmox-list-text=- are mutually exclusive")
        evidence = _load_evidence_stdin(args.evidence_stdin)
        payload = activation.activation_status(
            bundle_dir=Path(args.bundle_dir).resolve() if args.bundle_dir else None,
            bridge_status=_load_optional_json(args.bridge_status_json)
            or evidence.get("bridge_status"),
            fleet_report=_load_optional_json(args.fleet_report_json)
            or evidence.get("fleet_report")
            or _build_fleet_report_from_proxmox_text(args.proxmox_list_text, args.registry),
            mesh_status=_load_mesh_status(args.mesh_status_json, args.mesh_status_stdin)
            or evidence.get("mesh_status"),
            health=_load_optional_json(args.health_json) or evidence.get("health"),
        )
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0 if payload["status"] == "active" else 1
    parser.error("unknown storage-primary command")


def _load_optional_json(path: str | None) -> dict:
    if not path:
        return {}
    return read_json_file_or_default(Path(path), {})


def _load_mesh_status(path: str | None, from_stdin: bool) -> dict:
    if from_stdin:
        return json_object_or_default(sys.stdin.read())
    return _load_optional_json(path)


def _load_evidence_stdin(from_stdin: bool) -> dict:
    if not from_stdin:
        return {}
    return json_object_or_raise(sys.stdin.read(), ValueError("--evidence-stdin expects a JSON object"))


def _build_fleet_report_from_proxmox_text(path: str | None, registry_path: str) -> dict:
    if not path:
        return {}
    text = sys.stdin.read() if path == "-" else Path(path).read_text(encoding="utf-8")
    observed = ProxmoxListTextAdapter(None).parse(text)
    registry = RegistryAdapter(ROOT / registry_path)
    report = FleetTruthService().build_report(
        declared_nodes=registry.nodes(),
        observed_resources=observed,
        registry_organs=registry.organs(),
        organ_dirs=_organ_dirs(ROOT),
    )
    return asdict(report)


def _organ_dirs(root: Path) -> set[str]:
    organs_path = root / "organs"
    support_dirs = {"controllers", "state"}
    return {
        child.name
        for child in organs_path.iterdir()
        if child.is_dir() and not child.name.startswith(".") and child.name not in support_dirs
    }


if __name__ == "__main__":
    raise SystemExit(main())
