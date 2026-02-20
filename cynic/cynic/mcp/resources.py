"""
CYNIC MCP Resources — Expose CYNIC knowledge to Claude Code

Claude Code (via SDK) can query:
  1. Similar judgments (by pattern)
  2. Decision traces (for reasoning audit)
  3. Loop closure status (is CYNIC stuck?)
  4. Learned patterns (what does CYNIC know?)

These resources bridge L2 feedback loop:
  Claude Code → /ws/sdk → MCP resources → CYNIC state queries

MCP protocol: text/uri-list + application/json payloads
"""
from __future__ import annotations

import json
import time
from typing import Any, Optional


class MCPResourceManager:
    """Manages MCP resource queries against CYNIC state."""

    def __init__(self, state):
        """Initialize with app state."""
        self.state = state

    async def get_similar_judgments(
        self,
        query_q_score: float,
        query_verdict: str,
        limit: int = 10,
    ) -> dict[str, Any]:
        """
        Find similar past judgments.

        Resource: /mcp/judgments/similar?q_score=75&verdict=WAG&limit=10
        """
        try:
            # Get recent traces
            all_traces = await self.state.decision_tracer.recent_traces(limit=100)

            # Filter by verdict
            matching = [
                t for t in all_traces
                if t.final_verdict == query_verdict
            ]

            # Score by proximity to query_q_score
            scored = [
                (t, abs(t.final_q_score - query_q_score) if t.final_q_score else 999)
                for t in matching
            ]

            # Sort by proximity
            scored.sort(key=lambda x: x[1])
            results = [t.to_dict() for t, _ in scored[:limit]]

            return {
                "query": {
                    "q_score": query_q_score,
                    "verdict": query_verdict,
                    "limit": limit,
                },
                "results": results,
                "count": len(results),
            }
        except Exception as e:
            return {"error": str(e), "results": []}

    async def get_judgment_reasoning(
        self,
        judgment_id: str,
    ) -> dict[str, Any]:
        """
        Get full reasoning for a judgment (DAG + dog votes).

        Resource: /mcp/judgments/{judgment_id}/reasoning
        """
        try:
            # Get trace
            trace = await self.state.decision_tracer.get_trace_by_judgment(judgment_id)
            if not trace:
                return {
                    "error": f"No trace found for judgment {judgment_id}",
                    "judgment_id": judgment_id,
                }

            # Extract reasoning from JUDGE phase
            judge_phase = next(
                (p for p in trace.nodes if p.phase == "JUDGE"),
                None,
            )

            reasoning = {
                "judgment_id": judgment_id,
                "trace_id": trace.trace_id,
                "final_verdict": trace.final_verdict,
                "final_q_score": trace.final_q_score,
                "final_confidence": trace.final_confidence,
                "dag_depth": trace.max_depth,
                "dag_branching": trace.branching_factor,
                "total_duration_ms": trace.total_duration_ms,
            }

            if judge_phase:
                reasoning["dog_votes"] = [
                    {
                        "dog_id": v.dog_id,
                        "role": str(v.role),
                        "q_score": v.q_score,
                        "confidence": v.confidence,
                        "reasoning": v.reasoning,
                    }
                    for v in judge_phase.dog_votes
                ]

            reasoning["trace"] = trace.to_dict()

            return reasoning
        except Exception as e:
            return {"error": str(e), "judgment_id": judgment_id}

    async def get_loop_status(self) -> dict[str, Any]:
        """
        Get current feedback loop status (is CYNIC stuck?).

        Resource: /mcp/loops/status
        """
        try:
            stats = await self.state.loop_closure_validator.stats()
            open_cycles = await self.state.loop_closure_validator.get_open_cycles()
            stalled = await self.state.loop_closure_validator.get_stalled_phases()
            orphans = await self.state.loop_closure_validator.get_orphan_judgments()

            return {
                "timestamp_ms": time.time() * 1000.0,
                "stats": stats,
                "health": {
                    "open_cycles": len(open_cycles),
                    "stalled_phases": len(stalled),
                    "orphan_judgments": len(orphans),
                    "completion_rate": stats.get("completion_rate_percent", 0),
                    "is_healthy": (
                        len(stalled) == 0 and len(orphans) == 0 and len(open_cycles) < 100
                    ),
                },
                "alerts": {
                    "stalled": [c.judgment_id for c in stalled],
                    "orphaned": [c.judgment_id for c in orphans],
                },
            }
        except Exception as e:
            return {"error": str(e)}

    async def get_learned_patterns(
        self,
        limit: int = 20,
    ) -> dict[str, Any]:
        """
        Get what CYNIC has learned (verdict distribution, dog performance, etc).

        Resource: /mcp/learning/patterns?limit=20
        """
        try:
            # Get recent traces
            all_traces = await self.state.decision_tracer.recent_traces(limit=limit)

            # Analyze patterns
            verdict_counts = {}
            dog_scores = {}
            phase_durations = {}

            for trace in all_traces:
                # Verdict distribution
                if trace.final_verdict:
                    verdict_counts[trace.final_verdict] = (
                        verdict_counts.get(trace.final_verdict, 0) + 1
                    )

                # Dog performance (from traces)
                for node in trace.nodes:
                    if node.dog_votes:
                        for vote in node.dog_votes:
                            if vote.dog_id not in dog_scores:
                                dog_scores[vote.dog_id] = []
                            dog_scores[vote.dog_id].append(vote.q_score)

            # Average scores
            dog_stats = {}
            for dog_id, scores in dog_scores.items():
                dog_stats[dog_id] = {
                    "avg_q_score": sum(scores) / len(scores),
                    "judgments": len(scores),
                }

            return {
                "analyzed_traces": len(all_traces),
                "verdict_distribution": verdict_counts,
                "dog_performance": dog_stats,
                "patterns": {
                    "most_common_verdict": max(
                        verdict_counts.items(),
                        key=lambda x: x[1],
                    )[0]
                    if verdict_counts
                    else None,
                    "avg_q_score": (
                        sum(t.final_q_score for t in all_traces if t.final_q_score)
                        / len([t for t in all_traces if t.final_q_score])
                        if any(t.final_q_score for t in all_traces)
                        else None
                    ),
                },
            }
        except Exception as e:
            return {"error": str(e)}

    async def get_event_stream(
        self,
        since_ms: Optional[float] = None,
        limit: int = 50,
    ) -> dict[str, Any]:
        """
        Get recent event stream (for live monitoring).

        Resource: /mcp/events/recent?since_ms=1234567890&limit=50
        """
        try:
            if since_ms is None:
                # Default: last 5 minutes
                since_ms = (time.time() - 300) * 1000.0

            events = await self.state.event_journal.time_range(
                since_ms,
                time.time() * 1000.0,
            )

            # Limit results
            events = events[-limit:]

            return {
                "since_ms": since_ms,
                "events": [
                    {
                        "event_id": e.event_id,
                        "type": e.event_type,
                        "source": e.source,
                        "timestamp_ms": e.timestamp_ms,
                        "category": str(e.category),
                    }
                    for e in events
                ],
                "count": len(events),
            }
        except Exception as e:
            return {"error": str(e)}

    async def get_resource(self, uri: str) -> dict[str, Any]:
        """
        Route MCP resource URIs to handlers.

        Supported URIs:
          /mcp/judgments/similar
          /mcp/judgments/{judgment_id}/reasoning
          /mcp/loops/status
          /mcp/learning/patterns
          /mcp/events/recent
        """
        try:
            # Parse URI
            if uri.startswith("/mcp/judgments/similar"):
                # Extract query params (simplified)
                return await self.get_similar_judgments(
                    query_q_score=75.0,
                    query_verdict="WAG",
                    limit=10,
                )

            elif uri.startswith("/mcp/judgments/") and uri.endswith("/reasoning"):
                # Extract judgment_id
                judgment_id = uri.split("/")[3]
                return await self.get_judgment_reasoning(judgment_id)

            elif uri == "/mcp/loops/status":
                return await self.get_loop_status()

            elif uri.startswith("/mcp/learning/patterns"):
                return await self.get_learned_patterns(limit=20)

            elif uri.startswith("/mcp/events/recent"):
                return await self.get_event_stream(limit=50)

            else:
                return {"error": f"Unknown resource: {uri}"}

        except Exception as e:
            return {"error": str(e), "uri": uri}


def create_mcp_resources(state) -> MCPResourceManager:
    """Factory function for MCP resource manager."""
    return MCPResourceManager(state)
