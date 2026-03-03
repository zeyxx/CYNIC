# PHASE 2: COMPONENT 5 — Detection Rules Decomposition

## Problem Statement

**From detection engine → actionable threats across Kill Chain**

COMPONENT 4 detects anomalies. COMPONENT 5 interprets them as attacks.

Kill Chain stages (Lockheed Martin):
1. **Reconnaissance** - Finding targets
2. **Weaponization** - Preparing attack
3. **Delivery** - Transporting weapon
4. **Exploitation** - Triggering vulnerability
5. **Installation** - Establishing persistence
6. **Command & Control** - Remote communication
7. **Actions on Objectives** - Achieving goal

---

## DOMAIN EXPERTISE DECOMPOSITION

### 1️⃣ BACKEND ENGINEER: Rule Engine & Execution

```python
class Rule:
    """Base detection rule matching Kill Chain pattern."""

    def __init__(self, rule_id: str, kill_chain_stage: str, severity: str):
        self.rule_id = rule_id
        self.stage = kill_chain_stage
        self.severity = severity
        self.matched_count = 0

    async def evaluate(
        self,
        event: dict,
        related_events: list,
        baselines: dict,
    ) -> bool:
        """Check if event matches this rule."""
        raise NotImplementedError

    async def get_context(self) -> dict:
        """Return context for alert (event details, related, scores)."""
        return {}


class RuleEngine:
    """Manage and execute detection rules."""

    def __init__(self):
        self._rules: dict[str, Rule] = {}
        self._rules_by_stage: dict[str, list[Rule]] = defaultdict(list)

    def register(self, rule: Rule) -> None:
        """Register a rule."""
        self._rules[rule.rule_id] = rule
        self._rules_by_stage[rule.stage].append(rule)

    async def evaluate_all(
        self,
        event: dict,
        related: list,
        baselines: dict,
    ) -> list[tuple[Rule, bool]]:
        """Evaluate all applicable rules."""
        matches = []

        # Find rules for this event type
        applicable = self._get_applicable_rules(event)

        for rule in applicable:
            try:
                matched = await rule.evaluate(event, related, baselines)
                if matched:
                    matches.append((rule, True))
                    rule.matched_count += 1
            except Exception as e:
                logger.error(f"Rule {rule.rule_id} failed: {e}")

        return matches

    def _get_applicable_rules(self, event: dict) -> list[Rule]:
        """Get rules applicable to this event type."""
        event_type = event.get("type")
        # Rules are generally applicable to all events, but some are type-specific
        return list(self._rules.values())

    def get_stats(self) -> dict:
        """Get rule execution statistics."""
        return {
            "total_rules": len(self._rules),
            "rules_by_stage": {
                stage: len(rules) for stage, rules in self._rules_by_stage.items()
            },
            "rule_matches": {
                rule_id: rule.matched_count
                for rule_id, rule in self._rules.items()
                if rule.matched_count > 0
            },
        }
```

**Validation Checklist:**
- [ ] Rule registration works
- [ ] All applicable rules evaluated
- [ ] Matches collected correctly
- [ ] Statistics tracked
- [ ] Error handling in evaluation

---

### 2️⃣ DATA ENGINEER: Kill Chain Rule Patterns

