"""ConsciousnessService — 7-Layer HUB aggregating all consciousness layers.

This service sits on top of EcosystemObserver and CynicOrganism,
aggregating all 7 consciousness layers into a unified interface:

1. Ecosystem State — cross-bus event topology
2. Decision Trace — full path of decision through guardrails
3. Topology Consciousness — architecture consciousness (L0 system)
4. Guardrail Decisions — decisions made by immune system
5. Self Awareness — organism's meta-cognition (kernel_mirror insights)
6. Nervous System Audit — full audit trail of all components
7. (Reserved for future) Transcendence gates
"""

from __future__ import annotations

import logging
import time
from datetime import datetime
from typing import Any, Optional

from cynic.api.services.ecosystem_observer import EcosystemObserver
from cynic.api.state import get_app_container
from cynic.nervous import EventJournal, DecisionTracer, ServiceStateRegistry

logger = logging.getLogger(__name__)


class ConsciousnessService:
    """
    HUB for 7-layer consciousness aggregation.

    Each method queries nervous system components + aggregates results.

    Public methods:
    - get_ecosystem_state() → Layer 1: cross-bus event topology
    - get_decision_trace(decision_id) → Layer 2: guardrail decision path
    - get_topology_consciousness() → Layer 3: L0 architecture consciousness
    - get_guardrail_decisions(limit) → Layer 4: immune system decisions
    - get_self_awareness() → Layer 5: kernel_mirror insights
    - get_nervous_system_audit(limit) → Layer 6: full audit trail
    """

    def __init__(self):
        """Initialize ConsciousnessService (lazy-load components on demand)."""
        self._ecosystem_observer: Optional[EcosystemObserver] = None

    async def _get_ecosystem_observer(self) -> EcosystemObserver:
        """Get or create EcosystemObserver (lazy-load)."""
        if self._ecosystem_observer is None:
            try:
                container = get_app_container()
                organism = container.organism

                # Get nervous system components from organism
                journal = organism.event_journal
                decision_tracer = organism.decision_tracer
                registry = organism.service_registry

                self._ecosystem_observer = EcosystemObserver(
                    journal=journal,
                    decision_trace=decision_tracer,
                    handlers_registry=registry,
                )
            except RuntimeError:
                # AppContainer not initialized — create bare observer for testing
                self._ecosystem_observer = EcosystemObserver(
                    journal=EventJournal(),
                    decision_trace=DecisionTracer(),
                    handlers_registry=ServiceStateRegistry(),
                )

        return self._ecosystem_observer

    async def get_ecosystem_state(self) -> dict[str, Any]:
        """
        Get cross-bus event topology (Layer 1).

        Returns aggregated events from all 3 buses:
        - CORE: Core judgment events
        - AUTOMATION: Scheduler + automation events
        - AGENT: Agent-driven events

        Returns:
            Dict with core_events, automation_events, agent_events, timestamp
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Get recent events from journal
            all_events = await organism.event_journal.recent(limit=100)

            # Categorize by bus (infer from source or event type)
            core_events = [
                e.to_dict() for e in all_events
                if e.source in ["SAGE", "JUDGE", "ORACLE"]
            ]
            automation_events = [
                e.to_dict() for e in all_events
                if e.source in ["SCHEDULER", "MACRO", "AUTOMATION"]
            ]
            agent_events = [
                e.to_dict() for e in all_events
                if e.source in ["AGENT", "API", "PERCEIVE"]
            ]

            return {
                "core_events": core_events[-10:],
                "automation_events": automation_events[-10:],
                "agent_events": agent_events[-10:],
                "timestamp": datetime.now().timestamp(),
            }
        except RuntimeError:
            # AppContainer not initialized — return stub
            return {
                "core_events": [],
                "automation_events": [],
                "agent_events": [],
                "timestamp": datetime.now().timestamp(),
            }

    async def get_decision_trace(self, decision_id: str) -> Optional[dict[str, Any]]:
        """
        Get full path of decision through guardrails (Layer 2).

        Returns the complete decision trace including:
        - power_limiter check
        - alignment_checker check
        - audit_trail recording
        - human_gate approval
        - decision_validator execution

        Args:
            decision_id: ID of decision to trace

        Returns:
            Dict with decision_id, timestamp, and path array of stages
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Query decision trace from nervous system
            trace = await organism.decision_tracer.get_trace_by_judgment(decision_id)

            if not trace:
                # Return empty trace
                return {
                    "decision_id": decision_id,
                    "timestamp": datetime.now().timestamp(),
                    "path": [],
                }

            # Extract stages from trace nodes
            path = []
            for node in trace.nodes:
                stage_name = node.phase.lower() if hasattr(node.phase, 'lower') else str(node.phase)
                path.append({
                    "stage": stage_name,
                    "component": node.component,
                    "verdict": node.output_verdict or "pending",
                    "reason": f"processed_by_{node.component}",
                    "duration_ms": node.duration_ms,
                })

            return {
                "decision_id": decision_id,
                "timestamp": datetime.now().timestamp(),
                "path": path,
            }
        except RuntimeError:
            # AppContainer not initialized
            return {
                "decision_id": decision_id,
                "timestamp": datetime.now().timestamp(),
                "path": [],
            }

    async def get_topology_consciousness(self) -> dict[str, Any]:
        """
        Get architecture consciousness (Layer 3 - L0 system).

        Returns info about source topology, detected changes, and convergence validation.

        Returns:
            Dict with source_changes_detected, topology_deltas_computed, convergence_validations
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Get convergence validator if available
            convergence = getattr(organism.senses, 'convergence_validator', None)

            announcements = []
            outcomes = []
            if convergence:
                announcements = getattr(convergence, 'announcements', [])
                outcomes = getattr(convergence, 'outcomes', [])

            return {
                "source_changes_detected": len(announcements),
                "topology_deltas_computed": len([
                    a for a in announcements
                    if isinstance(a, dict) and a.get("type") == "topology_change"
                ]),
                "convergence_validations": {
                    "total_announced": len(announcements),
                    "verified": len([
                        o for o in outcomes
                        if isinstance(o, dict) and o.get("verified")
                    ]),
                    "pending": len([
                        a for a in announcements
                        if a not in [o.get("announcement") for o in outcomes if isinstance(o, dict)]
                    ]),
                },
                "recent_changes": announcements[-5:] if announcements else [],
                "timestamp": datetime.now().timestamp(),
            }
        except RuntimeError:
            # AppContainer not initialized
            return {
                "source_changes_detected": 0,
                "topology_deltas_computed": 0,
                "convergence_validations": {
                    "total_announced": 0,
                    "verified": 0,
                    "pending": 0,
                },
                "recent_changes": [],
                "timestamp": datetime.now().timestamp(),
            }

    async def get_guardrail_decisions(self, limit: int = 20) -> list[dict[str, Any]]:
        """
        Get decisions made by guardrails (Layer 4).

        Returns recent decisions from the immune system (power_limiter, alignment_checker, etc).

        Args:
            limit: Maximum number of decisions to return

        Returns:
            List of decision dicts with guardrail_type, decision, reason, timestamp
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Get audit trail from immune system
            audit_trail = getattr(organism.cognition, 'audit_trail', None)

            decisions = []
            if audit_trail and hasattr(audit_trail, 'get_recent_decisions'):
                decisions = await audit_trail.get_recent_decisions(limit=limit)

            return [
                {
                    "guardrail_type": d.get("type", "unknown"),
                    "decision": d.get("decision", "unknown"),
                    "reason": d.get("reason", ""),
                    "timestamp": d.get("timestamp", datetime.now().timestamp()),
                }
                for d in decisions
            ]
        except RuntimeError:
            # AppContainer not initialized
            return []

    async def get_self_awareness(self) -> dict[str, Any]:
        """
        Get organism's meta-cognition (Layer 5 - kernel_mirror insights).

        Returns introspective observations, insights, and improvement proposals
        from the kernel_mirror component.

        Returns:
            Dict with kernel_observations, meta_insights, improvement_proposals, self_assessment
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Get kernel_mirror from memory core
            kernel_mirror = organism.kernel_mirror

            observations = getattr(kernel_mirror, 'observations', [])
            insights = getattr(kernel_mirror, 'insights', [])
            proposals = getattr(kernel_mirror, 'proposals', [])

            return {
                "kernel_observations": observations[-10:] if observations else [],
                "meta_insights": insights[-5:] if insights else [],
                "improvement_proposals": proposals[-3:] if proposals else [],
                "self_assessment": {
                    "total_observations": len(observations) if observations else 0,
                    "insight_count": len(insights) if insights else 0,
                    "proposal_count": len(proposals) if proposals else 0,
                },
                "timestamp": datetime.now().timestamp(),
            }
        except RuntimeError:
            # AppContainer not initialized
            return {
                "kernel_observations": [],
                "meta_insights": [],
                "improvement_proposals": [],
                "self_assessment": {
                    "total_observations": 0,
                    "insight_count": 0,
                    "proposal_count": 0,
                },
                "timestamp": datetime.now().timestamp(),
            }

    async def get_nervous_system_audit(self, limit: int = 100) -> dict[str, Any]:
        """
        Get nervous system audit trail (Layer 6).

        Returns aggregated audit data from:
        - EventJournal: all events in sequence
        - DecisionTracer: reasoning paths
        - LoopClosureValidator: feedback loop verification

        Args:
            limit: Maximum number of events to return

        Returns:
            Dict with all_events, decision_reasons, loop_integrity_checks, counts, timestamp
        """
        try:
            container = get_app_container()
            organism = container.organism

            # Get components from nervous system
            event_journal = organism.event_journal
            decision_tracer = organism.decision_tracer
            loop_validator = organism.loop_closure_validator

            # Query all components
            all_events = await event_journal.recent(limit=limit)
            recent_traces = await decision_tracer.recent_traces(limit=20)
            loop_checks = getattr(loop_validator, 'recent_checks', [])

            # Format events
            formatted_events = [
                {
                    "type": e.event_type if isinstance(e.event_type, str) else getattr(e.event_type, 'name', str(e.event_type)),
                    "timestamp": getattr(e, 'timestamp_ms', datetime.now().timestamp() * 1000),
                    "source": getattr(e, 'source', 'unknown'),
                }
                for e in all_events
            ]

            # Format decision reasons
            formatted_reasons = [
                {
                    "judgment_id": t.judgment_id,
                    "phase_count": len(t.nodes) if hasattr(t, 'nodes') else 0,
                    "final_verdict": t.final_verdict if hasattr(t, 'final_verdict') else "pending",
                    "created_at_ms": t.created_at_ms if hasattr(t, 'created_at_ms') else 0,
                }
                for t in recent_traces
            ]

            # Format loop checks
            formatted_checks = [
                {
                    "check_type": c.get("type", "unknown"),
                    "result": c.get("result", "unknown"),
                    "timestamp": c.get("timestamp", datetime.now().timestamp()),
                }
                for c in loop_checks if isinstance(c, dict)
            ]

            return {
                "all_events": formatted_events,
                "decision_reasons": formatted_reasons[-10:],
                "loop_integrity_checks": formatted_checks,
                "event_count": len(all_events),
                "decision_count": len(recent_traces),
                "timestamp": datetime.now().timestamp(),
            }
        except RuntimeError:
            # AppContainer not initialized
            return {
                "all_events": [],
                "decision_reasons": [],
                "loop_integrity_checks": [],
                "event_count": 0,
                "decision_count": 0,
                "timestamp": datetime.now().timestamp(),
            }
