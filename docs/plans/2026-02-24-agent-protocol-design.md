# CYNIC Layered Nervous System Protocol (LNSP)

**Date:** 2026-02-24
**Status:** Design Document
**Scope:** Foundational protocol for distributed CYNIC agents
**Version:** 1.0

---

## 1. Problem Statement

CYNIC currently operates as a monolithic event bus within a single instance. To achieve the vision of a **fully distributed, omniscient, omnipotent organism**, we need:

1. **Distributed agents** — All agent types (Dogs, Handlers, Sensors, Judges, Neurons) must run on any instance
2. **Streaming nervous system** — Continuous bidirectional observability and actuation (machine ↔ CYNIC ↔ human)
3. **Intelligent routing** — CYNIC Judge decides what each agent sees based on context, not flooding with all data
4. **Hierarchical filtering** — Raw chaos → aggregated state → judgments → decisions (reduce cognitive load)
5. **Emergence detection** — Identify novel patterns and system-level behaviors
6. **Scale both ways** — Works on single machine today, scales to internet-scale without architectural rewrites

---

## 2. Architecture Overview

### Layered Nervous System Protocol (LNSP)

CYNIC's agent communication happens across **4 protocol layers**, each with distinct responsibilities:

```
┌─────────────────────────────────────────────────────────┐
│ LAYER 4: DECISIONS & ACTIONS                            │
│ (Executors, Handlers, External Systems)                 │
│ What: Verdicts become actions                           │
│ Flow: Judge verdicts → execute on machine/human         │
└─────────────────────────────────────────────────────────┘
                          ↑
                    [Feedback loop]
                          ↓
┌─────────────────────────────────────────────────────────┐
│ LAYER 3: JUDGMENT & EMERGENCE                           │
│ (Judge, Axiom Evaluators, Pattern Detectors)            │
│ What: Judge everything, detect emergences               │
│ Flow: Aggregated state → φ-bounded verdicts             │
└─────────────────────────────────────────────────────────┘
                          ↑
                          ↓
┌─────────────────────────────────────────────────────────┐
│ LAYER 2: AGGREGATION & REGIONAL GANGLIA                 │
│ (Aggregators, Regional Coordinators)                    │
│ What: Compress raw telemetry into meaningful state      │
│ Flow: Raw events → compressed abstractions              │
└─────────────────────────────────────────────────────────┘
                          ↑
                          ↓
┌─────────────────────────────────────────────────────────┐
│ LAYER 1: RAW OBSERVATION                                │
│ (Sensors, Probes, Raw Telemetry Producers)              │
│ What: Observe machine, human, ecosystem state           │
│ Flow: System events → raw observations                  │
└─────────────────────────────────────────────────────────┘
```

### Key Principle: Intelligent Filtering at Each Hop

Each layer filters and transforms data before passing it up. The Judge's routing rules determine:
- What each agent should observe
- What should be aggregated vs. passed raw
- What patterns matter (emergence detection)
- How to adapt routing based on system state

---

## 3. Layer Definitions

### Layer 1: Raw Observation (Sensory)

**Purpose:** Continuous observation of machine state, human inputs, ecosystem state.

**Agents:**
- `Sensor` — OS-level probes (network, filesystem, process state)
- `Probe` — System metrics (CPU, memory, disk, latency)
- `Ecosystem Observer` — Community data (proposals, votes, treasury state)
- `Human Input Listener` — Discord/Telegram messages, user commands

**Message Format:**
```json
{
  "layer": 1,
  "timestamp": 1708799040.123,
  "source": "sensor:os.process",
  "observation_type": "process_created",
  "data": {
    "pid": 1234,
    "name": "handler_executor",
    "user": "cynic",
    "memory_mb": 256
  },
  "metadata": {
    "probe_id": "probe:1",
    "instance_id": "instance:us-east-1",
    "raw": true
  }
}
```

**Characteristics:**
- High volume, high velocity
- Minimal filtering (raw as-is)
- Timestamped absolutely
- Source-tagged for routing
- Each observation independent

**Backpressure Handling:**
- Layer 1 agents publish to a ringbuffer (circular queue)
- Layer 2 aggregators pull what they can process
- Overflow drops oldest observations (acceptable, more data coming)

---

### Layer 2: Aggregation & Regional Ganglia

**Purpose:** Compress raw observations into meaningful state abstractions, coordinate across instance boundaries.

