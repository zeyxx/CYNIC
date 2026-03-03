"""
Alerting & Escalation Engine — L1/L2/L3 incident triage (PHASE 2, COMPONENT 6)

Architecture:
  RuleMatch → Alert → Deduplicator → Queue → Router → Channels
                           ↓
                    EscalationPath (L1→L2→L3)
"""

from __future__ import annotations

import asyncio
import logging
import time
import uuid
from collections import defaultdict
from enum import Enum
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.kernel.core.storage.interface import StorageInterface

logger = logging.getLogger("cynic.storage.alerting")


class AlertSeverity(Enum):
    """Alert severity levels with routing implications."""

    LOW = "LOW"           # Log only
    MEDIUM = "MEDIUM"     # Slack notification
    HIGH = "HIGH"         # Slack + Jira ticket
    CRITICAL = "CRITICAL" # Ops + PagerDuty + Slack + Jira


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
                        {"title": "Actor", "value": self.event.get("actor_id", "unknown"), "short": True},
                        {"title": "Timestamp", "value": str(self.timestamp), "short": True},
                        {
                            "title": "Composite Anomaly Score",
                            "value": f"{self.anomaly_scores.get('composite', 0):.2%}",
                        },
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
                "description": (
                    f"Alert: {self.alert_id}\n"
                    f"Rule: {self.rule_id}\n"
                    f"Actor: {self.event.get('actor_id', 'unknown')}\n"
                    f"Timestamp: {self.timestamp}"
                ),
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


class AlertDeduplicator:
    """Prevent alert spam by deduplicating similar alerts."""

    def __init__(self, dedup_window_sec: float = 300, dedup_threshold: float = 0.9):
        self.dedup_window = dedup_window_sec
        self.threshold = dedup_threshold
        self.recent_alerts: list[Alert] = []
        self.deduplicated_count = 0

    async def should_route(self, alert: Alert) -> bool:
        """Check if this alert should be routed or deduplicated."""
        now = time.time()

        # Clean up old alerts
        self.recent_alerts = [
            a for a in self.recent_alerts
            if now - a.timestamp < self.dedup_window
        ]

        # Check similarity with recent alerts
        for recent_alert in self.recent_alerts:
            similarity = self._calculate_similarity(alert, recent_alert)
            if similarity > self.threshold:
                logger.debug(f"Alert deduplicated: {alert.alert_id} (similar to {recent_alert.alert_id})")
                self.deduplicated_count += 1
                return False

        # New alert
        self.recent_alerts.append(alert)
        return True

    def _calculate_similarity(self, alert1: Alert, alert2: Alert) -> float:
        """Calculate similarity between two alerts."""
        # Same rule + same actor = duplicate
        if (
            alert1.rule_id == alert2.rule_id
            and alert1.event.get("actor_id") == alert2.event.get("actor_id")
        ):
            return 0.95

        # Same rule = likely duplicate
        if alert1.rule_id == alert2.rule_id:
            return 0.8

        # Different rules
        return 0.0

    def get_stats(self) -> dict:
        """Get deduplication statistics."""
        return {
            "recent_alerts": len(self.recent_alerts),
            "dedup_window": self.dedup_window,
            "deduplicated_count": self.deduplicated_count,
        }


class AlertAggregator:
    """Aggregate related alerts for situational awareness."""

    async def aggregate(self, alerts: list[Alert]) -> dict[str, list[Alert]]:
        """Group related alerts by actor."""
        grouped = defaultdict(list)

        for alert in alerts:
            actor = alert.event.get("actor_id", "unknown")
            grouped[actor].append(alert)

        return dict(grouped)


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

    async def dequeue(self) -> Alert | None:
        """Get alert from queue."""
        try:
            return await asyncio.wait_for(self.queue.get(), timeout=1.0)
        except asyncio.TimeoutError:
            return None

    async def process_all(self, router: AlertRouter) -> None:
        """Process all queued alerts."""
        while not self.queue.empty():
            alert = await self.dequeue()
            if alert:
                await router.route(alert)
                self.processed += 1

    def get_stats(self) -> dict:
        """Get queue statistics."""
        return {
            "queue_size": self.queue.qsize(),
            "max_size": self.max_size,
            "processed": self.processed,
            "failed": self.failed,
        }


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
                elif channel in ["ops_team", "security_team"]:
                    await self._route_to_team(alert, channel)

                results[channel] = True
            except Exception as e:
                logger.error(f"Failed to route to {channel}: {e}")
                results[channel] = False

        return results

    async def _route_to_log(self, alert: Alert) -> None:
        """Log the alert."""
        logger.warning(
            f"Alert: {alert.alert_id} | Rule: {alert.rule_id} | "
            f"Severity: {alert.severity.value} | Actor: {alert.event.get('actor_id')}"
        )

    async def _route_to_slack(self, alert: Alert) -> None:
        """Send to Slack webhook."""
        # TODO: Implement Slack webhook
        logger.debug(f"Would route to Slack: {alert.alert_id}")

    async def _route_to_jira(self, alert: Alert) -> None:
        """Create Jira ticket."""
        # TODO: Implement Jira API
        logger.debug(f"Would create Jira ticket: {alert.alert_id}")

    async def _route_to_pagerduty(self, alert: Alert) -> None:
        """Trigger PagerDuty incident."""
        # TODO: Implement PagerDuty API
        logger.debug(f"Would trigger PagerDuty: {alert.alert_id}")

    async def _route_to_team(self, alert: Alert, team: str) -> None:
        """Notify team directly."""
        # TODO: Implement team notification
        logger.debug(f"Would notify {team}: {alert.alert_id}")


class AlertAuditLog:
    """Audit trail for all alert operations."""

    def __init__(self, storage: StorageInterface | None = None):
        self.storage = storage

    async def log_alert_created(self, alert: Alert, rule_id: str) -> None:
        """Log when alert is created."""
        if self.storage:
            await self.storage.security_events.save_event({
                "type": "alert_created",
                "alert_id": alert.alert_id,
                "rule_id": rule_id,
                "severity": alert.severity.value,
                "actor_id": alert.event.get("actor_id"),
                "timestamp": time.time(),
            })

    async def log_alert_routed(self, alert: Alert, channel: str, success: bool) -> None:
        """Log when alert is routed."""
        if self.storage:
            await self.storage.security_events.save_event({
                "type": "alert_routed",
                "alert_id": alert.alert_id,
                "channel": channel,
                "success": success,
                "timestamp": time.time(),
            })

    async def log_alert_acknowledged(self, alert: Alert, acknowledger: str) -> None:
        """Log when analyst acknowledges alert."""
        if self.storage:
            await self.storage.security_events.save_event({
                "type": "alert_acknowledged",
                "alert_id": alert.alert_id,
                "acknowledger": acknowledger,
                "timestamp": time.time(),
            })

    async def log_alert_closed(self, alert: Alert, reason: str, closer: str) -> None:
        """Log when alert is closed."""
        if self.storage:
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

        # Return highest escalation level reached
        escalation_level = None
        for stage in self.stages:
            if age_minutes >= stage["delay_minutes"]:
                escalation_level = stage["level"]

        return escalation_level

    async def escalate(self, alert: Alert, from_level: str, to_level: str) -> None:
        """Escalate alert to next level."""
        logger.warning(f"Alert {alert.alert_id} escalated from {from_level} to {to_level}")


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
