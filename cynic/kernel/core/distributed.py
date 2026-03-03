"""
CYNIC Distributed Bridge — The Redis-backed synchronization layer.

Connects the local EventBus to a global Redis backbone.
Allows multiple CYNIC instances to share a unified nervous system.

Lentilles : Backend (Messaging), SRE (Reliability), AI Infra (Sync).
"""

from __future__ import annotations

import asyncio
import json
import logging
from typing import TYPE_CHECKING, Optional

from cynic.kernel.core.event_bus import Event

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import EventBus
    from cynic.kernel.core.vascular import VascularSystem

logger = logging.getLogger("cynic.kernel.distributed")

class RedisEventBridge:
    """
    Bridges local events to Redis Pub/Sub with robust management.
    """

    def __init__(self, bus: EventBus, vascular: VascularSystem, task_registry: Any = None, channel_prefix: str = "cynic:events"):
        self.bus = bus
        self.vascular = vascular
        self.task_registry = task_registry
        self.instance_id = bus.instance_id
        self.channel = f"{channel_prefix}:{self.instance_id}"
        
        self._running = False
        self._listen_task: Optional[asyncio.Task] = None

    async def start(self):
        """Awaken the bridge and start listening to the network."""
        if self._running:
            return
        
        self._running = True
        self._listen_task = asyncio.create_task(self._listen_loop())
        
        # Register task for clean shutdown
        if self.task_registry:
            await self.task_registry.register(self._listen_task)
            
        self.bus.set_bridge(self)
        logger.info(f"[{self.instance_id}] Redis Bridge active on channel: {self.channel}")

    async def stop(self):
        """Shutdown the bridge."""
        self._running = False
        if self._listen_task:
            self._listen_task.cancel()
            try:
                await self._listen_task
            except asyncio.CancelledError:
                pass
        self.bus.set_bridge(None)
        logger.info(f"[{self.instance_id}] Redis Bridge dormant.")

    async def publish(self, event: Event):
        """Broadcast a local event to the network."""
        try:
            redis_client = await self.vascular.get_redis()
            
            # Serialize event
            payload = {
                "type": event.type,
                "payload": event.dict_payload,
                "source": event.source,
                "instance_id": event.instance_id,
                "timestamp": event.timestamp,
                "event_id": event.event_id
            }
            
            await redis_client.publish(self.channel, json.dumps(payload))
        except Exception as e:
            logger.error(f"Redis Bridge: Failed to publish event {event.type}: {e}")

    async def _listen_loop(self):
        """Listen for events coming from the network with robust reconnection."""
        retry_delay = 1.0
        max_delay = 60.0

        while self._running:
            try:
                redis_client = await self.vascular.get_redis()
                async with redis_client.pubsub() as pubsub:
                    await pubsub.subscribe(self.channel)
                    logger.info(f"Redis Bridge: Subscribed to {self.channel}")
                    
                    # Reset backoff on successful connection
                    retry_delay = 1.0
                    
                    while self._running:
                        # Compatibility fix: try common metadata ignoring params
                        try:
                            message = await pubsub.get_message(ignore_subscribe_messages=True, timeout=1.0)
                        except TypeError:
                            # Fallback for older/different SDK versions
                            message = await pubsub.get_message(timeout=1.0)
                            
                        if message is not None and message["type"] == "message":
                            await self._handle_remote_message(message["data"])
                        await asyncio.sleep(0.01) # Yield to event loop
                        
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Redis Bridge: Connection lost ({e}). Retrying in {retry_delay:.1f}s...")
                await asyncio.sleep(retry_delay)
                retry_delay = min(retry_delay * 1.618, max_delay) # phi-scaled backoff

    async def _handle_remote_message(self, data_str: str):
        """Re-inject a network event into the local bus."""
        try:
            data = json.loads(data_str)
            
            # Avoid re-processing our own events if published to a shared channel
            # (Though here the channel is instance-specific by default)
            # if data["instance_id"] == self.instance_id:
            #    return

            event = Event(
                type=data["type"],
                payload=data["payload"],
                source=f"remote:{data['source']}",
                instance_id=data["instance_id"]
            )
            
            # Re-emit locally WITHOUT distributing back to Redis (avoid infinite loop)
            await self.bus.emit(event, distributed=False)
            
        except Exception as e:
            logger.error(f"Redis Bridge: Failed to handle remote message: {e}")
