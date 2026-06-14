import asyncio
import os
from typing import Callable, Awaitable, Optional
from src.core.ports import ICommandExecutor, IWorkspaceSync, IArtifactRetriever
from src.core.entities import BuildContext, BuildResult

class LocalCommandExecutor(ICommandExecutor):
    async def execute(self, context: BuildContext, command: str, on_log_chunk: Optional[Callable[[str], Awaitable[None]]] = None) -> BuildResult:
        env = os.environ.copy()
        env.update(context.env_vars)
        
        try:
            process = await asyncio.create_subprocess_shell(
                command,
                cwd=context.project_root,
                env=env,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.STDOUT
            )
            
            full_log = []
            while True:
                line = await process.stdout.readline()
                if not line:
                    break
                line_str = line.decode('utf-8')
                full_log.append(line_str)
                if on_log_chunk:
                    await on_log_chunk(line_str)
            
            await process.wait()
            
            return BuildResult(
                success=(process.returncode == 0),
                stdout="".join(full_log),
                stderr="",
                exit_code=process.returncode
            )
        except Exception as e:
            return BuildResult(
                success=False,
                stdout="",
                stderr=str(e),
                exit_code=-1
            )

class LocalWorkspaceSync(IWorkspaceSync):
    async def sync_to_target(self, context: BuildContext) -> bool:
        # In a local execution, the workspace is already synced.
        return True

class LocalArtifactRetriever(IArtifactRetriever):
    async def retrieve(self, context: BuildContext, artifact_path: str, destination_path: str) -> bool:
        # Artifacts are already local, but we could copy them if needed.
        # For a truly local run, they are already at artifact_path.
        return True
