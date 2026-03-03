# PHASE 2: COMPONENT 4 — Real-Time Detection Decomposition

## Problem Statement

**Detect attacks AS THEY HAPPEN, not in retrospect.**

Currently:
- Events are persisted to SurrealDB (Component 2 ✅)
- But no mechanism watches the stream in real-time
- Attacks go undetected until post-mortem analysis

**Solution: Real-Time Detection** — Two-layer approach:
1. **LIVE SELECT**: Stream-based rules (< 100ms latency)
2. **Baseline Correlation**: Offline analysis (every minute)

---

## DOMAIN EXPERTISE DECOMPOSITION

### 1️⃣ BACKEND ENGINEER: Event Stream Processing

**Problem:** How do we watch the security_event table in real-time without polling?

**Solution: SurrealDB LIVE SELECT**

SurrealDB's LIVE SELECT feature:
```sql
LIVE SELECT * FROM security_event
WHERE type = "judgment_created"
THEN { /* callback */ };
```

This is NOT polling. It's a **stream subscription**:
- Server notifies client on every insert/update
- Sub-100ms latency
- Efficient (WebSocket, no polling overhead)

**Architecture:**
```python
class StreamDetector:
    """Watch security_event table via LIVE SELECT."""

    async def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.db = storage._db  # SurrealDB client
        self._rules: dict[str, Rule] = {}

    async def watch(self, event_type: str) -> None:
        """Subscribe to events of a specific type."""
        query = f"""
            LIVE SELECT * FROM security_event
            WHERE type = '{event_type}'
            THEN {{
                -- callback fired on each new event
            }}
        """
        # Register callback with SurrealDB
        await self.db.query(query)
```

**Callback Handler:**
```python
async def on_event_detected(self, event: dict) -> None:
    """Fired when LIVE SELECT detects new event."""
    # 1. Correlate with recent events (< 5 min window)
    related = await self.storage.security_events.correlate(event)

    # 2. Run detection rules
    for rule in self._rules.values():
        if await rule.matches(event, related):
            # 3. Generate alert
            alert = await self._create_alert(event, rule)
            await self._emit_alert(alert)
```

**Validation Checklist:**
- [ ] LIVE SELECT query works on security_event table
- [ ] Callback fires on new events (tested with 10+ events)
- [ ] Latency < 100ms (measured)
- [ ] Multiple LIVE SELECT subscriptions work in parallel
- [ ] Handles connection loss gracefully
- [ ] No events missed (drift test)

---

### 2️⃣ DATA ENGINEER: Baseline Calculation & Event Windowing

**Problem:** How do we know what's "normal" so we can detect anomalies?

**Solution: Baseline Calculator**

Compute metrics from historical events:
```python
class BaselineCalculator:
    """Calculate baseline metrics for anomaly detection."""

    async def calculate_baselines(self) -> dict[str, Any]:
        """Calculate baselines from last 1 hour of events."""
        # Get recent events
        events = await self.storage.security_events.list_events(
            filters={
                "timestamp_gte": time.time() - 3600,  # Last hour
            },
            limit=10000,
        )

        return {
            # Voting velocity (votes per minute)
            "voting_velocity_baseline": self._calc_voting_velocity(events),

            # Proposal value (median, percentiles)
            "proposal_value_median": self._calc_proposal_value(events),
            "proposal_value_p95": self._percentile(events, 95),

            # Consensus variance (dog agreement level)
            "consensus_variance_baseline": self._calc_consensus_variance(events),

            # New actor rate (first-time voters)
            "new_actor_rate": self._calc_new_actor_rate(events),

            # Actor activity pattern (when do they typically vote?)
            "actor_activity_hours": self._calc_activity_hours(events),
        }

    def _calc_voting_velocity(self, events: list) -> float:
        """Events per minute from same actor."""
        actor_counts = defaultdict(int)
        for event in events:
            if event.get("type") == "governance_vote":
                actor_id = event.get("actor_id")
                actor_counts[actor_id] += 1

        # Median voting rate
        if actor_counts:
            return statistics.median(actor_counts.values()) / 60  # Per second
        return 0.0
```

