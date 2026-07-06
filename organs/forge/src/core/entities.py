from dataclasses import dataclass, field
from typing import List, Dict, Any

from organs.contracts.src.surface_contracts import json_object_or_default

@dataclass
class Branch:
    name: str
    commit_hash: str
    author: str
    date: str

@dataclass
class Alert:
    severity: str
    message: str
    context: Dict[str, Any] = field(default_factory=dict)

@dataclass
class RepoState:
    current_branch: str
    dirty_files: int
    untracked_files: int
    branches: List[Branch] = field(default_factory=list)
    gate_markers: Dict[str, bool] = field(default_factory=dict)
    health_score: float = 100.0
    alerts: List[Alert] = field(default_factory=list)

    def add_alert(self, severity: str, message: str, context: dict = None):
        self.alerts.append(Alert(severity, message, context or {}))


class FleetProbe(dict):
    @classmethod
    def not_run(cls) -> "FleetProbe":
        return cls({"enabled": False, "status": "not_run"})

    @classmethod
    def from_dict(cls, data: dict | None) -> "FleetProbe":
        if isinstance(data, cls):
            return data
        return cls(data or {"enabled": False, "status": "not_run"})

    def to_dict(self) -> dict:
        return dict(self)

    def is_ok(self) -> bool:
        return self.status == "ok"

    @property
    def status(self) -> str:
        return str(self.get("status", "unknown"))

    @property
    def error(self) -> str | None:
        error = self.get("error")
        return str(error) if error is not None else None

    @property
    def missing(self) -> list:
        missing = self.get("missing", [])
        return missing if isinstance(missing, list) else []

    @property
    def matches(self) -> dict:
        matches = self.get("matches", {})
        return matches if isinstance(matches, dict) else {}

    @property
    def targets(self) -> list:
        targets = self.get("targets", [])
        return targets if isinstance(targets, list) else []


@dataclass
class LifecycleTaskContent:
    lifecycle: str
    intent: str
    objective: str
    orchestrator_organ: str
    domain: str
    build_target: str
    build_runner_role: str
    deploy_target: str
    deploy_target_role: str
    required_node_status: str | None
    activation_requirements: Dict[str, Any]
    runtime_paths: Dict[str, str]
    runtime_path_proofs: Dict[str, Any]
    activation_evidence_contract: Dict[str, str]
    fleet_requirements: Dict[str, Any]
    mcp_freshness_controller: Dict[str, Any]
    requires_e2e: bool
    targets: List[str]
    coord_targets: List[str]
    deploy: bool
    request_profile: str | None = None
    patch_sha256: str | None = None
    activation_schema_version: str | None = None
    correlation_id: str | None = None
    action: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict) -> 'LifecycleTaskContent':
        return cls(
            lifecycle=data.get('lifecycle', ''),
            intent=data.get('intent', ''),
            objective=data.get('objective', ''),
            orchestrator_organ=data.get('orchestrator_organ', ''),
            domain=data.get('domain', ''),
            build_target=data.get('build_target', ''),
            build_runner_role=data.get('build_runner_role', ''),
            deploy_target=data.get('deploy_target', ''),
            deploy_target_role=data.get('deploy_target_role', ''),
            required_node_status=data.get('required_node_status'),
            activation_requirements=data.get('activation_requirements', {}),
            runtime_paths=data.get('runtime_paths', {}),
            runtime_path_proofs=data.get('runtime_path_proofs', {}),
            activation_evidence_contract=data.get('activation_evidence_contract', {}),
            fleet_requirements=data.get('fleet_requirements', {}),
            mcp_freshness_controller=data.get('mcp_freshness_controller', {}),
            requires_e2e=data.get('requires_e2e', True),
            targets=data.get('targets', []),
            coord_targets=data.get('coord_targets', []),
            deploy=data.get('deploy', False),
            request_profile=data.get('request_profile'),
            patch_sha256=data.get('patch_sha256'),
            activation_schema_version=data.get('activation_schema_version'),
            correlation_id=data.get('correlation_id'),
            action=data.get('action', {})
        )




@dataclass
class AgentTask:
    id: str
    kind: str
    domain: str
    agent_id: str
    content: LifecycleTaskContent
    status: str
    
    @classmethod
    def from_dict(cls, data: dict) -> 'AgentTask':
        content_raw = data.get('content', '{}')
        content_dict = json_object_or_default(content_raw, {})
            
        return cls(
            id=data.get('id', ''),
            kind=data.get('kind', ''),
            domain=data.get('domain', ''),
            agent_id=data.get('agent_id', ''),
            content=LifecycleTaskContent.from_dict(content_dict),
            status=data.get('status', 'pending')
        )
