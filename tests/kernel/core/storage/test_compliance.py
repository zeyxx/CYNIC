"""
Compliance & Audit Tests — Forensics, retention, and regulatory compliance
Tests audit logging, integrity verification, forensics queries, and compliance reports
"""

import pytest
import time

from cynic.kernel.core.storage.compliance import (
    AuditEvent,
    AuditLogger,
    IntegrityVerifier,
    ForensicsQuery,
    RetentionPolicy,
    ComplianceReport,
)


# ============================================================================
# FIXTURES
# ============================================================================


@pytest.fixture
def audit_event():
    """Create a sample audit event."""
    return AuditEvent(
        event_id="alert_123_1234567890",
        resource_type="alert",
        resource_id="alert_123",
        action="created",
        actor="detection_engine",
        timestamp=time.time(),
        details={"rule_id": "STAGE_2_WEAPONIZATION"},
        status="success",
    )


@pytest.fixture
def audit_logger():
    """Create audit logger."""
    return AuditLogger()


@pytest.fixture
def integrity_verifier():
    """Create integrity verifier."""
    return IntegrityVerifier()


@pytest.fixture
def forensics_query():
    """Create forensics query."""
    return ForensicsQuery()


@pytest.fixture
def retention_policy():
    """Create retention policy."""
    return RetentionPolicy(retention_days=365)


@pytest.fixture
def compliance_report():
    """Create compliance report generator."""
    return ComplianceReport()


# ============================================================================
# AUDIT EVENT TESTS
# ============================================================================


def test_audit_event_creation(audit_event):
    """Test audit event creation."""
    assert audit_event.event_id == "alert_123_1234567890"
    assert audit_event.resource_type == "alert"
    assert audit_event.action == "created"
    assert audit_event.actor == "detection_engine"


def test_audit_event_hash(audit_event):
    """Test hash is computed."""
    assert len(audit_event.hash) == 64  # SHA256 hex = 64 chars
    assert audit_event.hash.isalnum()


def test_audit_event_hash_deterministic():
    """Test hash is deterministic."""
    event1 = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="created",
        actor="system",
        timestamp=1000.0,
        details={},
        status="success",
    )

    event2 = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="created",
        actor="system",
        timestamp=1000.0,
        details={},
        status="success",
    )

    assert event1.hash == event2.hash


def test_audit_event_hash_changes_with_content():
    """Test hash changes when content changes."""
    event1 = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="created",
        actor="system",
        timestamp=1000.0,
    )

    event2 = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="modified",  # Different action
        actor="system",
        timestamp=1000.0,
    )

    assert event1.hash != event2.hash


def test_audit_event_to_dict(audit_event):
    """Test conversion to dictionary."""
    d = audit_event.to_dict()

    assert d["type"] == "audit_event"
    assert d["resource_type"] == "alert"
    assert d["action"] == "created"
    assert "hash" in d


# ============================================================================
# AUDIT LOGGER TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_audit_logger_log_event(audit_logger):
    """Test logging an event."""
    event_id = await audit_logger.log_event(
        resource_type="alert",
        resource_id="alert_123",
        action="created",
        actor="system",
        details={"rule_id": "STAGE_2"},
    )

    assert event_id is not None
    assert audit_logger.events_logged == 1


@pytest.mark.asyncio
async def test_audit_logger_log_alert_created(audit_logger):
    """Test logging alert creation."""
    event_id = await audit_logger.log_alert_created(
        alert_id="alert_123",
        rule_id="STAGE_2",
        actor="system",
    )

    assert event_id is not None
    assert audit_logger.events_logged == 1


@pytest.mark.asyncio
async def test_audit_logger_log_alert_routed(audit_logger):
    """Test logging alert routing."""
    event_id = await audit_logger.log_alert_routed(
        alert_id="alert_123",
        channel="slack",
        success=True,
        actor="system",
    )

    assert event_id is not None


@pytest.mark.asyncio
async def test_audit_logger_log_alert_acknowledged(audit_logger):
    """Test logging alert acknowledgement."""
    event_id = await audit_logger.log_alert_acknowledged(
        alert_id="alert_123",
        analyst="analyst_l1",
    )

    assert event_id is not None


