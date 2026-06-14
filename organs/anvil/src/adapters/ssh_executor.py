import subprocess
import os
from src.core.ports import ICommandExecutor, IWorkspaceSync, IArtifactRetriever
from src.core.entities import BuildContext, BuildResult

class SshWorkspaceSync(IWorkspaceSync):
    def sync_to_target(self, context: BuildContext) -> bool:
        if not context.target_host or not context.target_user:
            raise ValueError("Target host and user are required for SSH sync")
        
        # We assume the project root is mirrored to the same path on the remote,
        # or we could specify a remote path. Let's assume a known remote workspace dir.
        remote_path = context.project_root
        
        # rsync from local project_root to target_host:project_root
        # Excluding .git and target/ to speed up sync, but keeping what's needed to build.
        rsync_command = [
            "rsync", "-avz", "--delete",
            "--exclude", ".git",
            "--exclude", "target",
            f"{context.project_root}/",
            f"{context.target_user}@{context.target_host}:{remote_path}/"
        ]
        
        try:
            process = subprocess.run(rsync_command, capture_output=True, text=True)
            return process.returncode == 0
        except Exception:
            return False

class SshCommandExecutor(ICommandExecutor):
    def execute(self, context: BuildContext, command: str) -> BuildResult:
        if not context.target_host or not context.target_user:
            raise ValueError("Target host and user are required for SSH execution")
        
        # Build the environment variable string
        env_prefix = " ".join([f"{k}='{v}'" for k, v in context.env_vars.items()])
        
        # Command to run remotely
        remote_path = context.project_root
        remote_command = f"cd {remote_path} && {env_prefix} {command}"
        
        ssh_command = [
            "ssh",
            f"{context.target_user}@{context.target_host}",
            remote_command
        ]
        
        try:
            process = subprocess.run(
                ssh_command,
                capture_output=True,
                text=True
            )
            return BuildResult(
                success=(process.returncode == 0),
                stdout=process.stdout,
                stderr=process.stderr,
                exit_code=process.returncode
            )
        except Exception as e:
            return BuildResult(
                success=False,
                stdout="",
                stderr=str(e),
                exit_code=-1
            )

class SshArtifactRetriever(IArtifactRetriever):
    def retrieve(self, context: BuildContext, artifact_path: str, destination_path: str) -> bool:
        if not context.target_host or not context.target_user:
            raise ValueError("Target host and user are required for SSH retrieval")
            
        scp_command = [
            "scp",
            f"{context.target_user}@{context.target_host}:{artifact_path}",
            destination_path
        ]
        
        try:
            process = subprocess.run(scp_command, capture_output=True, text=True)
            return process.returncode == 0
        except Exception:
            return False
