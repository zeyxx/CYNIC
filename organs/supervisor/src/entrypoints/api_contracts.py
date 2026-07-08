#!/usr/bin/env python3
"""Organ-supervisor entrypoint for API contract validation and reporting."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    from organs.supervisor.controllers._bootstrap import bootstrap_repo_path
except ImportError:
    ROOT_HINT = Path(__file__).resolve()
    for parent in ROOT_HINT.parents:
        if (parent / "organs" / "supervisor").exists() and (parent / "axioms").exists():
            sys.path.insert(0, str(parent))
            break
    from organs.supervisor.controllers._bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from organs.supervisor.src.core.api_topology_contract import (  # noqa: E402
    ApiTopologyContractService,
    api_geometry_report,
    openapi_operations,
)
from organs.supervisor.src.core.openapi_route_contract import (  # noqa: E402
    OpenApiRouteContractService,
)
from organs.supervisor.src.core.repo_topology_host_validation import (  # noqa: E402
    Finding,
    load_json,
    relative as _relative,
)


OPENAPI = ROOT / "docs" / "api" / "openapi.json"
TOPOLOGY = ROOT / "axioms" / "api-topology-v1.json"
ALIGNMENT = ROOT / "axioms" / "api-layer-alignment-v1.json"
ROUTER = ROOT / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "mod.rs"
DISCOVERY_RS = ROOT / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "discovery.rs"
MIDDLEWARE_RS = ROOT / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "middleware.rs"


def relative(path: Path) -> str:
    return _relative(path, ROOT)


def route_service() -> OpenApiRouteContractService:
    return OpenApiRouteContractService(
        workspace_root=ROOT,
        router_path=ROUTER,
        openapi_path=OPENAPI,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def topology_service() -> ApiTopologyContractService:
    return ApiTopologyContractService(
        workspace_root=ROOT,
        openapi_path=OPENAPI,
        topology_path=TOPOLOGY,
        alignment_path=ALIGNMENT,
        discovery_rs_path=DISCOVERY_RS,
        middleware_rs_path=MIDDLEWARE_RS,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def print_findings(title: str, findings: list[Finding]) -> int:
    if not findings:
        return 0
    print(f"{title} failed:", file=sys.stderr)
    for finding in findings:
        print(f"  - {finding.path}: {finding.detail}", file=sys.stderr)
    return 1


def validate_openapi_routes() -> int:
    findings = route_service().validate()
    if print_findings("openapi route contract", findings):
        return 1
    openapi = load_json(OPENAPI)
    print(f"openapi route contract ok: {len(openapi.get('paths', {}))} paths")
    return 0


def validate_api_topology() -> int:
    findings = topology_service().validate()
    if print_findings("api topology contract", findings):
        return 1
    openapi = load_json(OPENAPI)
    operations = openapi_operations(openapi)
    print(f"api topology contract ok: {len(operations)} operations")
    return 0


def print_api_geometry_report() -> int:
    openapi = load_json(OPENAPI)
    topology = load_json(TOPOLOGY)
    report = api_geometry_report(
        openapi=openapi,
        topology=topology,
        openapi_source=relative(OPENAPI),
        topology_source=relative(TOPOLOGY),
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate and report organ-supervisor API contracts.")
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("validate-openapi-routes")
    subparsers.add_parser("validate-api-topology")
    subparsers.add_parser("api-geometry-report")
    args = parser.parse_args()

    if args.command == "validate-openapi-routes":
        return validate_openapi_routes()
    if args.command == "validate-api-topology":
        return validate_api_topology()
    if args.command == "api-geometry-report":
        return print_api_geometry_report()
    parser.error(f"unsupported command {args.command!r}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