**Window Semantics:**
- **Baseline window**: Last 1 hour (rolling)
- **Anomaly window**: Last 5 minutes
- **Event correlation window**: Configurable (default 300 seconds)

**Validation Checklist:**
- [ ] Baseline calculator processes 10,000+ events
- [ ] Handles missing data gracefully (sparse actors)
- [ ] Metrics update every 10 minutes
- [ ] Percentile calculations accurate (p95, p99)
- [ ] Actor activity patterns captured
- [ ] New actors identified correctly

---

### 3️⃣ INFRASTRUCTURE ENGINEER: Low-Latency Detection

**Problem:** How to detect attacks in < 100ms without blocking the event stream?

**Solution: Async Anomaly Scoring**

```python
class AnomalyScorer:
    """Score events for anomalies in real-time."""

    async def score(
        self,
        event: dict,
        related_events: list,
        baselines: dict,
    ) -> dict[str, float]:
        """
        Score event across multiple dimensions.
        Returns: {
            "voting_velocity_score": 0.0-1.0,
            "proposal_value_score": 0.0-1.0,
            "consensus_variance_score": 0.0-1.0,
            "new_actor_score": 0.0-1.0,
            "composite_anomaly_score": 0.0-1.0,
        }
        """
        scores = {}

        # 1. VOTING VELOCITY ANOMALY
        # Is this actor voting abnormally fast?
        recent_votes = [e for e in related_events if e["actor_id"] == event["actor_id"]]
        vote_rate = len(recent_votes) / 5  # Events in last 5 min
        baseline_rate = baselines["voting_velocity_baseline"]

        if baseline_rate > 0:
            ratio = vote_rate / baseline_rate
            scores["voting_velocity_score"] = min(ratio / 10, 1.0)  # 10x = max score

        # 2. PROPOSAL VALUE ANOMALY
        # Is the proposal unusually large?
        proposal_value = event.get("payload", {}).get("proposal_value", 0)
        baseline_value = baselines["proposal_value_median"]
        p95_value = baselines["proposal_value_p95"]

        if baseline_value > 0:
            ratio = proposal_value / baseline_value
            if ratio > 20:  # 20x median = definitely anomalous
                scores["proposal_value_score"] = 1.0
            elif ratio > 10:
                scores["proposal_value_score"] = 0.8
            else:
                scores["proposal_value_score"] = max(ratio / 10, 0)

        # 3. CONSENSUS VARIANCE ANOMALY
        # Are dogs suspiciously agreeing?
        variance = await self._calc_variance(related_events)
        baseline_variance = baselines["consensus_variance_baseline"]

        if baseline_variance > 0:
            ratio = variance / baseline_variance
            # Low variance = suspicious (too perfect consensus)
            if ratio < 0.1:
                scores["consensus_variance_score"] = 1.0
            elif ratio < 0.5:
                scores["consensus_variance_score"] = 0.8
            else:
                scores["consensus_variance_score"] = 0

        # 4. NEW ACTOR ANOMALY
        # Is this a first-time voter?
        is_new_actor = await self._is_new_actor(event["actor_id"])
        if is_new_actor:
            scores["new_actor_score"] = 0.6  # Moderately suspicious

        # 5. COMPOSITE SCORE (geometric mean)
        all_scores = list(scores.values())
        if all_scores:
            composite = (functools.reduce(operator.mul, all_scores)) ** (1 / len(all_scores))
            scores["composite_anomaly_score"] = min(composite, 1.0)
        else:
            scores["composite_anomaly_score"] = 0.0

        return scores

    async def _calc_variance(self, events: list) -> float:
        """Calculate consensus variance from events."""
        # Simplified: count dog agreements
        dog_verdicts = defaultdict(int)
        for event in events:
            verdict = event.get("payload", {}).get("verdict")
            if verdict:
                dog_verdicts[verdict] += 1

        # Variance = how split are the verdicts?
        total = sum(dog_verdicts.values())
        if total == 0:
            return 0

        proportions = [count / total for count in dog_verdicts.values()]
        variance = statistics.variance(proportions) if len(proportions) > 1 else 0
        return variance

    async def _is_new_actor(self, actor_id: str) -> bool:
        """Check if actor is voting for first time."""
        events = await self.storage.security_events.list_events(
            filters={"actor_id": actor_id},
            limit=2,
        )
        return len(events) <= 1  # If only 1 event, this is first vote
```

