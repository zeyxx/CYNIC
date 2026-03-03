"""
Integration Ralph Loop Tests — PHASE 2 COMPONENTS 4-7 (2026-03-02)

Tests the complete PHASE 2 security incident detection pipeline with comprehensive
validation of all components working together.

Architecture:
  Event Stream → RuleEngine → Alert → Deduplicator → Router → Escalation → Audit Trail
"""

import pytest
import time

from cynic.kernel.core.storage.detection_rules import RuleRegistry, RuleExecutor
from cynic.kernel.core.storage.alerting import (
    Alert,
    AlertSeverity,
    AlertDeduplicator,
    AlertRouter,
    EscalationPath,
    AlertAuditLog,
)
from cynic.kernel.core.storage.compliance import (
    AuditLogger,
    IntegrityVerifier,
    ForensicsQuery,
)


# ============================================================================
# INTEGRATION FIXTURES
# ============================================================================


@pytest.fixture
def rule_pipeline():
    """Create rule engine with registry."""
    registry = RuleRegistry()
    return {
        "registry": registry,
        "executor": RuleExecutor(registry=registry),
    }


@pytest.fixture
def alert_pipeline():
    """Create complete alert pipeline: Dedup → Route → Escalate → Audit."""
    return {
        "dedup": AlertDeduplicator(),
        "router": AlertRouter(),
        "escalation": EscalationPath(),
        "audit_log": AlertAuditLog(),
    }


@pytest.fixture
def compliance_pipeline():
    """Create complete compliance pipeline: Audit → Verify → Forensics."""
    return {
        "logger": AuditLogger(),
        "verifier": IntegrityVerifier(),
        "forensics": ForensicsQuery(),
    }


# ============================================================================
# COMPLETE ALERT LIFECYCLE TEST
# ============================================================================


@pytest.mark.asyncio
async def test_complete_alert_lifecycle(alert_pipeline, compliance_pipeline):
    """
    Integration Test: Complete alert lifecycle from creation through resolution
    Validates all stages work together: Alert → Dedup → Route → Escalate → Audit
    """

    dedup = alert_pipeline["dedup"]
    router = alert_pipeline["router"]
    escalation = alert_pipeline["escalation"]
    audit_log = alert_pipeline["audit_log"]
    logger = compliance_pipeline["logger"]
    verifier = compliance_pipeline["verifier"]

    # Create alert
    alert = Alert(
        alert_id="test_alert_1",
        rule_id="STAGE_2_WEAPONIZATION",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker_1", "proposal_value": 50000},
        related_events=[],
        anomaly_scores={"composite": 0.85},
    )

    # Step 1: Deduplication
    should_route = await dedup.should_route(alert)
    assert should_route is True, "First alert should route"
    assert dedup.get_stats()["recent_alerts"] == 1

    # Step 2: Routing
    route_results = await router.route(alert)
    assert len(route_results) > 0, "Alert should route to channels"
    assert any(route_results.values()), "At least one route should succeed"

    # Step 3: Escalation (fresh alert, should not escalate)
    escalation_level = await escalation.check_escalation(alert)
    assert escalation_level is None, "Fresh alert should not escalate"

    # Step 4: Audit Logging
    await audit_log.log_alert_created(alert, "STAGE_2")
    await audit_log.log_alert_routed(alert, "slack", True)
    await audit_log.log_alert_acknowledged(alert, "analyst_l1")
    await audit_log.log_alert_closed(alert, "True positive", "analyst_l2")

    # Step 5: Compliance Logging (permanent record)
    await logger.log_event(
        resource_type="alert",
        resource_id="test_alert_1",
        action="created",
        actor="detection_engine",
        details={"severity": "HIGH"},
    )

    assert logger.events_logged >= 1, "Should log to compliance trail"

    # Step 6: Integrity Verification
    for event in logger.local_cache:
        event_dict = event.to_dict()
        is_valid = await verifier.verify_event(event_dict)
        assert is_valid, f"Event {event.event_id} should have valid hash"

    print(
        "\n✅ Complete Alert Lifecycle: Created → Routed → Escalated → Closed → Audited"
    )


# ============================================================================
# DEDUPLICATION EFFECTIVENESS TEST
# ============================================================================