@pytest.mark.asyncio
async def test_audit_logger_log_alert_closed(audit_logger):
    """Test logging alert closure."""
    event_id = await audit_logger.log_alert_closed(
        alert_id="alert_123",
        reason="False positive",
        analyst="analyst_l2",
    )

    assert event_id is not None


@pytest.mark.asyncio
async def test_audit_logger_multiple_events(audit_logger):
    """Test logging multiple events."""
    for i in range(5):
        await audit_logger.log_event(
            resource_type="alert",
            resource_id=f"alert_{i}",
            action="created",
            actor="system",
        )

    assert audit_logger.events_logged == 5


def test_audit_logger_stats(audit_logger):
    """Test audit logger statistics."""
    stats = audit_logger.get_stats()

    assert "events_logged" in stats
    assert "local_cache_size" in stats


# ============================================================================
# INTEGRITY VERIFICATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_integrity_verify_valid_event():
    """Test verifying a valid event."""
    # Create event and verify immediately (same session)
    event = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="created",
        actor="system",
        timestamp=1000.0,
        details={},
        status="success",
    )
    event_dict = event.to_dict()

    verifier = IntegrityVerifier()
    is_valid = await verifier.verify_event(event_dict)

    assert is_valid is True


@pytest.mark.asyncio
async def test_integrity_verify_tampered_event(integrity_verifier, audit_event):
    """Test tampered event is detected."""
    event_dict = audit_event.to_dict()

    # Tamper with action
    event_dict["action"] = "modified"

    is_valid = await integrity_verifier.verify_event(event_dict)
    assert is_valid is False


@pytest.mark.asyncio
async def test_integrity_verify_chain():
    """Test verifying chain of events."""
    events = [
        AuditEvent(
            event_id=f"event_{i}",
            resource_type="alert",
            resource_id=f"alert_{i}",
            action="created",
            actor="system",
            timestamp=1000.0 + i,
        ).to_dict()
        for i in range(5)
    ]

    verifier = IntegrityVerifier()
    is_valid = await verifier.verify_chain(events)
    assert is_valid is True


def test_integrity_verifier_stats(integrity_verifier):
    """Test integrity verifier statistics."""
    stats = integrity_verifier.get_stats()

    assert "verified_hashes" in stats


# ============================================================================
# FORENSICS QUERY TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_forensics_get_events_by_resource(forensics_query):
    """Test getting events by resource."""
    # With no storage, should return empty list
    events = await forensics_query.get_events_by_resource("alert", "alert_123")
    assert events == []


@pytest.mark.asyncio
async def test_forensics_get_events_by_actor(forensics_query):
    """Test getting events by actor."""
    events = await forensics_query.get_events_by_actor("system")
    assert events == []


@pytest.mark.asyncio
async def test_forensics_get_events_by_action(forensics_query):
    """Test getting events by action."""
    events = await forensics_query.get_events_by_action("created")
    assert events == []


@pytest.mark.asyncio
async def test_forensics_get_events_by_time_range(forensics_query):
    """Test getting events by time range."""
    now = time.time()
    events = await forensics_query.get_events_by_time_range(now - 3600, now)
    assert events == []


@pytest.mark.asyncio
async def test_forensics_get_alert_timeline(forensics_query):
    """Test getting alert timeline."""
    timeline = await forensics_query.get_alert_timeline("alert_123")
    assert timeline == []


@pytest.mark.asyncio
async def test_forensics_get_actor_activity(forensics_query):
    """Test getting actor activity summary."""
    activity = await forensics_query.get_actor_activity("system", days=30)

    assert activity["actor"] == "system"
    assert "total_events" in activity
    assert "by_action" in activity
    assert "by_resource" in activity


# ============================================================================
# RETENTION POLICY TESTS
# ============================================================================


def test_retention_policy_creation():
    """Test retention policy creation."""
    policy = RetentionPolicy(retention_days=365)

    assert policy.retention_days == 365
    assert policy.critical_retention_days == 2555  # 7 years


@pytest.mark.asyncio
async def test_retention_policy_enforce(retention_policy):
    """Test retention policy enforcement."""
    result = await retention_policy.enforce_retention(storage=None)

    assert "deleted" in result
    assert "errors" in result