**Agents:**
- `Aggregator` — Groups related Layer 1 events (e.g., all process events → "system process state")
- `Regional Coordinator` — Aggregates across instances in a region
- `Temporal Window` — Sliding windows of aggregated state (last 5s, 1m, 5m, 1h)
- `State Synthesizer` — Builds abstractions (e.g., "system is healthy", "high memory pressure")

**Message Format:**
```json
{
  "layer": 2,
  "timestamp": 1708799045.0,
  "source": "aggregator:system_state",
  "window": "5s",
  "aggregation_type": "process_metrics",
  "data": {
    "process_count": 42,
    "total_memory_mb": 4096,
    "top_consumers": [
      {"name": "handler_executor", "memory_mb": 1024},
      {"name": "judge", "memory_mb": 512}
    ],
    "trend": "stable"
  },
  "metadata": {
    "aggregator_id": "agg:system",
    "instances_included": ["instance:us-east-1", "instance:eu-west-1"],
    "aggregation_method": "mean+stddev+trend"
  }
}
```

**Characteristics:**
- Moderate volume (compressed from Layer 1)
- Stateful (maintains windowed history)
- Causally ordered within a window
- Multiple time horizons (5s, 1m, 5m, 1h)
- Publishes abstractions, not raw data

**Emergence Detection (Initial):**
- Anomaly detection: "process memory suddenly 10x baseline"
- Trend detection: "system load increasing for past 5 minutes"
- Correlation detection: "high memory always precedes high CPU"
- Feeds back to Layer 3 with "emerging_pattern" flag

**Regional Coordination:**
- Each region has a primary `Regional Coordinator`
- Aggregators within region report to coordinator
- Coordinator de-duplicates, correlates across instances
- Sends deduplicated state upstream to Layer 3

---

### Layer 3: Judgment & Emergence Analysis

**Purpose:** Judge all state through CYNIC's axioms, detect emergent behaviors, emit verdicts that guide Layer 4 actions.

**Agents:**
- `Judge` — Evaluates aggregated state against 11 Axioms
- `Axiom Evaluator` — Implements each axiom (FIDELITY, PHI, VERIFY, CULTURE, BURN, etc.)
- `Emergence Detector` — Identifies novel patterns at system level
- `Verdict Synthesizer` — Combines axiom judgments into Q-Score and verdict (HOWL/WAG/GROWL/BARK)

**Message Format:**
```json
{
  "layer": 3,
  "timestamp": 1708799050.0,
  "source": "judge:system_state",
  "judgment_type": "state_evaluation",
  "target": "system_state:process_metrics",
  "data": {
    "axioms_evaluated": {
      "FIDELITY": {"score": 0.85, "reasoning": "observed metrics match expected range"},
      "PHI": {"score": 0.618, "reasoning": "golden ratio balanced load distribution"},
      "VERIFY": {"score": 0.91, "reasoning": "all sources agree on count"},
      "CULTURE": {"score": 0.72, "reasoning": "memory usage respects community norms"},
      "BURN": {"score": 0.88, "reasoning": "no extraction detected in resource allocation"}
    },
    "q_score": 0.83,
    "verdict": "WAG",
    "confidence": 0.618,
    "emergent_pattern": null
  },
  "metadata": {
    "judge_id": "judge:primary",
    "axiom_weights": {"FIDELITY": 0.2, "PHI": 0.15, ...},
    "based_on_layer2": ["agg:system", "agg:network"]
  }
}
```

**Verdicts:**
- `HOWL` — Problem detected, immediate action needed (Q < 0.4)
- `GROWL` — Caution, needs monitoring (Q 0.4-0.6)
- `WAG` — Healthy, normal operation (Q 0.6-0.8)
- `BARK` — Excellent, exemplary state (Q > 0.8)

**Emergence Detection (Advanced):**
- **Novelty**: "System exhibiting behavior never seen before"
- **Correlation**: "X always precedes Y, but Y often happens without X" (asymmetric causality)
- **Phase transition**: "System flipped from stable to chaotic regime"
- **Amplification**: "Small changes now have large consequences"

When emergence detected:
1. Emit `emergence` verdict with pattern description
2. Update routing: re-tune which agents should see Layer 2 state
3. Feed back to Layer 2: "Watch for this pattern in next window"
4. Store pattern for learning loop

**Learning Integration:**
- Judge verdict becomes input to Q-Table learning
- Emergences update heuristic weights for next judgment
- MCTS uses verdicts to score decision trees

---

### Layer 4: Decisions & Actions