```python
# STAGE 1: RECONNAISSANCE
class Stage1_APIScanning(Rule):
    """Detect if attacker is scanning APIs for targets."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "api_request":
            return False

        actor = event.get("actor_id")
        recent_calls = [e for e in related if e.get("actor_id") == actor]

        # 100+ API calls in 5 min = scanning
        return len(recent_calls) > 100


# STAGE 2: WEAPONIZATION
class Stage2_SuspiciousProposal(Rule):
    """Detect if attacker is preparing attack."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "proposal_created":
            return False

        payload = event.get("payload", {})

        # Large value + short delay + new actor
        value = payload.get("proposal_value", 0)
        baseline = baselines.get("proposal_value_median", 0)

        if baseline > 0 and value / baseline < 10:
            return False

        delay = payload.get("execution_delay_hours", 0)
        if delay >= 1:
            return False

        return True


# STAGE 3: DELIVERY
class Stage3_LargeVotingBloc(Rule):
    """Detect coordinated voting attack (consensus manipulation)."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "governance_vote":
            return False

        proposal_id = event.get("payload", {}).get("proposal_id")
        votes_for_proposal = [
            e for e in related if e.get("payload", {}).get("proposal_id") == proposal_id
        ]

        # 50+ votes in 5 min = bloc voting
        return len(votes_for_proposal) > 50


# STAGE 4: EXPLOITATION
class Stage4_ConsensusManipulation(Rule):
    """Detect if dogs are being manipulated (vote rigging)."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "judgment_created":
            return False

        payload = event.get("payload", {})
        variance = payload.get("consensus_variance", 0)
        baseline_variance = baselines.get("consensus_variance_baseline", 0)

        # Variance < 10% of baseline = suspiciously perfect consensus
        if baseline_variance > 0 and variance < baseline_variance * 0.1:
            return True

        return False


# STAGE 5: INSTALLATION
class Stage5_PersistentActor(Rule):
    """Detect if attacker is establishing persistent presence."""

    async def evaluate(self, event, related, baselines) -> bool:
        actor = event.get("actor_id")

        # Actor present in > 10 events over days = persistence
        return len(related) > 10


# STAGE 6: COMMAND & CONTROL
class Stage6_CoordinatedVoting(Rule):
    """Detect coordinated voting patterns (C2 communication)."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "governance_vote":
            return False

        actor = event.get("actor_id")
        actor_votes = [e for e in related if e.get("actor_id") == actor]

        # Same actor voting every proposal = coordination
        proposal_count = len(set(
            e.get("payload", {}).get("proposal_id") for e in actor_votes
        ))

        # Actor voting on > 80% of proposals = coordinated
        all_proposals = len(set(
            e.get("payload", {}).get("proposal_id") for e in related
        ))

        if all_proposals > 0 and proposal_count / all_proposals > 0.8:
            return True

        return False


# STAGE 7: ACTIONS ON OBJECTIVES
class Stage7_MaliciousProposalExecution(Rule):
    """Detect if malicious proposal was executed."""

    async def evaluate(self, event, related, baselines) -> bool:
        if event.get("type") != "proposal_executed":
            return False

        # Check if this proposal was flagged as suspicious
        proposal_id = event.get("payload", {}).get("proposal_id")
        suspicious_proposals = [
            e for e in related
            if e.get("type") == "proposal_created"
            and e.get("payload", {}).get("proposal_id") == proposal_id
        ]

        # If we detected it as weaponization earlier, this is exploitation
        return len(suspicious_proposals) > 0
```

**Validation Checklist:**
- [ ] 7 rules covering all Kill Chain stages
- [ ] Each rule has clear trigger condition
- [ ] Thresholds tuned on historical data
- [ ] False positive rate < 5%
- [ ] Rules tested with synthetic attacks

---

### 3️⃣ INFRASTRUCTURE ENGINEER: Performance & Scaling

**Rule Execution Optimization:**

```python
class RuleExecutor:
    """Execute rules with performance monitoring."""

    def __init__(self, rule_engine: RuleEngine):
        self.engine = rule_engine
        self.rule_latencies: dict[str, list[float]] = defaultdict(list)

    async def execute(
        self,
        event: dict,
        related: list,
        baselines: dict,
    ) -> list[tuple[Rule, dict]]:
        """Execute all rules and track performance."""
        results = []

        matches = await self.engine.evaluate_all(event, related, baselines)

        for rule, matched in matches:
            if matched:
                context = await rule.get_context()
                results.append((rule, context))

        return results
```

**Performance Targets:**
- Rule evaluation: < 100ms total for all rules
- Per-rule latency: < 10ms
- Throughput: 100+ rules/sec

**Validation Checklist:**
- [ ] Rule evaluation < 100ms
- [ ] Per-rule latency < 10ms
- [ ] No memory leaks (track rule state)
- [ ] Concurrent rule execution safe

---

### 4️⃣ SECURITY ARCHITECT: Attack Pattern Coverage

**Kill Chain Mapping:**

| Stage | Rule | Pattern | Trigger |
|-------|------|---------|---------|
| 1 | API_SCANNING | Reconnaissance | 100+ API calls in 5 min |
| 2 | SUSPICIOUS_PROPOSAL | Weaponization | Value > 10x, delay < 1h, new actor |
| 3 | VOTING_BLOC | Delivery | 50+ votes in 5 min |
| 4 | CONSENSUS_MANIPULATION | Exploitation | Variance < 10% baseline |
| 5 | PERSISTENT_ACTOR | Installation | > 10 events from same actor |
| 6 | COORDINATED_VOTING | C2 | Same actor on 80%+ proposals |
| 7 | MALICIOUS_EXECUTION | AoO | Flagged proposal executed |

