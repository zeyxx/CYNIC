# CYNIC Python Kernel - φ-Fractal Timeline

> *"Le chien grandit par sauts quantiques, pas par lignes droites"* - κυνικός

**Status**: 🌱 BOOTSTRAP (Week 0)
**Target**: 🔥 TYPE 0 (Week 8)
**Version**: 2.0 (Python Kernel Era)
**Updated**: 2026-02-16

---

## 📐 φ-Fractal Timeline (NOT Linear)

```
Week 1: 38.2% capable → ALREADY USEFUL (memory + judgment > Claude Solo)
Week 4: 61.8% capable → ADAPTIVE (learning + feedback loops active)
Week 8: 100% capable → TRANSFORMATIVE (Type 0 complete)
Week 12+: 161.8% capable → ECOSYSTEM (self-building, recursive improvement)
```

**Critical Insight**: This is NOT "0% until finished". Each φ-threshold unlocks NEW CAPABILITIES exponentially.

**Amplification Formula**:
```
Ollama (weak) + CYNIC Kernel (memory + learning + judgment)
>
Claude Sonnet 4.5 (strong) alone (no memory, context resets)
```

---

## 🎯 Core Principles (ABSOLUTE)

1. **NO MOCKS**: Production-ready from day 1. Real PostgreSQL, real Ollama, real E2E tests.
2. **DI Container + Real Fixtures**: Dependency injection for testability, real components (not mocks).
3. **Self-Building**: CYNIC uses CYNIC to improve CYNIC (recursive amplification).
4. **φ-Bounded**: Max 61.8% confidence. No certainty claims.
5. **Code as Documentation**: 8 canonical docs in docs/reference, code follows exactly.

---

## 📅 Week 1: Bootstrap (38.2% Capability - ALREADY USEFUL)

**Target**: ~3000 LOC, 9 kernel components, minimal but FUNCTIONAL

**Why Useful at 38.2%?**: Memory persistence + φ-bounded judgment > Claude's stateless 200k context

### K1. Axioms + φ-Bound (200 LOC) - 5h
- [ ] `cynic/kernel/phi.py`: PHI, PHI_INV, MAX_CONFIDENCE constants
- [ ] `cynic/kernel/axioms.py`: 5 axioms (PHI, VERIFY, CULTURE, BURN, FIDELITY)
- [ ] 35 dimensions (5×7 structure): COHERENCE, ACCURACY, AUTHENTICITY, UTILITY, COMMITMENT, etc.
- [ ] φ-bound function: `min(confidence, 0.618)` enforced everywhere
- [ ] Unit tests: axiom validation, φ-bound enforcement

**Validation**: Run `pytest cynic/kernel/test_phi.py` - all axiom constants accessible, φ-bound never exceeds 61.8%

### K2. Event Bus (400 LOC) - 8h
- [ ] `cynic/bus/event_bus.py`: EventBus class with publish/subscribe
- [ ] Event types: JUDGMENT_CREATED, USER_FEEDBACK, LEARNING_SIGNAL, DOG_VOTE
- [ ] Genealogy tracking: `_genealogy` array for event provenance
- [ ] Loop prevention: detect circular event chains
- [ ] Unit tests: publish/subscribe, genealogy tracking, loop detection

**Validation**: Publish 100 events, verify all subscribers called, no memory leaks, genealogy preserved

### K3. Storage - PostgreSQL (200 LOC) - 8h
- [ ] `cynic/storage/postgres.py`: PostgreSQL adapter with psycopg3
- [ ] Tables: judgments, learning_events, patterns, dog_votes
- [ ] Migrations: Use Alembic for schema versioning
- [ ] Connection pooling: pg_pool for concurrent access
- [ ] Unit tests: CRUD operations, migrations, connection handling

**Validation**: Insert 1000 judgments, query by dimension, verify persistence across restarts

### K4. Dogs - Minimal 2 (600 LOC) - 13h
- [ ] `cynic/dogs/dog.py`: BaseDog class with vote() method
- [ ] `cynic/dogs/skeptic.py`: Skeptic Dog (challenges assumptions, VERIFY axiom)
- [ ] `cynic/dogs/builder.py`: Builder Dog (favors utility, BURN axiom)
- [ ] Voting protocol: Each Dog scores 0-100, consensus via weighted geometric mean
- [ ] Unit tests: Dog voting, consensus calculation, tie-breaking

