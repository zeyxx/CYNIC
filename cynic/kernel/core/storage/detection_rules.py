"""
Detection Rules Engine — Kill Chain pattern matching (PHASE 2, COMPONENT 5)

Architecture:
  Rule (base class) → RuleRegistry (lifecycle management) → RuleEngine (evaluation)
                           ↓
                    25-50 rules by stage
                           ↓
                    Matches → Alert context
"""

from __future__ import annotations

import logging
import time
from collections import defaultdict
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.kernel.core.storage.interface import StorageInterface

logger = logging.getLogger("cynic.storage.detection_rules")


class Rule:
    """Base class for detection rules."""

    def __init__(self, rule_id: str, kill_chain_stage: str, severity: str):
        self.rule_id = rule_id
        self.stage = kill_chain_stage
        self.severity = severity
        self.match_count = 0
        self.created_at = time.time()

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


class RuleRegistry:
    """Centralized rule management and lifecycle."""

    def __init__(self):
        self._rules: dict[str, Rule] = {}
        self._disabled_rules: set[str] = set()
        self._rule_versions: dict[str, list[dict]] = defaultdict(list)

    def register(self, rule: Rule) -> None:
        """Register a new rule."""
        self._rules[rule.rule_id] = rule
        self._rule_versions[rule.rule_id].append({
            "version": 1,
            "timestamp": time.time(),
            "status": "active",
        })
        logger.info(f"Rule registered: {rule.rule_id} ({rule.stage})")

    def disable(self, rule_id: str) -> None:
        """Temporarily disable a rule (e.g., too many false positives)."""
        if rule_id in self._rules:
            self._disabled_rules.add(rule_id)
            logger.warning(f"Rule disabled: {rule_id}")

    def enable(self, rule_id: str) -> None:
        """Re-enable a disabled rule."""
        self._disabled_rules.discard(rule_id)
        logger.info(f"Rule enabled: {rule_id}")

    def get_active_rules(self) -> list[Rule]:
        """Get all active rules."""
        return [
            rule for rule_id, rule in self._rules.items()
            if rule_id not in self._disabled_rules
        ]

    def get_rules_by_stage(self, stage: str) -> list[Rule]:
        """Get rules for specific Kill Chain stage."""
        return [
            rule for rule in self._rules.values()
            if rule.stage == stage and rule.rule_id not in self._disabled_rules
        ]

    def get_stats(self) -> dict:
        """Get registry statistics."""
        return {
            "total_rules": len(self._rules),
            "active_rules": len(self.get_active_rules()),
            "disabled_rules": len(self._disabled_rules),
            "rules_by_stage": {
                stage: len(self.get_rules_by_stage(stage))
                for stage in set(r.stage for r in self._rules.values())
            },
            "total_matches": sum(rule.match_count for rule in self._rules.values()),
        }


class RuleExecutor:
    """Execute rules with performance monitoring."""

    def __init__(self, registry: RuleRegistry):
        self.registry = registry
        self.rule_latencies: dict[str, list[float]] = defaultdict(list)

    async def execute(
        self,
        event: dict,
        related: list,
        baselines: dict,
        anomaly_scores: dict,
    ) -> list[tuple[Rule, bool]]:
        """Execute all active rules and track performance."""
        results = []

        for rule in self.registry.get_active_rules():
            try:
                start = time.time()
                matched = await rule.evaluate(event, related, baselines, anomaly_scores)
                elapsed = (time.time() - start) * 1000

                self.rule_latencies[rule.rule_id].append(elapsed)

                if matched:
                    results.append((rule, matched))
                    rule.match_count += 1
            except Exception as e:
                logger.error(f"Rule {rule.rule_id} failed: {e}")

        return results

    def get_rule_performance_stats(self) -> dict:
        """Get rule execution performance statistics."""
        stats = {}
        for rule_id, latencies in self.rule_latencies.items():
            if latencies:
                stats[rule_id] = {
                    "count": len(latencies),
                    "min_ms": min(latencies),
                    "max_ms": max(latencies),
                    "avg_ms": sum(latencies) / len(latencies),
                }
        return stats


