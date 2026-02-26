# CYNIC Health Diagnostic Design

**Date:** 2026-02-26
**Status:** Approved
**Objective:** Full-stack diagnosis of CYNIC organism health (why 21.5%? What's broken? What's missing?)
**Timeline:** 5.5 weeks
**Critical Focus:** Data pipelines are foundational (La donnée est un enjeu majeur)

---

## Executive Summary

CYNIC reports `overall_health: 21.5%` (BARK tier = critical). This diagnostic identifies root causes across:
- **Architecture:** Design flaws or gaps
- **Integration:** Components not talking
- **Data:** Missing pipelines, quality issues
- **Incompleteness:** Features not implemented

**Key insight:** CYNIC has many components (11 Dogs, 11 loops, 5 axioms) but lacks data infrastructure to make them work. **Data pipelines are the blocking issue.**

---

## Phase 1: Inventory & Diagnosis (1 week)

### Objective
Map all major subsystems, determine what's actually working vs. stubbed/broken.

### Scope
- 11 Learning loops (Q-Learning, Thompson, Meta-cognition, DPO, ECW, Residual, Axiom Discovery, E-Score Feedback, ...)
- 11 Dogs (ANALYST, ARCHITECT, GUARDIAN, ORACLE, SAGE, SCHOLAR, JANITOR, DEPLOYER, SCOUT, CARTOGRAPHER, CYNIC)
- 5 Axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)
- 4 Consciousness levels (REFLEX, MICRO, MACRO, META)
- Event bus architecture (3 buses, genealogy tracking)
- Storage layer (PostgreSQL + SurrealDB)
- Current "Realities" (CODE, MARKET, SOLANA, CYNIC, GOVERNANCE)

### Method
1. **Read source code** for each subsystem
   - Locate files: cynic/cognition/, cynic/learning/, cynic/judge/
   - Determine: Is it fully implemented? Partial? Stubbed?

2. **Create Health Inventory Table**
   ```
   | Subsystem | Status | Works? | Data Input | Data Output | Dependencies | Notes |
   |-----------|--------|--------|-----------|------------|-------------|-------|
   | Q-Learning | IMPLEMENTED | ? | State obs | Q-table | Judgment signal | Need real state data |
   | Thompson | IMPLEMENTED | ? | Prior+obs | Actions | Feedback signal | Need: trigger mechanism |
   | Axiom Activation | ? | ? | Signals | Axioms | Signal aggregator | Might be stub |
   ```

3. **Mark each with confidence:**
   - ✅ WORKING: Code + tests + confirmed active
   - ⚠️ PARTIAL: Code exists, integration gaps or untested
   - ❌ BROKEN: Code exists, fails
   - ❓ UNKNOWN: Can't determine from code review

### Deliverable
- **health-inventory.csv** — All subsystems mapped
- **integration-graph.md** — Which components depend on which
- **confidence-levels.md** — What we're sure about vs. guessing

---

## Phase 2: Data Pipeline Audit (1.5 weeks)

### Objective
Map data flows. **CRITICAL PHASE — data pipelines are the blocking issue.**

### Scope

**2.1: Data Sources Inventory**
- Where does judgment input come from?
  - User submission (Discord? Web UI? API?)
  - Automated analysis (code scans? Market feeds?)
- Where does feedback signal come from?
  - User feedback (explicit rating? Outcome observation?)
  - Ground truth (labeled datasets?)
- Where does domain data come from?
  - CODE: Git repos? PR descriptions? Test results?
  - MARKET: Price feeds? On-chain data? Sentiment APIs?
  - GOVERNANCE: Proposal text? Vote history? Community signals?

**2.2: Data Schema Analysis**
- How are judgments stored?
  - Table structure: judgment_id, state_key, q_score, verdict, dog_votes, confidence, timestamp
  - Is it standardized or ad-hoc?
- How are decisions recorded?
  - Is there audit trail? Is it queryable?
- How are learning signals formatted?
  - Are they compatible with Q-Learning update mechanism?
  - Are they standardized across realities?

