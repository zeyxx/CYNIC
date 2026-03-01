"""
κ-NET Client — Somatic Nerve Receiver.
Connects to the κ-NET Server using IPv6 and WebSockets.
Implements 'Suturing' logic (Fibonacci backoff) for maximum reliability.
"""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import Callable
from typing import Any

import websockets

from cynic.kernel.protocol.kpulse import PulseMessage

logger = logging.getLogger("cynic.kernel.protocol.knet_client")

class KNetClient:
    """
    The Body's Nerve Receiver.
    Subscribes to the brain's pulse and updates local state.
    """

    def __init__(self, uri: str = "ws://[::1]:58766"):
        self.primary_uri = uri
        self.fallback_uri = "ws://127.0.0.1:58766"
        self._websocket = None
        self._running = False
        self._on_pulse_callbacks: list[Callable[[PulseMessage], Any]] = []
        self._status = "DISCONNECTED"
        self._retry_count = 0

    def on_pulse(self, callback: Callable[[PulseMessage], Any]):
        self._on_pulse_callbacks.append(callback)

    async def connect(self):
        """Start the connection loop with Suturing logic."""
        self._running = True
        asyncio.ensure_future(self._listen_loop())

    async def stop(self):
        self._running = False
        if self._websocket:
            await self._websocket.close()

    async def _listen_loop(self):
        """Infinite loop with Fibonacci backoff and Dual-Stack URI fallback."""
        fib = [1, 1, 2, 3, 5, 8, 13, 21, 34]
        
        while self._running:
            try:
                self._status = "SUTURING"
                # Switch URI every retry to test both stacks
                uri = self.primary_uri if self._retry_count % 2 == 0 else self.fallback_uri
                logger.debug(f"Nerve: Attempting connection to {uri}")
                
                async with websockets.connect(uri, open_timeout=2.0) as ws:
                    self._websocket = ws
                    self._status = f"CONNECTED ({'IPv6' if '::' in uri else 'IPv4'})"
                    self._retry_count = 0
                    logger.info(f"Nerve: Successfully sutured to {uri}")
                    
                    async for message in ws:
                        await self._handle_message(message)
                        
            except Exception as e:
                self._websocket = None
                self._status = "DISCONNECTED"
                
                # Calculate backoff
                wait = fib[min(self._retry_count, len(fib)-1)]
                logger.debug(f"Nerve: Connection failed ({type(e).__name__}). Retrying in {wait}s...")
                
                self._retry_count += 1
                await asyncio.sleep(wait)

    async def _handle_message(self, raw_message: str):
        """Process incoming κ-PULSE."""
        try:
            data = json.loads(raw_message)
            pulse = PulseMessage.from_dict(data)
            
            # Notify all registered callbacks
            for cb in self._on_pulse_callbacks:
                if asyncio.iscoroutinefunction(cb):
                    await cb(pulse)
                else:
                    cb(pulse)
                    
        except Exception as e:
            logger.warning(f"Nerve: Failed to process pulse: {e}")

    async def send(self, pulse: PulseMessage):
        """Send a message to the brain."""
        if self._websocket:
            await self._websocket.send(json.dumps(pulse.to_dict()))

    @property
    def status(self) -> str:
        return self._status
