# PHASE 2: COMPONENT 7 — Compliance & Audit Logging Decomposition

## Problem Statement

**Maintain audit trail for regulatory compliance (SOC2, GDPR, ISO27001)**

Currently:
- Events persisted (Component 2-3 ✅)
- Threats detected & alerted (Component 4-6 ✅)
- BUT: No audit trail of what happened, who did what, when

**Solution: Compliance Logging** — Immutable audit trail:
1. All security events logged
2. All rule evaluations logged
3. All alerts generated logged
4. All human actions logged
5. Retention policy enforced (365+ days)

---

## DOMAIN EXPERTISE DECOMPOSITION

### 1️⃣ BACKEND ENGINEER: Audit Event Schema

```python
class AuditEvent:
    """Immutable audit log entry."""

    def __init__(
        self,
        audit_id: str,
        event_type: str,  # "security_event", "rule_match", "alert_created", "action_taken"
        actor: str,       # Who triggered this? (system, user ID, service)
        resource: str,    # What was affected? (event_id, proposal_id, etc.)
        action: str,      # What happened? (created, evaluated, escalated, etc.)
        details: dict,    # Full context
        timestamp: float,
    ):
        self.audit_id = audit_id
        self.event_type = event_type
        self.actor = actor
        self.resource = resource
        self.action = action
        self.details = details
        self.timestamp = timestamp
        self.immutable_hash = self._compute_hash()

    def _compute_hash(self) -> str:
        """Compute immutable hash of audit event."""
        content = f"{self.audit_id}|{self.event_type}|{self.actor}|{self.action}|{self.timestamp}"
        return hashlib.sha256(content.encode()).hexdigest()

    def to_dict(self) -> dict:
        """Convert to storable dict."""
        return {
            "audit_id": self.audit_id,
            "event_type": self.event_type,
            "actor": self.actor,
            "resource": self.resource,
            "action": self.action,
            "details": self.details,
            "timestamp": self.timestamp,
            "immutable_hash": self.immutable_hash,
        }


class AuditLogger:
    """Log all security-relevant events to audit trail."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage

    async def log_security_event(
        self,
        event: dict,
        received_at: float,
    ) -> None:
        """Log when security event is received."""
        audit_event = AuditEvent(
            audit_id=str(uuid.uuid4()),
            event_type="security_event_received",
            actor="event_forwarder",
            resource=event.get("id", "unknown"),
            action="received",
            details={
                "event_type": event.get("type"),
                "actor_id": event.get("actor_id"),
                "source": event.get("source"),
            },
            timestamp=received_at,
        )
        await self._persist(audit_event)

    async def log_rule_evaluated(
        self,
        rule_id: str,
        event_id: str,
        matched: bool,
        timestamp: float,
    ) -> None:
        """Log when rule is evaluated."""
        audit_event = AuditEvent(
            audit_id=str(uuid.uuid4()),
            event_type="rule_evaluated",
            actor="rule_engine",
            resource=event_id,
            action="evaluated",
            details={
                "rule_id": rule_id,
                "matched": matched,
            },
            timestamp=timestamp,
        )
        await self._persist(audit_event)

    async def log_alert_generated(
        self,
        alert_id: str,
        rule_id: str,
        severity: str,
        timestamp: float,
    ) -> None:
        """Log when alert is generated."""
        audit_event = AuditEvent(
            audit_id=str(uuid.uuid4()),
            event_type="alert_generated",
            actor="alert_generator",
            resource=alert_id,
            action="generated",
            details={
                "rule_id": rule_id,
                "severity": severity,
            },
            timestamp=timestamp,
        )
        await self._persist(audit_event)

    async def log_alert_routed(
        self,
        alert_id: str,
        channel: str,
        success: bool,
        timestamp: float,
    ) -> None:
        """Log when alert is routed."""
        audit_event = AuditEvent(
            audit_id=str(uuid.uuid4()),
            event_type="alert_routed",
            actor="alert_router",
            resource=alert_id,
            action="routed",
            details={
                "channel": channel,
                "success": success,
            },
            timestamp=timestamp,
        )
        await self._persist(audit_event)

    async def log_human_action(
        self,
        action: str,  # "acknowledged", "closed", "escalated"
        resource: str,  # alert_id
        actor_id: str,  # user ID
        details: dict,
        timestamp: float,
    ) -> None:
        """Log human actions (triage, investigation, response)."""
        audit_event = AuditEvent(
            audit_id=str(uuid.uuid4()),
            event_type="human_action",
            actor=actor_id,
            resource=resource,
            action=action,
            details=details,
            timestamp=timestamp,
        )
        await self._persist(audit_event)

    async def _persist(self, audit_event: AuditEvent) -> None:
        """Persist audit event to storage."""
        await self.storage.security_events.save_event({
            "type": "audit_log",
            **audit_event.to_dict(),
        })
```