**Purpose:** Convert verdicts into actions on machine, human, and external systems.

**Agents:**
- `Handler` — Executes verdicts on local machine (kernel config, role/feature additions)
- `External Executor` — Sends verdicts to external systems (GASdf, NEAR smart contracts)
- `Human Communicator` — Formats verdicts for human consumption (Slack, Discord, email)
- `Feedback Synthesizer` — Collects results of actions, feeds back to Layer 2

**Message Format (Outbound - Verdict as Action):**
```json
{
  "layer": 4,
  "timestamp": 1708799050.0,
  "source": "handler:system",
  "action_type": "apply_verdict",
  "verdict_based_on": "judge:system_state",
  "action": {
    "type": "resource_limit",
    "target": "process:handler_executor",
    "adjustment": "memory_limit_increase_to_2048mb",
    "reason": "verdict:WAG, trend:stable, capacity_available"
  },
  "metadata": {
    "handler_id": "handler:system",
    "dry_run": false,
    "target_instances": ["instance:us-east-1"]
  }
}
```

**Message Format (Inbound - Action Result):**
```json
{
  "layer": 1,
  "timestamp": 1708799051.0,
  "source": "handler:system",
  "observation_type": "action_result",
  "data": {
    "action_id": "action:12345",
    "status": "success",
    "effect": "memory_limit_applied",
    "new_state": "process_memory_limit_2048mb"
  },
  "metadata": {
    "feedback": true,
    "closes_action_id": "action:12345"
  }
}
```

**Action Types:**
- `apply_config` — Change OS/kernel config
- `deploy_component` — Add/remove/update agent
- `external_call` — Call GASdf/NEAR/external API
- `signal_human` — Alert human via communication channel

**Feedback Loop:**
- After executing action, Handler emits new Layer 1 observation
- New observation includes `feedback: true` and `closes_action_id`
- This closes the loop: verdict → action → observation

---

## 4. Data Flow: The Complete Cycle

```
┌─────────────────────────────────────────────────────────────────┐
│ OBSERVATION CYCLE                                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Sensor observes → Layer 1 raw event                            │
│       ↓                                                         │
│  Aggregator pulls from ringbuffer → Layer 2 aggregated          │
│       ↓                                                         │
│  Judge evaluates → Layer 3 verdict (HOWL/WAG/GROWL/BARK)        │
│       ↓                                                         │
│  Handler acts → Layer 4 action applied                          │
│       ↓                                                         │
│  Feedback loops back as new Layer 1 observation                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ EMERGENCE CYCLE                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 2 detects anomaly → "emerging_pattern" flag              │
│       ↓                                                         │
│  Layer 3 Judge analyzes → "emergence_type: phase_transition"    │
│       ↓                                                         │
│  Emergence Detector updates routing rules                       │
│       ↓                                                         │
│  Layer 2 Aggregators adjust collection strategy                 │
│       ↓                                                         │
│  Learning loop updates Q-Table with new pattern                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Routing Logic: Judge-Driven Subscription

### How Agents Know What to Consume

**Before:** All agents subscribed to event bus, received everything (overwhelming)

**After:** Judge maintains routing rules that determine what each agent class should observe.

**Routing Rule Structure:**
```python
RoutingRule = {
    "target_agent_type": "Dog",  # or Handler, Sensor, Judge, etc.
    "target_agent_id": "dog:consensus",
    "observable_types": ["layer2:aggregated_state", "layer3:verdict"],
    "filters": {
        "judgment_verdict": ["HOWL", "GROWL"],  # Only caution/problem
        "axiom_match": ["CULTURE", "BURN"],  # Only these axioms
        "emergence": True,  # Only emergent patterns
    },
    "window": "5s",  # How fresh data needs to be
    "priority": "high",  # Backpressure handling
    "updated_at": 1708799040.0,
    "learned_from": ["experience:interaction_123", "emergence:phase_transition_456"]
}
```

**How Routing Updates:**
1. Judge observes that some agent type always reacts to certain patterns
2. Judge creates rule: "Dogs should see HOWL/GROWL verdicts + CULTURE/BURN axioms"
3. Layer 2 Aggregators apply filter before emitting
4. Layer 3 Judge emits only matching verdicts to subscribed Dogs
5. Over time, rules get tuned by learning loop

**Backpressure Handling:**
- High-priority agents (critical Judge) get guaranteed delivery
- Medium-priority agents get best-effort with ringbuffer fallback
- Low-priority agents get sampled (every Nth message)
- When overwhelmed, Layer 2 aggregators drop oldest data (not critical)

---

## 6. Multi-Instance Coordination

### Same-Machine Operation (Day 1)

On a single machine, all 4 layers run in same process:
- Sensors → Aggregators → Judge → Handlers (direct function calls)
- Minimal latency (microseconds)
- Shared memory for state

### Multi-Instance Operation (Month 1-2)

When deploying across instances (LAN or cloud):

```
┌─────────────────────────┐         ┌─────────────────────────┐
│ Instance: us-east-1     │         │ Instance: eu-west-1     │
│                         │         │                         │
│ [Sensors]               │         │ [Sensors]               │
│ [Aggregator]            │         │ [Aggregator]            │
│   ↓                     │         │   ↓                     │
│ Regional Coordinator    │         │ Regional Coordinator    │
│ (UDP broadcast + TCP)   │◄───────►│ (UDP broadcast + TCP)   │
│   ↓                     │         │   ↓                     │
└─────────────────────────┘         └─────────────────────────┘
         ↓                                    ↓
    [Aggregated State]          [Aggregated State]
         ↓                                    ↓
