#!/usr/bin/env python3
"""Machine-readable organ-contracts entrypoint."""
from __future__ import annotations
import json
from pathlib import Path
import sys

def _bootstrap_project_root() -> None:
    start = Path(__file__).resolve()
    for candidate in (start.parent, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            candidate_str = str(candidate)
            sys.path[:] = [item for item in sys.path if item != candidate_str and not _masks_organs(Path(item or "."))]
            sys.path.insert(0, candidate_str)
            return

def _masks_organs(path: Path) -> bool:
    return (path / "organs" / "__init__.py").exists() and not (path / "organs" / "contracts").exists()

_bootstrap_project_root()

from organs.contracts.src.adapters.hermes_agent_capabilities import HermesAgentCapabilityScanner
from organs.contracts.src.adapters.hermes_fleet_backend_readiness import probe_hermes_fleet_backend_readiness_from_args
from organs.contracts.src.adapters.hermes_fleet_config_projection import render_hermes_fleet_config_drift_from_args, render_hermes_fleet_config_projection_from_args
from organs.contracts.src.adapters.hermes_fleet_model import evaluate_hermes_fleet_model_state_from_args
from organs.contracts.src.adapters.hermes_execution_dependencies import dispatch_hermes_execution_dependency_tasks_from_args, evaluate_hermes_execution_dependencies_from_args, materialize_hermes_execution_dependency_tasks_from_args
from organs.contracts.src.adapters.dependency_inventory import DependencyInventoryScanner
from organs.contracts.src.core.agent_execution_envelope import example_agent_execution_envelope
from organs.contracts.src.core.hermes_kanban_graph import example_hermes_execution_result, example_hermes_kanban_graph_event
from organs.contracts.src.surface_contracts import ReportSurfaceSpec, SurfaceRegistry

USAGE = (
    "usage: surface-manifest | hermes-agent-capabilities | external-dependency-inventory | "
    "hermes-fleet-model-state | hermes-fleet-config-projection | hermes-fleet-config-drift | "
    "hermes-fleet-backend-readiness | hermes-execution-dependencies | "
    "hermes-execution-dependency-tasks | hermes-execution-dependency-dispatch | "
    "agent-execution-envelope-example"
)
CLI = "python3 organs/contracts/src/entrypoints/cli.py"
REPORTS = (
    ("cynic.contracts.surface_manifest.v1", "surface-manifest", None),
    ("cynic.agent_framework_capabilities.v1", "hermes-agent-capabilities", "hermes-agent-capabilities [--binary hermes] [--source-root PATH]"),
    ("cynic.external_dependency_inventory.v1", "external-dependency-inventory", "external-dependency-inventory [--root PATH]..."),
    ("cynic.hermes_fleet_model_state.v1", "hermes-fleet-model-state", "hermes-fleet-model-state [--backends-toml backends.toml] [--expected-backend qwen36-27b-gpu] [--binary hermes]"),
    ("cynic.hermes_fleet_config_projection.v1", "hermes-fleet-config-projection", "hermes-fleet-config-projection [--backends-toml backends.toml] [--expected-backend qwen36-27b-gpu]"),
    ("cynic.hermes_fleet_config_drift.v1", "hermes-fleet-config-drift", "hermes-fleet-config-drift [--backends-toml backends.toml] [--registry infra/registry.json] [--config ~/.hermes/config.yaml] [--expected-backend qwen36-27b-gpu]"),
    ("cynic.hermes_fleet_backend_readiness.v1", "hermes-fleet-backend-readiness", "hermes-fleet-backend-readiness [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"),
    ("cynic.hermes_execution_dependencies.v1", "hermes-execution-dependencies", "hermes-execution-dependencies [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"),
    ("cynic.hermes_execution_dependency_tasks.v1", "hermes-execution-dependency-tasks", "hermes-execution-dependency-tasks [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"),
    ("cynic.hermes_execution_dependency_dispatch.v1", "hermes-execution-dependency-dispatch", "hermes-execution-dependency-dispatch [--dry-run|--dispatch] [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"),
    ("cynic.hermes_kanban_graph_event.v1", "hermes-kanban-graph-event-example", None),
    ("cynic.hermes.execution_result.v1", "hermes-execution-result-example", None),
    ("cynic.agent_execution_envelope.v1", "agent-execution-envelope-example", None),
)

def contract_registry() -> SurfaceRegistry:
    return SurfaceRegistry(
        reports=tuple(_report(schema, command, usage) for schema, command, usage in REPORTS)
    )

def _report(schema: str, command: str, usage: str | None = None) -> ReportSurfaceSpec:
    return ReportSurfaceSpec(
        schema=schema,
        command=f"{CLI} {command}",
        json_command=f"{CLI} {command}",
        usage=usage or command,
    )

def surface_manifest() -> dict[str, object]:
    return {
        "schema": "cynic.contracts.surface_manifest.v1",
        "organ": "organ-contracts",
        "mutates": False,
        "mutates_mempool": False,
        "registry": contract_registry().manifest(),
    }

def hermes_agent_capabilities(args: list[str]) -> dict[str, object]:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="hermes")
    parser.add_argument("--source-root", type=lambda p: Path(p).expanduser())
    parsed = parser.parse_args(args)
    return HermesAgentCapabilityScanner(binary=parsed.binary, source_root=parsed.source_root).scan().to_dict()

def external_dependency_inventory(args: list[str]) -> dict[str, object]:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", action="append", type=lambda p: Path(p).expanduser())
    parsed = parser.parse_args(args)
    roots = parsed.root or [Path(".")]
    return DependencyInventoryScanner(tuple(roots)).scan().to_dict()

def agent_execution_envelope_example(args: list[str]) -> dict[str, object]:
    if args: raise RuntimeError("usage: agent-execution-envelope-example")
    return example_agent_execution_envelope().to_dict()

def hermes_kanban_graph_event_example(args: list[str]) -> dict[str, object]:
    if args: raise RuntimeError("usage: hermes-kanban-graph-event-example")
    return example_hermes_kanban_graph_event()

def hermes_execution_result_example(args: list[str]) -> dict[str, object]:
    if args: raise RuntimeError("usage: hermes-execution-result-example")
    return example_hermes_execution_result()

def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    command = args[0] if args else "surface-manifest"
    command_args = args[1:]
    handlers = {
        "surface-manifest": lambda _args: surface_manifest(),
        "hermes-agent-capabilities": hermes_agent_capabilities,
        "external-dependency-inventory": external_dependency_inventory,
        "hermes-fleet-model-state": lambda args: evaluate_hermes_fleet_model_state_from_args(args).to_dict(),
        "hermes-fleet-config-projection": render_hermes_fleet_config_projection_from_args,
        "hermes-fleet-config-drift": render_hermes_fleet_config_drift_from_args,
        "hermes-fleet-backend-readiness": probe_hermes_fleet_backend_readiness_from_args,
        "hermes-execution-dependencies": evaluate_hermes_execution_dependencies_from_args,
        "hermes-execution-dependency-tasks": materialize_hermes_execution_dependency_tasks_from_args,
        "hermes-execution-dependency-dispatch": dispatch_hermes_execution_dependency_tasks_from_args,
        "hermes-kanban-graph-event-example": hermes_kanban_graph_event_example,
        "hermes-execution-result-example": hermes_execution_result_example,
        "agent-execution-envelope-example": agent_execution_envelope_example,
    }
    try:
        handler = handlers.get(command)
        if handler is not None:
            print(json.dumps(handler(command_args), sort_keys=True))
            return 0
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    print(USAGE, file=sys.stderr)
    return 2

if __name__ == "__main__":
    raise SystemExit(main())
