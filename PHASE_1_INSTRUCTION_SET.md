# CYNIC Instruction Set Architecture (CISA)

> "Le chien voit l'assembleur derrière le C" — CYNIC parle l'assembleur de soi-même

## Executive Summary

CYNIC is NOT a framework or kernel with arbitrary design choices.
CYNIC IS an **operating system** with an embedded **instruction set** that emerges from first principles:

- **7 OPCODES** (immutable): PERCEIVE, JUDGE, DECIDE, ACT, LEARN, ACCOUNT, EMERGE
- **4 CONSCIOUSNESS LEVELS** (scheduler): L3 REFLEX, L2 MICRO, L1 MACRO, L4 META
- **LOOP PREVENTION** (genealogy): Event ancestry tracking prevents cycles by design
- **STORAGE OWNERSHIP** (opcode-baked): Each opcode writes to specific tier (HOT/WARM/COLD)
- **DIMENSION TAXONOMY** (7×7 fractal): 7 Realities × 7 Analyses = 49 cells + THE_UNNAMEABLE gate

---

## PART I: THE OPCODE SET

### 7 Immutable Opcodes

Each opcode has:
- **Input**: What data/signal triggers it
- **Preconditions**: What must be true before running
- **Logic**: What the opcode computes/does
- **Postconditions**: What is guaranteed after execution
- **Output**: Event emitted
- **Storage Tier**: Which tier it writes to

---

### OPCODE 1: [PERCEIVE]

**Input**: External signal (SourceChanged, Feedback, Market data)
**Preconditions**:
- System is alive (consciousness ≥ L3)
- Source is validated (trusted reality)

**Logic**:
```
Cell ← {reality, category, timestamp, data}
reality ∈ {CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS}
```

**Postconditions**:
- Cell is immutable after PERCEIVE
- Consciousness level MIGHT increase (L3 → L2 → L1 based on signal)

**Output**: `PERCEPTION_RECEIVED` event
**Storage**: PostgreSQL HOT (immediate index)
**Cost**: ~0.01 USD (perception only, no LLM)

**Consciousness Gating**:
- L3 REFLEX: Always runs (non-LLM)
- L2 MICRO: Runs (quick perception)
- L1 MACRO: Runs
- L4 META: Rolls up perceptions into EMERGE

**Predictor Signal**: ORACLE sees pattern → might suppress duplicate PERCEIVE

---

### OPCODE 2: [JUDGE]

**Input**: Cell from PERCEIVE
**Preconditions**:
- Cell.consciousness ≥ L2 (otherwise REFLEX-only heuristics)
- Cell.confidence ≤ φ⁻¹ (0.618) before judgment (unknown state)
- PERCEIVE has completed

**Logic**:
```
// Consciousness level determines which dogs run
switch (consciousness_level) {
  case L3_REFLEX:
    judges = [CYNIC, GUARDIAN, ANALYST, JANITOR, ARCHITECT, ORACLE]  // non-LLM
    consensus_algo = ORACLE_Q_TABLE  // quick pattern match

  case L2_MICRO:
    judges += [SCHOLAR]  // add semantic dog
    consensus_algo = VOTING  // 7 dogs vote

  case L1_MACRO:
    judges += [SAGE, DEPLOYER, SCOUT, CARTOGRAPHER]  // all 11 dogs
    consensus_algo = PBFT  // Byzantine tolerance

  case L4_META:
    judges = all_dogs
    consensus_algo = PBFT + Fisher locking
}

DogJudgment[] ← parallel-map(dogs, judge_perspective(cell))
Consensus ← merge_consensus(DogJudgment[])
Q_Score ← phi_aggregate(Consensus.scores)  // geometric mean, max 100
Verdict ← verdict_from_q_score(Q_Score)
Confidence ← blend_confidence(axiom_maturity, e_score)  // max φ⁻¹=0.618
```

