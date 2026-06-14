import asyncio
import logging
import os
import sys
import json
# Configure logging
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")

# Add the project root to sys.path if needed
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))

from src.core.services import BuildService
from src.adapters.local_executor import LocalCommandExecutor, LocalWorkspaceSync, LocalArtifactRetriever
from src.adapters.websocket_adapter import KernelWebSocketAdapter

async def main():
    logger = logging.getLogger("KernelWorker")
    logger.info("Initializing Organ-Anvil Worker...")

    # Wire up the Hexagonal core
    sync_port = LocalWorkspaceSync()
    executor_port = LocalCommandExecutor()
    retriever_port = LocalArtifactRetriever()
    
    build_service = BuildService(sync_port, executor_port, retriever_port)
    
    # Configuration from SSoT (registry.json)
    registry_path = os.path.join(os.path.dirname(__file__), "../../../../infra/registry.json")
    with open(registry_path) as f:
        registry = json.load(f)
        
    kernel_node = next((n for n in registry["nodes"] if n["role"] == "proxmox_host_kernel"), None)
    if not kernel_node:
        raise RuntimeError("Could not find proxmox_host_kernel in registry.json")
        
    kernel_ip = kernel_node["network"]["tailscale"]
    if kernel_ip.startswith('<') and kernel_ip.endswith('>'):
        env_var = kernel_ip[1:-1]
        kernel_ip = os.environ.get(env_var, kernel_ip)
    kernel_uri = f"ws://{kernel_ip}:3030/node/ws"

    api_key = os.getenv("CYNIC_API_KEY", "cynic-dev-key")
    public_key = os.getenv("CYNIC_NODE_PUBKEY", "test-pubkey")
    identity = "organ-anvil" # Identity is derived from the organ itself

    # Wire up the adapter
    ws_adapter = KernelWebSocketAdapter(
        uri=kernel_uri,
        api_key=api_key,
        public_key=public_key,
        identity=identity,
        build_service=build_service
    )
    
    # Start the event loop
    logger.info(f"Connecting to CYNIC Kernel at {kernel_uri}...")
    await ws_adapter.connect_and_listen()

if __name__ == "__main__":
    asyncio.run(main())
