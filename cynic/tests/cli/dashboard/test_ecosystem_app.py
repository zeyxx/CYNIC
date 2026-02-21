"""TDD Tests for EcosystemDashboardApp orchestrator."""
import pytest
from cynic.cli.dashboard_orchestrator.ecosystem_app import EcosystemDashboardApp


def test_ecosystem_app_init():
    """EcosystemDashboardApp initializes with all 7 layers."""
    app = EcosystemDashboardApp()
    assert app is not None
    assert app.ecosystem is not None
    assert app.perception is not None
    assert app.topology is not None
    assert app.nervous is not None
    assert app.guardrails is not None
    assert app.self_aware is not None


def test_ecosystem_app_dispatch_snapshot():
    """App dispatches snapshot to all 7 layers."""
    app = EcosystemDashboardApp()

    snapshot = {
        "core_events": [{}, {}],
        "automation_events": [{}],
        "agent_events": [],
        "perception_sources": {},
        "topology_consciousness": {"source_changes_detected": 5},
        "nervous_system_audit": {"event_count": 42, "decision_count": 10},
        "guardrail_decisions": [{"decision": "allow"}, {"decision": "block"}],
        "self_awareness": {"kernel_observations": [1, 2, 3]},
    }

    app._dispatch_snapshot(snapshot)

    assert app.ecosystem.core_count == 2
    assert app.perception.git_status is not None
    assert app.topology.source_changes == 5
    assert app.nervous.event_count == 42
    assert app.guardrails.allow_count == 1
    assert app.guardrails.block_count == 1
    assert app.self_aware.observation_count == 3