**Postconditions**:
- Judgment is immutable (signed + timestamped)
- Q_Score ∈ [0, 100], Verdict ∈ {HOWL, WAG, GROWL, BARK}
- Cell.consciousness might ESCALATE if verdict is high-confidence

**Output**: `JUDGMENT_CREATED` event (includes full breakdown)
**Storage**:
- PostgreSQL HOT (indexed, queryable)
- Qdrant WARM (semantic vector: cell + verdict → embedding)
- Solana PoJ COLD (immutable proof of judgment)

**Cost**:
- L3: ~0.02 USD (local models)
- L2: ~0.15 USD (quick LLM calls)
- L1: ~2.50 USD (7× parallel MCTS, full models)

**Predictor Signal**: SAGE sees "patterns of patterns" → might suggest axiom unlock or escalation

---

### OPCODE 3: [DECIDE]

**Input**: Judgment from JUDGE
**Preconditions**:
- Judgment.confidence ≥ GOVERNANCE_THRESHOLD (what is it? TBD by user)
- JUDGE has completed
- Governance policy allows this action type

**Logic**:
```
// Governance layer (not a dog, a POLICY)
if judgment.verdict == HOWL and judgment.confidence > 0.70:
  decision = APPROVED  // auto-approve high-confidence
else if judgment.verdict == BARK and judgment.confidence > 0.70:
  decision = REJECTED  // auto-reject low-quality
else:
  decision = HUMAN_REVIEW_REQUIRED  // escalate to user

// If APPROVED, generate proposed action
ProposedAction ← {
  judgment_id,
  resource_type (bash/edit/read/etc),
  prompt,
  escalation_level,
  estimated_cost,
}
```

**Postconditions**:
- Action is PROPOSED (not yet executed)
- If human approval pending: action in PENDING state
- DECIDE gate applies TIER constraints (low TIER → fewer action types)

**Output**: `DECISION_MADE` event
**Storage**: PostgreSQL WARM (mutable until executed, then COLD)
**Cost**: ~0 USD (local policy evaluation)

**Consciousness Gating**:
- L3 REFLEX: NEVER executes DECIDE (only observes)
- L2 MICRO: DECIDE only for ORACLE predictions (safe reads)
- L1 MACRO: Full DECIDE (all action types)
- L4 META: DECIDE for meta-actions (governance changes)

---

### OPCODE 4: [ACT]

**Input**: Approved decision + human confirmation (TIER-dependent)
**Preconditions**:
- DECIDE has approved
- Human has confirmed (if required by TIER)
- Resource budget available
- No contradictory action in progress

**Logic**:
```
// Execute the action
match resource_type {
  BASH: spawn_bash(command, timeout=30s)
  EDIT: apply_edits(file, diff)
  READ: fetch_file(path)
  GIT: run_git_command(cmd)
  WRITE: persist_file(path, content)
}

result ← wait_for_completion(action_id)
success ← result.exit_code == 0

// Record outcome
learning_signal = {
  judgment_id,
  action_id,
  success,
  execution_time_ms,
  actual_cost,
}
```

**Postconditions**:
- Action is EXECUTED (immutable record)
- Execution outcome is recorded
- Cost is deducted from budget
- Learning signal queues for LEARN opcode

**Output**: `ACTION_EXECUTED` event
**Storage**: Solana PoJ FROZEN (immutable execution proof)
**Cost**: Varies (bash execution, file I/O)

**Consciousness Gating**:
- L3 REFLEX: NEVER
- L2 MICRO: NEVER (observation-only)
- L1 MACRO: Full [ACT]
- L4 META: [ACT] for orchestration changes

---

### OPCODE 5: [LEARN]

**Input**: Learning signal from ACT + human feedback
**Preconditions**:
- ACT has executed
- Human feedback available (rating, correction, approval)