def test_retention_policy_get_policy(retention_policy):
    """Test getting retention policy."""
    policy = retention_policy.get_policy()

    assert "standard_retention_days" in policy
    assert "critical_retention_days" in policy


# ============================================================================
# COMPLIANCE REPORT TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_compliance_report_structure():
    """Test compliance report structure (without storage)."""
    report = ComplianceReport(storage=None)

    # Without storage, reports return empty dicts
    soc2_report = await report.generate_soc2_report(days=30)
    assert isinstance(soc2_report, dict)

    gdpr_report = await report.generate_gdpr_report(days=30)
    assert isinstance(gdpr_report, dict)


@pytest.mark.asyncio
async def test_compliance_report_incident_summary(compliance_report):
    """Test incident summary report."""
    report = await compliance_report.generate_incident_summary("incident_123")

    assert report["incident_id"] == "incident_123"
    assert "events" in report


# ============================================================================
# INTEGRATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_audit_and_verification_workflow():
    """Test complete audit and verification workflow."""
    # Log an event
    logger = AuditLogger()
    await logger.log_alert_created("alert_123", "STAGE_2", "system")

    # Get the logged event
    event = logger.local_cache[0]
    event_dict = event.to_dict()

    # Verify its integrity
    verifier = IntegrityVerifier()
    is_valid = await verifier.verify_event(event_dict)
    assert is_valid is True


@pytest.mark.asyncio
async def test_alert_lifecycle_logging(audit_logger):
    """Test complete alert lifecycle logging."""
    alert_id = "alert_123"

    # Created
    await audit_logger.log_alert_created(alert_id, "STAGE_2", "system")

    # Routed
    await audit_logger.log_alert_routed(alert_id, "slack", True, "system")
    await audit_logger.log_alert_routed(alert_id, "jira", True, "system")

    # Acknowledged
    await audit_logger.log_alert_acknowledged(alert_id, "analyst_l1")

    # Closed
    await audit_logger.log_alert_closed(alert_id, "True positive", "analyst_l2")

    # Should have 5 events
    assert audit_logger.events_logged == 5


# ============================================================================
# EDGE CASES
# ============================================================================


def test_audit_event_with_complex_details():
    """Test audit event with nested details."""
    event = AuditEvent(
        event_id="event_1",
        resource_type="alert",
        resource_id="alert_1",
        action="created",
        actor="system",
        timestamp=time.time(),
        details={
            "rule_id": "STAGE_2",
            "anomaly_scores": {"composite": 0.75, "voting_velocity": 0.5},
            "actor_id": "attacker",
        },
    )

    assert event.hash is not None
    assert event.to_dict()["details"]["anomaly_scores"]["composite"] == 0.75


def test_forensics_group_by_key():
    """Test forensics grouping utility."""
    events = [
        {"action": "created"},
        {"action": "created"},
        {"action": "modified"},
        {"action": "created"},
    ]

    fq = ForensicsQuery()
    grouped = fq._group_by_key(events, "action")

    assert grouped["created"] == 3
    assert grouped["modified"] == 1


def test_retention_policy_critical_retention():
    """Test critical retention is longer."""
    policy = RetentionPolicy(retention_days=365)

    assert policy.critical_retention_days > policy.retention_days
    assert policy.critical_retention_days == 2555  # 7 years


@pytest.mark.asyncio
async def test_large_event_logging(audit_logger):
    """Test logging many events."""
    for i in range(100):
        await audit_logger.log_event(
            resource_type="alert",
            resource_id=f"alert_{i}",
            action="created",
            actor="system",
        )

    assert audit_logger.events_logged == 100
    assert len(audit_logger.local_cache) == 100


def test_audit_event_timestamp_precision(audit_event):
    """Test event timestamp is recorded with precision."""
    assert audit_event.timestamp > 0
    assert isinstance(audit_event.timestamp, float)


@pytest.mark.asyncio
async def test_forensics_with_empty_results(forensics_query):
    """Test forensics queries handle empty results."""
    activity = await forensics_query.get_actor_activity("nonexistent_actor")

    assert activity["actor"] == "nonexistent_actor"
    assert activity["total_events"] == 0
    assert activity["by_action"] == {}