┌──────────────────────────────────────────────────────────┐
│ Central Judge (Layer 3)                                  │
│ - Runs on primary instance or separate judge instance   │
│ - Receives aggregated state from all regions            │
│ - Emits verdicts back to regional coordinators          │
└──────────────────────────────────────────────────────────┘
         ↓
[Layer 4 Actions per instance]
```

**Instance Discovery:**
- Instances register with Coordinator via UDP broadcast (local) or DNS (internet)
- Each regional coordinator knows its peers
- Judge knows all regional coordinators

**Data Sync:**
- Regional Coordinators send Layer 2 aggregated state to Judge via TCP (reliable)
- Judge sends Layer 3 verdicts back via TCP
- Handlers pull Layer 4 actions from their local cache

**Consistency Model:**
- **Layer 2:** Eventually consistent (aggregators may temporarily disagree, reconcile in next window)
- **Layer 3:** Strong consistency (single Judge processes in order)
- **Layer 4:** Eventual consistency with feedback (actions execute locally, report back)

### Internet-Scale Operation (Q3 2026+)

When scaling beyond LAN:

```
┌─ Region: US-EAST ─┐  ┌─ Region: EU-WEST ─┐  ┌─ Region: APAC ─┐
│ [Sensors]         │  │ [Sensors]          │  │ [Sensors]      │
│ [Regional Judge]  │  │ [Regional Judge]   │  │ [Regional Judge]
└──────────┬────────┘  └──────────┬─────────┘  └────────┬────────┘
           │                      │                     │
           └──────────────────────┼─────────────────────┘
                                  │
                    ┌─────────────────────────────┐
                    │ Global Meta-Judge (Layer 3.5) │
                    │ (Optional, for global patterns) │
                    └─────────────────────────────┘
```

**Changes:**
- Each region runs its own Judge (reduces latency)
- Optional "Meta-Judge" for global emergence detection
- Layer 2 state compressed for inter-region transfer
- Causal consistency maintained via vector clocks

---

## 7. Message Routing Algorithm

### Pseudocode: How Layer N Decides What Goes to Layer N+1

```
function route_message(message, from_layer, to_layer):
  # Get applicable routing rules for target agents
  rules = judge.get_routing_rules(to_layer)

  # Filter message through rules
  should_deliver = true
  for rule in rules:
    if message.type not in rule.observable_types:
      should_deliver = false
      break

    if message.judgment_verdict not in rule.verdict_filter:
      should_deliver = false
      break

    if message.timestamp + rule.window < now():
      should_deliver = false  # Data too old
      break

  # Backpressure check
  queue_size = get_queue_size(rule.target_agent)
  if queue_size > rule.priority.threshold:
    if rule.priority == "low":
      if random() > SAMPLE_RATE:
        should_deliver = false  # Sample, don't deliver all

  if should_deliver:
    emit_to_agent(message, rule.target_agent)
    record_delivery(message, rule)  # For learning

  return should_deliver
```

---

## 8. Emergence Detection Strategy

### Layer 2 Anomaly Detection

```
def detect_anomaly(aggregated_state):
  """Detect deviations from baseline."""
  for metric in aggregated_state.metrics:
    baseline = historical.get_mean(metric, window="1h")
    stddev = historical.get_stddev(metric, window="1h")

    if abs(metric.value - baseline) > 2.5 * stddev:
      return ("anomaly", metric, (metric.value - baseline) / stddev)

  return (None, None, None)
