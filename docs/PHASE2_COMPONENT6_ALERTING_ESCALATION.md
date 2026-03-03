# PHASE 2: COMPONENT 6 — Alerting & Escalation Decomposition

## Problem Statement

**From threat detection → human incident response**

Rules fire alerts. Alerts need routing to appropriate teams with right severity.

L1/L2/L3 Incident Response:
- **L1**: Automated triage (is this real?)
- **L2**: Human investigation (what happened?)
- **L3**: Incident response (what do we do?)

---

## DOMAIN EXPERTISE DECOMPOSITION

### 1️⃣ BACKEND ENGINEER: Alert Routing Pipeline

```python
class AlertSeverity(Enum):
    LOW = "LOW"           # Log only
    MEDIUM = "MEDIUM"     # Slack notification
    HIGH = "HIGH"         # Slack + Jira ticket
    CRITICAL = "CRITICAL" # Ops + PagerDuty + Slack

class Alert:
    """Security alert with full context."""

    def __init__(
        self,
        alert_id: str,
        rule_id: str,
        severity: AlertSeverity,
        event: dict,
        related_events: list,
        anomaly_scores: dict,
    ):
        self.alert_id = alert_id
        self.rule_id = rule_id
        self.severity = severity
        self.event = event
        self.related_events = related_events
        self.anomaly_scores = anomaly_scores
        self.timestamp = time.time()
        self.status = "OPEN"

    def get_routing_channels(self) -> list[str]:
        """Determine routing channels based on severity."""
        routing = {
            AlertSeverity.CRITICAL: ["ops_team", "pagerduty", "slack", "jira"],
            AlertSeverity.HIGH: ["security_team", "slack", "jira"],
            AlertSeverity.MEDIUM: ["slack"],
            AlertSeverity.LOW: ["log"],
        }
        return routing.get(self.severity, ["log"])

    def to_slack_message(self) -> dict:
        """Format alert for Slack."""
        return {
            "text": f"⚠️ Security Alert: {self.rule_id}",
            "attachments": [
                {
                    "color": self._severity_color(),
                    "fields": [
                        {"title": "Severity", "value": self.severity.value, "short": True},
                        {"title": "Rule", "value": self.rule_id, "short": True},
                        {"title": "Actor", "value": self.event.get("actor_id"), "short": True},
                        {"title": "Timestamp", "value": str(self.timestamp), "short": True},
                        {"title": "Anomaly Score", "value": f"{self.anomaly_scores.get('composite_anomaly_score', 0):.2%}"},
                    ],
                }
            ],
        }

    def to_jira_issue(self) -> dict:
        """Format alert as Jira issue."""
        return {
            "fields": {
                "project": {"key": "SECURITY"},
                "issuetype": {"name": "Bug"},
                "summary": f"[{self.severity.value}] {self.rule_id}",
                "description": f"Alert: {self.alert_id}\nRule: {self.rule_id}\nActor: {self.event.get('actor_id')}",
                "priority": self._severity_to_priority(),
            }
        }

    def _severity_color(self) -> str:
        """Slack color for severity."""
        return {
            AlertSeverity.CRITICAL: "danger",
            AlertSeverity.HIGH: "warning",
            AlertSeverity.MEDIUM: "good",
            AlertSeverity.LOW: "info",
        }.get(self.severity, "info")

    def _severity_to_priority(self) -> str:
        """Jira priority for severity."""
        return {
            AlertSeverity.CRITICAL: "Highest",
            AlertSeverity.HIGH: "High",
            AlertSeverity.MEDIUM: "Medium",
            AlertSeverity.LOW: "Lowest",
        }.get(self.severity, "Medium")


class AlertRouter:
    """Route alerts to appropriate channels."""

    async def route(self, alert: Alert) -> dict[str, bool]:
        """Route alert to all configured channels."""
        channels = alert.get_routing_channels()
        results = {}

        for channel in channels:
            try:
                if channel == "log":
                    await self._route_to_log(alert)
                elif channel == "slack":
                    await self._route_to_slack(alert)
                elif channel == "jira":
                    await self._route_to_jira(alert)
                elif channel == "pagerduty":
                    await self._route_to_pagerduty(alert)
                elif channel == "ops_team":
                    await self._route_to_ops_team(alert)

                results[channel] = True
            except Exception as e:
                logger.error(f"Failed to route to {channel}: {e}")
                results[channel] = False

        return results

    async def _route_to_log(self, alert: Alert) -> None:
        """Log the alert."""
        logger.warning(f"Alert: {alert.alert_id} | Rule: {alert.rule_id} | Severity: {alert.severity.value}")

    async def _route_to_slack(self, alert: Alert) -> None:
        """Send to Slack webhook."""
        # TODO: Implement Slack webhook
        pass

    async def _route_to_jira(self, alert: Alert) -> None:
        """Create Jira ticket."""
        # TODO: Implement Jira API
        pass

    async def _route_to_pagerduty(self, alert: Alert) -> None:
        """Trigger PagerDuty incident."""
        # TODO: Implement PagerDuty API
        pass

    async def _route_to_ops_team(self, alert: Alert) -> None:
        """Notify ops team directly."""
        # TODO: Implement ops notification
        pass
```

