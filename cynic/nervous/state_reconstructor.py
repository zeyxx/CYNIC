"""
StateReconstructor  Combines journal, traces, and loop closures for audit.

Answers two questions:
  1. audit_decision(judgment_id)  full causal record for one judgment
  2. events_in_window(start_ms, end_ms)  summary of all activity in a time window

NOTE: EventJournal stores payload_keys only (not values). Reconstruction
means "what events happened, in what order, with what causality" 
not full value replay.
"""
from __future__ import annotations

from cynic.kernel.core.formulas import HISTORY_REPLAY_BATCH
from cynic.nervous.decision_trace import DecisionTracer
from cynic.nervous.event_journal import EventJournal
from cynic.nervous.loop_closure import LoopClosureValidator


class StateReconstructor:
    """
    Audit service  combines journal, decision traces, and loop closure state.
    Thread-safe: delegates to async-safe sub-components.
    """

    def __init__(
        self,
        journal: EventJournal,
        tracer: DecisionTracer,
        validator: LoopClosureValidator,
    ) -> None:
        self._journal = journal
        self._tracer = tracer
        self._validator = validator

    async def audit_decision(self, judgment_id: str) -> dict:
        """
        Full causal audit for a single judgment.

        Returns:
            judgment_id: str
            trace: dict | None          DecisionTrace.to_dict()
            replay: list[dict]          nodes in topological order
            journal_context: list[dict] journal events in the trace time window
            errors: list[dict]          error entries from journal_context
            loop_stalled: bool          did this cycle stall?
            loop_orphan: bool           no ACT phase recorded?
        """
        trace = await self._tracer.get_trace_by_judgment(judgment_id)
        replay = await self._tracer.replay_by_judgment(judgment_id)

        journal_events = []
        errors = []
        if trace:
            # 1 s around the trace window
            start_ms = trace.created_at_ms - 1000.0
            end_ms = trace.created_at_ms + trace.total_duration_ms + 1000.0
            journal_events = await self._journal.time_range(start_ms, end_ms)
            errors = [e for e in journal_events if e.is_error]

        stalled = await self._validator.get_stalled_phases(threshold_ms=0)
        orphans = await self._validator.get_orphan_judgments()

        stalled_ids = {s.judgment_id for s in stalled}
        orphan_ids = {o.judgment_id for o in orphans}

        return {
            "judgment_id": judgment_id,
            "trace": trace.to_dict() if trace else None,
            "replay": replay,
            "journal_context": [e.to_dict() for e in journal_events],
            "errors": [e.to_dict() for e in errors],
            "loop_stalled": judgment_id in stalled_ids,
            "loop_orphan": judgment_id in orphan_ids,
        }

    async def events_in_window(self, start_ms: float, end_ms: float) -> dict:
        """
        Summary of all activity in a time window.

        Returns:
            start_ms, end_ms, duration_ms
            event_count: int
            events: list[dict]          JournalEntry.to_dict()
            traces: list[dict]          DecisionTrace.to_dict() started in window
            error_count: int
        """
        journal_events = await self._journal.time_range(start_ms, end_ms)
        recent = await self._tracer.recent_traces(limit=HISTORY_REPLAY_BATCH)
        window_traces = [
            t for t in recent if start_ms <= t.created_at_ms <= end_ms
        ]

        return {
            "start_ms": start_ms,
            "end_ms": end_ms,
            "duration_ms": end_ms - start_ms,
            "event_count": len(journal_events),
            "events": [e.to_dict() for e in journal_events],
            "traces": [t.to_dict() for t in window_traces],
            "error_count": sum(1 for e in journal_events if e.is_error),
        }