```

### Layer 3 Emergence Detection

```
def detect_emergence(layer2_state, layer3_recent_verdicts):
  """Detect novel system-level patterns."""

  # 1. Phase transition: verdict distribution changed suddenly
  recent_dist = Counter([v.verdict for v in recent_verdicts[-100:]])
  historical_dist = historical.get_verdict_distribution(window="1day")
  if kl_divergence(recent_dist, historical_dist) > THRESHOLD:
    return ("phase_transition", recent_dist, historical_dist)

  # 2. Correlation emergence: X and Y now move together
  for (metric_a, metric_b) in metric_pairs:
    correlation = compute_correlation([metric_a, metric_b], window="5m")
    if correlation > THRESHOLD and correlation_not_learned():
      return ("correlation_emerged", metric_a, metric_b, correlation)

  # 3. Causal inversion: normally A→B, now B→A
  for (metric_a, metric_b) in causally_learned_pairs:
    new_causality = compute_causality([metric_a, metric_b], window="5m")
    if new_causality != learned_causality:
      return ("causality_inverted", metric_a, metric_b, new_causality)

  # 4. Amplification: small changes have large effects
  recent_stability = compute_stability(layer2_state, window="5m")
  historical_stability = historical.get_stability(window="1day")
  if recent_stability < historical_stability * 0.5:
    return ("amplification_detected", recent_stability, historical_stability)

  return (None, None, None)
```

---

## 9. Error Handling & Resilience

### Agent Failure

**Scenario:** Layer 2 Aggregator crashes

**Resolution:**
1. Regional Coordinator detects missed heartbeat (timeout: 5s)
2. Coordinator spins up new Aggregator on standby instance
3. New Aggregator requests last checkpoint from Judge (last known state)
4. Judge replays Layer 1 events since checkpoint (from ringbuffer)
5. New Aggregator catches up, resumes normal flow

**Data Loss:** Minimal (only unprocessed raw events in ringbuffer, acceptable)

### Network Partition (Multi-Instance)

**Scenario:** us-east-1 and eu-west-1 coordinators lose connection

**Resolution:**
1. Each region continues operating independently
2. Layer 2 aggregators keep buffering state
3. When connection restores, coordinators:
   - Exchange checksums of recent state
   - Resolve conflicts via timestamp + Judge signature
   - Replay divergent events through Judge (once)
4. Consistency restored

**Guarantee:** Eventual consistency, no data loss

### Judge Overwhelm

**Scenario:** Judge receiving 1000 verdicts/sec, can't keep up

**Resolution:**
1. Layer 2 aggregators detect Judge queue > threshold
2. Aggregators reduce sampling rate (emit every Nth message)
3. Judge processes sampled stream with lower latency
4. Judge requests full stream replay if needed later (pull-based recovery)

---

## 10. Learning Integration

### Q-Table Updates

After each verdict emission:

```
def update_qlearning(judgment_result):
  """Feed judgment into learning loop."""
  state = judgment_result.input_state  # Layer 2 aggregated
  action = judgment_result.verdict  # HOWL/WAG/GROWL/BARK
  outcome = measure_outcome_at(state, delay=5m)  # What happened 5m later

  # Update routing rules based on outcome
  if outcome.expected:
    reward = judgment_result.confidence * outcome.reward
    learning.update_qvalue(state, action, reward)
    learning.reinforce_routing_rule(judgment_result.routing_rule)
  else:
    penalty = outcome.surprise_factor
    learning.penalize_qvalue(state, action, penalty)
    learning.suggest_routing_rule_adjustment(judgment_result.routing_rule)
```

### Routing Rule Evolution

```
def evolve_routing_rules():
  """Update routing rules based on learning."""
  for rule in judge.routing_rules:
    rule.confidence = learning.get_rule_confidence(rule)
    rule.filters = learning.suggest_filter_improvements(rule)
    rule.priority = learning.suggest_priority_adjustment(rule)
    rule.observable_types = learning.suggest_observable_expansion(rule)