**Validation Checklist:**
- [ ] All event types logged
- [ ] Immutable hashes computed
- [ ] Persistence working
- [ ] Timestamps accurate

---

### 2️⃣ DATA ENGINEER: Retention & Archival

```python
class RetentionPolicy:
    """Define and enforce audit log retention."""

    def __init__(
        self,
        retention_days: int = 365,
        archive_after_days: int = 90,
    ):
        self.retention_days = retention_days
        self.archive_after_days = archive_after_days

    async def should_delete(self, audit_event: dict) -> bool:
        """Check if event should be deleted (past retention period)."""
        event_age_days = (time.time() - audit_event["timestamp"]) / (24 * 3600)
        return event_age_days > self.retention_days

    async def should_archive(self, audit_event: dict) -> bool:
        """Check if event should be archived (past archive threshold)."""
        event_age_days = (time.time() - audit_event["timestamp"]) / (24 * 3600)
        return event_age_days > self.archive_after_days


class RetentionEnforcer:
    """Enforce retention policy on audit logs."""

    def __init__(self, storage: StorageInterface, policy: RetentionPolicy):
        self.storage = storage
        self.policy = policy

    async def enforce_retention(self) -> dict:
        """Execute retention policy."""
        cutoff_time = time.time() - (self.policy.retention_days * 24 * 3600)
        archive_cutoff = time.time() - (self.policy.archive_after_days * 24 * 3600)

        # Get events past retention
        old_events = await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "timestamp_lte": cutoff_time,
            },
            limit=10000,
        )

        deleted_count = 0
        archived_count = 0

        for event in old_events:
            # Delete if past retention
            if await self.policy.should_delete(event):
                # TODO: Delete from storage
                deleted_count += 1
            # Archive if past archive threshold
            elif await self.policy.should_archive(event):
                # TODO: Archive to cold storage
                archived_count += 1

        return {
            "deleted": deleted_count,
            "archived": archived_count,
            "total_processed": len(old_events),
        }

    async def run_scheduled_retention(self) -> None:
        """Run retention enforcement on schedule (daily)."""
        while True:
            try:
                result = await self.enforce_retention()
                logger.info(f"Retention enforcement: {result}")
            except Exception as e:
                logger.error(f"Retention enforcement failed: {e}")

            # Run daily
            await asyncio.sleep(24 * 3600)
```

**Validation Checklist:**
- [ ] Retention policy defined
- [ ] Archival working
- [ ] Deletion working
- [ ] Scheduled enforcement running

---

### 3️⃣ INFRASTRUCTURE ENGINEER: Query & Reporting

