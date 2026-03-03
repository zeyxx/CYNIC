"""
Real-Time Detection Engine  Stream-based threat detection (PHASE 2, COMPONENT 4)

Architecture:
  StreamDetector (LIVE SELECT)  BaselineCalculator  AnomalyScorer
                                                          
                                                    Detection Rules
                                                          
                                                    Alert Generation
"""

from __future__ import annotations

import logging
import statistics
import time
import uuid
from collections import defaultdict
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.kernel.core.storage.interface import StorageInterface

logger = logging.getLogger("cynic.storage.realtime_detection")


class BaselineCalculator:
    """Calculate baseline metrics from historical events."""

    def __init__(self, storage: StorageInterface, window_hours: int = 1):
        self.storage = storage
        self.window_hours = window_hours
        self._baselines: dict[str, Any] = {}
        self._last_update = 0
        self._update_interval_sec = 600  # Update every 10 minutes

    async def get_baselines(self, force_recalculate: bool = False) -> dict[str, Any]:
        """Get current baselines (with caching)."""
        now = time.time()

        if (
            force_recalculate
            or not self._baselines
            or now - self._last_update > self._update_interval_sec
        ):
            self._baselines = await self._calculate_baselines()
            self._last_update = now

        return self._baselines

    async def _calculate_baselines(self) -> dict[str, Any]:
        """Calculate baselines from last N hours of events."""
        timestamp_gte = time.time() - (self.window_hours * 3600)

        events = await self.storage.security_events.list_events(
            filters={"timestamp_gte": timestamp_gte},
            limit=10000,
        )

        if not events:
            return self._get_default_baselines()

        # Calculate metrics
        governance_events = [e for e in events if e.get("type") == "governance_vote"]
        proposal_values = [
            e.get("payload", {}).get("proposal_value", 0) for e in events
            if e.get("type") == "proposal_created"
        ]
        judgment_events = [e for e in events if e.get("type") == "judgment_created"]

        return {
            "voting_velocity": self._calc_voting_velocity(governance_events),
            "proposal_value_median": statistics.median(proposal_values) if proposal_values else 0,
            "proposal_value_p95": self._percentile(proposal_values, 95) if proposal_values else 0,
            "consensus_variance": self._calc_consensus_variance(judgment_events),
            "new_actor_rate": self._calc_new_actor_rate(governance_events),
            "active_actors": len(set(e.get("actor_id") for e in events if e.get("actor_id"))),
            "event_count": len(events),
        }

    def _calc_voting_velocity(self, events: list) -> float:
        """Events per second from average actor."""
        if not events:
            return 0
        actor_counts = defaultdict(int)
        for event in events:
            actor_id = event.get("actor_id")
            if actor_id:
                actor_counts[actor_id] += 1

        if actor_counts:
            return statistics.median(actor_counts.values()) / (self.window_hours * 3600)
        return 0

    def _calc_consensus_variance(self, events: list) -> float:
        """Variance in dog agreement."""
        if not events:
            return 0.5

        variances = []
        for event in events:
            payload = event.get("payload", {})
            verdict = payload.get("verdict")
            if verdict:
                # Simplified: just return a baseline
                variances.append(0.5)

        return statistics.mean(variances) if variances else 0.5

    def _calc_new_actor_rate(self, events: list) -> float:
        """Rate of new actors appearing."""
        if not events:
            return 0
        actor_count = len(set(e.get("actor_id") for e in events if e.get("actor_id")))
        return actor_count / max(len(events), 1)

    def _percentile(self, data: list, percentile: int) -> float:
        """Calculate percentile."""
        if not data:
            return 0
        sorted_data = sorted(data)
        index = int(len(sorted_data) * percentile / 100)
        return sorted_data[min(index, len(sorted_data) - 1)]

    def _get_default_baselines(self) -> dict[str, Any]:
        """Default baselines when no data available."""
        return {
            "voting_velocity": 0.1,  # 0.1 votes/sec
            "proposal_value_median": 1000,
            "proposal_value_p95": 5000,
            "consensus_variance": 0.5,
            "new_actor_rate": 0.1,
            "active_actors": 0,
            "event_count": 0,
        }