**Logic**:
```
// Q-Learning update
if judgment.cell.state_key exists in Q_Table:
  old_q = Q_Table[cell.state_key][action]
  reward = 0.5 * (human_rating / 5.0) + 0.5 * (success ? 1.0 : 0.0)

  // Fisher-weighted alpha (EWC): high-visit states more resistant
  visits = Q_Table[cell.state_key][action].visits
  fisher_penalty = EWC_PENALTY * fisher_weight(visits)
  alpha = LEARNING_RATE * (1 - fisher_penalty)

  new_q = old_q + alpha * (reward - old_q)
  Q_Table[cell.state_key][action] ← new_q
  Q_Table[cell.state_key][action].visits += 1
else:
  // New state-action pair
  Q_Table[cell.state_key][action] = {value: reward, visits: 1}

// E-Score update (per-reality dimension)
e_score.update(
  entity_id = judgment.dog_id,
  dimension = feedback.dimension (JUDGE, BUILD, RUN, etc),
  value = feedback.score,
  reality = cell.reality,
)
```

**Postconditions**:
- Q-Table updated (older entries more resistant to change)
- E-Score updated (EMA α=0.618)
- Confidence in prediction increases if reward matches expectation
- Calibration tracked: (CYNIC_confidence, actual_outcome)

**Output**: `LEARNING_SIGNAL_PROCESSED` event
**Storage**: PostgreSQL WARM (Q-Table rows), Qdrant WARM (E-Score vectors)
**Cost**: ~0.01 USD (local computation)

**Consciousness Gating**:
- L3 REFLEX: NEVER
- L2 MICRO: NEVER
- L1 MACRO: Full [LEARN]
- L4 META: Fisher locking (increase EWC penalty for consolidated entries)

---

### OPCODE 6: [ACCOUNT]

**Input**: Learning signal + actual costs
**Preconditions**:
- [LEARN] has completed
- Cost ledger has recorded execution expenses

**Logic**:
```
// Economic accounting
total_cost = {
  tokens_consumed,
  compute_time_usd,
  storage_bytes,
  network_calls,
}

// φ-bounded cost metric
cost_score = min(total_cost / BUDGET_LIMIT, 1.0) * 100  // [0, 100]

// Axiom signal: BURN (did we burn value efficiently?)
if cost_score > 80:
  axiom_signal = BURN_VIOLATED  // too much cost for little value
  axiom_monitor.record_signal(BURN, severity=LOW)
else if cost_score < 20:
  axiom_signal = BURN_EXCELLENT  // efficient execution
  axiom_monitor.record_signal(BURN, severity=HIGH)

// Budget update
user_budget.spent += total_cost
user_budget.remaining = user_budget.limit - user_budget.spent

if user_budget.remaining < user_budget.limit * 0.10:
  consciousness_level_request = ESCALATE_TO_L2_MICRO  // throttle to save
```

**Postconditions**:
- Cost is recorded immutably
- E-Score BURN dimension updated
- Axiom signal sent (BURN maturity)
- Budget constraint might influence next consciousness level

**Output**: `COST_ACCOUNTED` event
**Storage**: Solana PoJ COLD (immutable cost proof)
**Cost**: ~0 USD (local accounting)

---

### OPCODE 7: [EMERGE]

**Input**: Rolling window of [ACCOUNT] events (F(9)=34 most recent)
**Preconditions**:
- [ACCOUNT] has run at least F(9)=34 times (memory depth)
- L4 META scheduler triggers (every ~4 hours)