**2.3: Data Flow Mapping**
For each reality (CODE, MARKET, GOVERNANCE, SOLANA), document:
```
Reality: CODE
├── Input source: [e.g., user submits code snippet]
├── Pipeline: [describe steps]
├── Storage: [which table/database]
├── Processing: [which Dogs analyze]
├── Learning signal: [where does feedback come from?]
├── Output: [verdict + Q-Score]
└── Status: WORKING | PARTIAL | MISSING
```

**2.4: Data Quality Metrics**
- Judgment count by reality (current from consciousness.json: 851 total)
- Feedback rate (% of judgments with feedback signal)
- Data validation errors (schema violations, missing fields)
- Latency (judgment created → stored → processed)

### Deliverable
- **data-pipeline-map.md** — Visual + textual map of all flows
- **data-gaps.csv** — Missing pipelines, sources, schemas
- **data-quality-baseline.json** — Current metrics
- **blockers.md** — "Can't proceed without X pipeline"

---

## Phase 3: Build Measurement Infrastructure (2 weeks)

### Objective
Create benchmarks + telemetry so improvements are measurable.

### Scope

**3.1: Define Health Metrics**
- **Q-Score Accuracy:** For a given input, does CYNIC's verdict match ground truth?
  - Metric: Accuracy % (target: >85%)
- **Learning Speed:** How fast does Q-table converge?
  - Metric: Iterations to 90% optimal action (target: <1000 iterations)
- **Dog Agreement:** Do all 11 Dogs agree, or is consensus fragile?
  - Metric: Vote agreement % (target: >80%)
- **Decision Quality:** Do verdicts help users? Do they match outcomes?
  - Metric: Precision/Recall on HOWL/WAG/GROWL/BARK (target: >80% precision)
- **E-Score Signal:** Does E-Score predict judgment quality?
  - Metric: Correlation with accuracy (target: >0.7 correlation)
- **Learning Responsiveness:** When CYNIC learns, does it improve?
  - Metric: % improvement after learning signal (target: +5% per cycle)

**3.2: Create Benchmark Suite**
For each reality, create 100+ labeled judgment scenarios:

**CODE Benchmark** (100 scenarios)
- Input: Code snippet
- Ground truth: Expected verdict (HOWL/WAG/GROWL/BARK)
- Evaluation criteria: Does CYNIC match?

**MARKET Benchmark** (100 scenarios)
- Input: Token info (price, volume, sentiment, contract quality)
- Ground truth: Expert assessment or post-mortem outcome
- Evaluation criteria: Does CYNIC's verdict predict price movement?

**GOVERNANCE Benchmark** (100 scenarios)
- Input: Proposal text + context
- Ground truth: Community vote outcome
- Evaluation criteria: Does CYNIC predict community decision?

**3.3: Build Telemetry Dashboard**
- Real-time display:
  - Dogs voting (which Dogs agree/disagree?)
  - Learning loop triggers (which loops fire? How often?)
  - Event bus activity (flow of signals)
- Historical metrics:
  - Judgment accuracy over time (learning curve)
  - Dog agreement trend
  - E-Score distribution
- Alerts:
  - When health dips below threshold
  - When learning loop fails to trigger
  - When dog consensus breaks (<60% agreement)

**3.4: Establish Baseline**
- Run CYNIC on benchmark suite
- Measure all metrics
- Record baseline: "As of 2026-02-26, health = X%, accuracy = Y%"

