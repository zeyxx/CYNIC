from __future__ import annotations

import argparse
import json
from dataclasses import asdict, replace
from pathlib import Path

from organs.exposure.src.adapters.json_exposure_source import JsonExposureSource
from organs.exposure.src.core.services import (
    AntigravitySettingsProjection,
    ClaudeMcpJsonProjection,
    CodexTomlProjection,
    ExposureProjectionService,
)


def build_service(project_root: Path, source_path: Path | None = None) -> ExposureProjectionService:
    source = JsonExposureSource(source_path or project_root / "infra" / "agent-exposure.json")
    return ExposureProjectionService(
        source=source,
        projections={
            "codex": CodexTomlProjection(),
            "claude": ClaudeMcpJsonProjection(),
            "antigravity": AntigravitySettingsProjection(),
        },
    )


def build_parser(project_root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC organism exposure projection organ.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    render = subparsers.add_parser("render", help="Render an agent projection.")
    render.add_argument("--agent", choices=["codex", "claude", "antigravity"], required=True)
    render.add_argument("--source", help="Override exposure source JSON path.")

    inspect = subparsers.add_parser("inspect", help="Print exposure graph summary as JSON.")
    inspect.add_argument("--source", help="Override exposure source JSON path.")

    validate = subparsers.add_parser("validate", help="Validate exposure SSOT against MCP source and contracts.")
    validate.add_argument("--source", help="Override exposure source JSON path.")
    validate.add_argument(
        "--mcp-proxy",
        default=str(project_root / "crates" / "cynic-mcp" / "src" / "proxy.rs"),
        help="Path to crates/cynic-mcp/src/proxy.rs.",
    )
    validate.add_argument(
        "--contracts-dir",
        default=str(project_root / "docs" / "contracts"),
        help="Directory containing JSON contracts.",
    )
    validate.add_argument(
        "--python-fallback",
        default=str(project_root / ".cortex" / "mcp" / "python_rest_mcp.py"),
        help="Path to the minimal Python REST MCP fallback.",
    )
    validate.add_argument(
        "--project-root",
        default=str(project_root),
        help="Project root used for projection target drift checks.",
    )

    apply = subparsers.add_parser("apply", help="Write or check declared projection targets.")
    apply.add_argument("--source", help="Override exposure source JSON path.")
    apply.add_argument(
        "--project-root",
        default=str(project_root),
        help="Project root containing projection target files.",
    )
    apply.add_argument("--check", action="store_true", help="Fail if any projection target would change.")
    apply.add_argument("--dry-run", action="store_true", help="Report changes without writing files.")
    return parser


def main(project_root: Path) -> int:
    parser = build_parser(project_root)
    args = parser.parse_args()
    source_path = Path(args.source) if getattr(args, "source", None) else None
    service = build_service(project_root, source_path)

    if args.command == "render":
        print(service.render(args.agent), end="")
        return 0

    if args.command == "inspect":
        graph = service.graph()
        print(
            json.dumps(
                {
                    "version": graph.version,
                    "mcp_servers": [
                        {
                            "name": server.name,
                            "status": server.status,
                            "agents": server.agents,
                            "tool_count": len(server.tools),
                        }
                        for server in graph.mcp_servers
                    ],
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    if args.command == "validate":
        report = service.validate_contract_alignment(
            mcp_proxy_path=Path(args.mcp_proxy),
            contracts_dir=Path(args.contracts_dir),
            python_fallback_path=Path(args.python_fallback),
        )
        findings = list(report.findings)
        findings.extend(service.validate_projection_files(Path(args.project_root)))
        report = replace(
            report,
            findings=findings,
            ok=not any(item.severity == "critical" for item in findings),
        )
        print(json.dumps(asdict(report), indent=2, sort_keys=True))
        return 0 if report.ok else 1

    if args.command == "apply":
        result = service.apply_projection_files(
            Path(args.project_root),
            check=args.check,
            dry_run=args.dry_run,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result["ok"] else 1

    return 1
