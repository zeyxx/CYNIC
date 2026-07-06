from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, AsyncGenerator, Dict, List

from .entities import ContainerConfig, DeploymentTarget, ForgejoConfig, SmartContractPayload


class InfraRegistryPort(ABC):
    """Reads organism registry state and resolves role-to-target mappings."""

    @abstractmethod
    def load_registry(self) -> Dict[str, Any]:
        pass

    @abstractmethod
    def resolve_node(self, role: str) -> Dict[str, Any] | None:
        pass

    @abstractmethod
    def resolve_service(self, service_name: str) -> Dict[str, Any] | None:
        pass


class FleetStatusPort(ABC):
    """Reads fleet evidence for declared nodes."""

    @abstractmethod
    async def is_node_online(self, node_name: str, threshold_seconds: int = 30) -> bool:
        pass

    @abstractmethod
    async def get_node_last_seen(self, node_name: str) -> float:
        pass

    @abstractmethod
    async def get_node_status(self, node_name: str) -> Dict[str, Any]:
        pass


class ServiceHealthPort(ABC):
    """Reads declared service health without local fallbacks."""

    @abstractmethod
    async def check_service_health(self, service_name: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    async def service_status(self, service_name: str) -> Dict[str, Any]:
        pass


class StoragePrimaryPort(ABC):
    """Reads and prepares storage-primary activation evidence."""

    @abstractmethod
    async def status(self) -> Dict[str, Any]:
        pass

    @abstractmethod
    async def readiness(self) -> Dict[str, Any]:
        pass

    @abstractmethod
    async def bundle(self, output_dir: str) -> Dict[str, Any]:
        pass


class LifecycleDispatchPort(ABC):
    """Dispatches lifecycle work through the registry lifecycle path."""

    @abstractmethod
    async def dispatch(
        self,
        profile: str,
        intent: str,
        objective: str,
        payload: Dict[str, Any] | None = None,
    ) -> Dict[str, Any]:
        pass

    @abstractmethod
    async def payloads(self, profile: str) -> Dict[str, Any]:
        pass


class CoordinationPort(ABC):
    """Coordinates agent claims, observations, and lifecycle state."""

    @abstractmethod
    def register_agent(self, agent_id: str, intent: str, agent_type: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    def who(self) -> Dict[str, Any]:
        pass

    @abstractmethod
    def claim(self, agent_id: str, target: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    def release(self, agent_id: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    def list_pending_tasks(self, kind: str, domain: str | None = None) -> List[Dict[str, Any]]:
        pass

    @abstractmethod
    def list_completed_tasks(self, kind: str, domain: str | None = None) -> List[Dict[str, Any]]:
        pass

    @abstractmethod
    def claim_agent_task(self, task_id: str, agent_id: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    def update_agent_task_result(
        self,
        task_id: str,
        kind: str,
        domain: str,
        result: Dict[str, Any],
    ) -> Dict[str, Any]:
        pass


class ObservationPort(ABC):
    """Records observations and structured organism signals."""

    @abstractmethod
    def observe(self, event: Dict[str, Any]) -> Dict[str, Any]:
        pass

    @abstractmethod
    def audit_query(self, query: Dict[str, Any]) -> Dict[str, Any]:
        pass


class SupplyChainAnalysisPort(ABC):
    """Analyzes declared supply-chain and execution dependencies for cycles and drift."""

    @abstractmethod
    def analyze_supply_chain(self) -> Dict[str, Any]:
        pass


class SovereignInferencePort(ABC):
    """Routes bounded inference requests to a sovereign agentic runtime."""

    @abstractmethod
    def request_dependency_convergence(self, convergence_pack: Dict[str, Any]) -> Dict[str, Any]:
        pass


class MempoolPort(ABC):
    @abstractmethod
    def fetch_pending_tasks(self, kind: str, domain: str) -> List[Any]:
        pass

    @abstractmethod
    def claim_task(self, task_id: str) -> bool:
        pass

    @abstractmethod
    def submit_result(self, task_id: str, success: bool, result_data: Dict[str, Any]) -> bool:
        pass


class SovereignForgePort(ABC):
    """Reads and mutates the sovereign Forgejo lifecycle surface.

    Git is the repository substrate behind the forge; this port models the
    lifecycle authority, not raw git plumbing or GitHub-style assumptions.
    """

    @abstractmethod
    async def get_latest_commit(self, config: ForgejoConfig) -> str:
        pass

    @abstractmethod
    async def create_change_request(self, config: ForgejoConfig, title: str, description: str) -> str:
        pass

    @abstractmethod
    async def merge_change_request_after_evidence(self, config: ForgejoConfig, request_id: str) -> bool:
        pass


class VmProvisionerPort(ABC):
    @abstractmethod
    async def provision_vm(self, target: DeploymentTarget) -> bool:
        pass

    @abstractmethod
    async def destroy_vm(self, target: DeploymentTarget) -> bool:
        pass


class ContainerRuntimePort(ABC):
    @abstractmethod
    async def deploy_container(self, target: DeploymentTarget, config: ContainerConfig) -> bool:
        pass

    @abstractmethod
    async def get_container_logs(self, target: DeploymentTarget) -> str:
        pass


class ServerlessHostingPort(ABC):
    @abstractmethod
    async def deploy_project(self, target: DeploymentTarget, project_path: str) -> str:
        pass


class SmartContractRuntimePort(ABC):
    @abstractmethod
    async def execute_tests(self, payload: SmartContractPayload) -> bool:
        pass

    @abstractmethod
    async def deploy_contract(self, payload: SmartContractPayload) -> str:
        pass


class NetworkFleetPort(FleetStatusPort):
    """Legacy name kept for compatibility with existing adapters."""

    @abstractmethod
    async def is_node_online(self, node_name: str, threshold_seconds: int = 30) -> bool:
        pass

    @abstractmethod
    async def get_node_last_seen(self, node_name: str) -> float:
        pass
