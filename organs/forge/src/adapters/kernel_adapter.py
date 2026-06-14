import asyncio
import json
import logging
try:
    import websockets
except ImportError:
    pass

from ..core.ports import IKernelClient

logger = logging.getLogger(__name__)

import os
import pathlib

class KernelWebSocketClient(IKernelClient):
    def __init__(self, uri: str = None):
        if uri:
            self.uri = uri
        else:
            registry_path = os.path.join(os.path.dirname(__file__), "../../../../infra/registry.json")
            with open(registry_path) as f:
                registry = json.load(f)
            kernel_node = next((n for n in registry["nodes"] if n["role"] == "proxmox_host_kernel"), None)
            if not kernel_node:
                raise RuntimeError("proxmox_host_kernel not found in registry.json")
            ts_ip = kernel_node['network']['tailscale']
            if ts_ip.startswith('<') and ts_ip.endswith('>'):
                env_var = ts_ip[1:-1]
                ts_ip = os.environ.get(env_var, ts_ip)
            self.uri = f"ws://{ts_ip}:3030/node/ws"

    async def dispatch_task_and_stream(self, intent: str, patch: str):
        try:
            async with websockets.connect(self.uri) as ws:
                payload = {
                    "type": "post_task",
                    "intent": intent,
                    "patch": patch
                }
                await ws.send(json.dumps(payload))
                
                async for message in ws:
                    data = json.loads(message)
                    msg_type = data.get("type")
                    
                    if msg_type == "log_chunk":
                        yield data.get("stdout", "")
                        
                    elif msg_type == "task_result":
                        status = data.get("status")
                        if status != "success":
                            raise Exception(f"Task failed with status: {status}")
                        return
                        
        except Exception as e:
            logger.error(f"Failed to communicate with CYNIC Kernel: {e}")
            raise