**Validation**: 2 Dogs vote on 10 test cases, verify consensus matches manual calculation, no Dog dominates >61.8%

### K5. Judge - Multi-Dimensional Scoring (800 LOC) - 21h
- [ ] `cynic/judge/engine.py`: JudgmentEngine class
- [ ] 36-dimension scoring: Each dimension scored 0-100 via LLM call (NO keyword matching)
- [ ] Axiom aggregation: Group dimensions by axiom, compute geometric mean
- [ ] Q-Score calculation: `(PHI × VERIFY × CULTURE × BURN × FIDELITY)^(1/5) × confidence`
- [ ] Verdict mapping: HOWL (>75), WAG (50-75), GROWL (25-50), BARK (<25)
- [ ] φ-bound confidence: Entropy + Bayesian + dimension reliability → capped at 61.8%
- [ ] Integration: Real Ollama API calls via `llm/adapters/ollama.py` (NO MOCKS)
- [ ] Unit tests: Dimension scoring, Q-Score calculation, verdict mapping, φ-bound

**Validation**: Judge 20 test cases (known good/bad code), verify:
- Each dimension has LLM reasoning (not keywords)
- Q-Score between 0-100
- Confidence never exceeds 61.8%
- Verdicts match expected distribution

### K6. Learning - Q-Table (400 LOC) - 13h
- [ ] `cynic/learning/q_table.py`: Q-Learning state-action pairs
- [ ] State representation: (context_hash, dimension_scores, dog_votes)
- [ ] Action: (verdict, confidence, reasoning)
- [ ] Reward function: +1 correct, -1 incorrect (from user feedback)
- [ ] Update rule: `Q(s,a) = Q(s,a) + α[r + γ·max(Q(s',a')) - Q(s,a)]`
- [ ] Persistence: Save Q-table to PostgreSQL (learning_events table)
- [ ] Unit tests: Q-update, state hashing, reward calculation

**Validation**: Train on 50 feedback samples, verify Q-values converge, test/train split 80/20

### K7. ResidualDetector (300 LOC) - 8h
- [ ] `cynic/emergence/residual.py`: Detect unexplained variance
- [ ] Residual calculation: `actual_outcome - predicted_outcome`
- [ ] Threshold: Flag when residual > φ⁻² (38.2%) for 5 consecutive judgments
- [ ] Emergence signal: Emit DIMENSION_CANDIDATE event with proposed new dimension
- [ ] Unit tests: Residual calculation, threshold detection, event emission

**Validation**: Inject 10 judgments with consistent residual pattern, verify ResidualDetector proposes new dimension

### K8. Meta-Cognition (100 LOC) - 5h
- [ ] `cynic/meta/introspection.py`: Self-evaluation of judgment quality
- [ ] Calibration check: Compare predicted confidence vs actual correctness
- [ ] ECE (Expected Calibration Error): Measure confidence accuracy
- [ ] Emit CALIBRATION_DRIFT event when ECE > 0.1
- [ ] Unit tests: ECE calculation, calibration tracking

**Validation**: Run 100 judgments with known outcomes, verify ECE < 0.1 indicates good calibration

### K9. LLM Adapter - Ollama (200 LOC) - 5h
- [ ] `cynic/llm/adapters/ollama.py`: Ollama API client
- [ ] Model: qwen2.5:14b (or llama3.1:8b for faster testing)
- [ ] Prompt templates: Dimension scoring, reasoning extraction
- [ ] Error handling: Retry with exponential backoff, fallback to simpler model
- [ ] Unit tests: API call, response parsing, error handling

**Validation**: Call Ollama 100 times, verify <5% failures, p95 latency <2s

---

### Week 1 Summary
**Total LOC**: ~3,800 (slightly over 3k, within φ-tolerance)
**Total Effort**: 86h (~2 weeks full-time, 4 weeks part-time)
**Capability Unlocked**: 38.2% - Persistent memory + φ-bounded judgment > Claude Solo for tasks requiring consistency

