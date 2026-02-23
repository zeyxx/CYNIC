"""Tests for ConsciousnessService — 7-layer HUB aggregating all consciousness layers."""

import asyncio
import pytest
from datetime import datetime
from cynic.api.services.consciousness_service import ConsciousnessService


@pytest.fixture
def service():
    """Create fresh ConsciousnessService for tests."""
    return ConsciousnessService()


@pytest.mark.asyncio
async def test_get_ecosystem_state(service):
    """Test Layer 1: cross-bus event topology."""
    ecosystem = await service.get_ecosystem_state()

    assert isinstance(ecosystem, dict)
    assert "core_events" in ecosystem
    assert "automation_events" in ecosystem
    assert "agent_events" in ecosystem
    assert "timestamp" in ecosystem
    assert isinstance(ecosystem["core_events"], list)
    assert isinstance(ecosystem["automation_events"], list)
    assert isinstance(ecosystem["agent_events"], list)


@pytest.mark.asyncio
async def test_get_decision_trace(service):
    """Test Layer 2: full path of decision through guardrails."""
    trace = await service.get_decision_trace(decision_id="test_123")

    assert isinstance(trace, dict)
    assert "decision_id" in trace
    assert "timestamp" in trace
    assert "path" in trace
    assert isinstance(trace["path"], list)
    # Path should have stages for guardrails
    if len(trace["path"]) > 0:
        stage = trace["path"][0]
        assert "stage" in stage
        assert "verdict" in stage
        assert "reason" in stage


@pytest.mark.asyncio
async def test_get_topology_consciousness(service):
    """Test Layer 3: architecture consciousness (L0 system)."""
    topo = await service.get_topology_consciousness()

    assert isinstance(topo, dict)
    assert "source_changes_detected" in topo
    assert "topology_deltas_computed" in topo
    assert "convergence_validations" in topo
    assert "recent_changes" in topo
    assert "timestamp" in topo
    assert isinstance(topo["recent_changes"], list)
    assert isinstance(topo["convergence_validations"], dict)


@pytest.mark.asyncio
async def test_get_guardrail_decisions(service):
    """Test Layer 4: decisions made by guardrails."""
    decisions = await service.get_guardrail_decisions(limit=10)

    assert isinstance(decisions, list)
    # Each decision should have these fields
    for d in decisions:
        assert isinstance(d, dict)
        assert "guardrail_type" in d
        assert "decision" in d  # "allow" | "block" | "require_approval"
        assert "reason" in d
        assert "timestamp" in d


@pytest.mark.asyncio
async def test_get_self_awareness(service):
    """Test Layer 5: organism's meta-cognition (kernel_mirror insights)."""
    awareness = await service.get_self_awareness()

    assert isinstance(awareness, dict)
    assert "kernel_observations" in awareness
    assert "meta_insights" in awareness
    assert "improvement_proposals" in awareness
    assert "self_assessment" in awareness
    assert "timestamp" in awareness
    assert isinstance(awareness["kernel_observations"], list)
    assert isinstance(awareness["meta_insights"], list)
    assert isinstance(awareness["improvement_proposals"], list)
    assert isinstance(awareness["self_assessment"], dict)


@pytest.mark.asyncio
async def test_get_nervous_system_audit(service):
    """Test Layer 6: nervous system audit trail."""
    audit = await service.get_nervous_system_audit(limit=50)

    assert isinstance(audit, dict)
    assert "all_events" in audit
    assert "decision_reasons" in audit
    assert "loop_integrity_checks" in audit
    assert "event_count" in audit
    assert "decision_count" in audit
    assert "timestamp" in audit
    assert isinstance(audit["all_events"], list)
    assert isinstance(audit["decision_reasons"], list)
    assert isinstance(audit["loop_integrity_checks"], list)
    # Each event should have type and timestamp
    for e in audit["all_events"]:
        assert isinstance(e, dict)
        assert "type" in e
        assert "timestamp" in e