**Performance Targets:**
- Scoring latency: < 50ms
- Baseline calculation: < 200ms
- LIVE SELECT callback: < 100ms
- Total detection latency: < 100ms

**Validation Checklist:**
- [ ] Scoring latency < 50ms (measured with 1000 events)
- [ ] All score dimensions working
- [ ] Composite score calculation correct
- [ ] New actor detection working
- [ ] Baseline updates don't block detection

---

### 4️⃣ SECURITY ARCHITECT: Kill Chain Rule Matching

**Problem:** How do we detect attacks across the 7-stage Kill Chain?

**Solution: Rule Engine**

```python
class DetectionRule:
    """Base class for detection rules."""

    def __init__(self, rule_id: str, severity: str = "MEDIUM"):
        self.rule_id = rule_id
        self.severity = severity  # LOW, MEDIUM, HIGH, CRITICAL

    async def matches(
        self,
        event: dict,
        related_events: list,
        anomaly_scores: dict,
        baselines: dict,
    ) -> bool:
        """Check if event matches this rule."""
        raise NotImplementedError


# Stage 1: RECONNAISSANCE (Finding targets)
class ReconnaissanceRule(DetectionRule):
    """Detect if attacker is scanning for targets."""

    async def matches(self, event, related, scores, baselines) -> bool:
        """
        Trigger: API scan detected (100+ API calls in 5 min from single IP)
        """
        if event.get("type") != "api_request":
            return False

        actor_id = event.get("actor_id")
        api_calls = [e for e in related if e.get("actor_id") == actor_id]

        # 100+ calls in 5 min = scan
        return len(api_calls) > 100


# Stage 2: WEAPONIZATION (Preparing attack)
class WeaponizationRule(DetectionRule):
    """Detect if attacker is preparing attack."""

    async def matches(self, event, related, scores, baselines) -> bool:
        """
        Trigger: Suspicious proposal
        - Value > 10x median
        - Execution delay < 1 hour
        - From new actor
        """
        if event.get("type") != "proposal_created":
            return False

        payload = event.get("payload", {})

        # Check value
        value = payload.get("proposal_value", 0)
        baseline_value = baselines.get("proposal_value_median", 0)
        if baseline_value > 0 and value / baseline_value < 10:
            return False  # Value OK

        # Check execution delay
        execution_delay = payload.get("execution_delay_hours", 0)
        if execution_delay >= 1:
            return False  # Delay OK

        # Check if new actor
        is_new = await self._is_new_actor(event["actor_id"])
        if not is_new:
            return False  # Actor established

        return True  # All conditions met


# Stage 3-7: DELIVERY, EXPLOITATION, INSTALLATION, C&C, ACTIONS
# Similar patterns...
```

**Full Kill Chain Coverage:**
1. **Reconnaissance** (API scanning)
2. **Weaponization** (Suspicious proposals)
3. **Delivery** (Large voting blocs)
4. **Exploitation** (Consensus manipulation)
5. **Installation** (Persistent actor presence)
6. **Command & Control** (Coordinated voting patterns)
7. **Actions on Objectives** (Execution of malicious proposals)

