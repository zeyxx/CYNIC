from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default

from .entities import AgentExposureGraph, ExposureFinding, ExposureValidationReport
from .ports import AgentProjectionPort, ExposureSourcePort


class ExposureProjectionService:
    def __init__(self, source: ExposureSourcePort, projections: dict[str, AgentProjectionPort]):
        self.source = source
        self.projections = projections

    def graph(self) -> AgentExposureGraph:
        return self.source.load()

    def render(self, agent: str) -> str:
        projection = self.projections.get(agent)
        if projection is None:
            known = ", ".join(sorted(self.projections))
            raise ValueError(f"unsupported exposure projection agent: {agent}; known={known}")
        return projection.render(self.graph(), agent)

    def validate_contract_alignment(
        self,
        mcp_proxy_path: Path,
        contracts_dir: Path,
        server_name: str = "cynic-coord",
        python_fallback_path: Path | None = None,
    ) -> ExposureValidationReport:
        graph = self.graph()
        rust_tools = extract_rust_mcp_tools(mcp_proxy_path)
        fallback_tools = extract_python_fallback_tools(python_fallback_path) if python_fallback_path else set()
        implemented_contract_tools = extract_implemented_contract_tools(contracts_dir)
        server = next((item for item in graph.mcp_servers if item.name == server_name), None)
        findings: list[ExposureFinding] = []
        if server is None:
            findings.append(
                ExposureFinding(
                    "critical",
                    "mcp_server_missing",
                    "infra/agent-exposure.json",
                    f"expected MCP server {server_name!r} in exposure SSOT",
                )
            )
            exposed_tools: set[str] = set()
        else:
            exposed_tools = {tool.name for tool in server.tools}

        declared_agents = {
            agent for mcp_server in graph.mcp_servers for agent in mcp_server.agents
        }
        for agent in sorted(declared_agents - set(self.projections)):
            findings.append(
                ExposureFinding(
                    "critical",
                    "declared_agent_without_projection",
                    "infra/agent-exposure.json",
                    f"{agent} appears in MCP exposure agents but organ-exposure has no renderer for it.",
                )
            )

        for tool in sorted(exposed_tools - rust_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "exposed_tool_missing_in_rust",
                    str(mcp_proxy_path),
                    f"{tool} is exposed through {server_name} but no Rust MCP tool with that name exists.",
                )
            )

        domain_tools: set[str] = set()
        fallback_required_tools: set[str] = set()
        for domain, payload in sorted(graph.mcp_tool_domains.items()):
            tools = payload.get("tools", [])
            if not isinstance(tools, list) or not tools:
                findings.append(
                    ExposureFinding(
                        "critical",
                        "mcp_domain_without_tools",
                        "infra/agent-exposure.json",
                        f"mcp_tool_domains.{domain} must declare a non-empty tools list.",
                    )
                )
                continue
            domain_tools.update(tool for tool in tools if isinstance(tool, str))
            fallback_tools_for_domain = payload.get("fallback_tools", [])
            if fallback_tools_for_domain is not None and not isinstance(fallback_tools_for_domain, list):
                findings.append(
                    ExposureFinding(
                        "critical",
                        "mcp_domain_invalid_fallback_tools",
                        "infra/agent-exposure.json",
                        f"mcp_tool_domains.{domain}.fallback_tools must be a list when present.",
                    )
                )
                continue
            fallback_required_tools.update(
                tool for tool in fallback_tools_for_domain or [] if isinstance(tool, str)
            )

        for tool in sorted(exposed_tools - domain_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "exposed_tool_missing_domain",
                    "infra/agent-exposure.json",
                    f"{tool} is exposed through {server_name} but not classified in mcp_tool_domains.",
                )
            )

        for tool in sorted(domain_tools - exposed_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "domain_tool_not_exposed",
                    "infra/agent-exposure.json",
                    f"{tool} is classified in mcp_tool_domains but missing from {server_name} tools.",
                )
            )

        for tool in sorted(fallback_required_tools - exposed_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "fallback_tool_not_exposed",
                    "infra/agent-exposure.json",
                    f"{tool} is required by the Python fallback but missing from {server_name} exposure.",
                )
            )

        if python_fallback_path is not None:
            for tool in sorted(fallback_required_tools - fallback_tools):
                findings.append(
                    ExposureFinding(
                        "critical",
                        "fallback_tool_missing_in_python",
                        str(python_fallback_path),
                        f"{tool} is required by mcp_tool_domains fallback_tools but missing from the Python REST MCP fallback.",
                    )
                )

        for tool in sorted(implemented_contract_tools - exposed_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "implemented_contract_tool_not_exposed",
                    str(contracts_dir),
                    f"{tool} is implemented in a contract but missing from {server_name} exposure.",
                )
            )

        for tool in sorted(implemented_contract_tools - rust_tools):
            findings.append(
                ExposureFinding(
                    "critical",
                    "implemented_contract_tool_missing_in_rust",
                    str(mcp_proxy_path),
                    f"{tool} is implemented in a contract but no Rust MCP tool with that name exists.",
                )
            )

        return ExposureValidationReport(
            schema="cynic.agent_exposure_validation.v1",
            ok=not any(item.severity == "critical" for item in findings),
            findings=findings,
            rust_tool_count=len(rust_tools),
            exposed_tool_count=len(exposed_tools),
            implemented_contract_tool_count=len(implemented_contract_tools),
        )

    def validate_projection_files(self, project_root: Path) -> list[ExposureFinding]:
        graph = self.graph()
        findings: list[ExposureFinding] = []
        for agent, target in sorted(graph.projection_targets.items()):
            projection = self.projections.get(agent)
            if projection is None:
                findings.append(
                    ExposureFinding(
                        "critical",
                        "projection_target_without_renderer",
                        target,
                        f"{agent} has a projection target but no renderer.",
                    )
                )
                continue
            target_path = project_root / target
            if not target_path.exists():
                findings.append(
                    ExposureFinding(
                        "critical",
                        "projection_target_missing",
                        target,
                        f"{agent} projection target does not exist.",
                    )
                )
                continue
            expected = projection.render(graph, agent)
            actual = target_path.read_text(encoding="utf-8")
            if actual != expected:
                findings.append(
                    ExposureFinding(
                        "critical",
                        "projection_target_drift",
                        target,
                        f"{agent} projection target differs from organ-exposure render output.",
                    )
                )
        return findings

    def apply_projection_files(
        self,
        project_root: Path,
        *,
        check: bool = False,
        dry_run: bool = False,
    ) -> dict[str, Any]:
        graph = self.graph()
        results: list[dict[str, Any]] = []
        changed = False
        for agent, target in sorted(graph.projection_targets.items()):
            projection = self.projections.get(agent)
            if projection is None:
                results.append(
                    {
                        "agent": agent,
                        "target": target,
                        "status": "missing_renderer",
                        "changed": False,
                    }
                )
                continue
            target_path = project_root / target
            expected = projection.render(graph, agent)
            actual = target_path.read_text(encoding="utf-8") if target_path.exists() else None
            target_changed = actual != expected
            changed = changed or target_changed
            status = "changed" if target_changed else "ok"
            if target_changed and not check and not dry_run:
                target_path.parent.mkdir(parents=True, exist_ok=True)
                target_path.write_text(expected, encoding="utf-8")
                status = "written"
            results.append(
                {
                    "agent": agent,
                    "target": target,
                    "status": status,
                    "changed": target_changed,
                }
            )
        return {
            "schema": "cynic.agent_exposure_apply.v1",
            "ok": not any(item["status"] == "missing_renderer" for item in results)
            and (not check or not changed),
            "check": check,
            "dry_run": dry_run,
            "changed": changed,
            "results": results,
        }