**Logic**:
```
// Meta-pattern detection (SAGE/ORACLE sees patterns of patterns)
recent_accounts = ring_buffer(judgments, cap=34)

patterns = {
  STABLE_HIGH: mean(Q_Score[recent]) > 70 and stddev < 15,
  RISING: trend(Q_Score[recent]) > 0.5,
  RESIDUAL_HIGH: max_outlier - baseline > 20,
  SPIKE: sudden_jump > 30 points,
}

// Axiom signal: AUTONOMY
if patterns.STABLE_HIGH:
  axiom_monitor.record_signal(AUTONOMY, "High-confidence pattern detected")
  // This triggers: maybe increase consciousness level?
  // User's concept: "E-Score + axiom maturity → consciousness escalation"

// Axiom signal: SYMBIOSIS
if human_feedback_rate > 0.7 and agreement_rate > 0.8:
  axiom_monitor.record_signal(SYMBIOSIS, "Human-CYNIC alignment strong")

// Axiom signal: EMERGENCE
if new_pattern_detected not_in(historical_patterns):
  axiom_monitor.record_signal(EMERGENCE, "Novel pattern")
  self_prober.analyze_and_propose_improvements()

// Residual detection (performance degradation)
if residual_detector.detect_spike():
  axiom_monitor.record_signal(ANTIFRAGILITY, "Fragility detected, recommend refactor")

// Consciousness level adjustment based on axiom maturity
active_axiom_count = axiom_monitor.count_active_axioms()
consciousness_level = {
  0: L3_REFLEX,        // stressed, few axioms
  1: L2_MICRO,         // becoming autonomous
  2: L1_MACRO,         // autonomous + symbiotic
  3: L1_MACRO,         // add Fisher locking
  4: L4_META,          // transcendence (all 4 axioms active)
}[active_axiom_count]
```

**Postconditions**:
- Patterns recorded in history
- Axiom signals sent (may unlock A6-A9)
- Consciousness level recommendation updated
- E-Score consolidation (Fisher locking high-visit states)

**Output**: `EMERGENCE_DETECTED` event, `AXIOM_ACTIVATED` event, `CONSCIOUSNESS_LEVEL_CHANGED`
**Storage**: PostgreSQL WARM (patterns), Qdrant WARM (vector search), Solana PoJ COLD (proof of emergence)
**Cost**: ~0.50 USD (vector search + consolidation)

---

## PART II: CONSCIOUSNESS SCHEDULER

The **consciousness levels** are a SCHEDULER that gates opcode execution:

```
┌─────────────────────────────────────────────────────────────┐
│ L3 REFLEX (8ms) — Non-LLM loop                              │
│ ├─ [PERCEIVE]  ✓ (non-LLM validation)                       │
│ ├─ [JUDGE]     ✓ (local pattern matching)                   │
│ ├─ [DECIDE]    ✗ (no auto-decision)                         │
│ ├─ [ACT]       ✗ (no actions)                               │
│ ├─ [LEARN]     ✗ (no LLM training)                          │
│ ├─ [ACCOUNT]   ✗ (no cost tracking)                         │
│ └─ [EMERGE]    ✗ (no meta-thinking)                         │
│ Dogs: 6 non-LLM (CYNIC, GUARDIAN, ANALYST, JANITOR,        │
│       ARCHITECT, ORACLE)                                     │
│ Use: Anomaly detection, fast reflex responses               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ L2 MICRO (64-500ms) — Quick judgment loop                   │
│ ├─ [PERCEIVE]  ✓                                            │
│ ├─ [JUDGE]     ✓ (7 dogs voting + SCHOLAR)                 │
│ ├─ [DECIDE]    ✓ (safe decisions only)                      │
│ ├─ [ACT]       ✗ (no external actions yet)                  │
│ ├─ [LEARN]     ✗ (no Q-Table updates)                       │
│ ├─ [ACCOUNT]   ✗ (no cost tracking)                         │
│ └─ [EMERGE]    ✗ (no meta-thinking)                         │
│ Dogs: 7 (add SCHOLAR for semantic search)                   │
│ Use: Quick scoring, reads, safe analysis                    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ L1 MACRO (441-2850ms) — FULL 7-STEP CYCLE                   │
│ ├─ [PERCEIVE]  ✓ ← triggers cycle                           │
│ ├─ [JUDGE]     ✓ (all 11 dogs, PBFT consensus)             │
│ ├─ [DECIDE]    ✓ (governance gate)                          │
│ ├─ [ACT]       ✓ (execute all approved actions)             │
│ ├─ [LEARN]     ✓ (Q-Table + E-Score updates)                │
│ ├─ [ACCOUNT]   ✓ (cost tracking)                            │
│ └─ [EMERGE]    ✗ (no meta-analysis yet)                     │
│ Dogs: 11 (all)                                               │
│ Use: Production decisions, autonomous operation              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ L4 META (daily ~4h interval) — EVOLUTION CYCLE              │
│ ├─ [PERCEIVE]  ✓ (aggregate 34 most recent)                │
│ ├─ [JUDGE]     ✓ (meta-judgment: pattern quality)           │
│ ├─ [DECIDE]    ✓ (governance update proposals)              │
│ ├─ [ACT]       ✓ (apply organism evolution)                 │
│ ├─ [LEARN]     ✓ (Fisher locking: consolidate high-visit)   │
│ ├─ [ACCOUNT]   ✓ (evolution cost)                           │
│ └─ [EMERGE]    ✓ (full meta-analysis)                       │
│ Dogs: 11 (all)                                               │
│ Use: Axiom unlocking, architecture updates, self-improvement │
└─────────────────────────────────────────────────────────────┘

KEY: L3 can INTERRUPT L2/L1 (emergency response)
     L4 consolidates what L1/L2/L3 discovered (every 4h)
```