**Validation Checklist:**
- [ ] 7 kill chain stages covered
- [ ] 25-50 total rules defined
- [ ] Each rule tested with synthetic attack
- [ ] False positive rate < 5%
- [ ] Rule thresholds tuned on historical data
- [ ] Rules independently testable

---

### 5️⃣ SRE: Monitoring, Alerting, Incident Response

**Problem:** How do we escalate detections into actionable alerts?

**Solution: Alert Pipeline**

```python
class Alert:
    """Security alert from detection rule."""

    def __init__(
        self,
        alert_id: str,
        rule_id: str,
        severity: str,
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

    async def route(self) -> list[str]:
        """Route to appropriate channels based on severity."""
        channels = []

        if self.severity == "CRITICAL":
            channels.extend(["ops_team", "pagerduty", "slack"])
        elif self.severity == "HIGH":
            channels.extend(["security_team", "jira"])
        elif self.severity == "MEDIUM":
            channels.append("slack")
        else:
            channels.append("log_only")

        return channels


class AlertDeduplicator:
    """Prevent alert spam by deduplicating similar alerts."""

    def __init__(self, dedup_window_sec: float = 300):
        self.dedup_window = dedup_window_sec
        self.recent_alerts: dict[str, float] = {}  # rule_id -> timestamp

    async def should_alert(self, rule_id: str) -> bool:
        """Check if we should fire alert or suppress as duplicate."""
        now = time.time()
        last_alert_time = self.recent_alerts.get(rule_id, 0)

        if now - last_alert_time < self.dedup_window:
            return False  # Suppress duplicate

        self.recent_alerts[rule_id] = now
        return True


class AlertGenerator:
    """Generate and route alerts."""

    async def __init__(self, storage: StorageInterface):
        self.storage = storage
        self.deduplicator = AlertDeduplicator(dedup_window_sec=300)

    async def emit_alert(
        self,
        rule_id: str,
        severity: str,
        event: dict,
        related_events: list,
        anomaly_scores: dict,
    ) -> None:
        """Generate alert and route to appropriate channels."""
        # Check deduplication
        if not await self.deduplicator.should_alert(rule_id):
            logger.debug(f"Alert deduplicated: {rule_id}")
            return

        # Create alert
        alert = Alert(
            alert_id=str(uuid.uuid4()),
            rule_id=rule_id,
            severity=severity,
            event=event,
            related_events=related_events,
            anomaly_scores=anomaly_scores,
        )

        # Route
        channels = await alert.route()
        for channel in channels:
            await self._route_to_channel(channel, alert)

        # Log alert
        logger.warning(f"Alert fired: {alert.alert_id} (rule={rule_id}, severity={severity})")

    async def _route_to_channel(self, channel: str, alert: Alert) -> None:
        """Route alert to specific channel."""
        if channel == "log_only":
            return  # Already logged

        # TODO: Implement routing to each channel
        # pagerduty.trigger(alert)
        # slack.notify(alert)
        # jira.create_issue(alert)
        # ops_team.notify(alert)
```

**Validation Checklist:**
- [ ] Alerts route to correct channels by severity
- [ ] Deduplication prevents alert spam
- [ ] Alert contains all context (event, related, scores)
- [ ] Routing latency < 1s

---

### 6️⃣ SOLUTIONS ARCHITECT: System Integration

**Problem:** How does Real-Time Detection fit into PHASE 2?

**Architecture Context:**
```
PHASE 2 Data Flow:
EventBus (Component 2)
    ↓
EventForwarder (Component 2)
    ↓
SurrealDB security_event table (Component 1/3)
    ↓
Real-Time Detection (Component 4) ← YOU ARE HERE
    ├─ LIVE SELECT (< 100ms latency)
    ├─ Baseline Calculator (every 10 min)
    ├─ Anomaly Scorer (per event)
    ├─ Rule Engine (Kill Chain matching)
    └─ Alert Generator (routing & dedup)
         ↓
Detection Rules (Component 5)
    ↓
Alerting & Escalation (Component 6)
    ↓
Compliance Logging (Component 7)
```

