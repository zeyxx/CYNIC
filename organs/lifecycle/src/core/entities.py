from dataclasses import dataclass, field
from typing import Dict, Any, List

@dataclass
class DeploymentTarget:
    name: str
    role: str
    metadata: Dict[str, Any] = field(default_factory=dict)

@dataclass
class ContainerConfig:
    image: str
    ports: Dict[str, str]
    env: Dict[str, str] = field(default_factory=dict)

@dataclass
class SmartContractPayload:
    network: str
    contract_name: str
    bytecode: str | None = None
    rpc_url: str | None = None

@dataclass
class ForgejoConfig:
    repo_owner: str
    repo_name: str
    branch: str

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
    requires_e2e: bool
    targets: List[str]
    coord_targets: List[str]
    deploy: bool
    patch_sha256: str | None = None

@dataclass
class AgentTask:
    id: str
    kind: str
    domain: str
    agent_id: str
    content: LifecycleTaskContent
    status: str
