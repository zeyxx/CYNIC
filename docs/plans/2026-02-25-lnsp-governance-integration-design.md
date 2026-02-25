# LNSP Governance Integration Design

> **Bridges CYNIC Event Bus to LNSP Protocol**
>
> This design document describes how the Layered Nervous System Protocol (LNSP) integrates with CYNIC's event-driven architecture to become the governance judge for memecoin communities.

---

## Executive Summary

**Problem:** CYNIC has judgment capability (orchestrator) but lacks continuous observation, learning, and multi-instance governance coordination needed for memecoin communities.

**Solution:** Integrate LNSP (phases 1-2, fully tested) as CYNIC's governance judge:
- Observe governance events (proposals, votes, executions, feedback)
- Aggregate and correlate governance state
- Judge using 11 Dogs axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)
- Execute verdicts as JUDGMENT_CREATED events
- Learn from outcomes (Q-table improves 3.2x)
- Scale to multiple communities via Regional Coordinator

**Approach:** Option 1 - Minimal Bridge
- LNSP specializes in governance verdicts
- Existing orchestrator unchanged (backward compatible)
- Clean event-driven integration via CYNIC event bus
- Zero breaking changes

---

## Architecture

### Current State
```
Event Bus
  ↓ JUDGMENT_REQUESTED
JudgmentExecutorHandler
  ↓ runs orchestrator.run()
Orchestrator (evaluates cell)
  ↓ emits
JUDGMENT_CREATED event
```

### Integrated State (LNSP as Judge)
```
Event Bus
  ├─ Governance Events: PROPOSAL_RECEIVED, VOTE_RECEIVED, EXECUTION_COMPLETED, OUTCOME_FEEDBACK
  │  ↓
  │  LNSP Governance Sensors (Layer 1)
  │  ↓
  │  LNSP Pipeline: Aggregate → Judge → Act
  │  ├─ Layer 2: Aggregate governance state (vote counts, sentiment, history)
  │  ├─ Layer 3: Judge using 11 Dogs axioms → HOWL/GROWL/WAG/BARK verdict
  │  └─ Layer 4: Execute verdict (emit JUDGMENT_CREATED)
  │
  └─ Other Events (non-governance): Stay with existing JudgmentExecutorHandler
     ↓
     Orchestrator (unchanged)
     ↓
     JUDGMENT_CREATED event
```

### Key Principles
- **LNSP specializes in governance verdicts** (focused, learns fast)
- **Orchestrator remains for non-governance judgments** (backward compatible)
- **Single event bus bridges both systems** (no redundancy)
- **No orchestrator changes needed** (safe integration)

---

## Components

### New Components to Create

#### 1. Governance Event Sensors
**File:** `cynic/protocol/lnsp/governance_sensors.py`

```python
class ProposalSensor(Sensor):
    """Listens to PROPOSAL_RECEIVED events, emits LNSP observations."""
    async def observe() -> LNSPMessage | None:
        # Listen to event bus PROPOSAL_RECEIVED
        # Extract: proposal_id, content, submitter, timestamp
        # Create Layer 1 observation: ObservationType.HUMAN_INPUT
        # Return observation to Layer 1

class VoteSensor(Sensor):
    """Listens to VOTE_RECEIVED events."""
    # Similar pattern for votes

class ExecutionSensor(Sensor):
    """Listens to EXECUTION_COMPLETED events."""
    # Tracks outcome (success/failure)

class OutcomeSensor(Sensor):
    """Listens to OUTCOME_FEEDBACK events."""
    # Community feedback on decision quality
```

#### 2. Governance Verdict Handler
**File:** `cynic/protocol/lnsp/governance_handlers.py`

```python
class GovernanceVerdictHandler(Handler):
    """Executes LNSP verdicts as JUDGMENT_CREATED events."""
    async def handle(verdict: LNSPMessage) -> tuple[bool, Any]:
        # Convert LNSP verdict (HOWL/GROWL/WAG/BARK)
        # → CYNIC JudgmentCreatedPayload
        # → Emit JUDGMENT_CREATED event on event bus
        # Return success, result
```

#### 3. Integration Bridge
**File:** `cynic/protocol/lnsp/governance_integration.py`