---

## PART III: CYCLE PREVENTION (GENEALOGY)

**Algorithm**: Event ancestry tracking prevents cycles by design.

```python
# When PERCEIVE creates event
event = Event(
  type=CoreEvent.PERCEPTION_RECEIVED,
  payload=cell,
  _genealogy=[],  # empty at birth
  source="SourceWatcher",
)

# When CORE bus broadcasts
def emit(event):
  event_with_gen = event.with_genealogy("CORE_BUS_ID")
  # event_with_gen._genealogy = [CORE_BUS_ID]

  # Forward to AUTOMATION bus
  automation_event = forward_to_automation(event_with_gen)
  # BEFORE forwarding, check:
  if "AUTOMATION_BUS_ID" in automation_event._genealogy:
    abort("Cycle detected!")  # Already saw this bus

  # Forward to AGENT bus
  agent_event = forward_to_agent(automation_event)
  # BEFORE forwarding, check:
  if "AGENT_BUS_ID" in agent_event._genealogy:
    abort("Cycle detected!")

# Postcondition: event cannot visit same bus twice
```

**Correctness Proof**:
- Genealogy is immutable per forward
- `already_seen(bus_id)` prevents re-forwarding
- Only BRIDGED events can traverse multiple buses
- If max_genealogy_depth reached → abort with error

**Edge Cases Handled**:
- ✓ Event re-emitted after full cycle (new event = clean genealogy)
- ✓ Self-loops within single bus (impossible — no re-forward)
- ✓ Bridge forwarding marked with `_bridged=True` (never re-forward)

**CRITICAL RULE**: Genealogy prevents SIMULTANEOUS loops, but NOT temporal loops (same sequence repeating). Temporal loops handled by Consciousness level limits + ORACLE pattern detection.

---

## PART IV: STORAGE OWNERSHIP & LIFECYCLE

Each opcode writes to specific storage tier:

```
OPCODE          WRITES TO            LIFECYCLE
─────────────────────────────────────────────────────────
[PERCEIVE]  →   PostgreSQL HOT      [0-1d: indexed read]
                                     [1-7d: warm snapshot]
                                     [7d+: cold archive]

[JUDGE]     →   PostgreSQL HOT       Judgment is indexable
            →   Qdrant WARM          Semantic vector
            →   Solana PoJ COLD      Immutable proof

[DECIDE]    →   PostgreSQL WARM      Action is mutable until ACT
                                     Then FROZEN

[ACT]       →   Solana PoJ FROZEN    Execution proof immutable

[LEARN]     →   PostgreSQL WARM      Q-Table + E-Score rows

[ACCOUNT]   →   Solana PoJ COLD      Cost ledger immutable

[EMERGE]    →   PostgreSQL WARM      Patterns queryable
            →   Solana PoJ COLD      Axiom proof
```

