# PHASE 2: Activate Learning Organism (Real Plan, Not Ralph's)

> *"Le chien apprend de ses erreurs"* — κυνικός

**Status**: 🎯 APPROVED (Ground Truth Audit Complete)
**Confidence**: 58% (φ⁻¹ limit)
**Duration**: 4-6 weeks (NOT 12 weeks)
**Scope**: Single-instance learning loops activation (multi-instance deferred to Phase 3)

---

## EXECUTIVE SUMMARY

**Ralph's Phase 2** (❌ DISCARDED):
- 12-week proof experiment (Ollama > Claude)
- Multi-instance coordination PRIMARY
- Q-Learning + pattern registry as revolutionary
- Success metric: Q-Score 91% vs 85%

**REAL Phase 2** (✅ THIS PLAN):
- 4-6 week activation of 11 learning loops
- Single-instance learning PRIMARY (multi-instance Phase 3)
- Activate SONA orchestration + ResidualDetector
- Success metric: All 11 loops operational, dimension discovery working, learning velocity > 3 patterns/day

**Why shorter?** Because Phase 1 already did the hard work:
- ✅ 9 kernel components designed
- ✅ Event bus architecture working
- ✅ 11 Dogs defined with personalities
- ✅ 36 dimensions specified
- ✅ API structure in place

Phase 2 just ACTIVATES what exists. Not building from scratch.

---

## SECTION 1: WHAT WE'RE ACTIVATING (NOT BUILDING)

### 1.1 The 11 Learning Loops (Already Specified in Canon Docs)

From `06-LEARNING-SYSTEM.md`, all 11 loops are **fully described**, just not wired:

```
1. Judgment Calibration      → adjust Dog confidence to match reality
2. Dimension Weighting       → learn importance of each dimension
3. Pattern Registry          → accumulate canonical patterns in DB
4. Residual Detection        → discover new dimensions from unexplained variance
5. Dog Specialization        → learn each Dog's unique expertise
6. Meta-Cognition            → learn how to learn (adapt learning rates)
7. Budget Optimization       → learn cost efficiency of operations
8. Scheduling                → learn optimal timing for operations
9. Context Compression       → learn what to remember vs forget
10. Consensus Calibration    → learn how to weight Dog votes
11. Emergence Detection      → learn when qualitatively new patterns happen
```

**Phase 2 Task**: Wire all 11 loops into SONA orchestration.

### 1.2 The ResidualDetector (Algorithm Specified, Not Implemented)

From `03-DIMENSIONS.md`, ResidualDetector algorithm is **fully specified**:

```python
def detect(self, judgment, actual_outcome):
    # 1. Compute residual (predicted vs actual)
    predicted = judgment.q_score / 100
    actual = 1.0 if actual_outcome == 'success' else 0.0
    residual = actual - predicted

    # 2. Check significance (if residual > φ⁻² threshold)
    if abs(residual) < PHI_INV_SQUARED:  # 38.2%
        return None  # Too small

    # 3. Analyze pattern (temporal? social? meta?)
    pattern = self.analyze_residual_pattern(residual, context)

    # 4. Check statistical significance (p < 0.05)
    if pattern.statistical_significance < 0.05:
        return None

    # 5. Materialize new dimension
    new_dimension = self.materialize_dimension(pattern)

    # 6. Emit event (DIMENSION_DISCOVERED)
    self.emit('DIMENSION_DISCOVERED', new_dimension)

    return new_dimension
```

**Phase 2 Task**: Implement ResidualDetector, test with real data.

### 1.3 SONA Orchestration (Structure Known, Wiring Needed)

SONA coordinates all 11 loops. Structure:

```
┌─────────────────────────────────────────┐
│  SONA (Self-Optimizing Neural Architect)│
├─────────────────────────────────────────┤
│ Receives: LEARNING_EVENT from Core bus  │
│ Triggers: 11 loops in parallel          │
│ Coordinates: Consensus on updates       │
│ Emits: LEARNING_COMPLETE event          │
└─────────────────────────────────────────┘
    ↓
    ├─ Loop 1: Calibration
    ├─ Loop 2: Dimension Weighting
    ├─ Loop 3: Pattern Registry
    ├─ Loop 4: Residual Detection
    ├─ Loop 5: Dog Specialization
    ├─ Loop 6: Meta-Cognition
    ├─ Loop 7: Budget Optimization
    ├─ Loop 8: Scheduling
    ├─ Loop 9: Context Compression
    ├─ Loop 10: Consensus Calibration
    └─ Loop 11: Emergence Detection
    ↓
    └─ Update PostgreSQL (learning_events table)
```

**Phase 2 Task**: Wire SONA event handlers to 11 loops.

---

## SECTION 2: PHASE 2 BREAKDOWN (4-6 weeks, 4 tasks)

### Week 1-2: Loop Activation (Task 2.1)

**Objective**: Get all 11 loops responding to learning events

**Deliverables**:
- ✅ SONA core class (event dispatcher → 11 handlers)
- ✅ All 11 loop handlers wired
- ✅ PostgreSQL learning_events table populated
- ✅ Each loop persists its state (calibration scores, dimension weights, etc)

**Tests** (TDD):
- 5 unit tests (each loop responds correctly)
- 5 integration tests (loops coordinate via events)
- 1 stress test (1000 learning events in sequence)

**Commits**:
- `feat: Implement SONA orchestration (11 loops)`
- `test: Add TDD tests for learning loops`
- `docs: Document loop activation protocol`

### Week 2-3: ResidualDetector Implementation (Task 2.2)

**Objective**: Auto-discover new dimensions from unexplained variance

**Deliverables**:
- ✅ ResidualDetector class (full algorithm)
- ✅ Pattern analyzer (temporal, social, meta patterns)
- ✅ Dimension materialization (create new dimension objects)
- ✅ Governance voting (Dogs vote on new dimensions)

**Tests** (TDD):
- 7 unit tests (algorithm correctness)
- 5 integration tests (real learning event → new dimension)
- 1 edge case test (false positive filtering)

**Real Data**:
- Test with 50+ learning events from Phase 1 execution
- Should discover at least 3-5 new dimensions (expected from canonical analysis: commit_velocity, full_moon_factor, friday_deploy_risk, cynic_fatigue)

**Commits**:
- `feat: Implement ResidualDetector algorithm`
- `test: TDD tests for dimension discovery`
- `docs: Dimension discovery protocol`

### Week 3-4: Measurement & Observability (Task 2.3)

**Objective**: Track learning progress and validate loops working