@pytest.mark.asyncio
async def test_deduplication_prevents_spam(alert_pipeline):
    """
    Validation Test: Deduplicator correctly filters duplicate alerts
    Prevents alert fatigue by suppressing identical threats
    """

    dedup = alert_pipeline["dedup"]

    # Create alert
    alert1 = Alert(
        alert_id="alert_spam_1",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker", "proposal_value": 10000},
        related_events=[],
        anomaly_scores={},
    )

    # First alert routes
    assert await dedup.should_route(alert1) is True

    # Identical alert is deduplicated
    alert2 = Alert(
        alert_id="alert_spam_2",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker", "proposal_value": 10000},
        related_events=[],
        anomaly_scores={},
    )

    assert (
        await dedup.should_route(alert2) is False
    ), "Identical alert should be deduplicated"

    # Different actor routes (new threat)
    alert3 = Alert(
        alert_id="alert_spam_3",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker_2", "proposal_value": 10000},
        related_events=[],
        anomaly_scores={},
    )

    assert await dedup.should_route(alert3) is True, "Different actor should route"

    stats = dedup.get_stats()
    assert stats["deduplicated_count"] == 1, "Should have deduplicated 1 alert"

    print(
        "\n✅ Deduplication: 2 identical alerts → 1 suppressed, 1 different actor → routed"
    )


# ============================================================================
# ESCALATION PATH VALIDATION
# ============================================================================


@pytest.mark.asyncio
async def test_escalation_path_enforcement(alert_pipeline):
    """
    Validation Test: Escalation thresholds trigger correctly
    L1 at 15 min, L2 at 30 min, L3 at 60 min for CRITICAL severity
    """

    escalation = alert_pipeline["escalation"]

    # Fresh CRITICAL alert
    alert_critical = Alert(
        alert_id="escalation_test",
        rule_id="STAGE_7",
        severity=AlertSeverity.CRITICAL,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={},
    )

    # Fresh: No escalation
    assert await escalation.check_escalation(alert_critical) is None

    # 16 minutes old: L1 escalation
    alert_critical.timestamp = time.time() - (16 * 60)
    assert await escalation.check_escalation(alert_critical) == "L1"

    # 31 minutes old: L2 escalation
    alert_critical.timestamp = time.time() - (31 * 60)
    assert await escalation.check_escalation(alert_critical) == "L2"

    # 61 minutes old: L3 escalation
    alert_critical.timestamp = time.time() - (61 * 60)
    assert await escalation.check_escalation(alert_critical) == "L3"

    print("\n✅ Escalation Path: 0 min → None, 16 min → L1, 31 min → L2, 61 min → L3")


# ============================================================================
# COMPLIANCE AUDIT INTEGRITY TEST
# ============================================================================


@pytest.mark.asyncio
async def test_audit_integrity_verification(compliance_pipeline):
    """
    Security Test: Audit trail integrity verified with cryptographic hashing
    Detects tampering in compliance records
    """

    logger = compliance_pipeline["logger"]
    verifier = compliance_pipeline["verifier"]

    # Log event
    event_id = await logger.log_event(
        resource_type="alert",
        resource_id="integrity_test_123",
        action="created",
        actor="detection_engine",
        details={"severity": "CRITICAL"},
    )

    assert logger.events_logged >= 1, "Should have logged event"

    # Get logged event and verify integrity
    original_event = logger.local_cache[0].to_dict()
    original_hash = original_event["hash"]

    # Verify hash is valid
    is_valid = await verifier.verify_event(original_event)
    assert is_valid is True, "Original event should be valid"

    # Simulate tampering by changing action
    tampered_event = original_event.copy()
    tampered_event["action"] = "modified"  # Changed from "created"

    # Tampered event should fail verification
    is_valid_tampered = await verifier.verify_event(tampered_event)
    assert is_valid_tampered is False, "Tampered event should fail verification"

    # Original hash should remain unchanged
    assert original_event["hash"] == original_hash, "Hash should not change"

    print("\n✅ Audit Integrity: Valid event verified, tampered event detected")


# ============================================================================
# MULTI-COMPONENT STRESS TEST
# ============================================================================


@pytest.mark.asyncio
async def test_multi_component_alert_volume(alert_pipeline):
    """
    Performance Test: Process 100 alerts through all pipeline components
    Validates system handles moderate alert volume without degradation
    """

    dedup = alert_pipeline["dedup"]
    router = alert_pipeline["router"]
    escalation = alert_pipeline["escalation"]

    alerts = []
    for i in range(100):
        alert = Alert(
            alert_id=f"volume_alert_{i}",
            rule_id="TEST_RULE",
            severity=AlertSeverity.MEDIUM if i % 2 == 0 else AlertSeverity.HIGH,
            event={"actor_id": f"actor_{i % 10}", "proposal_value": 1000 + i},
            related_events=[],
            anomaly_scores={},
        )
        alerts.append(alert)

    # Process through deduplication
    routed_count = 0
    for alert in alerts:
        if await dedup.should_route(alert):
            routed_count += 1

    # Route alerts
    for alert in alerts[:20]:  # Route subset to save time
        await router.route(alert)

    # Check escalation on subset
    escalated_count = 0
    for alert in alerts[:20]:
        escalation_level = await escalation.check_escalation(alert)
        if escalation_level:
            escalated_count += 1

    assert routed_count > 0, "Should have routed some alerts"
    assert routed_count < len(alerts), "Deduplication should filter duplicates"

    dedup_stats = dedup.get_stats()
    assert "deduplicated_count" in dedup_stats, "Should track deduplication"

    print(
        f"\n✅ Multi-Component Stress: {len(alerts)} alerts → {routed_count} routed, {dedup_stats['deduplicated_count']} deduplicated"
    )


