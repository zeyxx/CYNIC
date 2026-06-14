import logging
from src.core.ports import ICommandExecutor, IWorkspaceSync, IArtifactRetriever
from src.core.entities import BuildContext, BuildResult
from typing import Callable, Awaitable, Optional

logger = logging.getLogger(__name__)

class BuildService:
    """
    Application service that orchestrates the build process across environments.
    """
    def __init__(
        self,
        sync_port: IWorkspaceSync,
        executor_port: ICommandExecutor,
        retriever_port: IArtifactRetriever
    ):
        self.sync_port = sync_port
        self.executor_port = executor_port
        self.retriever_port = retriever_port

    async def run_build(
        self, 
        context: BuildContext, 
        command: str, 
        on_log_chunk: Optional[Callable[[str], Awaitable[None]]] = None,
        artifact_path: str = None, 
        destination_path: str = None
    ) -> BuildResult:
        logger.info(f"Starting build on target environment: {context.target_env.value}")
        
        # 1. Sync Workspace
        logger.info("Synchronizing workspace...")
        sync_success = await self.sync_port.sync_to_target(context)
        if not sync_success:
            logger.error("Workspace synchronization failed.")
            return BuildResult(success=False, stdout="", stderr="Sync failed", exit_code=1)
            
        # 2. Execute Command
        logger.info(f"Executing command: {command}")
        result = await self.executor_port.execute(context, command, on_log_chunk)
        
        if not result.success:
            logger.error(f"Command execution failed with exit code {result.exit_code}")
            return result
            
        # 3. Retrieve Artifacts
        if artifact_path and destination_path:
            logger.info(f"Retrieving artifact from {artifact_path} to {destination_path}...")
            retrieve_success = await self.retriever_port.retrieve(context, artifact_path, destination_path)
            if not retrieve_success:
                logger.error("Artifact retrieval failed.")
                return BuildResult(success=False, stdout=result.stdout, stderr="Artifact retrieval failed", exit_code=2)
                
        logger.info("Build process completed successfully.")
        return result