class CodexTomlProjection(AgentProjectionPort):
    def render(self, graph: AgentExposureGraph, agent: str) -> str:
        lines: list[str] = [
            "# Generated from infra/agent-exposure.json via organ-exposure.",
            "# Do not edit this block by hand; change the SSOT and re-render.",
            "",
        ]
        for server in graph.mcp_servers:
            if agent not in server.agents:
                continue
            lines.append(f"[mcp_servers.{server.name}]")
            lines.append(f"command = {json.dumps(server.command)}")
            if server.args:
                lines.append(f"args = {json.dumps(server.args)}")
            lines.append("")
            for tool in server.tools:
                lines.append(f"[mcp_servers.{server.name}.tools.{tool.name}]")
                lines.append(f"approval_mode = {json.dumps(tool.approval_mode)}")
                lines.append("")
        return "\n".join(lines).rstrip() + "\n"


class ClaudeMcpJsonProjection(AgentProjectionPort):
    def render(self, graph: AgentExposureGraph, agent: str) -> str:
        payload = {
            "mcpServers": {
                json_mcp_server_name(server.name): {"command": server.command, "args": server.args}
                for server in graph.mcp_servers
                if agent in server.agents
            }
        }
        return json.dumps(payload, indent=2, sort_keys=True) + "\n"


class AntigravitySettingsProjection(AgentProjectionPort):
    def render(self, graph: AgentExposureGraph, agent: str) -> str:
        payload = {
            "mcpServers": {
                json_mcp_server_name(server.name): {"command": server.command, "args": server.args}
                for server in graph.mcp_servers
                if agent in server.agents
            },
            "agents": {
                "codebase_investigator": {"enabled": True},
                "cli_help": {"enabled": True},
                "generalist": {"enabled": True},
            },
        }
        return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def json_mcp_server_name(name: str) -> str:
    aliases = {
        "cynic-coord": "cynic",
        "cynic-browser": "browser",
    }
    return aliases.get(name, name)


def extract_rust_mcp_tools(proxy_path: Path) -> set[str]:
    search_root = proxy_path.parent
    tools: set[str] = set()
    for path in sorted(search_root.rglob("*.rs")):
        content = path.read_text(encoding="utf-8")
        tools.update(re.findall(r'#\[tool\(\s*name\s*=\s*"([^"]+)"', content, flags=re.DOTALL))
    return tools


def extract_python_fallback_tools(fallback_path: Path) -> set[str]:
    if not fallback_path.exists():
        return set()
    content = fallback_path.read_text(encoding="utf-8")
    return set(re.findall(r'"([^"]+)"\s*:\s*_tool\(', content))


def extract_implemented_contract_tools(contracts_dir: Path) -> set[str]:
    tools: set[str] = set()
    for path in sorted(contracts_dir.glob("*.json")):
        payload = read_json_file_or_default(path, {})
        collect_implemented_contract_tools(payload, tools)
    return tools


def collect_implemented_contract_tools(payload: Any, tools: set[str]) -> None:
    if isinstance(payload, dict):
        tool = payload.get("mcp_tool")
        implementation_status = str(payload.get("implementation_status", ""))
        if isinstance(tool, str) and implementation_status.startswith("implemented"):
            tools.add(tool)
        for value in payload.values():
            collect_implemented_contract_tools(value, tools)
    elif isinstance(payload, list):
        for value in payload:
            collect_implemented_contract_tools(value, tools)