# ============================================================================
# KILL CHAIN STAGE RULES
# ============================================================================


class Stage1_APIScanning(Rule):
    """Stage 1 (Reconnaissance): Detect API scanning for target enumeration."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "api_request":
            return False

        actor = event.get("actor_id")
        actor_calls = [e for e in related if e.get("actor_id") == actor]

        # 100+ API calls in related window = scanning
        return len(actor_calls) > 100


class Stage2_SuspiciousProposal(Rule):
    """Stage 2 (Weaponization): Detect suspicious proposal creation."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        payload = event.get("payload", {})
        value = payload.get("proposal_value", 0)
        baseline = baselines.get("proposal_value_median", 0)
        delay = payload.get("execution_delay_hours", 0)

        # Large value (10x+) + short delay (< 1 hour) = weaponization
        if baseline == 0 or value / baseline < 10:
            return False
        if delay >= 1:
            return False

        return True


class Stage3_LargeVotingBloc(Rule):
    """Stage 3 (Delivery): Detect large coordinated voting blocs."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        proposal_id = event.get("payload", {}).get("proposal_id")
        votes = [e for e in related if e.get("payload", {}).get("proposal_id") == proposal_id]

        # 50+ votes for same proposal = bloc attack
        return len(votes) > 50


class Stage4_ConsensusManipulation(Rule):
    """Stage 4 (Exploitation): Detect consensus manipulation (suspiciously perfect agreement)."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "judgment_created":
            return False

        variance = event.get("payload", {}).get("consensus_variance", 0)
        baseline_variance = baselines.get("consensus_variance", 0.5)

        # Variance < 10% of baseline = suspiciously perfect consensus
        if baseline_variance > 0 and variance < baseline_variance * 0.1:
            return True

        return False


class Stage5_PersistentActor(Rule):
    """Stage 5 (Installation): Detect persistent actor presence."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        # Actor present in > 10 events = establishing persistence
        return len(related) > 10


class Stage6_CoordinatedVoting(Rule):
    """Stage 6 (Command & Control): Detect coordinated voting patterns."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        actor = event.get("actor_id")
        actor_votes = [e for e in related if e.get("actor_id") == actor]
        proposal_count = len(set(
            e.get("payload", {}).get("proposal_id") for e in actor_votes
        ))
        all_proposals = len(set(
            e.get("payload", {}).get("proposal_id") for e in related
        ))

        if all_proposals == 0:
            return False

        # Actor voting on 80%+ of proposals = coordinated C2
        return proposal_count / all_proposals > 0.8


class Stage7_MaliciousExecution(Rule):
    """Stage 7 (Actions on Objectives): Detect malicious proposal execution."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_executed":
            return False

        proposal_id = event.get("payload", {}).get("proposal_id")
        suspicious = [
            e for e in related
            if e.get("type") == "proposal_created"
            and e.get("payload", {}).get("proposal_id") == proposal_id
        ]

        # If proposal was flagged as suspicious, execution is exploitation
        return len(suspicious) > 0


# ============================================================================
# ADDITIONAL RULES (Attack Variations)
# ============================================================================


class NewActorHighValueProposal(Rule):
    """Detect new actor targeting high-value proposals."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        # New actor (< 2 events total)
        actor = event.get("actor_id")
        actor_events = [e for e in related if e.get("actor_id") == actor]
        if len(actor_events) >= 2:
            return False

        # High value proposal
        value = event.get("payload", {}).get("proposal_value", 0)
        baseline = baselines.get("proposal_value_median", 0)
        if baseline == 0 or value / baseline < 5:
            return False

        return True