```python
class ComplianceQuery:
    """Execute compliance queries on audit logs."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage

    async def all_events_by_actor(self, actor: str, days: int = 30) -> list[dict]:
        """Get all events from specific actor (user, service, etc.)."""
        timestamp_gte = time.time() - (days * 24 * 3600)

        return await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "actor": actor,
                "timestamp_gte": timestamp_gte,
            },
            limit=10000,
        )

    async def all_events_by_resource(self, resource: str, days: int = 30) -> list[dict]:
        """Get all actions on specific resource (alert, proposal, etc.)."""
        timestamp_gte = time.time() - (days * 24 * 3600)

        return await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "resource": resource,
                "timestamp_gte": timestamp_gte,
            },
            limit=10000,
        )

    async def all_events_by_action(self, action: str, days: int = 30) -> list[dict]:
        """Get all events of specific action type."""
        timestamp_gte = time.time() - (days * 24 * 3600)

        return await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "action": action,
                "timestamp_gte": timestamp_gte,
            },
            limit=10000,
        )

    async def verify_integrity(self) -> bool:
        """Verify audit log integrity (no tampering)."""
        # Get all audit events
        all_events = await self.storage.security_events.list_events(
            filters={"type": "audit_log"},
            limit=100000,
        )

        # Verify immutable hashes
        for event in all_events:
            stored_hash = event.get("immutable_hash")
            computed_hash = self._compute_hash(event)

            if stored_hash != computed_hash:
                logger.error(f"Audit integrity violation in event {event.get('audit_id')}")
                return False

        return True

    def _compute_hash(self, event: dict) -> str:
        """Recompute hash from stored data."""
        content = f"{event['audit_id']}|{event['event_type']}|{event['actor']}|{event['action']}|{event['timestamp']}"
        return hashlib.sha256(content.encode()).hexdigest()


class ComplianceReport:
    """Generate compliance reports."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.query = ComplianceQuery(storage)

    async def generate_soc2_report(self, days: int = 30) -> dict:
        """Generate SOC2 compliance report."""
        timestamp_gte = time.time() - (days * 24 * 3600)

        all_events = await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "timestamp_gte": timestamp_gte,
            },
            limit=100000,
        )

        return {
            "report_period": f"Last {days} days",
            "total_events": len(all_events),
            "by_event_type": self._count_by(all_events, "event_type"),
            "by_action": self._count_by(all_events, "action"),
            "by_actor": self._count_by(all_events, "actor"),
            "integrity_verified": await self.query.verify_integrity(),
        }

    async def generate_gdpr_report(self, days: int = 30) -> dict:
        """Generate GDPR compliance report (data access audit)."""
        timestamp_gte = time.time() - (days * 24 * 3600)

        all_events = await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "timestamp_gte": timestamp_gte,
            },
            limit=100000,
        )

        return {
            "report_period": f"Last {days} days",
            "data_access_events": len([e for e in all_events if e.get("action") == "accessed"]),
            "data_deletion_events": len([e for e in all_events if e.get("action") == "deleted"]),
            "retention_policy_enforced": True,
        }

    def _count_by(self, events: list, field: str) -> dict:
        """Count events by field."""
        counts = defaultdict(int)
        for event in events:
            counts[event.get(field, "unknown")] += 1
        return dict(counts)
```

**Validation Checklist:**
- [ ] Query methods working
- [ ] Integrity verification working
- [ ] Reports generated correctly
- [ ] All required fields present

---

### 4️⃣ SECURITY ARCHITECT: Evidence Collection & Chain of Custody

```python
class ChainOfCustody:
    """Maintain chain of custody for audit logs (forensics)."""

    async def __init__(self, storage: StorageInterface):
        self.storage = storage

    async def collect_evidence(
        self,
        alert_id: str,
        scope_days: int = 7,
    ) -> dict:
        """Collect all evidence related to an alert."""
        # Get the alert
        alert_events = await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "action": "generated",
                "resource": alert_id,
            },
            limit=1,
        )

        if not alert_events:
            return {}

        alert_event = alert_events[0]
        timestamp = alert_event["timestamp"]
        scope_start = timestamp - (scope_days * 24 * 3600)

        # Collect related events
        related_actor = alert_event.get("details", {}).get("actor_id")

        evidence = {
            "alert_id": alert_id,
            "collected_at": time.time(),
            "collected_by": "forensics_system",
            "scope_days": scope_days,
            "events": [],
        }

        # 1. Get all events from this actor
        if related_actor:
            actor_events = await self.storage.security_events.list_events(
                filters={
                    "type": "audit_log",
                    "actor": related_actor,
                    "timestamp_gte": scope_start,
                },
                limit=10000,
            )
            evidence["events"].extend(actor_events)

        # 2. Get all alert-related events
        alert_events = await self.storage.security_events.list_events(
            filters={
                "type": "audit_log",
                "resource": alert_id,
                "timestamp_gte": scope_start,
            },
            limit=10000,
        )
        evidence["events"].extend(alert_events)

        # 3. Compute integrity hash
        evidence["integrity_hash"] = self._compute_evidence_hash(evidence)

        return evidence

    def _compute_evidence_hash(self, evidence: dict) -> str:
        """Compute immutable hash of evidence package."""
        content = f"{evidence['alert_id']}|{evidence['collected_at']}|{len(evidence['events'])}"
        return hashlib.sha256(content.encode()).hexdigest()
```