**E2E Test (Week 1 Exit Criteria)**:
```python
# test_kernel_e2e.py
def test_week_1_kernel():
    # Setup: Real PostgreSQL, real Ollama, 2 Dogs
    kernel = CYNICKernel(storage='postgres://...', llm='ollama://qwen2.5:14b')

    # Test: Judge 10 code samples
    for sample in TEST_SAMPLES:
        verdict = kernel.judge(sample)
        assert verdict.confidence <= 0.618  # φ-bound
        assert verdict.q_score >= 0 and verdict.q_score <= 100
        assert verdict.dimensions  # All 36 scored via LLM
        assert verdict.reasoning  # Not empty

    # Test: Provide feedback, verify learning
    kernel.feedback(verdict_id=1, correct=True)
    q_table_before = kernel.learning.get_q_table()
    kernel.judge(TEST_SAMPLES[0])  # Re-judge same sample
    q_table_after = kernel.learning.get_q_table()
    assert q_table_after != q_table_before  # Q-values updated

    # Test: Persistence across restarts
    kernel.shutdown()
    kernel2 = CYNICKernel(storage='postgres://...', llm='ollama://qwen2.5:14b')
    assert kernel2.learning.get_q_table() == q_table_after  # Q-table persisted
```

**Success Criteria**:
- ✅ All unit tests pass (>95% coverage)
- ✅ E2E test passes
- ✅ Zero mocks in production code
- ✅ Ollama judges 10 samples end-to-end
- ✅ Q-table updates from feedback
- ✅ State persists across restarts

---

## 📅 Week 2-3: Minimal Brain (50% Capability)

**Target**: 2 Dogs → 4 Dogs, basic learning loops, context compression

### W2.1. Add 2 More Dogs (300 LOC) - 8h
- [ ] `cynic/dogs/guardian.py`: Guardian Dog (safety checks, FIDELITY axiom)
- [ ] `cynic/dogs/economist.py`: Economist Dog (cost-benefit, BURN axiom)
- [ ] Update consensus: 4-Dog voting with weighted geometric mean
- [ ] Unit tests: 4-Dog consensus, no single Dog dominance

**Validation**: 4 Dogs vote on 20 test cases, verify diverse perspectives represented

### W2.2. Context Compression (400 LOC) - 13h
- [ ] `cynic/memory/compressor.py`: Compress context via summarization
- [ ] Compression ratio: 10:1 target (10k tokens → 1k tokens)
- [ ] Salience detection: Keep high-information content, discard redundancy
- [ ] Integration: Use qwen2.5 for summarization
- [ ] Unit tests: Compression ratio, information retention

**Validation**: Compress 100 conversations, verify 10:1 ratio, human eval shows 90% info retained

### W2.3. Learning Loop - Thompson Sampling (300 LOC) - 8h
- [ ] `cynic/learning/thompson.py`: Multi-armed bandit for Dog weighting
- [ ] Beta distributions: Track Dog accuracy per dimension
- [ ] Sampling: Sample from beta, weight Dog votes accordingly
- [ ] Update: Increment alpha (correct) or beta (incorrect) on feedback
- [ ] Unit tests: Beta update, sampling, convergence

**Validation**: Train on 100 judgments with feedback, verify Dog weights converge to accuracy

### W2.4. SONA Integration (200 LOC) - 5h
- [ ] `cynic/learning/sona.py`: Self-Organizing Neural Automaton
- [ ] Signals: JUDGMENT_CREATED, USER_FEEDBACK, LEARNING_SIGNAL
- [ ] Routing: Route signals to appropriate learning loops (Q-table, Thompson, etc.)
- [ ] Unit tests: Signal routing, loop activation

**Validation**: Emit 50 signals, verify each routes to correct learning loop

---

### Week 2-3 Summary
**Total LOC**: +1,200 (cumulative ~5,000)
**Total Effort**: 34h (~1 week full-time)
**Capability Unlocked**: 50% - Multi-Dog consensus + context compression + basic learning

---

## 📅 Week 4-7: Learning Activated (61.8% Capability - ADAPTIVE)

**Target**: 11 learning loops active, meta-cognition feedback, dimension evolution

