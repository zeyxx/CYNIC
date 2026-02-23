"""
CYNIC Tier 1 Nervous System — Decision Trace

Component 3 (Reasoning Path): Converts event sequences into executable DAGs.

Every judgment produces a decision trace:
  - Input: what perception triggered it
  - Reasoning: which dogs judged it (votes/scores)
  - Output: verdict, Q-score, confidence
  - Effects: what actions were proposed/accepted/rejected

Pattern: DAG structure (edges = causality), replayable, auditable.
Rolling cap: F(10) = 55 traces (oldest dropped when 56th arrives).

Queryable via:
  - get_trace(judgment_id) — single trace
  - recent_traces(limit=10) — N most recent
  - traces_by_verdict(verdict) — filter by HOWL/WAG/GROWL/BARK
  - trace_stats() — timing/depth/branching metrics
  - replay(trace_id) — re-execute decision path

Enables:
  - Component 4 (LoopClosureValidator): Detects broken DAGs
  - L2 Feedback loop: Claude Code can audit judgment reasoning
  - Meta-cognition: CYNIC analyzes its own decision patterns
"""
from __future__ import annotations

import asyncio
import hashlib
import logging
import sys
import time
from collections import deque
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any, Optional

# Python 3.9 compatibility: StrEnum added in Python 3.11
if sys.version_info >= (3, 11):
    from enum import StrEnum
else:
    class StrEnum(str, Enum):
        """Polyfill for Python <3.11."""
        pass

from cynic.core.formulas import DECISION_TRACE_CAP

logger = logging.getLogger("cynic.nervous.decision_trace")

# φ-derived rolling cap: F(10) = 55 (imported from formulas.py)
TRACE_CAP = DECISION_TRACE_CAP


class DogRole(StrEnum):
    """Role of a dog in judgment."""
    PRIMARY = "primary"        # Initiated judgment
    VOTER = "voter"            # Scored + voted
    WITNESS = "witness"        # Observed but didn't influence
    OVERRIDE = "override"      # Rejected other dogs' votes


@dataclass
class DogVote:
    """Single dog's contribution to judgment."""
    dog_id: str
    role: DogRole
    q_score: float              # Their score (0-100)
    confidence: float           # Their confidence (0-φ⁻¹)
    reasoning: Optional[str] = None  # Short reasoning summary


@dataclass
class TraceNode:
    """Single node in decision trace DAG."""
    node_id: str                # Hash of (judgment_id, phase)
    phase: str                  # PERCEIVE / JUDGE / DECIDE / ACT / LEARN / etc
    component: str              # Which component emitted
    timestamp_ms: float
    duration_ms: float

    # Inputs to this phase
    input_keys: list[str]       # What data entered
    input_sources: list[str]    # Where it came from

    # Processing in this phase
    dog_votes: list[DogVote] = field(default_factory=list)  # If JUDGE phase

    # Outputs from this phase
    output_verdict: Optional[str] = None  # BARK/GROWL/WAG/HOWL
    output_q_score: Optional[float] = None
    output_keys: list[str] = field(default_factory=list)

    # Errors
    is_error: bool = False
    error_message: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["role"] = str(self.phase)
        d["dog_votes"] = [
            {
                "dog_id": v.dog_id,
                "role": str(v.role),
                "q_score": v.q_score,
                "confidence": v.confidence,
                "reasoning": v.reasoning,
            }
            for v in self.dog_votes
        ]
        return d


@dataclass
class DecisionTrace:
    """Complete trace of a judgment decision."""
    trace_id: str               # Hash of judgment_id
    judgment_id: str            # Which judgment this traces
    created_at_ms: float

    # DAG structure
    nodes: list[TraceNode] = field(default_factory=list)
    edges: list[tuple[str, str]] = field(default_factory=list)  # (from_node, to_node)

    # Summary metrics
    total_duration_ms: float = 0.0
    max_depth: int = 0          # Longest path from root to leaf
    branching_factor: float = 0.0  # Avg children per node

    # Final verdict (from leaf node)
    final_verdict: Optional[str] = None
    final_q_score: Optional[float] = None
    final_confidence: Optional[float] = None

    # Anomalies
    has_errors: bool = False
    error_count: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "trace_id": self.trace_id,
            "judgment_id": self.judgment_id,
            "created_at_ms": self.created_at_ms,
            "nodes": [n.to_dict() for n in self.nodes],
            "edges": self.edges,
            "total_duration_ms": self.total_duration_ms,
            "max_depth": self.max_depth,
            "branching_factor": self.branching_factor,
            "final_verdict": self.final_verdict,
            "final_q_score": self.final_q_score,
            "final_confidence": self.final_confidence,
            "has_errors": self.has_errors,
            "error_count": self.error_count,
        }


