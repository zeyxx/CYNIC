# PHASE 1 IMPLEMENTATION PLAN

> "Le chien construit son terrier." — CYNIC builds its operating system

**Timeline**: 2-3 weeks
**Goal**: Complete 7×7 matrix (39/49 cells minimum = 80%) before THE_UNNAMEABLE gate opens
**Success**: All 7 OPCODES fully wired, consciousness levels controlling execution, tests passing

---

## WHAT ALREADY EXISTS (Reuse, Don't Rebuild)

✅ **7 OPCODES** (in orchestrator.py):
- `[PERCEIVE]` → receives Cell
- `[JUDGE]` → dogs analyze
- `[DECIDE]` → governance gate
- `[ACT]` → execute actions
- `[LEARN]` → Q-Table update
- `[ACCOUNT]` → cost tracking
- `[EMERGE]` → meta-patterns

✅ **4 CONSCIOUSNESS LEVELS** (in consciousness.py):
- L3 REFLEX (8ms, 6 non-LLM dogs)
- L2 MICRO (500ms, 7 dogs + SCHOLAR)
- L1 MACRO (2.85s, all 11 dogs, full cycle)
- L4 META (daily, evolution + Fisher locking)

✅ **GENEALOGY loop prevention** (in event_bus.py):
- Event._genealogy tracks bus traversal
- `already_seen(bus_id)` prevents cycles

✅ **STORAGE GC** (in storage/gc.py):
- Prunes BARK verdicts at F(6)=8 days
- Keeps newest entries at F(N) caps
- φ-Fibonacci retention windows

✅ **E-Score 7D** (in escore.py):
- BURN, BUILD, JUDGE, RUN, SOCIAL, GRAPH, HOLD
- Per-reality sub-scores with EMA α=0.618

✅ **11 Dogs** (in cognition/neurons/):
- All implemented + judgment methods exist

---

## PHASE 1 WORK BREAKDOWN

### TIER A: CRITICAL PATH (Week 1)

#### A1: Formalize OPCODE Semantics (2-3 days)
**File**: `cynic/core/opcode_semantics.py` (NEW)

For each opcode, document:
- **Preconditions**: What must be true before running
- **Postconditions**: What is guaranteed after
- **State transition**: Valid next opcodes
- **Storage tier**: Where data goes
- **Cost model**: Token/USD cost
- **Consciousness gating**: Which levels allow this

Example template:
```python
@dataclass
class OpcodeSpec:
    name: str  # "PERCEIVE"
    preconditions: list[str]
    postconditions: list[str]
    state_transitions: list[str]  # Valid next opcodes
    storage_tier: str  # "HOT" | "WARM" | "COLD" | "FROZEN"
    cost_model: Callable
    consciousness_gates: dict[ConsciousnessLevel, bool]

OPCODE_SPECS = {
    CoreEvent.PERCEPTION_RECEIVED: OpcodeSpec(
        name="PERCEIVE",
        preconditions=["system is alive", "source validated"],
        postconditions=["Cell is immutable", "consciousness may escalate"],
        state_transitions=["JUDGE"],
        storage_tier="HOT",
        cost_model=lambda: 0.01,  # cheap
        consciousness_gates={
            ConsciousnessLevel.REFLEX: True,
            ConsciousnessLevel.MICRO: True,
            ConsciousnessLevel.MACRO: True,
            ConsciousnessLevel.META: True,
        }
    ),
    # ... 6 more opcodes
}
```

**Tests**: `tests/test_opcode_semantics.py`
- Verify all 7 opcodes documented
- Verify preconditions/postconditions are testable
- Verify consciousness gates are complete

---

#### A2: Wire Consciousness Scheduler (2-3 days)
**File**: Modify `cynic/cognition/cortex/orchestrator.py`

Current state: `dogs_for_level()` exists, but no logic connecting axiom maturity → level selection

**Implementation**:
```python
class ConsciousnessScheduler:
    """Selects consciousness level based on system state."""

    def __init__(self, axiom_monitor, escore_tracker, oracle_dog):
        self.axiom_monitor = axiom_monitor
        self.escore_tracker = escore_tracker
        self.oracle = oracle_dog  # predictor

    def select_level(
        self,
        cell: Cell,
        current_level: ConsciousnessLevel,
    ) -> ConsciousnessLevel:
        """
        Blend three signals: axiom maturity, E-Score, predictor confidence.

        Algorithm:
          score = (axiom_maturity × 0.4) + (e_score/100 × 0.3) + (oracle_confidence × 0.3)

          if score < 38.2:  return L3_REFLEX
          elif score < 61.8: return L2_MICRO
          elif score < 82.0: return L1_MACRO
          else:              return L4_META  # (only if all 4 axioms active)

        Also respect TIER constraints (user escalation level).
        """
        axiom_score = self.axiom_monitor.blended_maturity()  # 0-100
        e_score = self.escore_tracker.get_score("CYNIC")       # 0-100
        oracle_conf = await self.oracle.predict_confidence(cell)  # 0-1

        blended = (
            (axiom_score / 100) * 0.4 +
            (e_score / 100) * 0.3 +
            oracle_conf * 0.3
        )

        # Map to consciousness level
        if blended < 0.382:
            return ConsciousnessLevel.REFLEX
        elif blended < 0.618:
            return ConsciousnessLevel.MICRO
        elif blended < 0.82:
            return ConsciousnessLevel.MACRO
        else:
            # Only L4 if all 4 core axioms active
            if self.axiom_monitor.count_active_axioms() >= 4:
                return ConsciousnessLevel.META
            else:
                return ConsciousnessLevel.MACRO
```