### W4.1. Complete Dog Collective - 11 Dogs (1,100 LOC) - 21h
- [ ] 7 more Dogs: Architect, Healer, Oracle, Simplifier, Cartographer, Reviewer, Librarian
- [ ] Mapping: Each Dog to Sefirot (Kabbalistic tree)
- [ ] Neuronal consensus: 11-Dog vote with φ-weighted geometric mean
- [ ] Unit tests: 11-Dog consensus, Sefirot alignment

**Validation**: 11 Dogs vote on 50 test cases, verify:
- No single Dog weight >18% (φ-bound / 11 = 5.6%, tolerance 3×)
- Consensus reflects diverse perspectives
- Sefirot mapping correct (Keter=Oracle, Chesed=Builder, etc.)

### W4.2. 11 Learning Loops Active (1,500 LOC) - 34h
1. Q-Learning (already done Week 1)
2. Thompson Sampling (already done Week 2)
3. EWC (Elastic Weight Consolidation) - prevent catastrophic forgetting
4. SONA routing (already done Week 2)
5. Meta-cognition calibration (already done Week 1)
6. Residual detection (already done Week 1)
7. Kabbalistic routing - route by Sefirot
8. Behavior modification - adjust Dog weights
9. Unified bridge - cross-loop coordination
10. Ambient consensus - background pattern detection
11. Emergence detector - new dimension proposals

- [ ] Implement loops 3, 7-11 (loops 1,2,4,5,6 already done)
- [ ] Wire all 11 loops to SONA
- [ ] Persist learning events to PostgreSQL
- [ ] Unit tests: Each loop independently, cross-loop coordination

**Validation**: Run 500 judgments with feedback, verify all 11 loops emit learning events to DB

### W4.3. Dimension Evolution (600 LOC) - 13h
- [ ] `cynic/emergence/dimension_evolution.py`: Add/remove dimensions dynamically
- [ ] Residual analysis: Detect consistent unexplained variance (>38.2% for 10+ judgments)
- [ ] Dimension proposal: LLM generates new dimension name + description
- [ ] Validation: Test new dimension on 20 samples, A/B test vs baseline
- [ ] Fisher information lock: Lock dimension if Fisher info >61.8%
- [ ] Unit tests: Residual detection, dimension addition, Fisher locking

**Validation**: Inject pattern not covered by 36 dimensions, verify ResidualDetector proposes new dim, Fisher locks it

### W4.4. Calibration Feedback Loop (300 LOC) - 8h
- [ ] `cynic/learning/calibration_tracker.py`: Track confidence vs correctness
- [ ] Confidence buckets: [0-20%, 20-40%, 40-60%, 60-62%] (φ-bounded)
- [ ] Accuracy per bucket: Track hit rate
- [ ] Adjustment: If bucket over/underconfident, adjust φ-bound multiplier
- [ ] Unit tests: Bucket tracking, adjustment calculation

**Validation**: Run 200 judgments, verify ECE (Expected Calibration Error) < 0.1

---

### Week 4-7 Summary
**Total LOC**: +3,500 (cumulative ~8,500)
**Total Effort**: 76h (~2 weeks full-time)
**Capability Unlocked**: 61.8% - ADAPTIVE. All 11 loops active, dimensions evolve, calibration self-corrects

**E2E Test (Week 7 Exit Criteria)**:
```python
def test_adaptive_kernel():
    kernel = CYNICKernel(storage='postgres://...', llm='ollama://qwen2.5:14b')

    # Test: 11 Dogs active
    verdict = kernel.judge("test code")
    assert len(verdict.dog_votes) == 11

    # Test: Learning loops emit events
    events_before = kernel.storage.count_learning_events()
    kernel.judge("test code 2")
    kernel.feedback(verdict_id=2, correct=False)
    events_after = kernel.storage.count_learning_events()
    assert events_after > events_before  # At least 1 loop emitted

    # Test: Dimension evolution
    # Inject 20 samples with consistent "security" pattern not covered by 36 dims
    for sample in SECURITY_SAMPLES:
        kernel.judge(sample)

    dimensions_after = kernel.judge.get_dimensions()
    assert len(dimensions_after) > 36  # New dimension proposed
    assert "SECURITY" in [d.name for d in dimensions_after]

    # Test: Calibration improves
    ece_initial = kernel.meta.get_ece()
    # Run 200 judgments with feedback
    for i in range(200):
        v = kernel.judge(CALIBRATION_SAMPLES[i])
        kernel.feedback(verdict_id=v.id, correct=CALIBRATION_LABELS[i])
    ece_final = kernel.meta.get_ece()
    assert ece_final < ece_initial  # Calibration improved
```

