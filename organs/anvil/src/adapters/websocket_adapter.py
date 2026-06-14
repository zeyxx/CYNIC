import asyncio
import json
import logging
import websockets
from typing import Dict, Any

from src.core.entities import BuildContext, TargetEnvironment
from src.core.services import BuildService

logger = logging.getLogger(__name__)

class KernelWebSocketAdapter:
    def __init__(self, uri: str, api_key: str, public_key: str, identity: str, build_service: BuildService):
        self.uri = uri
        self.api_key = api_key
        self.public_key = public_key
        self.identity = identity
        self.build_service = build_service
        self.ws = None

    async def connect_and_listen(self):
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "X-Node-Public-Key": self.public_key,
            "X-Node-Identity": self.identity,
        }
        
        while True:
            try:
                async with websockets.connect(self.uri, additional_headers=headers) as websocket:
                    self.ws = websocket
                    logger.info(f"Connected to CYNIC Kernel WS at {self.uri}")
                    
                    async for message in websocket:
                        await self._handle_message(message)
            except websockets.ConnectionClosed:
                logger.warning("Connection closed, reconnecting in 2 seconds...")
                await asyncio.sleep(2)
            except Exception as e:
                logger.error(f"WebSocket error: {e}")
                await asyncio.sleep(2)

    async def _handle_message(self, message: str):
        try:
            data = json.loads(message)
            msg_type = data.get("type")
            
            if msg_type == "route_task":
                task_id = data.get("task_id")
                patch = data.get("patch")
                intent = data.get("intent")
                
                # Spawn asynchronously to not block the event loop listening to WS
                asyncio.create_task(self._execute_build(task_id, patch, intent))
                
            elif msg_type == "register_response":
                logger.info(f"Registered successfully: {data.get('node_id')}")
            
            elif msg_type == "ping":
                await self.ws.send(json.dumps({
                    "type": "pong",
                    "timestamp": data.get("timestamp")
                }))
                
        except Exception as e:
            logger.error(f"Failed to handle message: {e}")

    async def _execute_build(self, task_id: str, patch: str, intent: str):
        logger.info(f"Starting task {task_id} [{intent}]")
        
        context = BuildContext(
            project_root="/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC", # Configurable in reality
            target_env=TargetEnvironment.LOCAL,
            env_vars={"CARGO_TERM_COLOR": "always"}
        )
        
        async def on_log_chunk(chunk: str):
            if self.ws:
                await self.ws.send(json.dumps({
                    "type": "log_chunk",
                    "task_id": task_id,
                    "stdout": chunk
                }))
                
        # Command to run (normally parsed from intent or configuration)
        command = "cargo test --workspace --no-run"
        
        if patch:
            patch_file = "/tmp/current_build.patch"
            with open(patch_file, "w") as f:
                f.write(patch)
            
            # Reset workspace first to be safe
            reset_cmd = "git reset --hard HEAD && git clean -fd"
            await self.build_service.executor_port.execute(context, reset_cmd, on_log_chunk)
            
            # Apply patch
            apply_cmd = f"git apply {patch_file}"
            apply_result = await self.build_service.executor_port.execute(context, apply_cmd, on_log_chunk)
            
            if not apply_result.success:
                logger.error("Failed to apply patch")
                if self.ws:
                    await self.ws.send(json.dumps({
                        "type": "task_result",
                        "task_id": task_id,
                        "status": "failed",
                        "error": "Failed to apply git patch"
                    }))
                return

        result = await self.build_service.run_build(context, command, on_log_chunk=on_log_chunk)
        
        if patch:
            # Cleanup workspace after build
            cleanup_cmd = "git reset --hard HEAD && git clean -fd"
            await self.build_service.executor_port.execute(context, cleanup_cmd, on_log_chunk)
        
        if self.ws:
            await self.ws.send(json.dumps({
                "type": "task_result",
                "task_id": task_id,
                "status": "success" if result.success else "failed",
                "artifact": None
            }))
        logger.info(f"Task {task_id} completed (Success: {result.success})")