class DecisionTracer:
    """
    Converts event sequences into executable DAGs.

    Tracks reasoning path for each judgment: inputs → voting → decision → action.
    Thread-safe (asyncio.Lock), rolling buffer with cap F(10)=55.
    """

    def __init__(self):
        self._traces: deque = deque(maxlen=TRACE_CAP)  # Auto-drops oldest
        self._lock = asyncio.Lock()
        self._trace_map: dict[str, DecisionTrace] = {}  # trace_id → trace
        self._judgment_to_trace: dict[str, str] = {}  # judgment_id → trace_id
        self._stats = {
            "total_traced": 0,
            "by_verdict": {},
            "last_updated_ms": 0.0,
            "avg_duration_ms": 0.0,
            "avg_depth": 0.0,
        }

    async def start_trace(
        self,
        judgment_id: str,
        initial_phase: str = "PERCEIVE",
    ) -> str:
        """
        Start a new decision trace for a judgment.

        Returns trace_id.
        """
        async with self._lock:
            timestamp_ms = time.time() * 1000.0

            # Generate trace_id
            trace_id = hashlib.sha256(
                f"{timestamp_ms}:{judgment_id}".encode()
            ).hexdigest()[:16]

            # Create root node (PERCEIVE phase)
            root_node = TraceNode(
                node_id=f"{trace_id}_perceive",
                phase=initial_phase,
                component="perceiver",
                timestamp_ms=timestamp_ms,
                duration_ms=0.0,
                input_keys=[],
                input_sources=[],
            )

            trace = DecisionTrace(
                trace_id=trace_id,
                judgment_id=judgment_id,
                created_at_ms=timestamp_ms,
                nodes=[root_node],
            )

            self._traces.append(trace)
            self._trace_map[trace_id] = trace
            self._judgment_to_trace[judgment_id] = trace_id

            logger.debug(f"Trace started: {trace_id} for judgment {judgment_id}")
            return trace_id

    async def add_node(
        self,
        trace_id: str,
        phase: str,
        component: str,
        duration_ms: float,
        input_keys: list[str],
        input_sources: list[str],
        output_keys: list[str],
        output_verdict: Optional[str] = None,
        output_q_score: Optional[float] = None,
    ) -> str:
        """
        Add a node to the trace DAG.

        Returns node_id.
        """
        async with self._lock:
            trace = self._trace_map.get(trace_id)
            if not trace:
                logger.warning(f"Trace not found: {trace_id}")
                return ""

            timestamp_ms = time.time() * 1000.0
            node_id = f"{trace_id}_{phase.lower()}_{len(trace.nodes)}"

            node = TraceNode(
                node_id=node_id,
                phase=phase,
                component=component,
                timestamp_ms=timestamp_ms,
                duration_ms=duration_ms,
                input_keys=input_keys,
                input_sources=input_sources,
                output_keys=output_keys,
                output_verdict=output_verdict,
                output_q_score=output_q_score,
            )

            # Connect to previous node
            if trace.nodes:
                prev_node = trace.nodes[-1]
                trace.edges.append((prev_node.node_id, node_id))

            trace.nodes.append(node)
            logger.debug(f"Node added to trace {trace_id}: {phase}")

            return node_id

    async def add_dog_votes(
        self,
        trace_id: str,
        phase_index: int,
        dog_votes: list[DogVote],
    ) -> None:
        """Add dog votes to a specific phase node."""
        async with self._lock:
            trace = self._trace_map.get(trace_id)
            if not trace or phase_index >= len(trace.nodes):
                logger.warning(f"Cannot add votes: trace={trace_id}, phase={phase_index}")
                return

            trace.nodes[phase_index].dog_votes = dog_votes

    async def close_trace(
        self,
        trace_id: str,
        final_verdict: Optional[str] = None,
        final_q_score: Optional[float] = None,
        final_confidence: Optional[float] = None,
    ) -> None:
        """
        Finalize a trace (compute metrics, detect anomalies).
        """
        async with self._lock:
            trace = self._trace_map.get(trace_id)
            if not trace:
                return

            trace.final_verdict = final_verdict
            trace.final_q_score = final_q_score
            trace.final_confidence = final_confidence

            # Compute metrics
            if trace.nodes:
                trace.total_duration_ms = sum(n.duration_ms for n in trace.nodes)
                trace.max_depth = self._compute_depth(trace)
                trace.branching_factor = self._compute_branching(trace)
                trace.has_errors = any(n.is_error for n in trace.nodes)
                trace.error_count = sum(1 for n in trace.nodes if n.is_error)

            # Update stats
            self._stats["total_traced"] += 1
            if final_verdict:
                self._stats["by_verdict"][final_verdict] = (
                    self._stats["by_verdict"].get(final_verdict, 0) + 1
                )
            self._stats["last_updated_ms"] = time.time() * 1000.0

            # Running average of duration and depth
            if self._stats["total_traced"] > 0:
                n = self._stats["total_traced"]
                self._stats["avg_duration_ms"] = (
                    (self._stats["avg_duration_ms"] * (n - 1) + trace.total_duration_ms) / n
                )
                self._stats["avg_depth"] = (
                    (self._stats["avg_depth"] * (n - 1) + trace.max_depth) / n
                )

            logger.debug(f"Trace finalized: {trace_id} | verdict={final_verdict}")

    def _compute_depth(self, trace: DecisionTrace) -> int:
        """Longest path in DAG (topological sort)."""
        if not trace.nodes:
            return 0
        if not trace.edges:
            return len(trace.nodes)

        # Build adjacency
        graph: dict[str, list[str]] = {}
        in_degree: dict[str, int] = {}

        for node in trace.nodes:
            graph[node.node_id] = []
            in_degree[node.node_id] = 0

        for src, dst in trace.edges:
            if src in graph:
                graph[src].append(dst)
            if dst in in_degree:
                in_degree[dst] += 1

        # Kahn's algorithm (simplified depth calc)
        depth = {node.node_id: 0 for node in trace.nodes}
        queue = [n.node_id for n in trace.nodes if in_degree[n.node_id] == 0]

        while queue:
            node_id = queue.pop(0)
            for child in graph.get(node_id, []):
                depth[child] = max(depth[child], depth[node_id] + 1)
                in_degree[child] -= 1
                if in_degree[child] == 0:
                    queue.append(child)

        return max(depth.values()) if depth else 1

    def _compute_branching(self, trace: DecisionTrace) -> float:
        """Average children per node."""
        if not trace.nodes:
            return 0.0

        children_count = [0] * len(trace.nodes)
        for src, dst in trace.edges:
            src_idx = next((i for i, n in enumerate(trace.nodes) if n.node_id == src), -1)
            if src_idx >= 0:
                children_count[src_idx] += 1

        non_leaf = sum(1 for c in children_count if c > 0)
        if non_leaf == 0:
            return 1.0

        return sum(children_count) / non_leaf

    async def get_trace(self, trace_id: str) -> Optional[DecisionTrace]:
        """Look up single trace."""
        async with self._lock:
            return self._trace_map.get(trace_id)

    async def get_trace_by_judgment(self, judgment_id: str) -> Optional[DecisionTrace]:
        """Get trace for a judgment."""
        async with self._lock:
            trace_id = self._judgment_to_trace.get(judgment_id)
            if trace_id:
                return self._trace_map.get(trace_id)
            return None

    async def recent_traces(self, limit: int = 10) -> list[DecisionTrace]:
        """Get last N traces (most recent first)."""
        async with self._lock:
            traces = list(self._traces)
            return traces[-limit:][::-1]

    async def traces_by_verdict(self, verdict: str, limit: int = 50) -> list[DecisionTrace]:
        """Get traces with specific verdict."""
        async with self._lock:
            matching = [
                t for t in self._traces
                if t.final_verdict == verdict
            ]
            return matching[-limit:][::-1]

    async def traces_by_component(self, component: str, limit: int = 50) -> list[DecisionTrace]:
        """Get traces that involved a component."""
        async with self._lock:
            matching = [
                t for t in self._traces
                if any(n.component == component for n in t.nodes)
            ]
            return matching[-limit:][::-1]

    async def stats(self) -> dict[str, Any]:
        """Get tracer statistics."""
        async with self._lock:
            return {
                **self._stats,
                "buffer_size": len(self._traces),
                "buffer_cap": TRACE_CAP,
                "unique_verdicts": len(self._stats["by_verdict"]),
            }

    async def snapshot(self) -> dict[str, Any]:
        """Get complete tracer state."""
        async with self._lock:
            return {
                "traces": [t.to_dict() for t in self._traces],
                "stats": {
                    **self._stats,
                    "buffer_size": len(self._traces),
                    "buffer_cap": TRACE_CAP,
                },
            }

    async def clear(self) -> None:
        """Clear all traces (testing)."""
        async with self._lock:
            self._traces.clear()
            self._trace_map.clear()
            self._judgment_to_trace.clear()
            self._stats = {
                "total_traced": 0,
                "by_verdict": {},
                "last_updated_ms": 0.0,
                "avg_duration_ms": 0.0,
                "avg_depth": 0.0,
            }