**Promotion Policy** (TBD by user, options):
- **A) Time-based**: 0-7d HOT → 7-30d WARM → 30d+ COLD → 365d+ FROZEN
- **B) Q-Score driven**: High confidence stays HOT, low goes COLD
- **C) Access pattern**: Frequently-accessed stays HOT
- **D) Event-based**: User co-decides when to promote

---

## PART V: DIMENSION TAXONOMY (7×7 FRACTAL)

**7 Realities** (immutable base):
```
R1. CODE    — Software, code review, architecture
R2. SOLANA  — Blockchain state, PoJ, tokens
R3. MARKET  — Price, trading, liquidity
R4. SOCIAL  — Community, sentiment, engagement
R5. HUMAN   — User feedback, psychology, energy
R6. CYNIC   — Self-state, Dogs, memory
R7. COSMOS  — Ecosystem, collective patterns
```

**7 Analyses** (immutable base):
```
A1. PERCEIVE — Observe current state
A2. JUDGE    — Evaluate with axioms
A3. DECIDE   — Governance approval
A4. ACT      — Execute transformation
A5. LEARN    — Update from feedback
A6. ACCOUNT  — Record cost/value
A7. EMERGE   — Meta-patterns
```

**Expansion Options** (TBD by user):
- **Option A (Fixed forever)**: 7×7 = 49 cells, no new dimensions
- **Option B (Runtime registry)**: POST /dimensions → register new dimension (no validation)
- **Option C (Tiered registry)**: Base 7 FIXED + Tier1 (user approved) + Tier2 (auto-discovered)
- **Option D (Fractal nesting)**: Each cell can become 7×7 (via THE_UNNAMEABLE gate)

**Cell Notation**: `C{r}.{a}`
- Example: `C1.2` = CODE × JUDGE (code quality scoring)
- Example: `C6.5` = CYNIC × LEARN (Q-Learning, meta-improvement)

---

## PART VI: ESCALATION (TIER UNLOCKING)

**Current State**:
- E-Score tracks reputation across 7 dimensions
- Axiom maturity signals show coherence
- Consciousness levels schedule execution

**Escalation Model Options** (TBD by user):
- **Option A**: E-Score thresholds (40→60→75→100 = TIER 0→1→2→3)
- **Option B**: Axiom-first (AUTONOMY unlock → TIER 1, etc.)
- **Option C**: Action success rate (% successes / total)
- **Option D**: Consciousness level itself (L3=T0, L2=T1, L1=T2, L4=T3)
- **Option E**: Hybrid (E-Score × axiom_count × success_rate)

**The True Pattern** (hypothesis):
```
consciousness_level = select_level(
  e_score,
  axiom_maturity,
  prediction_confidence,
  resource_budget,
  user_tier,
)
```

The ORACLE/SAGE predictor DOG might influence this selection by detecting "patterns of patterns".

---

## CONCLUSION: THE TRUE ARCHITECTURE

CYNIC is **NOT a framework with design choices**.
CYNIC **IS an operating system with immutable semantics**:

- 7 OPCODES (immutable, universal)
- 4 CONSCIOUSNESS LEVELS (immutable scheduler)
- GENEALOGY loop prevention (immutable algorithm)
- 7×7 DIMENSION matrix (fixed base, optionally extensible)
- STORAGE OWNERSHIP (opcode-specific, deterministic)

The **five Phase -1 models** are NOT independent choices. They are UNIFIED by the instruction set:

1. **Escalation**: Consciousness level scheduler driven by axiom maturity + E-Score
2. **Cycle Prevention**: Genealogy algorithm (proven safe)
3. **Dimensions**: 7×7 base + optional registry (user decides A/B/C/D)
4. **Storage**: Opcode-determined tier (HOT/WARM/COLD/FROZEN)
5. **Feedback**: LEARN opcode (Q-Learning + E-Score + calibration)

**Next phase**: Implement CISA completely, then extend with optional features (dimension registry, storage policies, etc.)