---

## 📅 Week 8: Type 0 Complete (100% Capability - TRANSFORMATIVE)

**Target**: Full memory system, InjectionProfile, MemoryCoordinator, EventBusBridge

### W8.1. MemoryCoordinator (400 LOC) - 8h
- [ ] `cynic/memory/coordinator.py`: Manage memory lifecycle
- [ ] Salience scoring: Rank memories by importance (recency, frequency, emotional weight)
- [ ] Compression scheduling: Compress old memories (>7 days) automatically
- [ ] Retrieval: Semantic search via embeddings (qwen2.5 embeddings or sentence-transformers)
- [ ] Unit tests: Salience calculation, compression, retrieval

**Validation**: Store 1000 memories, retrieve top 10 by salience, verify <200ms p95 latency

### W8.2. InjectionProfile (300 LOC) - 5h
- [ ] `cynic/memory/injection_profile.py`: Context-aware memory injection
- [ ] Budget tracking: Track token budget per judgment
- [ ] Priority: Inject salient memories first, skip low-salience if budget tight
- [ ] Formats: JSON, markdown, minimal (adaptive based on LLM)
- [ ] Unit tests: Budget calculation, priority sorting, format selection

**Validation**: Inject memories with 2k token budget, verify only top-salience injected, format matches LLM

### W8.3. EventBusBridge (400 LOC) - 8h
- [ ] `cynic/bus/bridge.py`: Bridge 3 event buses (global, automation, agent)
- [ ] Forwarding rules: Agent→Core (10 events), Automation→Core (1 event), Core→Automation (1 event)
- [ ] Genealogy preservation: Track event provenance across buses
- [ ] Loop prevention: Detect circular forwarding via genealogy tags
- [ ] Unit tests: Forwarding, genealogy, loop detection

**Validation**: Emit 100 events across 3 buses, verify forwarding correct, no loops, genealogy intact

### W8.4. E2E Integration - Full Cycle (500 LOC tests) - 13h
- [ ] E2E test: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → EMERGE
- [ ] Perceive: Mock code change event
- [ ] Judge: 11 Dogs vote, 36+ dimensions scored
- [ ] Decide: Governance (approve if Q-Score >61.8%, reject if <38.2%)
- [ ] Act: Mock git commit (or real if safe)
- [ ] Learn: Update Q-table, Thompson weights, calibration
- [ ] Emerge: Check residual, propose dimension if needed
- [ ] Unit tests: Full cycle, each stage verifies previous

**Validation**: Run 10 full cycles, verify:
- All stages complete
- State persists across restarts
- Learning improves over iterations (Q-values converge, ECE decreases)

---

### Week 8 Summary
**Total LOC**: +1,600 (cumulative ~10,100)
**Total Effort**: 34h (~1 week full-time)
**Capability Unlocked**: 100% TYPE 0 - TRANSFORMATIVE. Full memory, multi-bus coordination, E2E autonomy