class MultiActorCoordination(Rule):
    """Detect coordination patterns between multiple actors."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        proposal_id = event.get("payload", {}).get("proposal_id")
        votes_for_proposal = [
            e for e in related
            if e.get("payload", {}).get("proposal_id") == proposal_id
        ]

        # Count unique actors voting on same proposal
        unique_actors = set(e.get("actor_id") for e in votes_for_proposal)

        # 20+ actors all voting same way = coordination
        return len(unique_actors) > 20


class ProposalValueExplosion(Rule):
    """Detect sudden spike in proposal values (outlier)."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        value = event.get("payload", {}).get("proposal_value", 0)
        baseline = baselines.get("proposal_value_median", 0)

        # Value > 100x median = extreme outlier
        if baseline == 0 or value / baseline < 100:
            return False

        return True


class VotingVelocitySpike(Rule):
    """Detect sudden increase in voting velocity."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        # Check anomaly score for voting velocity
        velocity_score = anomaly_scores.get("voting_velocity", 0)

        # Score > 0.8 indicates significant velocity anomaly
        return velocity_score > 0.8


class GovernanceParameterChange(Rule):
    """Detect proposals that modify governance parameters."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        payload = event.get("payload", {})
        proposal_type = payload.get("type", "")

        # Proposals modifying governance, treasury, or core parameters
        suspicious_types = ["governance", "treasury", "core_parameter", "admin"]
        return proposal_type in suspicious_types


class TreasuryAddressChange(Rule):
    """Detect attempts to change treasury address."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        description = event.get("payload", {}).get("description", "").lower()

        # Check for treasury address change indicators
        treasury_keywords = ["treasury", "address", "fund", "withdraw", "transfer"]
        return any(keyword in description for keyword in treasury_keywords)


class AnomalousConsensusVariance(Rule):
    """Detect unusual agreement patterns (too high or too low variance)."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "judgment_created":
            return False

        variance = event.get("payload", {}).get("consensus_variance", 0.5)
        baseline_variance = baselines.get("consensus_variance", 0.5)

        if baseline_variance == 0:
            return False

        # Check for both extremes: too perfect (< 10%) or too chaotic (> 500%)
        ratio = variance / baseline_variance
        return ratio < 0.1 or ratio > 5.0


class RapidProposalCreation(Rule):
    """Detect rapid creation of multiple proposals."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "proposal_created":
            return False

        actor = event.get("actor_id")
        actor_proposals = [
            e for e in related
            if e.get("type") == "proposal_created" and e.get("actor_id") == actor
        ]

        # 10+ proposals from same actor = proposal spam
        return len(actor_proposals) > 10


class BotVotingPattern(Rule):
    """Detect bot-like voting patterns (identical voting on all proposals)."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        actor = event.get("actor_id")
        actor_votes = [e for e in related if e.get("actor_id") == actor]

        if len(actor_votes) < 5:
            return False

        # Check if all votes have same properties (choice, timestamp pattern)
        choices = [e.get("payload", {}).get("choice") for e in actor_votes]
        unique_choices = set(choices)

        # Bot: voting same way on everything
        return len(unique_choices) == 1


class MissingJustification(Rule):
    """Detect votes without justification (suspicious)."""

    async def evaluate(
        self, event: dict, related: list, baselines: dict, anomaly_scores: dict
    ) -> bool:
        if event.get("type") != "governance_vote":
            return False

        justification = event.get("payload", {}).get("justification", "")

        # Votes without explanation = suspicious (especially in burst)
        if justification and len(justification) > 10:
            return False

        # Check if many votes from same actor lack justification
        actor = event.get("actor_id")
        actor_votes = [e for e in related if e.get("actor_id") == actor]
        no_justification = sum(
            1 for e in actor_votes
            if not e.get("payload", {}).get("justification")
        )

        # > 50% without justification = suspicious
        return len(actor_votes) > 5 and no_justification / len(actor_votes) > 0.5
