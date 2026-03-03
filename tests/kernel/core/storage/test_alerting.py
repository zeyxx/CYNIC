"""
Alerting & Escalation Tests — L1/L2/L3 incident triage
Tests Alert, deduplication, routing, escalation, and audit logging
"""

import pytest
import time

from cynic.kernel.core.storage.alerting import (
    AlertSeverity,
    Alert,
    AlertDeduplicator,
    AlertAggregator,
    AlertQueue,
    AlertRouter,
    AlertAuditLog,
    AlertAccessControl,
    EscalationPath,
    AlertMetrics,
)


# ============================================================================
# FIXTURES
# ============================================================================


@pytest.fixture
def alert():
    """Create a sample alert."""
    return Alert(
        alert_id="alert_1",
        rule_id="STAGE_2_WEAPONIZATION",
        severity=AlertSeverity.HIGH,
        event={
            "type": "proposal_created",
            "actor_id": "attacker",
            "payload": {"proposal_value": 50000},
        },
        related_events=[],
        anomaly_scores={"composite": 0.75},
    )


@pytest.fixture
def critical_alert():
    """Create a critical alert."""
    return Alert(
        alert_id="alert_critical",
        rule_id="STAGE_4_CONSENSUS",
        severity=AlertSeverity.CRITICAL,
        event={"type": "judgment_created", "actor_id": "attacker"},
        related_events=[],
        anomaly_scores={"composite": 0.95},
    )


@pytest.fixture
def deduplicator():
    """Create alert deduplicator."""
    return AlertDeduplicator(dedup_window_sec=300, dedup_threshold=0.9)


@pytest.fixture
def aggregator():
    """Create alert aggregator."""
    return AlertAggregator()


@pytest.fixture
def queue():
    """Create alert queue."""
    return AlertQueue(max_queue_size=100)


@pytest.fixture
def router():
    """Create alert router."""
    return AlertRouter()


@pytest.fixture
def escalation():
    """Create escalation path."""
    return EscalationPath()


@pytest.fixture
def access_control():
    """Create access control."""
    return AlertAccessControl()


@pytest.fixture
def audit_log():
    """Create audit log."""
    return AlertAuditLog()


@pytest.fixture
def metrics():
    """Create metrics."""
    return AlertMetrics()


# ============================================================================
# ALERT CLASS TESTS
# ============================================================================


def test_alert_creation(alert):
    """Test alert creation."""
    assert alert.alert_id == "alert_1"
    assert alert.rule_id == "STAGE_2_WEAPONIZATION"
    assert alert.severity == AlertSeverity.HIGH
    assert alert.status == "OPEN"


