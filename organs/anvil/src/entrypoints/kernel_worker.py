import asyncio
import logging
import os
import sys

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
    
    # Configuration
    kernel_uri = os.getenv("CYNIC_KERNEL_WS_URI", "ws://127.0.0.1:3030/ws")
    api_key = os.getenv("CYNIC_API_KEY", "cynic-dev-key")
    public_key = os.getenv("CYNIC_NODE_PUBKEY", "test-pubkey")
    identity = os.getenv("CYNIC_NODE_IDENTITY", "organ-anvil-proxmox")

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