**Validation Checklist:**
- [ ] Evidence collection working
- [ ] Chain of custody tracked
- [ ] Integrity hashes verified
- [ ] Forensics ready for investigation

---

### 5️⃣ SRE: Logging Infrastructure & Durability

```python
class AuditLogDurability:
    """Ensure audit logs are durable and reliable."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.write_failures = 0
        self.write_successes = 0

    async def persist_with_retry(
        self,
        audit_event: dict,
        max_retries: int = 3,
    ) -> bool:
        """Persist audit event with retry logic."""
        for attempt in range(max_retries):
            try:
                await self.storage.security_events.save_event(audit_event)
                self.write_successes += 1
                return True
            except Exception as e:
                logger.warning(f"Audit log write failed (attempt {attempt + 1}/{max_retries}): {e}")
                self.write_failures += 1

                if attempt < max_retries - 1:
                    # Exponential backoff
                    await asyncio.sleep(2 ** attempt)

        # Final failure - log locally as fallback
        logger.error(f"Audit log write permanently failed: {audit_event}")
        return False

    def get_durability_stats(self) -> dict:
        """Get audit log durability statistics."""
        total = self.write_successes + self.write_failures
        return {
            "writes_successful": self.write_successes,
            "writes_failed": self.write_failures,
            "success_rate": (
                self.write_successes / total
                if total > 0
                else 1.0
            ),
        }
```

**Validation Checklist:**
- [ ] Retry logic working
- [ ] Fallback logging working
- [ ] Durability stats accurate
- [ ] No audit events lost

---

### 6️⃣ SOLUTIONS ARCHITECT: Integration with PHASE 2

**Audit Trail Flow:**
```
Component 2: EventForwarder
    ↓ logs
Component 7: AuditLogger
    ├─ log_security_event()
    ↓

Component 4: Real-Time Detection
    ↓ logs
Component 7: AuditLogger
    ├─ log_rule_evaluated()
    ↓

Component 5: Detection Rules
    ↓ logs
Component 7: AuditLogger
    ├─ log_rule_evaluated()
    ├─ log_alert_generated()
    ↓

Component 6: Alerting
    ↓ logs
Component 7: AuditLogger
    ├─ log_alert_routed()
    ├─ log_human_action()
    ↓

Component 7: Compliance Logging ← YOU ARE HERE
    ├─ AuditLogger (collect events)
    ├─ RetentionEnforcer (delete/archive)
    ├─ ComplianceQuery (search)
    ├─ ComplianceReport (SOC2/GDPR)
    └─ ChainOfCustody (forensics)
         ↓
    SurrealDB audit_log records (immutable)
```

**Integration Code:**
```python
# Everywhere in PHASE 2:
async def log_audit_event(event_type: str, actor: str, action: str, details: dict):
    audit_logger = get_audit_logger()
    await audit_logger.log_event(
        event_type=event_type,
        actor=actor,
        action=action,
        details=details,
    )
```

**Validation Checklist:**
- [ ] All components log to audit trail
- [ ] Logging doesn't block operations
- [ ] Compliance reports generated correctly

---

## Implementation Plan

**1 day of work:**
- AuditLogger + event types
- RetentionPolicy + RetentionEnforcer
- ComplianceQuery + ComplianceReport
- ChainOfCustody
- AuditLogDurability
- Factory integration
- 15+ tests

**Total: 15 tests, 1 commit**

---

## Success Criteria

✅ **Component 7 is DONE when:**
1. All security events logged to audit trail
2. All rule evaluations logged
3. All alerts logged
4. All human actions logged
5. Immutable hashes computed & verified
6. Retention policy enforced (365+ days)
7. Archival working for old events
8. Compliance queries working (by actor, resource, action)
9. SOC2 & GDPR reports generated
10. Chain of custody for forensics working
11. Evidence integrity verified
12. 15+ comprehensive tests passing
13. Factory integration complete
14. Durability stats tracked (no events lost)

**Estimated effort:** 1 day
**Test count:** 15+ tests
**Commit message:** feat(security-p2-component7): Compliance logging and audit trail