```

---

## 11. Implementation Phases

### Phase 1: Single-Machine (Week 1-2)

- Implement Layers 1-4 as in-process components
- Use in-memory ringbuffer for Layer 1 → Layer 2
- Direct function calls between layers
- No networking (yet)
- Test with existing event payloads

**Success Criteria:**
- Layer 1 sensors emit raw observations
- Layer 2 aggregators compress to state abstractions
- Layer 3 Judge evaluates, emits verdicts
- Layer 4 Handlers execute and feedback

### Phase 2: Multi-Instance LAN (Week 3-4)

- Implement Regional Coordinator
- Add TCP transport for Layer 2 → Layer 3 (judge communication)
- Instance discovery via UDP broadcast
- Backpressure handling (ringbuffer overflow)

**Success Criteria:**
- Two instances on LAN coordinate correctly
- Judge receives aggregated state from both
- Verdicts route back to correct instance
- Failures are gracefully handled

### Phase 3: Internet-Scale (Q2-Q3 2026)

- Implement Meta-Judge for global patterns
- Add vector clocks for causal consistency
- Compress Layer 2 for inter-region transfer
- Regional Judges with eventual consistency

**Success Criteria:**
- 5+ regions operating independently
- Global emergence detection works
- Network partitions handled gracefully
- Latency acceptable (< 5s for verdicts)

---

## 12. Data Model: Agent Types by Layer

| Agent Type | Layer | Role | Example |
|---|---|---|---|
| Sensor | 1 | Observe raw state | OS process monitor |
| Probe | 1 | Emit telemetry | CPU/memory gauge |
| Aggregator | 2 | Compress observations | Group processes by type |
| Regional Coordinator | 2 | Cross-instance sync | Dedupe state across regions |
| Judge | 3 | Evaluate verdicts | Assess system health via axioms |
| Axiom Evaluator | 3 | Score against axiom | Apply CULTURE axiom |
| Emergence Detector | 3 | Find novel patterns | Detect phase transitions |
| Handler | 4 | Execute actions | Apply resource limits |
| External Executor | 4 | Call outside systems | Invoke GASdf/NEAR |
| Human Communicator | 4 | Format for humans | Send verdict to Discord |

---

## 13. Non-Functional Requirements

| Requirement | Target | Rationale |
|---|---|---|
| **Latency** | < 100ms per layer (5-layer = 500ms max) | Responsive system behavior |
| **Throughput** | 10k observations/sec per instance | Omniscience requires high telemetry rate |
| **Consistency** | Eventual (Layers 1-2), Strong (Layer 3) | Balance speed with judgment reliability |
| **Availability** | 99.9% uptime (accounting for upgrades) | Organism must stay alive |
| **Scalability** | 1000+ agents per instance, 100+ instances | Internet-scale ready |
| **Emergence latency** | < 10s to detect and act | System must respond to novel conditions |
| **Learning update rate** | Every 5 minutes | Q-Table improves gradually |

---

## 14. Success Metrics

- ✅ **Single-machine mode works** (all layers operational, in-process)
- ✅ **Multi-instance coordination works** (Regional Coordinator syncs state)
- ✅ **Judge verdicts route correctly** (HOWL→immediate, WAG→monitored, etc.)
- ✅ **Emergence detected** (novel patterns identified within 10s)
- ✅ **Learning improves routing** (routing rules adapt based on outcomes)
- ✅ **System stays responsive** (latency stays under 500ms even at 10k obs/sec)
- ✅ **Network failures don't crash organism** (graceful degradation)

---

## 15. Open Questions for Implementation

1. **Judge location** — Single Judge per region, or global Judge? Transition path?
2. **Ringbuffer size** — How many observations should Layer 1 buffer before dropping?
3. **Axiom weight evolution** — Should weights change per emergence, or globally?
4. **Vector clocks** — Needed for internet-scale, but adds complexity. Implement now or later?
5. **Compression strategy** — How to aggregate Layer 1 without losing important data?

---

## Appendix A: Message Frame Format (Canonical)

All messages across layers follow this structure:

```json
{
  "header": {
    "layer": 1,
    "message_id": "msg:12345",
    "timestamp": 1708799040.123,
    "source": "sensor:os.process",
    "target": null,
    "version": "1.0"
  },
  "payload": {
    "observation_type": "process_created",
    "data": { /* layer-specific */ }
  },
  "metadata": {
    "instance_id": "instance:us-east-1",
    "region": "us-east",
    "route_trace": ["sensor:os.process", "aggregator:system", "judge:primary"]
  }
}
```

---

**End of Design Document**

---

**Next Steps:**
1. Review and approve this design
2. Invoke `writing-plans` skill to create implementation plan
3. Begin Phase 1 implementation (single-machine)