**Validation Checklist:**
- [ ] Routing channels mapped to severity
- [ ] All 5 routing channels functional
- [ ] Alert formatting correct for each channel
- [ ] Error handling for failed routes

---

### 2️⃣ DATA ENGINEER: Alert Deduplication & Aggregation

```python
class AlertDeduplicator:
    """Prevent alert spam by deduplicating similar alerts."""

    def __init__(self, dedup_window_sec: float = 300, dedup_threshold: float = 0.9):
        self.dedup_window = dedup_window_sec
        self.threshold = dedup_threshold  # Similarity threshold
        self.recent_alerts: list[Alert] = []

    async def should_route(self, alert: Alert) -> bool:
        """Check if this alert should be routed or deduplicated."""
        # Clean up old alerts
        now = time.time()
        self.recent_alerts = [
            a for a in self.recent_alerts
            if now - a.timestamp < self.dedup_window
        ]

        # Check similarity with recent alerts
        for recent_alert in self.recent_alerts:
            similarity = self._calculate_similarity(alert, recent_alert)
            if similarity > self.threshold:
                logger.debug(f"Alert deduplicated: {alert.alert_id} (similar to {recent_alert.alert_id})")
                return False

        # New alert
        self.recent_alerts.append(alert)
        return True

    def _calculate_similarity(self, alert1: Alert, alert2: Alert) -> float:
        """Calculate similarity between two alerts."""
        # Same rule + same actor = duplicate
        if alert1.rule_id == alert2.rule_id and alert1.event.get("actor_id") == alert2.event.get("actor_id"):
            return 0.95  # Very similar

        # Same severity + same rule = likely duplicate
        if alert1.rule_id == alert2.rule_id:
            return 0.8

        # Different rules = not duplicate
        return 0.0

    def get_stats(self) -> dict:
        """Get deduplication statistics."""
        return {
            "recent_alerts": len(self.recent_alerts),
            "dedup_window": self.dedup_window,
        }


class AlertAggregator:
    """Aggregate related alerts for better situational awareness."""

    async def aggregate(self, alerts: list[Alert]) -> dict[str, list[Alert]]:
        """Group related alerts."""
        grouped = defaultdict(list)

        for alert in alerts:
            # Group by actor for coordinated attack detection
            actor = alert.event.get("actor_id", "unknown")
            grouped[actor].append(alert)

        return dict(grouped)
```

**Validation Checklist:**
- [ ] Deduplication detects similar alerts
- [ ] Aggregation groups related alerts
- [ ] False deduplication < 1%
- [ ] Statistics accurate

---

### 3️⃣ INFRASTRUCTURE ENGINEER: Performance & Reliability

