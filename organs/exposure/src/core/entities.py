from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List


@dataclass(frozen=True)
class ToolExposure:
    name: str
    approval_mode: str = "approve"

    @classmethod
    def from_dict(cls, payload: Dict[str, Any]) -> "ToolExposure":
        return cls(
            name=payload["name"],
            approval_mode=payload.get("approval_mode", "approve"),
        )


@dataclass(frozen=True)
class McpServerExposure:
    name: str
    command: str
    status: str
    agents: List[str]
    args: List[str] = field(default_factory=list)
    tools: List[ToolExposure] = field(default_factory=list)
    description: str = ""
    requires: List[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, payload: Dict[str, Any]) -> "McpServerExposure":
        return cls(
            name=payload["name"],
            command=payload["command"],
            args=list(payload.get("args", [])),
            status=payload.get("status", "planned"),
            agents=list(payload.get("agents", [])),
            tools=[ToolExposure.from_dict(tool) for tool in payload.get("tools", []) if isinstance(tool, dict)],
            description=payload.get("description", ""),
            requires=list(payload.get("requires", [])),
        )


@dataclass(frozen=True)
class AgentExposureGraph:
    version: int
    mcp_servers: List[McpServerExposure]
    projection_targets: Dict[str, str] = field(default_factory=dict)
    mcp_tool_domains: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, payload: Dict[str, Any]) -> "AgentExposureGraph":
        return cls(
            version=int(payload.get("version", 1)),
            projection_targets=dict(payload.get("projection_targets", {})),
            mcp_tool_domains=dict(payload.get("mcp_tool_domains", {})),
            mcp_servers=[
                McpServerExposure.from_dict(server)
                for server in payload.get("mcp_servers", [])
            ],
        )


@dataclass(frozen=True)
class ExposureFinding:
    severity: str
    kind: str
    path: str
    detail: str


@dataclass(frozen=True)
class ExposureValidationReport:
    schema: str
    ok: bool
    findings: List[ExposureFinding]
    rust_tool_count: int
    exposed_tool_count: int
    implemented_contract_tool_count: int