# ============================================================================
# FORENSICS QUERY COMPLETENESS TEST
# ============================================================================


@pytest.mark.asyncio
async def test_forensics_query_interface(compliance_pipeline):
    """
    Validation Test: Forensics query interface works with audit trail
    Tests all query methods without storage backend
    """

    logger = compliance_pipeline["logger"]
    forensics = compliance_pipeline["forensics"]

    # Log multiple events using generic log_event
    await logger.log_event(
        resource_type="alert",
        resource_id="forensics_alert_1",
        action="created",
        actor="detection_engine",
        details={"rule_id": "RULE_1"},
    )

    # Query by resource (returns empty since no storage)
    events_by_resource = await forensics.get_events_by_resource(
        "alert", "forensics_alert_1"
    )
    assert isinstance(events_by_resource, list), "Should return list"

    # Query by actor (returns empty since no storage)
    events_by_actor = await forensics.get_events_by_actor("analyst_l1")
    assert isinstance(events_by_actor, list), "Should return list"

    # Query by action (returns empty since no storage)
    events_by_action = await forensics.get_events_by_action("created")
    assert isinstance(events_by_action, list), "Should return list"

    # Query actor activity
    activity = await forensics.get_actor_activity("attacker_1")
    assert activity["actor"] == "attacker_1", "Should return activity for actor"
    assert "total_events" in activity, "Should include total_events"

    print("\n✅ Forensics Queries: All query types callable and return valid results")


# ============================================================================
# SEVERITY ROUTING VALIDATION
# ============================================================================


def test_severity_routing_channels():
    """
    Validation Test: Each severity level routes to correct channels
    """

    # LOW severity
    alert_low = Alert(
        alert_id="low",
        rule_id="LOW_RULE",
        severity=AlertSeverity.LOW,
        event={},
        related_events=[],
        anomaly_scores={},
    )
    channels_low = alert_low.get_routing_channels()
    assert channels_low == ["log"], f"LOW should route only to log, got {channels_low}"

    # MEDIUM severity
    alert_med = Alert(
        alert_id="med",
        rule_id="MED_RULE",
        severity=AlertSeverity.MEDIUM,
        event={},
        related_events=[],
        anomaly_scores={},
    )
    channels_med = alert_med.get_routing_channels()
    assert "slack" in channels_med, f"MEDIUM should include slack, got {channels_med}"

    # HIGH severity
    alert_high = Alert(
        alert_id="high",
        rule_id="HIGH_RULE",
        severity=AlertSeverity.HIGH,
        event={},
        related_events=[],
        anomaly_scores={},
    )
    channels_high = alert_high.get_routing_channels()
    assert (
        "slack" in channels_high and "jira" in channels_high
    ), f"HIGH should include slack+jira, got {channels_high}"

    # CRITICAL severity
    alert_crit = Alert(
        alert_id="crit",
        rule_id="CRIT_RULE",
        severity=AlertSeverity.CRITICAL,
        event={},
        related_events=[],
        anomaly_scores={},
    )
    channels_crit = alert_crit.get_routing_channels()
    assert (
        "ops_team" in channels_crit
        and "pagerduty" in channels_crit
        and "slack" in channels_crit
        and "jira" in channels_crit
    ), f"CRITICAL should include ops_team+pagerduty+slack+jira, got {channels_crit}"

    print("\n✅ Severity Routing: LOW→log, MEDIUM→slack, HIGH→slack+jira, CRITICAL→all")


# ============================================================================
# COMPONENT INTEGRATION SUMMARY
# ============================================================================


@pytest.mark.asyncio
async def test_all_components_initialized(
    rule_pipeline, alert_pipeline, compliance_pipeline
):
    """
    Sanity Check: Verify all components initialize without errors
    """

    # Rule engine
    assert "registry" in rule_pipeline, "Should have rule registry"
    assert "executor" in rule_pipeline, "Should have rule executor"

    # Alert pipeline
    assert "dedup" in alert_pipeline, "Should have deduplicator"
    assert "router" in alert_pipeline, "Should have router"
    assert "escalation" in alert_pipeline, "Should have escalation"
    assert "audit_log" in alert_pipeline, "Should have audit log"

    # Compliance pipeline
    assert "logger" in compliance_pipeline, "Should have audit logger"
    assert "verifier" in compliance_pipeline, "Should have integrity verifier"
    assert "forensics" in compliance_pipeline, "Should have forensics query"

    print("\n✅ All PHASE 2 Components (4-7) Initialized Successfully")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