```python
class AlertQueue:
    """Queue alerts for reliable delivery."""

    def __init__(self, max_queue_size: int = 1000):
        self.max_size = max_queue_size
        self.queue: asyncio.Queue = asyncio.Queue(maxsize=max_queue_size)
        self.processed = 0
        self.failed = 0

    async def enqueue(self, alert: Alert) -> bool:
        """Add alert to queue."""
        try:
            await asyncio.wait_for(self.queue.put(alert), timeout=1.0)
            return True
        except asyncio.TimeoutError:
            logger.error(f"Alert queue full, dropping alert {alert.alert_id}")
            self.failed += 1
            return False

    async def dequeue(self) -> Alert:
        """Get alert from queue."""
        return await self.queue.get()

    async def process_all(self, router: AlertRouter) -> None:
        """Process all queued alerts."""
        while not self.queue.empty():
            try:
                alert = await asyncio.wait_for(self.dequeue(), timeout=1.0)
                await router.route(alert)
                self.processed += 1
            except asyncio.TimeoutError:
                break

    def get_stats(self) -> dict:
        """Get queue statistics."""
        return {
            "queue_size": self.queue.qsize(),
            "max_size": self.max_size,
            "processed": self.processed,
            "failed": self.failed,
        }
```

**Performance Targets:**
- Alert routing latency: < 100ms
- Queue throughput: 100+ alerts/sec
- Failed routing recovery: < 1%

**Validation Checklist:**
- [ ] Queueing latency < 100ms
- [ ] Throughput > 100 alerts/sec
- [ ] Queue doesn't overflow (backpressure)
- [ ] Failed routes recovered

---

### 4️⃣ SECURITY ARCHITECT: Access Control & Audit Trail

```python
class AlertAuditLog:
    """Audit trail for all alert operations."""

    async def __init__(self, storage: StorageInterface):
        self.storage = storage

    async def log_alert_created(self, alert: Alert, rule: Rule) -> None:
        """Log when alert is created."""
        await self.storage.security_events.save_event({
            "type": "alert_created",
            "alert_id": alert.alert_id,
            "rule_id": rule.rule_id,
            "severity": alert.severity.value,
            "actor_id": alert.event.get("actor_id"),
            "timestamp": time.time(),
        })

    async def log_alert_routed(self, alert: Alert, channel: str, success: bool) -> None:
        """Log when alert is routed."""
        await self.storage.security_events.save_event({
            "type": "alert_routed",
            "alert_id": alert.alert_id,
            "channel": channel,
            "success": success,
            "timestamp": time.time(),
        })

    async def log_alert_acknowledged(self, alert: Alert, acknowledger: str) -> None:
        """Log when analyst acknowledges alert."""
        await self.storage.security_events.save_event({
            "type": "alert_acknowledged",
            "alert_id": alert.alert_id,
            "acknowledger": acknowledger,
            "timestamp": time.time(),
        })

    async def log_alert_closed(self, alert: Alert, reason: str, closer: str) -> None:
        """Log when alert is closed."""
        await self.storage.security_events.save_event({
            "type": "alert_closed",
            "alert_id": alert.alert_id,
            "reason": reason,
            "closer": closer,
            "timestamp": time.time(),
        })


class AlertAccessControl:
    """Control who can view/manage alerts."""

    def __init__(self):
        self.roles = {
            "analyst_l1": {"view": True, "acknowledge": True, "close": False},
            "analyst_l2": {"view": True, "acknowledge": True, "close": True},
            "security_lead": {"view": True, "acknowledge": True, "close": True},
        }

    def can_view_alert(self, user_role: str, alert: Alert) -> bool:
        """Check if user can view alert."""
        return self.roles.get(user_role, {}).get("view", False)

    def can_acknowledge_alert(self, user_role: str) -> bool:
        """Check if user can acknowledge alert."""
        return self.roles.get(user_role, {}).get("acknowledge", False)

    def can_close_alert(self, user_role: str) -> bool:
        """Check if user can close alert."""
        return self.roles.get(user_role, {}).get("close", False)
```

**Validation Checklist:**
- [ ] Audit trail captures all operations
- [ ] Access control enforced
- [ ] Roles defined correctly
- [ ] Compliance requirements met

---

### 5️⃣ SRE: Monitoring & Escalation Paths