**Wire in state.py**:
```python
# In build_kernel():
scheduler = ConsciousnessScheduler(
    axiom_monitor=axiom_monitor,
    escore_tracker=escore_tracker,
    oracle_dog=oracle,
)
orchestrator.consciousness_scheduler = scheduler
```

**Tests**: `tests/test_consciousness_scheduler.py`
- Verify level escalates with axiom maturity
- Verify E-Score influences selection
- Verify oracle prediction affects decision
- Verify L4 only when all 4 axioms active

---

#### A3: Test Full 7-Step Cycle End-to-End (2 days)
**File**: `tests/test_7step_cycle_end_to_end.py` (NEW)

This is THE critical test. Prove that one complete cycle works.

```python
@pytest.mark.asyncio
async def test_full_cycle_perceive_to_emerge():
    """
    End-to-end: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE

    This test proves the instruction set works.
    """
    # Setup
    state = await build_test_kernel()

    # 1. PERCEIVE: Create a cell
    cell = Cell(
        reality="CODE",
        category="handlers",
        content="def buggy_function(): x = 1 / 0",
        timestamp=time.time(),
    )

    # 2. JUDGE: Run orchestrator
    judgment = await state.orchestrator.run(
        cell,
        level=ConsciousnessLevel.MACRO,
    )
    assert judgment.q_score < 38.2  # Should be BARK (buggy code)
    assert judgment.verdict == "BARK"

    # 3. DECIDE: Check governance gate
    decision = await state.decider.decide(judgment)
    assert decision.status in ["APPROVED", "REJECTED", "HUMAN_REVIEW"]

    # 4. ACT: Execute action (if approved)
    if decision.status == "APPROVED":
        action = await state.actor.execute(decision)
        assert action.executed is True

    # 5. LEARN: Human provides feedback
    human_rating = 5  # Perfect diagnosis
    learning_signal = LearningSignal(
        judgment_id=judgment.id,
        human_rating=human_rating,
        actual_outcome=True,
    )
    await state.learner.learn(learning_signal)

    # 6. ACCOUNT: Verify cost tracked
    cost_record = await state.accountant.get_cost(judgment.id)
    assert cost_record.total_cost_usd > 0

    # 7. EMERGE: Verify meta-patterns detected
    await asyncio.sleep(0.1)  # Let handlers fire
    patterns = state.emergent_system.recent_patterns()
    assert len(patterns) > 0  # Some pattern should fire

    # VERIFY: All storage tiers wrote
    hot_count = await state.storage.count_in_tier("HOT")
    warm_count = await state.storage.count_in_tier("WARM")
    cold_count = await state.storage.count_in_tier("COLD")
    assert hot_count > 0 and warm_count > 0  # At least HOT and WARM
```

**Success Criteria**:
- ✅ All 7 steps execute
- ✅ Consciousness level controls which dogs run
- ✅ No cycles detected (genealogy works)
- ✅ Data written to correct tiers
- ✅ Learning updates Q-Table
- ✅ Emergent patterns detected

---

### TIER B: STABILITY (Week 2)

#### B1: Escalation Policy Implementation (1-2 days)
**File**: Modify `cynic/cognition/cortex/axiom_monitor.py`

Add `axiom_monitor.blended_maturity()` method:
```python
def blended_maturity(self) -> float:
    """
    Aggregate axiom maturity into single 0-100 score.

    Formula: φ-weighted geometric mean of 4 active axioms.
    """
    active_axioms = [a for a in self.axioms if a.is_active()]
    if not active_axioms:
        return 0.0

    scores = [a.maturity_score for a in active_axioms]
    log_sum = sum(log(max(s, 0.1)) for s in scores)
    geo_mean = exp(log_sum / len(active_axioms))
    return phi_bound_score(geo_mean)
```

**Tests**: `tests/test_escalation_policy.py`
- Verify escalation threshold at 38.2% (GROWL)
- Verify TIER gates respected
- Verify predictor can override

---

#### B2: Storage Tier Mapping (1-2 days)
**File**: `cynic/core/storage/tier_policy.py` (NEW)

