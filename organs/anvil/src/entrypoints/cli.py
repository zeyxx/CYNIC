import argparse
import sys
import os
import logging
from src.core.entities import BuildContext, TargetEnvironment
from src.core.services import BuildService
from src.adapters.local_executor import LocalWorkspaceSync, LocalCommandExecutor, LocalArtifactRetriever
from src.adapters.ssh_executor import SshWorkspaceSync, SshCommandExecutor, SshArtifactRetriever

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

def main():
    parser = argparse.ArgumentParser(description="Organ Anvil: Distributed Build Executor")
    parser.add_argument("--target", choices=["local", "proxmox", "docker"], default="local", help="Target environment for execution")
    parser.add_argument("--command", required=True, help="Command to execute (e.g., 'cargo build --release')")
    parser.add_argument("--host", help="Target host for SSH (e.g., IP or hostname)")
    parser.add_argument("--user", help="Target user for SSH")
    parser.add_argument("--artifact", help="Path of the artifact on the remote to retrieve")
    parser.add_argument("--dest", help="Local destination path for the retrieved artifact")
    
    args = parser.parse_args()
    
    project_root = os.environ.get("PROJECT_DIR", os.getcwd())
    target_env = TargetEnvironment(args.target)
    
    context = BuildContext(
        project_root=project_root,
        target_env=target_env,
        target_host=args.host,
        target_user=args.user,
        env_vars={"RUST_MIN_STACK": "2147483648"} # Propagate CYNIC requirements
    )
    
    if target_env == TargetEnvironment.LOCAL:
        sync_port = LocalWorkspaceSync()
        executor_port = LocalCommandExecutor()
        retriever_port = LocalArtifactRetriever()
    elif target_env == TargetEnvironment.PROXMOX:
        sync_port = SshWorkspaceSync()
        executor_port = SshCommandExecutor()
        retriever_port = SshArtifactRetriever()
    else:
        print(f"Target environment {target_env.value} is not fully implemented yet.")
        sys.exit(1)
        
    service = BuildService(sync_port, executor_port, retriever_port)
    
    result = service.run_build(
        context=context,
        command=args.command,
        artifact_path=args.artifact,
        destination_path=args.dest
    )
    
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)
        
    if not result.success:
        sys.exit(result.exit_code)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()
