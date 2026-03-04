"""
CYNIC Synaptic Tracer - Low-level Latency & Jitter Monitoring.
Respects SRE, Robotics & AI Infra Lenses.

Traces the path of a 'Cell' through the nervous system.
Measures 'Synaptic Latency': the time between Perception and Judgment.
Identifies bottlenecks in the agentic event loop.
"""

from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from typing import Dict, List, Optional

from typing import Any

logger = logging.getLogger("cynic.kernel.synaptic_tracer")


@dataclass
class SynapticTrace:
    cell_id: str
    start_time: float
    hops: List[Dict[str, Any]] = field(default_factory=list)
    total_latency_ms: float = 0.0


class SynapticTracer:
    """
    The 'eBPF' of CYNIC.
    Monitors the flow of thoughts to detect cognitive jitter.
    """

    def __init__(self) -> None:
        self._active_traces: Dict[str, SynapticTrace] = {}
        self._history: List[float] = []  # EMA of latencies

    def start_trace(self, cell_id: str) -> None:
        """Mark the birth of a thought."""
        self._active_traces[cell_id] = SynapticTrace(
            cell_id=cell_id, start_time=time.perf_counter()
        )

    def record_hop(
        self, cell_id: str, component: str, metadata: Optional[Dict[str, Any]] = None
    ) -> None:
        """Record when a thought passes through a specific organ (e.g., Gateway, Dog)."""
        if cell_id in self._active_traces:
            trace = self._active_traces[cell_id]
            hop_time = time.perf_counter() - trace.start_time
            trace.hops.append(
                {
                    "component": component,
                    "at_ms": hop_time * 1000,
                    "metadata": metadata or {},
                }
            )

    def complete_trace(self, cell_id: str) -> Optional[float]:
        """Mark the completion of a judgment."""
        if cell_id in self._active_traces:
            trace = self._active_traces.pop(cell_id)
            latency = (time.perf_counter() - trace.start_time) * 1000
            trace.total_latency_ms = latency

            # Maintain Moving Average
            self._history.append(latency)
            if len(self._history) > 100:
                self._history.pop(0)

            if latency > 5000:  # 5 seconds is a 'Cognitive Stroke'
                logger.warning(
                    f"Synaptic Jitter detected! Cell {cell_id} took {latency:.2f}ms"
                )

            return latency
        return None

    def get_stats(self) -> Dict[str, float]:
        """Return global cognitive health metrics."""
        if not self._history:
            return {"avg_latency_ms": 0.0, "jitter": 0.0}

        avg = sum(self._history) / len(self._history)
        return {
            "avg_latency_ms": avg,
            "max_latency_ms": max(self._history),
            "thought_count": len(self._history),
        }