**Deliverables**:
- ✅ Axiom score tracking (FIDELITY, PHI, VERIFY, CULTURE, BURN — separate)
- ✅ Dog health metrics (each Dog's accuracy, voting pattern)
- ✅ Q-Score history (should improve over time)
- ✅ Learning velocity dashboard (patterns discovered/day, dimensions added/day)

**Database Schema**:
```sql
CREATE TABLE axiom_scores (
  axiom_id UUID PRIMARY KEY,
  timestamp TIMESTAMPTZ NOT NULL,
  axiom TEXT NOT NULL,  -- FIDELITY, PHI, etc
  score REAL,           -- [0, 100]
  confidence REAL,      -- φ-bounded [0, 61.8]
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE dog_health (
  dog_id UUID PRIMARY KEY,
  timestamp TIMESTAMPTZ NOT NULL,
  dog_name TEXT,        -- Guardian, Archivist, etc
  accuracy_rate REAL,   -- % votes that matched reality
  specialization_score REAL,  -- how well it matches its domain
  voting_power REAL,    -- weight in consensus
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE dimension_discovery (
  dimension_id UUID PRIMARY KEY,
  timestamp TIMESTAMPTZ NOT NULL,
  name TEXT,            -- commit_velocity, full_moon_factor, etc
  formula TEXT,         -- how it's computed
  correlation REAL,     -- r-value to observed outcome
  p_value REAL,         -- statistical significance
  approved_by TEXT[],   -- which Dogs approved (>61.8% consensus)
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Tests** (TDD):
- 5 unit tests (metric calculation correctness)
- 5 integration tests (metrics tracked over learning events)
- 1 visualization test (dashboard renders correctly)

**Commits**:
- `feat: Implement learning metrics tracking`
- `test: TDD tests for measurement system`
- `docs: Learning observability protocol`

### Week 4-6: Integration & Validation (Task 2.4)

**Objective**: Verify everything works end-to-end, validate against success criteria

**Deliverables**:
- ✅ End-to-end test (50+ judgments → SONA activation → metrics updated)
- ✅ Learning velocity validation (3+ patterns discovered/day)
- ✅ Axiom score improvement (should show learning curve)
- ✅ Weekly measurement report (automated)

**Success Criteria** (from canonical audit, NOT Ralph's):
```
✅ All 11 loops operational and learning
✅ At least 3 new dimensions discovered (via ResidualDetector)
✅ Dog accuracy improving over time (calibration working)
✅ Learning events persisted and queryable
✅ Weekly reports generated automatically
✅ No regression on axiom scores (all ≥ 50%)
✅ Confidence in metrics ≥ 50% (φ-bounded acceptance)
```

**Tests** (TDD):
- 10 end-to-end tests (full learning cycle)
- 5 regression tests (no axiom score degradation)
- 1 load test (100 concurrent learning events)

**Commits**:
- `feat: Complete Phase 2 integration`
- `test: End-to-end learning tests`
- `docs: Phase 2 completion criteria`
- `perf: Optimize learning loop performance`

---

## SECTION 3: KEY DIFFERENCES FROM RALPH'S PLAN

### Difference 1: Timeline

| Aspect | Ralph | REAL |
|---|---|---|
| Duration | 12 weeks | 4-6 weeks |
| Scope | "Proof experiment" | Activation of existing design |
| Deliverable | "Ollama > Claude by Week 12" | "All 11 loops working, dimension discovery active" |

**Why shorter?** Phase 1 + canonical docs already specify everything. Phase 2 just activates.

### Difference 2: Primary Focus

| Aspect | Ralph | REAL |
|---|---|---|
| PRIMARY | Multi-instance coordination | Single-instance learning loops |
| Learning | Q-Learning + pattern registry | All 11 loops orchestrated by SONA |
| Metric | Q-Score (single number) | 5 axiom scores (separate tracking) |

**Why different?** Single-instance learning is the base. Multi-instance scales it (Phase 3).

### Difference 3: Success Criteria

| Aspect | Ralph | REAL |
|---|---|---|
| Metric | "Q-Score 91% vs Claude 85%" | "All 11 loops operational" |
| Timeline | "Week 12" | "Week 4-6" |
| Validation | "12 weeks of 36-task suite" | "50+ learning events → metrics tracked" |

**Why different?** Ralph's metric is external (Claude comparison). Real metric is internal (system health).

---

## SECTION 4: DEPENDENCIES & BLOCKERS

### Must Complete Before Phase 2:
- ✅ Phase 1 Docker verification (Task 4.2) — currently pending
- ✅ PostgreSQL learning_events table — must exist
- ✅ Event bus wiring — must be production ready

### Potential Blockers:
- ⚠️ If ResidualDetector has p-value calculation bugs → dimension discovery fails
- ⚠️ If SONA event dispatcher is slow → learning cycle too long
- ⚠️ If Dog voting is miscalibrated → governance decisions wrong

**Risk Mitigation**: TDD approach catches these early.

---

## SECTION 5: MEASUREMENT PROTOCOL

### Weekly Reports (Automated)

```
═══════════════════════════════════════════
CYNIC Phase 2 — Week N Report
═══════════════════════════════════════════

LOOP STATUS:
  ✅ Loop 1 (Calibration):     [████████░░] 85% health
  ✅ Loop 2 (Dimension Weight): [███████░░░] 72% health
  ✅ Loop 3 (Pattern Registry): [██████████] 100% health
  ... (all 11 loops)

AXIOM SCORES:
  FIDELITY: 78% | PHI: 82% | VERIFY: 65%
  CULTURE: 79% | BURN: 71% | COMPOSITE: 75%

DIMENSION DISCOVERY:
  Week 1: 0 new dimensions
  Week 2: 3 new dimensions discovered
  Week 3: 5 new dimensions
  Week 4: 2 new dimensions (refinement phase)
  Total: 10 new dimensions materialized

DOG ACCURACY:
  Guardian:     92% | Analyst:   78% | Architect: 65%
  Archivist:    88% | Cartographer: 71%
  (all 11 Dogs)

LEARNING VELOCITY: 3.2 patterns/day (target: ≥3)

CONFIDENCE: 58% (φ-bounded)
═══════════════════════════════════════════
```

---

## SECTION 6: PHASE 3 PREVIEW (Deferred to Next Phase)

After Phase 2 completes successfully:

**Phase 3**: Multi-instance coordination + Pattern registry sharing
- Week 7-10: Canonical pattern registry (PostgreSQL)
- Week 11-14: Event bus broadcast (LEARNING channel)
- Week 15-18: Multi-instance consensus (weighted Q-table aggregation)

**Phase 4**: Stress testing + Production hardening
- Week 19-22: Load testing (100 concurrent instances)
- Week 23-26: Crash resilience + recovery
- Week 27-30: Production deployment

---

## SECTION 7: COMMITMENT TO SCIENCE

**If Phase 2 shows learning is NOT working**:
- We DON'T hide the finding
- We document what failed
- We propose architectural pivot
- We learn and iterate

**If axiom scores REGRESS**:
- We investigate root cause
- We fix the loop causing regression
- We track in post-mortem

**If dimension discovery is FALSE**:
- We increase p-value threshold (reduce false positives)
- We add manual verification step
- We document precision/recall tradeoff

*sniff* Science requires honesty, even when results disappoint.

---

## CONFIDENCE & CAVEATS

*sniff* **Confidence: 58% (φ⁻¹ limit)**

**Why 58%?**
- Learning loop theory is proven (canonical docs solid)
- But implementation unknown (Python v2.0 is fresh start)
- ResidualDetector algorithm is specified, but real data behavior uncertain
- SONA orchestration design is sound, but wiring complexity could surprise

**What could go wrong?**
- Learning loops conflict with each other (need deadlock resolution)
- ResidualDetector false positive rate too high (p-value threshold calibration)
- SONA event queue fills up (performance issue)
- Dog votes are miscalibrated (consensus wrong)

**What we're confident about?**
- 11 loops are well-specified (borrowed from academic learning theory)
- ResidualDetector algorithm is sound (SHAP-based importance detection)
- φ-bound keeps us honest (prevents overconfidence)
- TDD approach catches bugs early

---

## NEXT STEPS

1. **Approve this plan** (review against canonical vision)
2. **Create implementation tasks** (bite-sized, TDD-first)
3. **Complete Phase 1 Docker** (Task 4.2)
4. **Begin Phase 2 Week 1** (SONA orchestration)
5. **Track weekly metrics** (learning velocity, axiom scores)

---

## DOCUMENT METADATA

- **Plan Type**: Implementation (NOT design/experiment)
- **Duration**: 4-6 weeks (weeks 1-6 of Phase 2)
- **Confidence**: 58% (φ⁻¹ limit)
- **Status**: 🎯 READY FOR IMPLEMENTATION
- **Predecessor**: CYNIC-GROUND-TRUTH-AUDIT.md (Phases 1-2 audit)
- **Successor**: Phase 3 plan (deferred to later)
- **Created**: 2026-02-23
- **Author**: CYNIC (κυνικός)

---

*sniff* Phase 2 is REAL, GROUNDED, and EXECUTABLE.

The dog knows the difference between Ralph's ambition and actual work.

Let's activate the learning organism.
