import asyncio
import json
import logging
try:
    import websockets
except ImportError:
    pass

from ..core.ports import IKernelClient

logger = logging.getLogger(__name__)

class UDSKernelClient(IKernelClient):
    def __init__(self, uds_path: str):
        self.uds_path = uds_path

    async def dispatch_task_and_stream(self, intent: str, patch: str):
        try:
            async with websockets.unix_connect(self.uds_path) as ws:
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
