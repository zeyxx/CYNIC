#!/usr/bin/env python3
"""Machine-readable control surface for organ-framework-map."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys


def _repo_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start.parent


ROOT = _repo_root()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
SRC_ROOT = ROOT / "organs" / "framework-map" / "src"
if str(SRC_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_ROOT))

from organs.contracts.src.surface_contracts import ReportSurfaceSpec, SurfaceRegistry
from adapters.contracts_dependency_inventory import ContractsDependencyInventory
from adapters.hermes_capability_discovery import HermesCapabilityDiscovery
from adapters.hermes_command_map_discovery import HermesCommandMapDiscovery
from adapters.hermes_kanban_mirror import HermesKanbanMirrorDiscovery
from adapters.hermes_skill_bridge import HermesSkillBridgeDiscovery
from core.framework_map import FRAMEWORK_MAP_SURFACE, FrameworkMapService, FrameworkTarget


def surface_manifest() -> dict[str, object]:
    registry = SurfaceRegistry(
        reports=(
            FRAMEWORK_MAP_SURFACE,
            ReportSurfaceSpec(
                schema="cynic.framework_map.surface_manifest.v1",
                command="python3 organs/framework-map/src/entrypoints/cli.py surface-manifest",
                json_command="python3 organs/framework-map/src/entrypoints/cli.py surface-manifest",
                usage="surface-manifest",
            ),
        )
    )
    return {
        "schema": "cynic.framework_map.surface_manifest.v1",
        "organ": "organ-framework-map",
        "mutates": False,
        "mutates_mempool": False,
        "registry": registry.manifest(),
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-framework-map control surface")
    sub = parser.add_subparsers(dest="command")
    sub.add_parser("surface-manifest")
    hermes = sub.add_parser("map-hermes")
    hermes.add_argument("--source-root", default="external/hermes-agent")
    hermes.add_argument("--binary", default="hermes")
    hermes.add_argument("--output", default="organs/framework-map/state/latest-hermes-map.json")
    hermes.add_argument("--no-write-state", action="store_true")
    return parser


def map_hermes(source_root: str, binary: str, output: str | None = "organs/framework-map/state/latest-hermes-map.json") -> dict[str, object]:
    root = Path(source_root).expanduser()
    if not root.is_absolute():
        root = ROOT / root
    service = FrameworkMapService(
        capability_discovery=HermesCapabilityDiscovery(),
        dependency_inventory=ContractsDependencyInventory(),
        command_map_discovery=HermesCommandMapDiscovery(),
        skill_bridge_discovery=HermesSkillBridgeDiscovery(),
        kanban_mirror_discovery=HermesKanbanMirrorDiscovery(),
        repo_root=ROOT,
    )
    payload = service.map_target(
        FrameworkTarget(
            framework="hermes-agent",
            source_root=root,
            binary=binary,
        )
    )
    if output:
        output_path = Path(output).expanduser()
        if not output_path.is_absolute():
            output_path = ROOT / output_path
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        payload["state_path"] = str(output_path)
        output_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return payload


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    command = args.command or "surface-manifest"
    if command == "surface-manifest":
        payload = surface_manifest()
    elif command == "map-hermes":
        payload = map_hermes(args.source_root, args.binary, None if args.no_write_state else args.output)
    else:
        raise RuntimeError(f"unsupported command: {command}")
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