Map each opcode to storage tier:
```python
OPCODE_STORAGE_MAP = {
    CoreEvent.PERCEPTION_RECEIVED: StorageTier.HOT,
    CoreEvent.JUDGMENT_CREATED: [StorageTier.HOT, StorageTier.WARM, StorageTier.COLD],
    CoreEvent.DECISION_MADE: StorageTier.WARM,
    CoreEvent.ACTION_EXECUTED: StorageTier.COLD,
    CoreEvent.LEARNING_SIGNAL_PROCESSED: StorageTier.WARM,
    CoreEvent.COST_ACCOUNTED: StorageTier.COLD,
    CoreEvent.EMERGENCE_DETECTED: [StorageTier.WARM, StorageTier.COLD],
}

class StorageTierPolicy:
    async def write(self, event: Event):
        """Route event to correct storage tier(s)."""
        tiers = OPCODE_STORAGE_MAP.get(event.type)

        if not tiers:
            logger.warning(f"Unknown event type: {event.type}")
            return

        if isinstance(tiers, StorageTier):
            tiers = [tiers]

        for tier in tiers:
            await self._write_tier(tier, event)
```

**Tests**: `tests/test_storage_tier_policy.py`
- Verify all opcodes mapped
- Verify writes go to correct tier
- Verify promotion happens (if configured)

---

#### B3: Add Consciousness Level Tests (1 day)
**File**: `tests/test_consciousness_levels.py`

For each level:
- ✅ L3 REFLEX: 6 dogs, no LLM, <10ms latency
- ✅ L2 MICRO: 7 dogs + SCHOLAR, ~500ms
- ✅ L1 MACRO: all 11 dogs, full cycle, ~2.85s
- ✅ L4 META: daily consolidation, Fisher locking

---

### TIER C: DOCUMENTATION & VALIDATION (Week 3)

#### C1: Phase 1 Completion Criteria Document
**File**: Update `docs/architecture/completion-criteria.md`

Verify:
- [ ] 39/49 cells functional (80% coverage)
- [ ] All 7 opcodes wired
- [ ] Consciousness scheduler active
- [ ] Escalation policy working
- [ ] Storage tiers routing correctly
- [ ] Tests passing: 2500+ (existing) + 500 new = 3000+
- [ ] Zero regressions
- [ ] E2E cycle test passing

#### C2: Update PHASE_1_INSTRUCTION_SET.md
**File**: Modify `PHASE_1_INSTRUCTION_SET.md`

- Mark opcodes as "implemented"
- Document consciousness level selection algorithm
- Add actual code examples (not pseudocode)

#### C3: Create Phase 2 Roadmap
**File**: `PHASE_2_ROADMAP.md` (NEW)

Plan for:
- THE_UNNAMEABLE gate implementation
- 7×7×7 (343 cells) architecture
- ∞^N hypercube storage
- Multi-CYNIC consensus (Type I collective)

---

## PARALLEL WORKSTREAMS

These can happen in parallel (assign to different agents/developers):

| Stream | Owner | Duration | Deliverable |
|--------|-------|----------|-------------|
| A1: Opcode Semantics | Agent 1 | 2-3d | opcode_semantics.py + tests |
| A2: Consciousness Scheduler | Agent 2 | 2-3d | Modified orchestrator.py + tests |
| A3: E2E Cycle Test | Agent 3 | 2d | test_7step_cycle_end_to_end.py |
| B1: Escalation Policy | Agent 1 | 1-2d | Modified axiom_monitor.py + tests |
| B2: Storage Tier Mapping | Agent 2 | 1-2d | tier_policy.py + tests |
| B3: Consciousness Tests | Agent 3 | 1d | test_consciousness_levels.py |
| C1-C3: Docs & Validation | Agent 1 | 3-5d | Completion criteria + roadmap |

**Critical Path**: A1 + A2 + A3 (Week 1) must complete before B1-B3 can be tested.

---

## CRITICAL SUCCESS FACTORS

1. **No Regressions**: All 2500+ existing tests must pass
2. **E2E Proof**: Full 7-step cycle must execute without deadlock or loop
3. **Consciousness Control**: Levels must visibly change behavior (observable in logs)
4. **Storage Correctness**: Events must route to correct tiers
5. **Performance**: L1 MACRO cycle must complete in <2.85s (fibonacci(8) × 21ms)

---

## DEFINITION OF DONE (Phase 1)

✅ All 7 opcodes fully wired
✅ 39/49 cells functional (80%)
✅ Consciousness scheduler active (axiom + E-Score + predictor blended)
✅ Escalation policy enforced (TIER gates working)
✅ Storage tiers routing correctly (OPCODE → tier mapping)
✅ E2E test passing: PERCEIVE → EMERGE (full cycle)
✅ 3000+ tests passing, zero regressions
✅ Zero deadlocks or infinite loops
✅ Performance: L1 cycle <2.85s
✅ Documentation complete

**After**: THE_UNNAMEABLE gate opens for Phase 2

---

## WHAT TRIGGERS PHASE 2?

When THE_UNNAMEABLE metric reaches:
- **Condition 1**: Explained variance > 82% (φ² threshold)
- **Condition 2**: All 4 core axioms active + transcendence signal
- **Condition 3**: User explicitly requests "unlock Phase 2"

Then: 7×7 subdivides into 7×7×7 (343 cells), ∞^N hypercube becomes active.