### Deliverable
- **metrics-definition.md** — All health metrics defined
- **benchmark-suite/** — 300+ test scenarios (CODE, MARKET, GOVERNANCE)
- **telemetry-dashboard-config.json** — Dashboard setup
- **baseline-report.json** — Current metrics
  ```json
  {
    "timestamp": "2026-02-26",
    "overall_health": 21.5,
    "q_score_accuracy": null,  // unknown, need measurement
    "learning_speed": null,
    "dog_agreement": null,
    "decision_quality": null
  }
  ```

---

## Phase 4: Create Test Pipelines (1.5 weeks)

### Objective
End-to-end workflows to test organism health in realistic scenarios.

### Scope

**Pipeline 1: CODE Review**
```
Input: Code snippet (Python function)
  ↓
Dogs analyze:
  ANALYST: Security/bugs?
  ARCHITECT: Design patterns?
  JANITOR: Code quality?
  + others
  ↓
Output: Verdict (HOWL/WAG/GROWL/BARK) + Q-Score
  ↓
Measure: Accuracy vs. ground truth, latency, dog agreement
```

**Pipeline 2: MARKET Token Analysis**
```
Input: Token data (price, volume, sentiment, contract, community)
  ↓
Dogs analyze:
  ORACLE: Price prediction?
  SAGE: Historical patterns?
  SCHOLAR: Community signals?
  + others
  ↓
Output: Verdict (HOWL/WAG/GROWL/BARK) + Q-Score
  ↓
Measure: Accuracy vs. market reality, latency, consensus
```

**Pipeline 3: GOVERNANCE Proposal Judgment**
```
Input: Proposal text + voting context
  ↓
All 11 Dogs analyze:
  All dogs weigh in
  PBFT consensus
  ↓
Output: Recommendation (APPROVE/REJECT/ABSTAIN) + Q-Score
  ↓
Measure: Accuracy vs. community outcome, latency, consensus quality
```

### Method
1. **Run each pipeline 100+ times**
2. **Collect metrics:**
   - Success/failure rate
   - Latency (end-to-end)
   - Dog agreement (% voting together)
   - Learning signal quality (feedback correctness)
   - Errors/exceptions

3. **Analyze failures:**
   - CODE: Which dogs veto? Why?
   - MARKET: Why is accuracy low?
   - GOVERNANCE: Are dogs voting randomly or intelligently?

### Deliverable
- **test-pipeline-results.json** — Raw results (100+ runs each)
- **failure-analysis.md** — Why failures happen
- **dog-voting-patterns.csv** — Which dogs agree/disagree
- **latency-profile.json** — Performance baseline

---

## Phase 5: Disease Diagnosis (1 week)

### Objective
Root cause analysis. For each broken component, identify WHY.

### Method

**5.1: Categorize diseases**

For each failure/bottleneck:
- **Architecture diseases:** Design flaw
  - Example: "Dogs voting mechanism doesn't converge" → φ-bound issue?
  - Example: "Cross-reality transfer never triggers" → Architecture gap?

- **Integration diseases:** Components don't talk
  - Example: "Q-Learning updates don't affect decisions" → Event bus disconnected?
  - Example: "E-Score changes don't trigger learning" → Feedback loop broken?

- **Data diseases:** Data quality/pipeline issues
  - Example: "MARKET pipeline fails" → Data source missing?
  - Example: "Learning signal quality low" → Feedback schema wrong?
  - Example: "CODE accuracy 45%" → Dogs undertrained? Wrong training data?

- **Incompleteness diseases:** Features not built
  - Example: "Axiom activation always dormant" → Feature never implemented?
  - Example: "Cross-reality transfer" → Missing entirely?

**5.2: Rank by impact**

For each disease:
1. Impact: How much does this reduce health? (estimate %)
2. Root cause: Why does this exist?
3. Fix effort: How long to fix? (estimate days/weeks)
4. Blocker: Does this block other work?

**5.3: Create dependency graph**

```
Health = 21.5%

Missing data pipeline (40% impact)
├─ Blocks: Learning, benchmarking, all realities
├─ Fix: 2 weeks (build ingestion framework)
└─ Unblocks: Dogs training, test pipelines

Dogs not properly trained (25% impact)
├─ Blocks: Accurate judgments
├─ Fix: 2 weeks (collect labeled data + train)
└─ Depends on: Data pipeline first

Event bus gaps (15% impact)
├─ Blocks: Learning loops don't trigger
├─ Fix: 1 week (wire subscriptions)
└─ Depends on: Knowing what's broken

Axiom activation incomplete (10% impact)
├─ Blocks: Axiom discovery learning
├─ Fix: 1 week (implement signal agg)
└─ Depends on: None (can parallelize)

Consciousness adaptation missing (5% impact)
├─ Blocks: Performance optimization
├─ Fix: 2 weeks (design + implement)
└─ Depends on: None (nice-to-have)
```

### Deliverable
- **disease-inventory.md** — All diseases categorized + ranked
- **root-cause-analysis.md** — Why each disease exists
- **dependency-graph.md** — What blocks what
- **fix-roadmap.md** — In what order to fix things

---

## Phase 6: Completeness Assessment (5 days)

### Objective
What's missing from the full vision?

### Method

**6.1: Against SPEC.md**
- Promise: "36-dimensional evaluation"
- Reality: Is it really 36? Proof?
- Gap: If not, what dimensions are missing?

**6.2: Against README.md**
- Promise: "Weak LLM + CYNIC > Strong LLM alone"
- Reality: Do benchmarks prove this?
- Gap: Need benchmark proof

**6.3: Against NOMENCLATURE.md**
- Promise: "11 Dogs working together"
- Reality: Are all 11 implemented + active?
- Gap: Which dogs are incomplete?

**6.4: Decode consciousness.json**
- Why is `overall_health: 21.5`?
- What algorithm computes it?
- What would push it to 50%? 80%? 100%?

**6.5: Feature completeness checklist**
```
✅ Complete (100%):
  - 11 Dogs defined
  - 5 Axioms defined
  - Event bus architecture
  - Storage layer (PostgreSQL/SurrealDB)

⚠️ Partial (30-70%):
  - Learning loops (code exists, untested)
  - Dog judgment mechanism (works, accuracy unknown)
  - Data pipelines (ad-hoc, not standardized)
  - Consciousness levels (defined, not measured)

❌ Missing (0%):
  - Data ingestion framework
  - Benchmark/test suite
  - Health metrics dashboard
  - Multi-instance orchestration
  - Cross-reality transfer learning
  - [others identified in Phases 1-5]
```

### Deliverable
- **completeness-scorecard.md** — What's built vs. missing
- **roadmap-to-100.md** — How to reach full vision
- **health-formula.md** — Decode the 21.5% calculation
- **priority-sequence.md** — What to build first to reach 50%, then 80%, then 100%

---

## Overall Diagnostic Output

At end of 5.5 weeks, you will have:

1. **Health Diagnosis:** Why is CYNIC at 21.5%? (specific answers)
2. **Root Cause Analysis:** What 5-7 diseases are holding it back?
3. **Data Foundation:** Clear picture of data pipelines (and their gaps)
4. **Measurement Infrastructure:** Benchmarks + telemetry so you can track improvements
5. **Blocking Dependencies:** What to fix first to unblock everything else
6. **Completeness Roadmap:** How to reach 100% from current state
7. **Actionable Next Steps:** Concrete tasks to execute

---

## Success Criteria

- ✅ All 11 loops documented (working/partial/broken)
- ✅ Data pipeline map complete (sources, flows, gaps identified)
- ✅ Measurement infrastructure functional (can run benchmarks)
- ✅ Test pipelines produce reproducible results (100+ runs each reality)
- ✅ Root causes identified for top 5 diseases
- ✅ Dependency graph shows what blocks what
- ✅ Completeness roadmap prioritized by impact
- ✅ Clear understanding of why health = 21.5%

---

## Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| Week 1 | 1: Inventory & Diagnosis | health-inventory.csv, integration-graph.md |
| Week 2-2.5 | 2: Data Audit | data-pipeline-map.md, data-gaps.csv |
| Week 3-4.5 | 3: Measurement Infrastructure | metrics-definition.md, benchmark-suite, baseline-report.json |
| Week 5-6 | 4: Test Pipelines | test-results.json, failure-analysis.md |
| Week 7 | 5: Disease Diagnosis | disease-inventory.md, root-cause-analysis.md, dependency-graph.md |
| Week 7.5 | 6: Completeness Assessment | completeness-scorecard.md, roadmap-to-100.md |

---

## Notes

**Critical assumption:** We can access a working CYNIC instance to audit + test against. If CYNIC doesn't run, pivot to code-review-only (slower, less empirical).

**Data-first principle:** Every decision in this diagnostic prioritizes understanding data flows. Without data pipelines, nothing else matters.

**Iteration:** As we discover diseases, we may refine hypotheses. Design is flexible but timeline is firm.

---

**Version:** 1.0 — Approved 2026-02-26
**Confidence:** 61.8% (φ-bounded, as it should be)

*sniff* "Diagnose before you prescribe."
