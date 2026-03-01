"""
κ-NET Server — Somatic WebSocket Broadcaster.
Runs on a dedicated high port (58765) using IPv6.
Distributes κ-PULSE messages to all connected nerves (CLI, Web, etc.).
"""

from __future__ import annotations

import json
import logging
import socket
from typing import Any

import websockets

from cynic.kernel.protocol.kpulse import PulseMessage, PulseType

logger = logging.getLogger("cynic.kernel.protocol.knet_server")

class KNetServer:
    """
    The Brain's Somatic Broadcaster.
    Maintains a pool of WebSocket connections and streams the organism's state.
    """

    def __init__(self, host: str = "::", port: int = 58766):
        self.host = host
        self.port = port
        self.clients: set[Any] = set()
        self._server = None
        self._running = False

    async def start(self):
        """Start the WebSocket server with IPv4 fallback."""
        try:
            self._server = await websockets.serve(
                self._handler,
                self.host,
                self.port,
                family=socket.AF_INET6 if ":" in self.host else socket.AF_INET
            )
            self._running = True
            logger.info(f"K-NET Server active on [{self.host}]:{self.port}")
        except Exception as e:
            if self.host == "::":
                self.host = "0.0.0.0"
                await self.start()
            else:
                logger.error(f"Failed to start K-NET Server: {e}")

    async def stop(self):
        """Gracefully stop the server."""
        self._running = False
        if self._server:
            self._server.close()
            await self._server.wait_closed()
            logger.info("κ-NET Server detached.")

    async def _handler(self, websocket, path):
        """Handle new nerve connections."""
        self.clients.add(websocket)
        
        # Immediate welcome pulse for instant UI update
        try:
            from cynic.interfaces.api.state import get_state
            org = get_state()
            if org:
                stats = org.state.get_stats()
                data = {
                    "mind": {
                        "status": "AWAKE",
                        "thinking": stats.get("current_analysis", "Synchronizing..."),
                        "confidence": stats.get("confidence", 0.618),
                        "e_score": stats.get("e_score", 50.0),
                        "axiom_scores": stats.get("axiom_scores", {})
                    }
                }
                # Add hardware
                if hasattr(org.metabolism, "body") and org.metabolism.body:
                    body_state = getattr(org.metabolism.body, "_last_state", None)
                    if body_state:
                        data["hardware"] = body_state.to_dict()

                welcome_pulse = PulseMessage(type=PulseType.SOMATIC_SYNC, data=data)
                await websocket.send(json.dumps(welcome_pulse.to_dict()))
        except Exception:
            pass

        try:
            async for message in websocket:
                # Handle incoming messages from the body (e.g. Sensory Input)
                await self._process_incoming(message, websocket)
        except websockets.exceptions.ConnectionClosed:
            pass
        finally:
            self.clients.remove(websocket)
            logger.info(f"κ-NET: Nerve detached. Total: {len(self.clients)}")

    async def broadcast(self, pulse: PulseMessage):
        """Stream a κ-PULSE to all connected nerves."""
        if not self.clients:
            return
        
        payload = json.dumps(pulse.to_dict())
        disconnected = set()
        
        for client in self.clients:
            try:
                await client.send(payload)
            except Exception:
                disconnected.add(client)
        
        for client in disconnected:
            self.clients.remove(client)

    async def _process_incoming(self, raw_message: str, websocket):
        """Handle data sent FROM the body to the brain."""
        try:
            data = json.loads(raw_message)
            pulse = PulseMessage.from_dict(data)
            logger.debug(f"κ-NET: Received {pulse.type.value} from body")
            # TODO: Dispatch to Organism Event Bus
        except Exception as e:
            logger.warning(f"κ-NET: Failed to process message: {e}")

# Global instance for the container
_KNET_SERVER: KNetServer | None = None

async def get_knet_server() -> KNetServer:
    global _KNET_SERVER
    if _KNET_SERVER is None:
        _KNET_SERVER = KNetServer()
        await _KNET_SERVER.start()
    return _KNET_SERVER