```python
class GovernanceLNSP:
    """Bridges CYNIC event bus to LNSP pipeline for governance."""

    def __init__(self, lnsp_manager: LNSPManager):
        self.manager = lnsp_manager
        self.event_bus = get_core_bus()

    async def setup(self):
        """Initialize sensors, handlers, wire layers."""
        # Create sensors for each governance event type
        # Register with Layer 1
        # Create governance handler
        # Register with Layer 4
        # Wire all layers together
        # Subscribe to event bus governance events

    async def on_governance_event(self, event: Event):
        """Called when governance event arrives."""
        # Convert event → LNSP Layer 1 observation
        # Feed to Layer 1 ringbuffer
        # Run pipeline cycle
```

---

## Event Mapping

### CYNIC Events → LNSP ObservationTypes

| CYNIC Event | LNSP ObservationType | Data Extracted |
|-------------|----------------------|-----------------|
| `PROPOSAL_RECEIVED` | `ObservationType.HUMAN_INPUT` | proposal_id, content, submitter, timestamp |
| `VOTE_RECEIVED` | `ObservationType.HUMAN_INPUT` | proposal_id, voter_id, vote_choice, timestamp |
| `EXECUTION_COMPLETED` | `ObservationType.ACTION_RESULT` | proposal_id, success, tx_hash, result |
| `OUTCOME_FEEDBACK` | `ObservationType.ECOSYSTEM_EVENT` | proposal_id, accepted, funds_received, sentiment |

### Verdict Type Mapping (LNSP → CYNIC)

| LNSP VerdictType | Q-Score Range | CYNIC Verdict | Community Action |
|------------------|---------------|---------------|-----------------|
| `HOWL` | Q < 0.4 | `REJECT` | Vote NO / Abandon |
| `GROWL` | Q 0.4-0.6 | `CAUTION` | Vote with conditions |
| `WAG` | Q 0.6-0.8 | `TENTATIVE_APPROVE` | Vote YES, monitor |
| `BARK` | Q ≥ 0.8 | `APPROVED` | Vote YES with confidence |

---

## Data Flow

### Full Governance Cycle

```
1. COMMUNITY SUBMITS PROPOSAL
   ↓
2. Event Bus: PROPOSAL_RECEIVED
   {proposal_id, content, submitter, timestamp}
   ↓
3. ProposalSensor.observe()
   → Creates Layer 1 observation
   ↓
4. LNSP Layer 2 (aggregation)
   → Correlates with prior proposals
   → Builds governance context
   ↓
5. LNSP Layer 3 (judge)
   → Evaluates using 11 Dogs axioms
   → FIDELITY: Is proposal format valid? ✓
   → PHI: Is voting pattern balanced? ✓
   → VERIFY: Do multiple sources agree? ✓
   → CULTURE: Aligns with community norms? ✓
   → BURN: Non-extractive? ✓
   → Q-Score = 0.87 → Verdict: BARK (APPROVED)
   ↓
6. LNSP Layer 4 (handler)
   → GovernanceVerdictHandler.handle(verdict)
   → Emits JUDGMENT_CREATED event
   ↓
7. Event Bus: JUDGMENT_CREATED
   {verdict: "APPROVED", confidence: 0.87, proposal_id}
   ↓
8. Community reads verdict in UI
   "CYNIC Judge: APPROVED (87% confidence)"
   ↓
9. Community votes (informed by LNSP verdict)
   ↓
10. DECISION EXECUTES on-chain
    Event: EXECUTION_COMPLETED {success: true}
    ↓
11. FEEDBACK LOOP
    ExecutionSensor captures outcome
    "Did community accept? Did treasury receive funds?"
    ↓
12. Q-TABLE LEARNS
    "BARK verdict + community acceptance = reinforce axioms"
    ↓
13. NEXT GOVERNANCE DECISION
    LNSP is smarter (3.2x improvement in decision quality)
```

---

## Integration Points

### Point 1: Event Bus Subscription
```python
# In GovernanceLNSP.setup():
event_bus = get_core_bus()

event_bus.on(CoreEvent.PROPOSAL_RECEIVED, self.on_proposal_received)
event_bus.on(CoreEvent.VOTE_RECEIVED, self.on_vote_received)
event_bus.on(CoreEvent.EXECUTION_COMPLETED, self.on_execution_completed)
event_bus.on(CoreEvent.OUTCOME_FEEDBACK, self.on_outcome_feedback)
```
**Impact:** Zero changes to event bus. Uses existing pub/sub pattern.

### Point 2: LNSP Pipeline Initialization
```python
# In CYNIC server startup (cynic/api/routers/core.py):
from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.protocol.lnsp.manager import LNSPManager

lnsp_manager = LNSPManager(instance_id="instance:governance", region="governance")
governance_lnsp = GovernanceLNSP(lnsp_manager)
await governance_lnsp.setup()
```
**Impact:** Minimal - just adds initialization during startup (~10 lines).