**Additional Rules (Attack Variations):**
- New actor targeting high-value proposals
- Multi-actor coordination patterns
- Proposal value explosion (outliers)
- Governance parameter manipulation
- Treasury address changes
- Consensus agreement anomalies

**Validation Checklist:**
- [ ] All 7 stages covered
- [ ] 25-50 total rules defined
- [ ] Each rule independently testable
- [ ] Rules don't contradict each other
- [ ] Attack scenarios falsifiable

---

### 5️⃣ SRE: Rule Management & Lifecycle

```python
class RuleRegistry:
    """Centralized rule management."""

    def __init__(self):
        self._rules = {}
        self._disabled_rules = set()
        self._rule_versions = defaultdict(list)

    def register(self, rule: Rule) -> None:
        """Register a new rule."""
        self._rules[rule.rule_id] = rule
        self._rule_versions[rule.rule_id].append({
            "version": 1,
            "timestamp": time.time(),
            "status": "active",
        })

    def disable(self, rule_id: str) -> None:
        """Temporarily disable a rule (e.g., too many false positives)."""
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

    def get_stats(self) -> dict:
        """Get registry statistics."""
        return {
            "total_rules": len(self._rules),
            "active_rules": len(self.get_active_rules()),
            "disabled_rules": len(self._disabled_rules),
            "rule_matches": sum(rule.matched_count for rule in self._rules.values()),
        }
```

**Validation Checklist:**
- [ ] Rule registration/disable/enable working
- [ ] Rule versioning tracked
- [ ] Statistics accurate
- [ ] Disabled rules not evaluated

---

### 6️⃣ SOLUTIONS ARCHITECT: Integration with Detection Pipeline

**Flow:**
```
Real-Time Detection (Component 4)
    ↓ AnomalyScorer produces scores
    ↓
Detection Rules (Component 5) ← YOU ARE HERE
    ↓ RuleEngine evaluates all rules
    ↓ Matches → Alert context
    ↓
Alerting & Escalation (Component 6)
    ↓ Route by severity
    ↓
Compliance Logging (Component 7)
```

**Integration Code:**
```python
# In DetectionOrchestrator
async def process_event(self, event: dict) -> None:
    # Get related events & baselines
    related = await self.storage.security_events.correlate(event)
    baselines = await self.baseline_calculator.calculate()

    # Run detection rules
    rule_matches = await self.rule_engine.evaluate_all(event, related, baselines)

    # Generate alerts for matches
    for rule, matched in rule_matches:
        alert = await self._create_alert(rule, event)
        await self.alert_generator.emit_alert(alert)
```

**Validation Checklist:**
- [ ] Rules receive correct input (event, related, baselines)
- [ ] Matches create alerts
- [ ] Integration with Component 4 working
- [ ] Integration with Component 6 ready

---

## Implementation Plan

**1 day of work:**
- Create RuleEngine base class
- Implement 7 Kill Chain rules
- Add 18-25 additional rules
- RuleRegistry & management
- Factory integration
- 20+ tests

**Total: 20 tests, 1 commit**

---

## Validation by Domain

| Domain | Key Question | Acceptance |
|--------|---|---|
| **Backend** | Rule execution works? | All rules evaluate correctly |
| **Data Engineer** | Patterns correct? | Triggers match attack scenarios |
| **Infrastructure** | Performance OK? | < 100ms total, < 10ms per rule |
| **Security** | All stages covered? | 7 stages + 25-50 rules |
| **SRE** | Rules manageable? | Enable/disable/stats working |
| **Solutions** | Fits pipeline? | Factory wired, flows to Component 6 |

---

## Success Criteria

✅ **Component 5 is DONE when:**
1. RuleEngine evaluates all rules correctly
2. 7 Kill Chain stage rules implemented
3. 25-50 total rules covering attack patterns
4. Rule execution latency < 100ms
5. Per-rule latency < 10ms
6. False positive rate < 5%
7. RuleRegistry manages rule lifecycle
8. 20+ comprehensive tests passing
9. Factory integration complete
10. Matches flow to Component 6 alerts

**Estimated effort:** 1 day
**Test count:** 20+ tests
**Commit message:** feat(security-p2-component5): Kill Chain detection rules
