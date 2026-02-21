"""
Nervous System API Routes

GET  /nervous/journal/recent        — Last N events
GET  /nervous/journal/by-type       — Events filtered by type
GET  /nervous/journal/by-source     — Events from a component
GET  /nervous/journal/by-category   — Events in a phase
GET  /nervous/journal/time-range    — Events in time window
GET  /nervous/journal/causality     — Causality chain from event
GET  /nervous/journal/errors        — Errors since timestamp
GET  /nervous/journal/stats         — Journal statistics
POST /nervous/journal/clear         — Clear journal (testing only)
"""
from __future__ import annotations

import logging
from fastapi import APIRouter, HTTPException, Depends

from cynic.api.state import CynicOrganism, get_state
from cynic.nervous import EventCategory
from typing import Optional

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/nervous", tags=["nervous"])


@router.get("/journal/recent")
async def get_recent_events(
    limit: int = 10,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get last N events from the journal."""
    try:
        events = await state.event_journal.recent(limit=limit)
        return {
            "events": [e.to_dict() for e in events],
            "count": len(events),
        }
    except Exception as e:
        logger.error(f"Error fetching recent events: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/by-type")
async def get_events_by_type(
    event_type: str,
    limit: int = 50,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get events of a specific type."""
    try:
        events = await state.event_journal.filter_by_type(event_type, limit=limit)
        return {
            "event_type": event_type,
            "events": [e.to_dict() for e in events],
            "count": len(events),
        }
    except Exception as e:
        logger.error(f"Error filtering events by type: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/by-source")
async def get_events_by_source(
    source: str,
    limit: int = 50,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get events from a specific component."""
    try:
        events = await state.event_journal.filter_by_source(source, limit=limit)
        return {
            "source": source,
            "events": [e.to_dict() for e in events],
            "count": len(events),
        }
    except Exception as e:
        logger.error(f"Error filtering events by source: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/by-category")
async def get_events_by_category(
    category: str,
    limit: int = 50,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get events in a category (phase)."""
    try:
        # Validate category
        try:
            cat = EventCategory(category)
        except ValueError:
            raise HTTPException(
                status_code=400,
                detail=f"Invalid category. Valid: {[c.value for c in EventCategory]}"
            )

        events = await state.event_journal.filter_by_category(cat, limit=limit)
        return {
            "category": category,
            "events": [e.to_dict() for e in events],
            "count": len(events),
        }
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error filtering events by category: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/time-range")
async def get_events_in_time_range(
    start_ms: float,
    end_ms: float,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get events within time window [start_ms, end_ms]."""
    try:
        if start_ms > end_ms:
            raise HTTPException(status_code=400, detail="start_ms must be <= end_ms")

        events = await state.event_journal.time_range(start_ms, end_ms)
        return {
            "start_ms": start_ms,
            "end_ms": end_ms,
            "events": [e.to_dict() for e in events],
            "count": len(events),
        }
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error fetching events in time range: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/causality")
async def get_causality_chain(
    event_id: str,
    direction: str = "down",
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Trace causality chain (up=causes, down=effects)."""
    try:
        if direction not in ["up", "down"]:
            raise HTTPException(status_code=400, detail="direction must be 'up' or 'down'")

        chain = await state.event_journal.causality_chain(event_id, direction=direction)
        return {
            "event_id": event_id,
            "direction": direction,
            "chain": [e.to_dict() for e in chain],
            "count": len(chain),
        }
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error tracing causality: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/errors")
async def get_recent_errors(
    since_ms: Optional[float] = None,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get error events since timestamp."""
    try:
        import time
        if since_ms is None:
            # Default: last hour
            since_ms = (time.time() - 3600) * 1000.0

        errors = await state.event_journal.errors_since(since_ms)
        return {
            "since_ms": since_ms,
            "errors": [e.to_dict() for e in errors],
            "count": len(errors),
        }
    except Exception as e:
        logger.error(f"Error fetching error events: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/stats")
async def get_journal_stats(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get journal statistics."""
    try:
        stats = await state.event_journal.stats()
        return {"stats": stats}
    except Exception as e:
        logger.error(f"Error getting journal stats: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/journal/snapshot")
async def get_journal_snapshot(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get complete journal state (for debugging)."""
    try:
        snapshot = await state.event_journal.snapshot()
        return snapshot
    except Exception as e:
        logger.error(f"Error getting journal snapshot: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/journal/clear")
async def clear_journal(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Clear the event journal (testing only - consider adding auth gate)."""
    try:
        await state.event_journal.clear()
        logger.warning("Event journal cleared by API")
        return {"message": "Event journal cleared"}
    except Exception as e:
        logger.error(f"Error clearing journal: {e}")
        raise HTTPException(status_code=500, detail=str(e))


# ════════════════════════════════════════════════════════════════════════════
# DECISION TRACE ENDPOINTS
# ════════════════════════════════════════════════════════════════════════════


@router.get("/trace/recent")
async def get_recent_traces(
    limit: int = 10,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get last N decision traces."""
    try:
        traces = await state.decision_tracer.recent_traces(limit=limit)
        return {
            "traces": [t.to_dict() for t in traces],
            "count": len(traces),
        }
    except Exception as e:
        logger.error(f"Error fetching recent traces: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/{trace_id}")
async def get_trace(
    trace_id: str,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get a specific decision trace by ID."""
    try:
        trace = await state.decision_tracer.get_trace(trace_id)
        if not trace:
            raise HTTPException(status_code=404, detail=f"Trace {trace_id} not found")
        return {"trace": trace.to_dict()}
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error fetching trace: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/judgment/{judgment_id}")
async def get_trace_by_judgment(
    judgment_id: str,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get decision trace for a judgment."""
    try:
        trace = await state.decision_tracer.get_trace_by_judgment(judgment_id)
        if not trace:
            raise HTTPException(
                status_code=404,
                detail=f"No trace for judgment {judgment_id}"
            )
        return {"trace": trace.to_dict()}
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error fetching trace by judgment: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/by-verdict")
async def get_traces_by_verdict(
    verdict: str,
    limit: int = 50,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get traces filtered by final verdict."""
    try:
        traces = await state.decision_tracer.traces_by_verdict(verdict, limit=limit)
        return {
            "verdict": verdict,
            "traces": [t.to_dict() for t in traces],
            "count": len(traces),
        }
    except Exception as e:
        logger.error(f"Error fetching traces by verdict: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/by-component")
async def get_traces_by_component(
    component: str,
    limit: int = 50,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get traces that involved a component."""
    try:
        traces = await state.decision_tracer.traces_by_component(
            component, limit=limit
        )
        return {
            "component": component,
            "traces": [t.to_dict() for t in traces],
            "count": len(traces),
        }
    except Exception as e:
        logger.error(f"Error fetching traces by component: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/stats")
async def get_trace_stats(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get trace statistics."""
    try:
        stats = await state.decision_tracer.stats()
        return {"stats": stats}
    except Exception as e:
        logger.error(f"Error getting trace stats: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/trace/snapshot")
async def get_trace_snapshot(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get complete tracer state (for debugging)."""
    try:
        snapshot = await state.decision_tracer.snapshot()
        return snapshot
    except Exception as e:
        logger.error(f"Error getting trace snapshot: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/trace/clear")
async def clear_traces(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Clear all traces (testing only)."""
    try:
        await state.decision_tracer.clear()
        logger.warning("Decision traces cleared by API")
        return {"message": "Decision traces cleared"}
    except Exception as e:
        logger.error(f"Error clearing traces: {e}")
        raise HTTPException(status_code=500, detail=str(e))


# ════════════════════════════════════════════════════════════════════════════
# LOOP CLOSURE ENDPOINTS
# ════════════════════════════════════════════════════════════════════════════


@router.get("/closure/open")
async def get_open_cycles(
    max_age_ms: Optional[float] = None,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get cycles currently in progress."""
    try:
        cycles = await state.loop_closure_validator.get_open_cycles(max_age_ms)
        return {
            "open_cycles": [c.to_dict() for c in cycles],
            "count": len(cycles),
        }
    except Exception as e:
        logger.error(f"Error fetching open cycles: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/closure/stalled")
async def get_stalled_phases(
    threshold_ms: float = 5000,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get cycles with stalled phases."""
    try:
        stalled = await state.loop_closure_validator.get_stalled_phases(threshold_ms)
        return {
            "threshold_ms": threshold_ms,
            "stalled_cycles": [c.to_dict() for c in stalled],
            "count": len(stalled),
        }
    except Exception as e:
        logger.error(f"Error fetching stalled phases: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/closure/orphans")
async def get_orphan_judgments(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get judgments never acted upon."""
    try:
        orphans = await state.loop_closure_validator.get_orphan_judgments()
        return {
            "orphan_judgments": [c.to_dict() for c in orphans],
            "count": len(orphans),
        }
    except Exception as e:
        logger.error(f"Error fetching orphan judgments: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/closure/recent")
async def get_recent_closures(
    limit: int = 10,
    complete_only: bool = False,
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get recent cycle closures."""
    try:
        closures = await state.loop_closure_validator.recent_closures(
            limit=limit,
            include_complete_only=complete_only,
        )
        return {
            "recent_closures": [c.to_dict() for c in closures],
            "count": len(closures),
            "complete_only": complete_only,
        }
    except Exception as e:
        logger.error(f"Error fetching recent closures: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/closure/stats")
async def get_closure_stats(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get loop closure statistics."""
    try:
        stats = await state.loop_closure_validator.stats()
        return {"stats": stats}
    except Exception as e:
        logger.error(f"Error getting closure stats: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/closure/snapshot")
async def get_closure_snapshot(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Get complete loop closure state."""
    try:
        snapshot = await state.loop_closure_validator.snapshot()
        return snapshot
    except Exception as e:
        logger.error(f"Error getting closure snapshot: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/closure/clear")
async def clear_closures(
    state: CynicOrganism = Depends(get_state),
) -> dict:
    """Clear all closure records (testing only)."""
    try:
        await state.loop_closure_validator.clear()
        logger.warning("Loop closure records cleared by API")
        return {"message": "Loop closure records cleared"}
    except Exception as e:
        logger.error(f"Error clearing closures: {e}")
        raise HTTPException(status_code=500, detail=str(e))