**Wiring:**
```python
# In factory.py
class _OrganismAwakener:
    async def build(self):
        # ... existing code ...

        # 0i. REAL-TIME DETECTION (PHASE 2: Component 4)
        self.stream_detector = None
        self.baseline_calculator = None
        self.anomaly_scorer = None
        self.alert_generator = None

        try:
            from cynic.kernel.core.storage.realtime_detection import (
                StreamDetector,
                BaselineCalculator,
                AnomalyScorer,
                AlertGenerator,
            )

            if self.storage:
                self.stream_detector = StreamDetector(
                    storage=self.storage,
                    event_forwarder=self.event_forwarder,
                )
                self.baseline_calculator = BaselineCalculator(
                    storage=self.storage,
                )
                self.anomaly_scorer = AnomalyScorer(
                    storage=self.storage,
                    baseline_calculator=self.baseline_calculator,
                )
                self.alert_generator = AlertGenerator(
                    storage=self.storage,
                )

                logger.info("Factory: Real-Time Detection initialized (PHASE 2)")
        except Exception as e:
            logger.warning(f"Factory: Real-Time Detection not available: {e}")
```

**Validation Checklist:**
- [ ] StreamDetector wired to EventBus via EventForwarder
- [ ] BaselineCalculator runs every 10 minutes
- [ ] AnomalyScorer called on each event
- [ ] AlertGenerator receives alerts from rules
- [ ] Factory initialization complete
- [ ] Organism.start() starts detection
- [ ] Organism.stop() stops detection gracefully

---

## Implementation Order (Critical Path)

### Week 2: Real-Time Detection (2 days)

**Day 1: Core Detection Engine**
- [ ] StreamDetector + LIVE SELECT
- [ ] BaselineCalculator (rolling 1-hour window)
- [ ] AnomalyScorer (5-dimension scoring)
- [ ] 10 tests: LIVE SELECT, baselines, scoring

**Day 2: Rule Engine & Alerting**
- [ ] DetectionRule base class
- [ ] 7 Kill Chain rules (Reconnaissance → Actions)
- [ ] AlertGenerator + deduplication
- [ ] Factory integration
- [ ] 10 tests: rule matching, alerting

**Total: 20 tests, 1 commit**

---

## Validation by Domain

| Domain | Key Questions | Acceptance Criteria |
|--------|---|---|
| **Backend** | Does LIVE SELECT work? | Callback fires on events, < 100ms latency |
| **Data Engineer** | Are baselines calculated correctly? | Percentiles match historical distribution |
| **Infrastructure** | Scoring latency < 50ms? | Load test with 1000 events |
| **Security** | All 7 kill chain stages covered? | 25-50 rules, < 5% false positive rate |
| **SRE** | Alerts route correctly? | Severity levels → correct channels |
| **Solutions** | Fits into PHASE 2 pipeline? | Factory wired, Organism integrated |

---

## Success Criteria

✅ **Component 4 is DONE when:**
1. StreamDetector watches security_event table via LIVE SELECT
2. LIVE SELECT callbacks trigger in < 100ms
3. BaselineCalculator computes metrics from last 1 hour
4. AnomalyScorer scores events across 5 dimensions
5. DetectionRule engine matches 7 Kill Chain stages
6. AlertGenerator routes by severity (CRITICAL/HIGH/MEDIUM/LOW)
7. AlertDeduplicator prevents alert spam (< 5% duplicate rate)
8. 20 comprehensive tests passing
9. Factory integration complete + Organism lifecycle wired
10. No regression in EventForwarder or SecurityEventRepo

**Estimated effort:** 2 days
**Test count:** 20 tests
**Commit message:** feat(security-p2-component4): Real-time threat detection engine