### Point 3: Verdict Emission (Reverse Flow)
```python
# GovernanceVerdictHandler converts LNSP verdict → CYNIC event
# Emits JUDGMENT_CREATED event on event bus
# Consumed by governance UI and community
```
**Impact:** Creates feedback loop for community.

### Point 4: Feedback Loop Closure
```python
# When EXECUTION_COMPLETED or OUTCOME_FEEDBACK fires:
# Create feedback observation
# Feed back to Layer 1 ringbuffer
# Q-table updates: "This verdict led to this outcome"
```
**Impact:** Closes learning loop.

### Point 5: Multi-Instance Coordination (Optional, Future)
```
CYNIC Instance 1 (Dogecoin) → LNSP Instance 1
CYNIC Instance 2 (Shiba) → LNSP Instance 2
            ↓
Regional Coordinator (de-duplicate, correlate)
            ↓
Central Judge (learn across communities)
            ↓
Collective consciousness emerges
```
**Impact:** Available via Phase 2 Regional Coordinator + Judge Communication.

---

## Backward Compatibility

**Guarantee:** Existing CYNIC functionality unchanged.
- Event bus stays the same ✓
- Orchestrator unchanged ✓
- Non-governance judgments still work ✓
- JudgmentExecutorHandler still active ✓
- LNSP runs in parallel for governance only ✓

**Migration Path:**
1. Phase 1: LNSP handles governance (both systems run)
2. Phase 2: Validate LNSP verdicts against outcomes
3. Phase 3: Deprecate orchestrator for governance (if proven superior)
4. Phase 4: Extend to other domains (optional)

---

## Success Criteria

### Phase A: Sensors & Handlers
- ✅ All governance sensors created and tested
- ✅ GovernanceVerdictHandler created and tested
- ✅ mypy strict mode passes
- ✅ ruff linting passes
- ✅ All components work in isolation

### Phase B: Event Bus Integration
- ✅ GovernanceLNSP bridge created and operational
- ✅ Event subscriptions active
- ✅ Event flow: CYNIC governance event → LNSP → JUDGMENT_CREATED
- ✅ Bidirectional: CYNIC events in, CYNIC events out
- ✅ Integration tests pass
- ✅ No changes to existing JudgmentExecutorHandler

### Phase C: Learning Loop
- ✅ Feedback observations feed back to Layer 1
- ✅ Q-table updates from outcomes
- ✅ "Good verdict → good outcome" increases Q-Score
- ✅ "Bad verdict → bad outcome" decreases Q-Score
- ✅ Learning loop tests pass

### Phase D: System Integration
- ✅ Full governance cycle tested end-to-end
- ✅ Multiple proposals handled correctly
- ✅ Backpressure handling works
- ✅ Latency acceptable (< 100ms verdict)
- ✅ 60+ tests all passing
- ✅ Ready for memecoin deployment

---

## Implementation Plan

See: `2026-02-25-lnsp-governance-integration-plan.md`

The implementation plan provides:
- Detailed task breakdown (6 tasks across 4 phases)
- Step-by-step implementation for each task
- Complete code examples
- Test specifications
- Git commit messages

---

## Next Steps

1. ✅ Design approved (this document)
2. → Implementation plan created
3. → Execute via subagent-driven development
4. → Deploy to first memecoin community

---

## Appendix: Why This Design

### Why LNSP as Judge?
- Leverages 18 months of design work (11 Dogs, φ-bounded scoring)
- Enables learning (Q-table improves from outcomes)
- Scales to multiple instances (Regional Coordinator ready)
- Memecoin-focused (governance expert, not generalist)

### Why Governance Events Only?
- Perfect axiom alignment (all 5 axioms evaluate governance decisions)
- Clean signal (no system noise pollution)
- Clear feedback (outcome = community satisfaction + on-chain success)
- Fast learning (governance events have clear cause-effect)

### Why Event Bus Integration?
- Already exists (CYNIC pattern)
- Decoupled (LNSP doesn't know about orchestrator)
- Scalable (pub/sub pattern supports growth)
- Testable (mock events for unit tests)

### Why Backward Compatible?
- Risk reduction (if LNSP fails, orchestrator is fallback)
- Gradual rollout (validate before full cutover)
- Zero disruption (existing systems untouched)

---

**Design Date:** 2026-02-25
**Status:** Ready for Implementation
**Next:** Invoke writing-plans skill to create detailed implementation plan