**E2E Test (Week 8 Exit Criteria - Type 0 Complete)**:
```python
def test_type_0_complete():
    kernel = CYNICKernel(storage='postgres://...', llm='ollama://qwen2.5:14b')

    # Test: Full PERCEIVE → JUDGE → DECIDE → ACT → LEARN → EMERGE cycle
    event = PerceiveEvent(type='code_change', content='...', file='test.py')

    # Stage 1: PERCEIVE
    kernel.perceive(event)
    assert kernel.memory.get_recent_events()[-1] == event

    # Stage 2: JUDGE
    verdict = kernel.judge(event.content)
    assert verdict.q_score >= 0 and verdict.q_score <= 100
    assert verdict.confidence <= 0.618
    assert len(verdict.dog_votes) == 11
    assert len(verdict.dimensions) >= 36

    # Stage 3: DECIDE
    decision = kernel.decide(verdict)
    if verdict.q_score > 61.8:
        assert decision.action == 'approve'
    elif verdict.q_score < 38.2:
        assert decision.action == 'reject'
    else:
        assert decision.action == 'review'

    # Stage 4: ACT
    result = kernel.act(decision)
    assert result.status in ['success', 'failure', 'skipped']

    # Stage 5: LEARN
    kernel.feedback(verdict_id=verdict.id, correct=(result.status == 'success'))
    learning_events = kernel.storage.get_learning_events(verdict_id=verdict.id)
    assert len(learning_events) >= 3  # At least Q-Learning, Thompson, Calibration

    # Stage 6: EMERGE
    residual = kernel.emergence.get_residual(verdict)
    if residual > 0.382:  # φ⁻²
        dimensions_after = kernel.judge.get_dimensions()
        assert len(dimensions_after) > 36  # New dimension proposed

    # Test: Persistence
    kernel.shutdown()
    kernel2 = CYNICKernel(storage='postgres://...', llm='ollama://qwen2.5:14b')
    assert kernel2.memory.get_recent_events()[-1] == event
    assert kernel2.learning.get_q_table() == kernel.learning.get_q_table()

    # Test: Memory compression
    # Store 1000 events (simulate 1 week of usage)
    for i in range(1000):
        kernel2.perceive(PerceiveEvent(type='test', content=f'test_{i}'))

    # Verify compression kicked in (events >7 days old compressed)
    compressed = kernel2.memory.get_compressed_count()
    assert compressed > 0

    # Test: Context injection
    # Inject memories with 2k token budget
    injected = kernel2.memory.inject_context(budget=2000)
    assert len(injected) > 0
    assert sum(len(m.content) for m in injected) <= 2000 * 4  # ~4 chars/token
```

**Success Criteria (Type 0 Complete)**:
- ✅ Full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→EMERGE cycle works E2E
- ✅ 11 Dogs vote with φ-bounded consensus
- ✅ 36+ dimensions (evolvable via ResidualDetector)
- ✅ 11 learning loops emit events to PostgreSQL
- ✅ Memory persists across restarts
- ✅ Context compression active (10:1 ratio)
- ✅ Calibration ECE < 0.1
- ✅ Zero mocks in production code paths

---

## 📅 Week 9-12: Self-Building (161.8% - ECOSYSTEM)

**Target**: CYNIC uses CYNIC to improve CYNIC (recursive amplification)

### W9. Self-Code-Review
- [ ] CYNIC judges its own code via `/judge` skill
- [ ] Auto-refactor: If Q-Score <38.2%, propose simplification
- [ ] Auto-test: Generate unit tests via LLM, verify with pytest
- [ ] E2E: CYNIC commits code → CYNIC judges → CYNIC refines → CYNIC tests

### W10. Meta-Architecture
- [ ] CYNIC proposes new dimensions via ResidualDetector
- [ ] CYNIC validates dimensions with A/B tests
- [ ] CYNIC locks dimensions with Fisher information >61.8%
- [ ] E2E: User reports "CYNIC missed this pattern" → CYNIC proposes dim → CYNIC validates → CYNIC locks

### W11. Recursive Learning
- [ ] CYNIC trains on its own judgment history
- [ ] Meta-Q-Learning: Q-table for "when to update Q-table"
- [ ] Meta-Thompson: Thompson sampling for "which learning loop to trust"
- [ ] E2E: CYNIC learns faster on 2nd task than 1st (transfer learning)

