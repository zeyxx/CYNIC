from abc import ABC, abstractmethod
from typing import Callable, Awaitable, Optional
from .entities import BuildContext, BuildResult

class IWorkspaceSync(ABC):
    """
    Port for synchronizing the local workspace to the target execution environment.
    """
    @abstractmethod
    async def sync_to_target(self, context: BuildContext) -> bool:
        pass

class ICommandExecutor(ABC):
    """
    Port for executing commands on the target environment with real-time log streaming.
    """
    @abstractmethod
    async def execute(self, context: BuildContext, command: str, on_log_chunk: Optional[Callable[[str], Awaitable[None]]] = None) -> BuildResult:
        pass

class IArtifactRetriever(ABC):
    """
    Port for retrieving built artifacts back from the target environment.
    """
    @abstractmethod
    async def retrieve(self, context: BuildContext, artifact_path: str, destination_path: str) -> bool:
        pass
