"""
CYNIC Somatic Gateway — The Scalable Perception Baselayer.

This is the 'Thalamus' of CYNIC. It manages all incoming sensory data,
providing:
  1. Multi-protocol Conduits (HTTP, WS, Browser).
  2. High-speed normalization and validation (Reality Schemas).
  3. Somatic Filtering & Backpressure (Sampling, Thresholds).
  4. Unified Event Emission to the Nervous System.

Lentilles : AI Infra (Scalability), Data Engineer (Pipeline), SRE (Backpressure).
"""

from __future__ import annotations

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from typing import Any, Callable, Dict, Optional

from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event
from cynic.kernel.core.realities import validate_content
from cynic.kernel.core.judgment import Cell

logger = logging.getLogger("cynic.perception.gateway")

class Conduit(ABC):
    """Abstract driver for a data source."""
    
    def __init__(self, conduit_id: str):
        self.conduit_id = conduit_id
        self._ingest_cb: Optional[Callable[[str, Any], None]] = None

    @abstractmethod
    async def start(self, ingest_cb: Callable[[str, Any], None]):
        """Start the conduit and register the ingestion callback."""
        self._ingest_cb = ingest_cb

    @abstractmethod
    async def stop(self):
        """Stop the conduit."""
        ...

class SomaticGateway:
    """
    The central ingestion point for all perception.
    Protects the brain from high-frequency noise via Metabolic Filtering.
    """

    def __init__(self, bus: EventBus, buffer_size: int = 1000):
        self.bus = bus
        self.instance_id = bus.instance_id
        self._conduits: Dict[str, Conduit] = {}
        self._mappings: Dict[str, str] = {} # conduit_id -> reality
        self._buffer: asyncio.Queue = asyncio.Queue(maxsize=buffer_size)
        self._running = False
        self._task: Optional[asyncio.Task] = None
        
        # Metabolic Flow Control (Lentille : SRE)
        self._last_emission_time: Dict[str, float] = {}
        self._last_content_hash: Dict[str, int] = {}
        self._min_interval_s: Dict[str, float] = {
            "GAMBLING": 0.05,  # 20Hz max for high-freq games
            "MARKET": 0.5,     # 2Hz for prices
            "INTERNAL": 1.0,   # 1Hz for health/proprioception
        }
        self._default_interval = 0.1
        
        # Metrics
        self._ingested_count = 0
        self._dropped_count = 0
        self._emitted_count = 0

    async def start(self):
        """Awaken the gateway."""
        if self._running:
            return
        self._running = True
        self._task = asyncio.create_task(self._process_loop())
        
        # Start all registered conduits
        for conduit in self._conduits.values():
            await conduit.start(ingest_cb=self.ingest)
            
        logger.info(f"[{self.instance_id}] Somatic Gateway: Active.")

    async def stop(self):
        """Shutdown the gateway and all conduits."""
        self._running = False
        if self._task:
            self._task.cancel()
        
        for conduit in self._conduits.values():
            await conduit.stop()
        
        logger.info(f"[{self.instance_id}] Somatic Gateway: Dormant.")

    def register_conduit(self, conduit: Conduit, reality: str = "INTERNAL"):
        """Add a new sensor driver to the gateway."""
        self._conduits[conduit.conduit_id] = conduit
        self._mappings[conduit.conduit_id] = reality
        logger.debug(f"[{self.instance_id}] Registered conduit: {conduit.conduit_id} (Reality: {reality})")

    async def ingest(self, conduit_id: str, data: Any):
        """
        High-speed entry point. 
        Places data in the somatic buffer for asynchronous processing.
        """
        self._ingested_count += 1
        reality = self._mappings.get(conduit_id, "INTERNAL")
        
        logger.debug(f"[{self.instance_id}] Ingesting from {conduit_id} (Reality: {reality})")
        
        payload = {
            "source": conduit_id,
            "reality": reality,
            "data": data,
            "timestamp": time.time()
        }
        
        try:
            # We use put_nowait to ensure we don't block the conduit's IO thread
            self._buffer.put_nowait(payload)
        except asyncio.QueueFull:
            self._dropped_count += 1
            # Backpressure: Drop the observation if buffer is full
            if self._dropped_count % 100 == 0:
                logger.warning(f"[{self.instance_id}] Somatic Gateway: Buffer full. Dropped {self._dropped_count} observations.")

    async def _process_loop(self):
        """
        The 'Processing Plant' of perception.
        Normalizes, filters, and emits.
        """
        while self._running:
            try:
                payload = await self._buffer.get()
                
                # 1. Normalization & Validation
                reality = payload["reality"]
                raw_data = payload["data"]
                
                # Validate against Reality Schemas
                validated_data = validate_content(reality, raw_data)
                
                # 2. Filtering (Metabolic Regulation)
                # TODO: Implement pluggable filters
                if not self._should_filter(reality, validated_data):
                    await self._emit(payload["source"], reality, validated_data)
                
                self._buffer.task_done()
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Somatic Gateway process error: {e}")
                await asyncio.sleep(0.1)

    def _should_filter(self, reality: str, data: Any) -> bool:
        """
        Flow control: Drop events if they arrive too fast OR if content is identical.
        """
        now = time.time()
        last_time = self._last_emission_time.get(reality, 0.0)
        min_interval = self._min_interval_s.get(reality, self._default_interval)
        
        # 1. Time-based filtering
        if now - last_time < min_interval:
            self._dropped_count += 1
            logger.debug(f"[{self.instance_id}] Filtered {reality} (Time: delta={now - last_time:.3f}s < {min_interval}s)")
            return True
            
        # 2. Content-based deduplication (Lentille : Data Engineer)
        try:
            content_hash = hash(str(data))
            if self._last_content_hash.get(reality) == content_hash:
                self._dropped_count += 1
                logger.debug(f"[{self.instance_id}] Filtered {reality} (Deduplication: content matches)")
                return True
        except Exception:
            pass # Fallback to time-only if hash fails
            
        return False

    async def _emit(self, source: str, reality: str, data: Any):
        """Translate to internal Cell and emit to Nervous System."""
        self._emitted_count += 1
        self._last_emission_time[reality] = time.time()
        try:
            self._last_content_hash[reality] = hash(str(data))
        except Exception:
            pass
        
        logger.info(f"[{self.instance_id}] Somatic Gateway: Emitting PERCEPTION_RECEIVED for {reality} from {source}")
        
        # Wrap in a Cell
        cell = Cell(
            reality=reality,
            analysis="PERCEPTION",
            content={
                **(data if isinstance(data, dict) else {"raw": data}),
                "origin_instance": self.instance_id
            },
            budget_usd=0.0
        )
        
        await self.bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            cell.model_dump(),
            source=source
        ))

    async def drain(self, timeout: float = 5.0):
        """Wait for the buffer to be empty."""
        start_wait = time.time()
        while not self._buffer.empty():
            if time.time() - start_wait > timeout:
                logger.warning(f"[{self.instance_id}] Somatic Gateway: Drain timeout.")
                break
            await asyncio.sleep(0.1)
        await self._buffer.join()

    def stats(self) -> dict:
        """SRE metrics for the gateway."""
        return {
            "ingested": self._ingested_count,
            "dropped": self._dropped_count,
            "emitted": self._emitted_count,
            "buffer_load": self._buffer.qsize() / self._buffer.maxsize if self._buffer.maxsize else 0
        }