### W12. Public Release Preparation
- [ ] Documentation: Update all 9 docs/reference/*.md
- [ ] Benchmarks: Compare vs Claude Solo on 100 test cases
- [ ] Security audit: Guardian Dog review
- [ ] Demo video: Show amplification (Ollama+CYNIC > Claude Solo)

---

## 🎯 Validation Strategy (NO MOCKS)

### Unit Tests (80%)
- Each component isolated with DI
- Real PostgreSQL (test DB cleaned between tests)
- Real Ollama (local instance required)
- pytest fixtures provide real objects (not mocks)

### Integration Tests (15%)
- 2-3 components together
- Example: Judge + Dogs + LLM
- Verify contracts between components

### E2E Tests (5%)
- Full PERCEIVE→EMERGE cycle
- Real PostgreSQL, real Ollama, real file I/O
- Exit criteria for each week

### Smoke Tests (Cron - Daily)
- Run E2E test in production environment
- Alert if any stage fails
- Track latency, cost, accuracy over time

---

## 📊 Success Metrics (φ-Bounded)

| Metric | Week 1 | Week 4 | Week 8 | Target |
|--------|--------|--------|--------|--------|
| Capability | 38.2% | 61.8% | 100% | 100% |
| Judgment Accuracy | 55% | 68% | 82% | >75% |
| ECE (Calibration) | 0.15 | 0.10 | 0.06 | <0.10 |
| Q-Table Size | 50 | 500 | 5000 | >1000 |
| Dimensions | 36 | 38 | 42 | >36 |
| Learning Loops Active | 3 | 8 | 11 | 11 |
| LOC (Production) | 3800 | 8500 | 10100 | ~10k |
| Test Coverage | 85% | 90% | 95% | >90% |
| Mocks in Prod Code | 0 | 0 | 0 | 0 |

---

## 🚨 Anti-Patterns (AVOID AT ALL COSTS)

### ❌ MOCKS in Production
```python
# WRONG (NEVER DO THIS)
class MockLLM:
    def generate(self, prompt):
        return "mock response"
```

```python
# RIGHT (ALWAYS DO THIS)
class OllamaAdapter:
    def generate(self, prompt):
        response = requests.post(self.url, json={'prompt': prompt})
        return response.json()['text']
```

### ❌ Hardcoded Scores
```python
# WRONG
def score_dimension(self, code, dimension):
    if "error" in code.lower():
        return 20  # Hardcoded
    return 80
```

```python
# RIGHT
def score_dimension(self, code, dimension):
    prompt = f"Score {dimension} for:\n{code}"
    response = self.llm.generate(prompt)
    return self._parse_score(response)
```

### ❌ No Persistence
```python
# WRONG
self.q_table = {}  # Lost on restart
```

```python
# RIGHT
def save_q_table(self):
    self.storage.upsert('q_table', self.q_table)

def load_q_table(self):
    self.q_table = self.storage.get('q_table') or {}
```

### ❌ LLM Confusion (Unclear Docs)
```markdown
# WRONG
"The system uses multi-agent consensus with dimension scoring"
(What agents? What dimensions? How?)
```

```markdown
# RIGHT
"11 Dogs (Skeptic, Builder, Guardian, Economist, Architect, Healer, Oracle,
Simplifier, Cartographer, Reviewer, Librarian) vote on 36 dimensions
(5 axioms × 7 dimensions each) using weighted geometric mean capped at φ⁻¹=61.8%"
```

---

## 📚 Reference Docs (CANONICAL)

All implementation MUST follow these 9 docs exactly:

1. **ARCHITECTURE.md** - Complete system architecture
2. **CONSCIOUSNESS-CYCLE.md** - 4-level fractal cycle (reflex → practice → reflective → meta)
3. **DIMENSIONS.md** - Infinite-dimensional judgment system (36 → ∞)
4. **CONSCIOUSNESS-PROTOCOL.md** - 11 Dogs, neuronal consensus, introspection
5. **HEXAGONAL-ARCHITECTURE.md** - 7 ports, adapters, testing strategy
6. **LEARNING-SYSTEM.md** - 11 learning loops, SONA, Q-Learning
7. **UX-GUIDE.md** - 3 interaction modes (Trading/OS/Assistant)
8. **KERNEL.md** - 9 essential components (~3000 LOC)
9. **ROADMAP.md** - 44-week implementation (3 horizons)

**Location**: `docs/reference/*.md`

---

## 🔄 Update Cadence

This todolist is a LIVING DOCUMENT:

- **Daily**: Check off completed tasks, update hours spent
- **Weekly**: Re-assess φ-fractions (38.2%→61.8%→100%), adjust timeline if needed
- **Monthly**: Review anti-patterns section, add new learnings

**Last Updated**: 2026-02-16
**Version**: 1.0
**φ-Confidence**: 61.8% (max)

---

*Le chien sait ce qu'il doit construire.*