def test_alert_routing_channels_high():
    """Test routing channels for HIGH severity."""
    alert = Alert(
        alert_id="alert_high",
        rule_id="TEST_RULE",
        severity=AlertSeverity.HIGH,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    channels = alert.get_routing_channels()
    assert "slack" in channels
    assert "jira" in channels
    assert "security_team" in channels


def test_alert_routing_channels_critical():
    """Test routing channels for CRITICAL severity."""
    alert = Alert(
        alert_id="alert_critical",
        rule_id="TEST_RULE",
        severity=AlertSeverity.CRITICAL,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    channels = alert.get_routing_channels()
    assert "ops_team" in channels
    assert "pagerduty" in channels
    assert "slack" in channels
    assert "jira" in channels


def test_alert_routing_channels_low():
    """Test routing channels for LOW severity."""
    alert = Alert(
        alert_id="alert_low",
        rule_id="TEST_RULE",
        severity=AlertSeverity.LOW,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    channels = alert.get_routing_channels()
    assert channels == ["log"]


def test_alert_slack_formatting(alert):
    """Test Slack message formatting."""
    message = alert.to_slack_message()

    assert "attachments" in message
    assert len(message["attachments"]) > 0
    assert alert.rule_id in message["text"]


def test_alert_jira_formatting(alert):
    """Test Jira issue formatting."""
    issue = alert.to_jira_issue()

    assert "fields" in issue
    assert issue["fields"]["project"]["key"] == "SECURITY"
    assert alert.severity.value in issue["fields"]["summary"]


# ============================================================================
# DEDUPLICATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_deduplication_identical_alerts(deduplicator):
    """Test deduplication of identical alerts."""
    alert1 = Alert(
        alert_id="alert_1",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={},
    )

    alert2 = Alert(
        alert_id="alert_2",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={},
    )

    # First alert should route
    assert await deduplicator.should_route(alert1) is True

    # Second identical alert should be deduplicated
    assert await deduplicator.should_route(alert2) is False


@pytest.mark.asyncio
async def test_deduplication_different_actors(deduplicator):
    """Test deduplication doesn't block different actors."""
    alert1 = Alert(
        alert_id="alert_1",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker1"},
        related_events=[],
        anomaly_scores={},
    )

    alert2 = Alert(
        alert_id="alert_2",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker2"},
        related_events=[],
        anomaly_scores={},
    )

    assert await deduplicator.should_route(alert1) is True
    assert await deduplicator.should_route(alert2) is True


@pytest.mark.asyncio
async def test_deduplication_window_expiry(deduplicator):
    """Test deduplication window expiration."""
    alert1 = Alert(
        alert_id="alert_1",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={},
    )

    # Set old timestamp (outside dedup window)
    alert1.timestamp = time.time() - 400

    alert2 = Alert(
        alert_id="alert_2",
        rule_id="STAGE_2",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={},
    )

    # First alert is old, should be cleaned up
    await deduplicator.should_route(alert1)

    # Second alert should route (first is expired)
    assert await deduplicator.should_route(alert2) is True


def test_deduplication_stats(deduplicator):
    """Test deduplication statistics."""
    stats = deduplicator.get_stats()

    assert "recent_alerts" in stats
    assert "dedup_window" in stats
    assert "deduplicated_count" in stats


# ============================================================================
# AGGREGATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_aggregation_by_actor(aggregator):
    """Test alert aggregation by actor."""
    alerts = [
        Alert(
            alert_id=f"alert_{i}",
            rule_id="STAGE_2",
            severity=AlertSeverity.HIGH,
            event={"actor_id": "attacker" if i < 3 else "other"},
            related_events=[],
            anomaly_scores={},
        )
        for i in range(5)
    ]

    grouped = await aggregator.aggregate(alerts)

    assert "attacker" in grouped
    assert "other" in grouped
    assert len(grouped["attacker"]) == 3
    assert len(grouped["other"]) == 2


# ============================================================================
# QUEUE TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_queue_enqueue(queue, alert):
    """Test enqueueing alert."""
    result = await queue.enqueue(alert)
    assert result is True
    assert queue.queue.qsize() == 1


@pytest.mark.asyncio
async def test_queue_dequeue(queue, alert):
    """Test dequeueing alert."""
    await queue.enqueue(alert)
    dequeued = await queue.dequeue()

    assert dequeued is not None
    assert dequeued.alert_id == alert.alert_id


@pytest.mark.asyncio
async def test_queue_full():
    """Test queue overflow."""
    queue = AlertQueue(max_queue_size=2)
    alert1 = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.LOW,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    # Fill queue
    await queue.enqueue(alert1)
    await queue.enqueue(alert1)

    # Third should fail
    result = await queue.enqueue(alert1)
    assert result is False


def test_queue_stats(queue):
    """Test queue statistics."""
    stats = queue.get_stats()

    assert "queue_size" in stats
    assert "max_size" in stats
    assert "processed" in stats
    assert "failed" in stats


# ============================================================================
# ROUTING TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_routing_to_log(router, alert):
    """Test routing to log."""
    results = await router.route(alert)

    assert "log" not in results  # Log routing is implicit
    # Alert should log without error


@pytest.mark.asyncio
async def test_routing_critical_alert(router, critical_alert):
    """Test critical alert routes to all channels."""
    results = await router.route(critical_alert)

    # Should attempt to route to all channels for critical
    assert len(results) >= 3  # slack, jira, ops_team, pagerduty


# ============================================================================
# ESCALATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_escalation_l1_at_15_minutes(escalation):
    """Test escalation to L1 at 15 minutes."""
    alert = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.HIGH,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    # Set alert age to 16 minutes (just past L1 threshold)
    alert.timestamp = time.time() - (16 * 60)

    level = await escalation.check_escalation(alert)
    assert level == "L1"


@pytest.mark.asyncio
async def test_escalation_l2_at_30_minutes(escalation):
    """Test escalation to L2 at 30 minutes."""
    alert = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.CRITICAL,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    # Set alert age to 31 minutes (just past L2 threshold)
    alert.timestamp = time.time() - (31 * 60)

    level = await escalation.check_escalation(alert)
    assert level == "L2"


@pytest.mark.asyncio
async def test_escalation_l3_at_60_minutes(escalation):
    """Test escalation to L3 at 60 minutes."""
    alert = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.CRITICAL,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    # Set alert age to 61 minutes (just past L3 threshold)
    alert.timestamp = time.time() - (61 * 60)

    level = await escalation.check_escalation(alert)
    assert level == "L3"


@pytest.mark.asyncio
async def test_escalation_no_escalation_needed(escalation):
    """Test alert doesn't escalate if fresh."""
    alert = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.HIGH,
        event={},
        related_events=[],
        anomaly_scores={},
    )

    # Alert is fresh (just created)
    level = await escalation.check_escalation(alert)
    assert level is None


# ============================================================================
# ACCESS CONTROL TESTS
# ============================================================================


def test_access_control_l1_analyst(access_control, alert):
    """Test L1 analyst access."""
    assert access_control.can_view_alert("analyst_l1", alert) is True
    assert access_control.can_acknowledge_alert("analyst_l1") is True
    assert access_control.can_close_alert("analyst_l1") is False


def test_access_control_l2_analyst(access_control, alert):
    """Test L2 analyst access."""
    assert access_control.can_view_alert("analyst_l2", alert) is True
    assert access_control.can_acknowledge_alert("analyst_l2") is True
    assert access_control.can_close_alert("analyst_l2") is True


def test_access_control_unknown_role(access_control, alert):
    """Test unknown role has no access."""
    assert access_control.can_view_alert("unknown", alert) is False
    assert access_control.can_acknowledge_alert("unknown") is False
    assert access_control.can_close_alert("unknown") is False


# ============================================================================
# AUDIT LOG TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_audit_log_alert_created(audit_log, alert):
    """Test audit log for alert creation."""
    # Should not crash (storage is None)
    await audit_log.log_alert_created(alert, "STAGE_2")


@pytest.mark.asyncio
async def test_audit_log_alert_routed(audit_log, alert):
    """Test audit log for alert routing."""
    await audit_log.log_alert_routed(alert, "slack", True)


@pytest.mark.asyncio
async def test_audit_log_alert_acknowledged(audit_log, alert):
    """Test audit log for alert acknowledgement."""
    await audit_log.log_alert_acknowledged(alert, "analyst_l1")


@pytest.mark.asyncio
async def test_audit_log_alert_closed(audit_log, alert):
    """Test audit log for alert closure."""
    await audit_log.log_alert_closed(alert, "False positive", "analyst_l2")


# ============================================================================
# METRICS TESTS
# ============================================================================


def test_metrics_initialization(metrics):
    """Test metrics initialization."""
    m = metrics.get_metrics()

    assert m["alerts_created"] == 0
    assert m["alerts_routed"] == 0
    assert m["failure_rate"] == 0


def test_metrics_failure_rate(metrics):
    """Test failure rate calculation."""
    metrics.alerts_routed = 10
    metrics.routing_failures = 2

    m = metrics.get_metrics()
    assert m["failure_rate"] == 0.2


def test_metrics_no_division_by_zero(metrics):
    """Test metrics handle zero routed alerts."""
    metrics.alerts_routed = 0
    metrics.routing_failures = 0

    m = metrics.get_metrics()
    assert m["failure_rate"] == 0


# ============================================================================
# INTEGRATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_end_to_end_alert_lifecycle(
    deduplicator, router, escalation, access_control
):
    """Test complete alert lifecycle."""
    # Create alert
    alert = Alert(
        alert_id="alert_1",
        rule_id="STAGE_2_WEAPONIZATION",
        severity=AlertSeverity.HIGH,
        event={"actor_id": "attacker"},
        related_events=[],
        anomaly_scores={"composite": 0.75},
    )

    # Check if should route (deduplication)
    should_route = await deduplicator.should_route(alert)
    assert should_route is True

    # Route alert
    results = await router.route(alert)
    assert len(results) > 0

    # Check escalation (fresh alert)
    escalation_level = await escalation.check_escalation(alert)
    assert escalation_level is None

    # Check access control
    can_acknowledge = access_control.can_acknowledge_alert("analyst_l1")
    assert can_acknowledge is True


@pytest.mark.asyncio
async def test_alert_severity_routing_coverage():
    """Test all severity levels have proper routing."""
    for severity in AlertSeverity:
        alert = Alert(
            alert_id=f"alert_{severity.value}",
            rule_id="TEST",
            severity=severity,
            event={},
            related_events=[],
            anomaly_scores={},
        )

        channels = alert.get_routing_channels()
        assert len(channels) > 0  # Every severity has at least one channel


# ============================================================================
# EDGE CASES
# ============================================================================


@pytest.mark.asyncio
async def test_queue_with_missing_actor_id():
    """Test alert queue with missing actor ID."""
    queue = AlertQueue()
    alert = Alert(
        alert_id="alert_1",
        rule_id="TEST",
        severity=AlertSeverity.LOW,
        event={},  # No actor_id
        related_events=[],
        anomaly_scores={},
    )

    result = await queue.enqueue(alert)
    assert result is True


def test_alert_timestamp_tracking(alert):
    """Test alert timestamp is set."""
    assert alert.timestamp > 0
    assert alert.timestamp <= time.time()


@pytest.mark.asyncio
async def test_large_alert_aggregation():
    """Test aggregation with many alerts."""
    aggregator = AlertAggregator()
    alerts = [
        Alert(
            alert_id=f"alert_{i}",
            rule_id="TEST",
            severity=AlertSeverity.LOW,
            event={"actor_id": f"actor_{i % 10}"},
            related_events=[],
            anomaly_scores={},
        )
        for i in range(100)
    ]

    grouped = await aggregator.aggregate(alerts)

    assert len(grouped) == 10  # 10 unique actors
    for actor_group in grouped.values():
        assert len(actor_group) == 10