```python
class EscalationPath:
    """Escalation chain for unresolved alerts."""

    def __init__(self):
        self.stages = [
            {"level": "L1", "delay_minutes": 15, "role": "analyst_l1"},
            {"level": "L2", "delay_minutes": 30, "role": "analyst_l2"},
            {"level": "L3", "delay_minutes": 60, "role": "security_lead"},
        ]

    async def check_escalation(self, alert: Alert) -> str | None:
        """Check if alert needs escalation."""
        age_minutes = (time.time() - alert.timestamp) / 60

        for stage in self.stages:
            if age_minutes >= stage["delay_minutes"]:
                return stage["level"]

        return None

    async def escalate(self, alert: Alert, from_level: str, to_level: str) -> None:
        """Escalate alert to next level."""
        logger.warning(f"Alert {alert.alert_id} escalated from {from_level} to {to_level}")
        # TODO: Notify next level


class AlertMetrics:
    """Export metrics for alerting system."""

    def __init__(self):
        self.alerts_created = 0
        self.alerts_routed = 0
        self.alerts_deduplicated = 0
        self.alerts_escalated = 0
        self.routing_failures = 0

    def get_metrics(self) -> dict:
        """Get alerting system metrics."""
        return {
            "alerts_created": self.alerts_created,
            "alerts_routed": self.alerts_routed,
            "alerts_deduplicated": self.alerts_deduplicated,
            "alerts_escalated": self.alerts_escalated,
            "routing_failures": self.routing_failures,
            "failure_rate": (
                self.routing_failures / max(self.alerts_routed, 1)
                if self.alerts_routed > 0
                else 0
            ),
        }
```

**Validation Checklist:**
- [ ] Escalation paths triggered correctly
- [ ] Metrics exported (Prometheus)
- [ ] No alerts stuck in system

---

### 6️⃣ SOLUTIONS ARCHITECT: L1/L2/L3 Triage Workflow

**L1 Triage (Automated):**
```
Alert created
    ↓
Deduplication (is this new?)
    ↓
Routing to channels
    ↓
If suspicious: mark for L2
```

**L2 Investigation (Human):**
```
Alert in Slack/Jira
    ↓
Analyst reviews context
    ↓
Options:
  - Acknowledge (true positive)
  - Close as false positive
  - Escalate to L3
```

**L3 Response (Incident):**
```
L2 escalates to L3
    ↓
Security lead initiates incident response
    ↓
Forensics, containment, recovery
    ↓
Post-incident review
```

**Integration:**
```
Component 5 (Rules)
    ↓ Generate alerts
    ↓
Component 6 (Alerting) ← YOU ARE HERE
    ├─ AlertRouter (to channels)
    ├─ AlertDeduplicator (prevent spam)
    ├─ AlertQueue (reliable delivery)
    ├─ AlertAuditLog (compliance)
    └─ EscalationPath (L1→L2→L3)
         ↓
Component 7 (Compliance Logging)
```

**Validation Checklist:**
- [ ] L1/L2/L3 workflow working
- [ ] Alerts flow to all channels
- [ ] Escalation path triggered
- [ ] Audit trail complete

---

## Implementation Plan

**1 day of work:**
- Alert class + routing
- AlertDeduplicator + AlertAggregator
- AlertQueue + AlertRouter
- AlertAuditLog + AccessControl
- EscalationPath + Metrics
- Factory integration
- 20+ tests

**Total: 20 tests, 1 commit**

---

## Success Criteria

✅ **Component 6 is DONE when:**
1. Alerts route to correct channels by severity
2. CRITICAL → ops + PagerDuty + Slack + Jira
3. HIGH → security team + Slack + Jira
4. MEDIUM → Slack
5. LOW → Log only
6. Deduplication prevents alert spam
7. Queue handles 100+ alerts/sec
8. Audit trail captures all operations
9. L1/L2/L3 escalation workflow works
10. 20+ comprehensive tests passing
11. Factory integration complete
12. Metrics exported (Prometheus)

**Estimated effort:** 1 day
**Test count:** 20+ tests
**Commit message:** feat(security-p2-component6): Alert routing and L1/L2/L3 escalation