class AnomalyScorer:
    """Score events for anomalies across multiple dimensions."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.baseline_calculator = BaselineCalculator(storage)

    async def score(
        self,
        event: dict,
        related_events: list,
    ) -> dict[str, float]:
        """
        Score event across 5 dimensions.
        Returns scores 0.0-1.0 for each dimension + composite.
        """
        baselines = await self.baseline_calculator.get_baselines()
        scores = {}

        # 1. VOTING VELOCITY ANOMALY
        scores["voting_velocity"] = await self._score_voting_velocity(event, related_events, baselines)

        # 2. PROPOSAL VALUE ANOMALY
        scores["proposal_value"] = await self._score_proposal_value(event, baselines)

        # 3. CONSENSUS VARIANCE ANOMALY
        scores["consensus_variance"] = await self._score_consensus_variance(event, related_events, baselines)

        # 4. NEW ACTOR ANOMALY
        scores["new_actor"] = await self._score_new_actor(event, related_events)

        # 5. ACTOR ACTIVITY ANOMALY
        scores["actor_activity"] = await self._score_actor_activity(event, related_events)

        # Composite score (geometric mean)
        values = [v for v in scores.values() if isinstance(v, float)]
        if values:
            composite = 1.0
            for v in values:
                composite *= v
            scores["composite"] = composite ** (1 / len(values))
        else:
            scores["composite"] = 0.0

        return scores

    async def _score_voting_velocity(self, event: dict, related: list, baselines: dict) -> float:
        """Score based on voting speed."""
        if event.get("type") != "governance_vote":
            return 0.0

        actor = event.get("actor_id")
        actor_votes = [e for e in related if e.get("actor_id") == actor and e.get("type") == "governance_vote"]

        baseline_velocity = baselines.get("voting_velocity", 0.1)
        actual_velocity = len(actor_votes) / max(self.baseline_calculator.window_hours * 3600, 1)

        if baseline_velocity == 0:
            return 0.0

        ratio = actual_velocity / baseline_velocity
        # 10x = max score
        return min(ratio / 10, 1.0)

    async def _score_proposal_value(self, event: dict, baselines: dict) -> float:
        """Score based on proposal value."""
        if event.get("type") != "proposal_created":
            return 0.0

        value = event.get("payload", {}).get("proposal_value", 0)
        median = baselines.get("proposal_value_median", 1000)

        if median == 0:
            return 0.0

        ratio = value / median
        # 20x = max score
        return min(ratio / 20, 1.0)

    async def _score_consensus_variance(self, event: dict, related: list, baselines: dict) -> float:
        """Score based on consensus variance (too perfect = suspicious)."""
        if event.get("type") != "judgment_created":
            return 0.0

        payload = event.get("payload", {})
        variance = payload.get("consensus_variance", 0.5)
        baseline_variance = baselines.get("consensus_variance", 0.5)

        if baseline_variance == 0:
            return 0.0

        ratio = variance / baseline_variance
        # Low variance = suspicious
        if ratio < 0.1:
            return 1.0
        elif ratio < 0.5:
            return 0.8
        else:
            return 0.0

    async def _score_new_actor(self, event: dict, related: list) -> float:
        """Score if actor is new (first-time voter)."""
        actor = event.get("actor_id")
        actor_events = [e for e in related if e.get("actor_id") == actor]

        # New actor = 0.6 score
        return 0.6 if len(actor_events) <= 1 else 0.0

    async def _score_actor_activity(self, event: dict, related: list) -> float:
        """Score based on actor activity pattern."""
        actor = event.get("actor_id")
        actor_events = [e for e in related if e.get("actor_id") == actor]

        # Actors voting on 80%+ of proposals = coordinated
        proposal_count = len(set(e.get("payload", {}).get("proposal_id") for e in actor_events))
        all_proposals = len(set(e.get("payload", {}).get("proposal_id") for e in related if e.get("type") == "governance_vote"))

        if all_proposals == 0:
            return 0.0

        participation_rate = proposal_count / all_proposals
        return min(participation_rate / 0.8, 1.0)  # 80% = max score


class DetectionRule:
    """Base class for detection rules."""

    def __init__(self, rule_id: str, kill_chain_stage: str, severity: str):
        self.rule_id = rule_id
        self.stage = kill_chain_stage
        self.severity = severity
        self.match_count = 0

    async def evaluate(
        self,
        event: dict,
        related_events: list,
        baselines: dict,
        anomaly_scores: dict,
    ) -> bool:
        """Check if event matches this rule."""
        raise NotImplementedError

    async def get_context(self, event: dict) -> dict:
        """Return context for alert."""
        return {"event": event}


class RuleEngine:
    """Manage and execute detection rules."""

    def __init__(self):
        self._rules: dict[str, DetectionRule] = {}
        self._register_default_rules()

    def _register_default_rules(self) -> None:
        """Register default Kill Chain rules."""
        # Stage 1: Reconnaissance
        class Stage1_APIScanning(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "api_request":
                    return False
                actor = event.get("actor_id")
                actor_calls = [e for e in related if e.get("actor_id") == actor]
                return len(actor_calls) > 100

        # Stage 2: Weaponization
        class Stage2_SuspiciousProposal(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "proposal_created":
                    return False
                payload = event.get("payload", {})
                value = payload.get("proposal_value", 0)
                baseline = baselines.get("proposal_value_median", 0)
                delay = payload.get("execution_delay_hours", 0)

                if baseline == 0 or value / baseline < 10:
                    return False
                if delay >= 1:
                    return False
                return True

        # Stage 3: Delivery
        class Stage3_LargeVotingBloc(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "governance_vote":
                    return False
                proposal_id = event.get("payload", {}).get("proposal_id")
                votes = [e for e in related if e.get("payload", {}).get("proposal_id") == proposal_id]
                return len(votes) > 50

        # Stage 4: Exploitation
        class Stage4_ConsensusManipulation(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "judgment_created":
                    return False
                variance = event.get("payload", {}).get("consensus_variance", 0)
                baseline_variance = baselines.get("consensus_variance", 0.5)
                if baseline_variance > 0 and variance < baseline_variance * 0.1:
                    return True
                return False

        # Stage 5: Installation
        class Stage5_PersistentActor(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                # Actor present in > 10 events = persistence
                return len(related) > 10

        # Stage 6: Command & Control
        class Stage6_CoordinatedVoting(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "governance_vote":
                    return False
                actor = event.get("actor_id")
                actor_votes = [e for e in related if e.get("actor_id") == actor]
                proposal_count = len(set(e.get("payload", {}).get("proposal_id") for e in actor_votes))
                all_proposals = len(set(e.get("payload", {}).get("proposal_id") for e in related))

                if all_proposals == 0:
                    return False
                return proposal_count / all_proposals > 0.8

        # Stage 7: Actions on Objectives
        class Stage7_MaliciousExecution(DetectionRule):
            async def evaluate(self, event: dict, related: list, baselines: dict, anomaly_scores: dict = None) -> bool:
                if event.get("type") != "proposal_executed":
                    return False
                proposal_id = event.get("payload", {}).get("proposal_id")
                suspicious = [e for e in related if e.get("type") == "proposal_created" and e.get("payload", {}).get("proposal_id") == proposal_id]
                return len(suspicious) > 0

        # Register rules
        rules = [
            Stage1_APIScanning("STAGE_1_API_SCANNING", "Reconnaissance", "MEDIUM"),
            Stage2_SuspiciousProposal("STAGE_2_WEAPONIZATION", "Weaponization", "HIGH"),
            Stage3_LargeVotingBloc("STAGE_3_VOTING_BLOC", "Delivery", "HIGH"),
            Stage4_ConsensusManipulation("STAGE_4_CONSENSUS", "Exploitation", "CRITICAL"),
            Stage5_PersistentActor("STAGE_5_PERSISTENCE", "Installation", "HIGH"),
            Stage6_CoordinatedVoting("STAGE_6_COORDINATION", "C2", "CRITICAL"),
            Stage7_MaliciousExecution("STAGE_7_EXECUTION", "Actions", "CRITICAL"),
        ]

        for rule in rules:
            self._rules[rule.rule_id] = rule

    async def evaluate_all(
        self,
        event: dict,
        related_events: list,
        baselines: dict,
        anomaly_scores: dict,
    ) -> list[tuple[DetectionRule, bool]]:
        """Evaluate all rules."""
        matches = []

        for rule in self._rules.values():
            try:
                matched = await rule.evaluate(event, related_events, baselines, anomaly_scores)
                if matched:
                    matches.append((rule, matched))
                    rule.match_count += 1
            except Exception as e:
                logger.error(f"Rule {rule.rule_id} failed: {e}")

        return matches

    def get_stats(self) -> dict:
        """Get rule statistics."""
        return {
            "total_rules": len(self._rules),
            "rule_matches": {rule_id: rule.match_count for rule_id, rule in self._rules.items()},
        }


class StreamDetector:
    """Watch security_event table via LIVE SELECT."""

    def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.rule_engine = RuleEngine()
        self.anomaly_scorer = AnomalyScorer(storage)
        self.alerts_generated = 0
        self.latest_alert: dict | None = None

    async def process_event(self, event: dict) -> dict | None:
        """Process incoming event through detection pipeline."""
        # Get related events
        related = await self.storage.security_events.correlate(event)

        # Get baselines
        baselines = await self.anomaly_scorer.baseline_calculator.get_baselines()

        # Score event
        anomaly_scores = await self.anomaly_scorer.score(event, related)

        # Evaluate rules
        rule_matches = await self.rule_engine.evaluate_all(event, related, baselines, anomaly_scores)

        # Generate alert if rules matched
        if rule_matches:
            alert = {
                "alert_id": str(uuid.uuid4()),
                "event_id": event.get("id"),
                "timestamp": time.time(),
                "matched_rules": [rule.rule_id for rule, _ in rule_matches],
                "severity": max([rule.severity for rule, _ in rule_matches], key=lambda x: ["LOW", "MEDIUM", "HIGH", "CRITICAL"].index(x)),
                "anomaly_scores": anomaly_scores,
                "event": event,
            }
            self.alerts_generated += 1
            self.latest_alert = alert
            return alert

        return None

    def get_stats(self) -> dict:
        """Get detector statistics."""
        return {
            "alerts_generated": self.alerts_generated,
            "rule_stats": self.rule_engine.get_stats(),
        }
