from abc import ABC, abstractmethod
from typing import List, Dict, Any
from .entities import (
    AgentTask, 
    DeploymentTarget, 
    ContainerConfig, 
    SmartContractPayload,
    ForgejoConfig
)

class MempoolPort(ABC):
    @abstractmethod
    def fetch_pending_tasks(self, kind: str, domain: str) -> List[AgentTask]:
        pass

    @abstractmethod
    def claim_task(self, task_id: str) -> bool:
        pass

    @abstractmethod
    def submit_result(self, task_id: str, success: bool, result_data: Dict[str, Any]) -> bool:
        pass

class GitProviderPort(ABC):
    @abstractmethod
    async def get_latest_commit(self, config: ForgejoConfig) -> str:
        pass
        
    @abstractmethod
    async def create_pull_request(self, config: ForgejoConfig, title: str, description: str) -> str:
        pass
        
    @abstractmethod
    async def merge_pull_request(self, config: ForgejoConfig, pr_id: str) -> bool:
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

class NetworkFleetPort(ABC):
    @abstractmethod
    async def is_node_online(self, node_name: str, threshold_seconds: int = 30) -> bool:
        pass

    @abstractmethod
    async def get_node_last_seen(self, node_name: str) -> float:
        pass
