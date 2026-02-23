# CYNIC Phase 2: Learning Organism Design

> **Status**: Design Complete & Approved (2026-02-23)
> **Confidence**: 58% (φ⁻¹ limit)
> **Next Step**: Invoke superpowers:writing-plans to create implementation plan

---

## Executive Summary

Phase 2 builds CYNIC from a chat/code interface (Phase 1) into a **learning organism** that amplifies weak LLMs through persistent memory, multi-instance coordination, human validation gates, and measurable emergence.

**The Thesis**: Ollama (weak LLM) + CYNIC Kernel (memory + learning + judgment) > Claude Sonnet 4.5 (strong LLM) by Week 12.

**Architecture**: Multi-instance Q-Learning with human approval gates, pattern registry, event-driven coordination, and comprehensive measurement across 5 axioms × 7 dimensions.

**Success Criteria**:
- Primary: CYNIC reaches 91% Q-Score vs Claude's 85% by Week 12
- Secondary: 3-user multi-instance CYNIC outperforms 1-user by 26%
- Tertiary: User agreement ≥80%, no axiom regressions

---

## SECTION 1: ARCHITECTURE ELEVATION

### 1.1 The Multi-Instance Organism

CYNIC becomes a distributed system where:
- **Each instance** = local Q-table, session memory, user interactions
- **Shared registry** = PostgreSQL canonical patterns (approved by Senior Dev)
- **Event bus** = LEARNING channel broadcasts approved patterns to all instances
- **Consensus** = Senior Dev approval creates ground truth, instances converge on it

```
┌─────────────────────────────────────────────────────┐
│ CYNIC MULTI-INSTANCE ARCHITECTURE                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  USER ALICE                    USER BOB             │
│  Instance A                    Instance B           │
│  ┌──────────────────┐    ┌──────────────────┐      │
│  │ Local Q-Table    │    │ Local Q-Table    │      │
│  │ (session memory) │    │ (session memory) │      │
│  └────────┬─────────┘    └────────┬─────────┘      │
│           │                       │                 │
│           │ Gate Request          │ Gate Request    │
│           │ (if novel/uncertain)  │                 │
│           └──────────┬────────────┘                 │
│                      ▼                              │
│        ┌──────────────────────────────┐             │
│        │ CANONICAL PATTERN REGISTRY    │             │
│        │ (PostgreSQL)                 │             │
│        │                              │             │
│        │ Senior Dev: APPROVE          │             │
│        │ Emit: PATTERN_APPROVED       │             │
│        └──────────────────────────────┘             │
│                      △                              │
│                      │ Event Bus LEARNING           │
│           ┌──────────┴──────────┐                   │
│           ▼                      ▼                   │
│      Instance A             Instance B              │
│      Load pattern           Load pattern            │
│      Q(s,a) ← 0.72          Q(s,a) ← 0.72          │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 1.2 Database Schema

**canonical_patterns table** (PostgreSQL):
```sql
CREATE TABLE canonical_patterns (
    pattern_id TEXT PRIMARY KEY,
    state_key TEXT NOT NULL,              -- "CODE:JUDGE:PRESENT:1"
    action TEXT NOT NULL,                 -- verdict (WAG, HOWL, etc)
    q_value_canonical REAL,               -- Approved Q-value
    visits_total INT DEFAULT 0,           -- Sum across all instances
    confidence_canonical REAL,            -- φ-bounded [0, 61.8]
    approved_by TEXT[],                   -- Which Senior Devs approved
    approved_at TIMESTAMPTZ,
    tags TEXT[] DEFAULT ARRAY[],          -- ["canonical", "high_confidence", ...]
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_state (state_key)
);
```

**gate_requests table** (PostgreSQL):
```sql
CREATE TABLE gate_requests (
    gate_id TEXT PRIMARY KEY,
    state_key TEXT NOT NULL,
    action TEXT NOT NULL,
    trigger_type TEXT,                    -- NOVELTY | LOW_CONFIDENCE | CONSENSUS_DISAGREEMENT
    q_value_before REAL,
    q_value_proposed REAL,
    confidence_score REAL,
    judgment_id TEXT REFERENCES judgments,
    status TEXT DEFAULT 'PENDING',        -- PENDING | APPROVED | REJECTED
    reviewed_by TEXT,                     -- Senior Dev
    reviewed_at TIMESTAMPTZ,
    review_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_pending (status, created_at)
);
```

### 1.3 Event Bus Extension

Add **LEARNING channel** to existing 3-bus architecture:

```python
# CORE bus events
CoreEvent.LEARNING_EVENT              # Q-Learning input
CoreEvent.PATTERN_APPROVED            # Senior Dev approved pattern
CoreEvent.PATTERN_REJECTED            # Senior Dev rejected pattern
CoreEvent.MEASUREMENT_AGGREGATED      # Hourly axiom scores ready
```

### 1.4 Files to Create

- `cynic/organism/pattern_registry.py` — Read/write canonical patterns
- `cynic/api/routers/patterns.py` — GET /api/patterns endpoints
- `cynic/db/migrations/` — Add canonical_patterns, gate_requests tables
- `cynic/core/measurement_pipeline.py` — Collect axiom measurements

### 1.5 Success Criteria (Section 1)

- ✓ Multiple CYNIC instances run simultaneously
- ✓ Pattern learned in Instance A available to Instance B within 2 seconds
- ✓ All instances sync Q-table on startup
- ✓ Event broadcast works (pattern approval → instances notified)

---

## SECTION 2: LEARNING GATES

### 2.1 The Problem & Solution

**Problem**: Q-table learns blindly from reward signal. No human oversight before patterns consolidate.

**Solution**: Gate novel/uncertain/conflicting patterns. Require Senior Dev approval before Q-table updates.

### 2.2 Gate Types & Triggers

```
1. NOVELTY GATE: First time seeing state_key
   → New state-action pair (visits == 0 → 1)

2. LOW_CONFIDENCE GATE: Judgment confidence < 50%
   → Pattern might be unreliable

3. CONSENSUS_DISAGREEMENT GATE: Pattern conflicts with peer instances
   → Local Q differs from registry by >5%
```

### 2.3 Workflow (5 Steps)

```
1. LEARNING_EVENT fired
2. CHECK GATE CONDITIONS
   └─ YES → CREATE GATE REQUEST
   └─ NO → UPDATE Q-TABLE (normal)
3. SENIOR DEV REVIEWS (REST API)
4. APPROVED → Update Q-table + mark trusted
   REJECTED → Revert Q-value
5. If approved with ["canonical"]: BROADCAST to all instances
```

### 2.4 REST API for Approvals

```
GET /api/learning/gates/pending
  → Returns pending gate requests

POST /api/learning/gates/{gate_id}/approve
  → reviewed_by, approval_reason, tags, apply_to_all_instances

POST /api/learning/gates/{gate_id}/reject
  → reviewed_by, rejection_reason

POST /api/learning/gates/{gate_id}/defer
  → Defer decision
```

### 2.5 Data Model

```sql
CREATE TABLE gate_approvals (
    approval_id TEXT PRIMARY KEY,
    gate_id TEXT REFERENCES gate_requests,
    approved_by TEXT NOT NULL,
    approved_at TIMESTAMPTZ DEFAULT NOW(),
    q_value_approved REAL,
    tags TEXT[] DEFAULT ARRAY[],         -- ["canonical", "high_confidence", ...]
    notes TEXT,
    applied_to_instances TEXT[],         -- ["instance-prod-1", "instance-prod-2"]
    applied_at TIMESTAMPTZ
);
```

### 2.6 Files to Create

- `cynic/api/routers/learning_gates.py` — Gate request/approval endpoints
- `cynic/learning/gate_handler.py` — Gate trigger logic on LEARNING_EVENT
- `cynic/db/migrations/` — Add gate_requests, gate_approvals tables

### 2.7 Success Criteria (Section 2)

- ✓ Gate triggers on NOVELTY, LOW_CONFIDENCE, CONSENSUS_DISAGREEMENT
- ✓ Senior Dev can approve/reject/defer via REST API
- ✓ Approved patterns broadcast to all instances
- ✓ Review latency < 5 minutes (p50)
- ✓ Approval rate > 80%

---

## SECTION 3: MEASUREMENT SYSTEM

### 3.1 Framework: 5 Axioms × 7 Dimensions

```
FIDELITY (🐕 loyalty to truth):
  1. Consistency    2. Transparency    3. Error Handling
  4. Recovery       5. Honesty         6. Predictability     7. Accountability

PHI (φ proportions):
  1. Structure      2. Proportions     3. Balance
  4. Elegance       5. Symmetry        6. Recursion          7. Emergence

VERIFY (✓ verification):
  1. Testing        2. Validation      3. Evidence
  4. Audit Trail    5. Reproducibility 6. Precision          7. Verification

CULTURE (⛩ alignment):
  1. Convention     2. Idiom           3. Alignment
  4. Belonging      5. Momentum        6. Narrative          7. Identity

BURN (🔥 simplicity):
  1. Simplicity     2. Necessity       3. Efficiency
  4. Focus          5. Minimalism      6. Speed              7. Directness
```

**Q-Score Formula**: Geometric mean of 5 axiom scores
```
Q = 100 × ⁵√(F × Φ × V × C × B / 100⁵)
```

### 3.2 Symbiosis Emergence Metrics (8 dimensions)

Track human-CYNIC alignment:

| Metric | Target |
|--------|--------|
| Agreement Rate | ≥ 80% (user feedback aligns with CYNIC) |
| Trust Score | ≥ 4.0 / 5.0 (user confidence) |
| Learning Velocity | ≥ 5 patterns/day (discovery rate) |
| Execution Success | ≥ 70% (user executes recommendation) |
| Energy Alignment | Inverse correlation (user energy vs workload) |
| Feedback Quality | ≥ 50 chars avg (detailed feedback) |
| Emergence Index | ≥ 60% (novel solutions from interaction) |
| Symbiosis Index | (Agreement + Trust + Learning + Execution)/4 |

### 3.3 Comparative Measurement

Track CYNIC vs Claude across same 36 dimensions:

- **Control**: Claude Code (static, no learning)
- **Treatment**: CYNIC (with learning loops)
- **Duration**: 12 weeks
- **Tasks**: 36 tasks (one per dimension)

Expected convergence:
```
Week 1:  CYNIC 52%, Claude 85% (CYNIC learning)
Week 4:  CYNIC 61.8%, Claude 85% (Inflection point)
Week 8:  CYNIC 78%, Claude 85% (Catching up)
Week 12: CYNIC 91%, Claude 85% (CYNIC wins ✓)
```

### 3.4 Database Tables for Metrics

```sql
CREATE TABLE metrics (
    metric_id TEXT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    instance_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,           -- "fidelity_consistency", etc
    value REAL NOT NULL,                 -- [0, 100]
    dimension TEXT,                      -- "Consistency", "Trust", etc
    axiom TEXT,                          -- "FIDELITY", "PHI", etc
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE axiom_scores (
    axiom_id TEXT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    instance_id TEXT NOT NULL,
    axiom TEXT NOT NULL,                 -- "FIDELITY", etc
    score REAL,                          -- Weighted average of 7 dims
    dimensions JSONB,                    -- {"Consistency": 92, ...}
    confidence REAL,                     -- φ-bounded [0, 61.8]
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE symbiosis_metrics (
    symbiosis_id TEXT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    session_id TEXT REFERENCES chat_sessions,
    user_id TEXT,
    agreement_vote TEXT,                 -- "agree" | "disagree" | "partial"
    trust_rating REAL,                   -- 1.0 - 5.0
    energy_level INT,                    -- 1-10
    execution_success BOOLEAN,
    symbiosis_index REAL,                -- Composite [0, 100]
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE comparative_baseline (
    baseline_id TEXT PRIMARY KEY,
    week INT,
    instance_type TEXT,                  -- "cynic_instance" | "claude_code"
    dimension TEXT,
    axiom TEXT,
    score REAL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 3.5 Files to Create

- `cynic/core/measurement_pipeline.py` — Collect axiom data
- `cynic/api/routers/metrics.py` — GET /api/metrics endpoints
- `cynic/ui/dashboard.html` — Real-time measurement dashboard
- `scripts/weekly_report.py` — Generate weekly proof reports

### 3.6 Success Criteria (Section 3)

- ✓ All 36 dimensions measurable and tracked
- ✓ Symbiosis metrics collected from user feedback
- ✓ Comparative baseline established (CYNIC vs Claude)
- ✓ Dashboard shows axiom scores in real-time
- ✓ Weekly reports generated automatically

---

## SECTION 4: MULTI-USER PATTERNS & EMERGENCE

### 4.1 User-Specific vs Canonical Learning

```
ALICE discovers pattern:
  Local Q-table: Q(state_key="CODE:JUDGE:PRESENT:1", action="WAG") = 0.72

SENIOR DEV approves:
  Create canonical pattern with Alice's Q-value

BOB encounters same state:
  Loads canonical Q = 0.72 (from Alice)
  Bob's feedback improves/challenges it

COLLECTIVE LEARNING:
  If both agree: canonical Q strengthens (visits++)
  If they disagree: conflict resolution (weighted average)
```

### 4.2 User Profile Tracking

```sql
CREATE TABLE user_profiles (
    user_id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    learning_domain TEXT[],              -- ["code_generation", "social_analysis", ...]
    pattern_contribution_count INT,      -- # patterns discovered
    senior_dev_approval_rate REAL,       -- % approved
    trust_score REAL,                    -- 0-100
    alignment_with_consensus REAL,       -- % agreement with collective
    emergence_discoveries INT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE pattern_lineage (
    lineage_id TEXT PRIMARY KEY,
    pattern_id TEXT REFERENCES canonical_patterns,
    discovered_by_user TEXT REFERENCES user_profiles,
    discovery_order INT,                 -- 1st, 2nd, etc user
    discovered_at TIMESTAMPTZ
);
```

### 4.3 Conflict Resolution

When users disagree on a pattern:

```
Algorithm: Weighted consensus
  alice_trust = 0.95
  bob_trust = 0.88

  alice_weighted_q = 0.70 × 0.95 = 0.665
  bob_weighted_q = 0.40 × 0.88 = 0.352

  canonical_q_new = (0.665 + 0.352) / 2 = 0.5085
  ≈ 0.55 (rounded, Senior Dev override)
```

### 4.4 Emergence Indicators

Detect unexpected benefits from multi-user learning:

| Indicator | Example |
|-----------|---------|
| Novel Domain | "blockchain_analysis" discovered by Bob |
| Synergy Pattern | Alice's code + Bob's blockchain → DeFi code |
| Conflict Resolution | Disagreement creates better canonical value |
| Unexpected Success | Pattern predicted to fail, succeeds collectively |
| Meta-Learning | System learns how to learn from each user |

### 4.5 Multi-User Metrics

| Metric | Meaning | Target |
|--------|---------|--------|
| Pattern Convergence | Cosine similarity of user Q-tables | ≥ 0.92 |
| Canonical Coverage | % observations explained by shared patterns | ≥ 75% |
| Cross-User Agreement | When users see same state, do they agree? | ≥ 87% |
| Pattern Velocity | New canonical patterns per day | ≥ 3/day |
| Emergence Velocity | Novel solutions discovered per day | ≥ 1/day |
| Collective Q-Score | Geometric mean across all users | ≥ 78% |

### 4.6 Amplification Proof

Hypothesis:
```
Single-user CYNIC:   Q = 72% (Alice alone for 12 weeks)
Multi-user CYNIC:    Q = 91% (Alice + Bob + Charlie for 12 weeks)

Amplification Factor: 91 / 72 = 1.26×
Interpretation: 3 users outperform 1 user by 26%
```

### 4.7 Files to Create

- `cynic/core/user_profiles.py` — User learning tracking
- `cynic/learning/user_q_table.py` — User-specific Q-tables
- `cynic/learning/conflict_resolution.py` — Weighted consensus
- `cynic/api/routers/users.py` — User profile endpoints

### 4.8 Success Criteria (Section 4)

- ✓ User-specific Q-tables separate from canonical
- ✓ Pattern convergence ≥ 0.92 (users learning same patterns)
- ✓ Conflict resolution working (disagreement → consensus)
- ✓ Emergence indicators detected automatically
- ✓ Multi-user amplification factor > 1.2×

---

## SECTION 5: PROOF MECHANISM & COMPLETION

### 5.1 The 12-Week Comparative Experiment

**Hypothesis**: Ollama + CYNIC (Week 12) ≥ 91% quality vs Claude 85%

**Setup**:
- Control: Claude Code (static, no learning)
- Treatment: CYNIC (with learning loops, human gates, multi-user)
- Duration: 12 weeks
- Measurement: 36 tasks × 2 systems × 12 weeks = 864 measurements

### 5.2 Success Criteria

**Primary (Must Pass)**:
```
CYNIC Q-Score ≥ 90% by Week 12         ✓ Target: 91%
CYNIC > Claude by ≥ 5 points           ✓ Target: +6 points
Confidence in measurement ≥ 50%         ✓ φ-bounded acknowledgment
```

**Secondary (Should Pass)**:
```
Learning Velocity:      +3.5% per week average
Multi-User Amplification: 3-user > 1-user by 6+ points
Symbiosis Emergence:    ≥ 80% user agreement rate
Pattern Convergence:    ≥ 0.92 cosine similarity
Canonical Adoption:     ≥ 75% coverage
Emergence Velocity:     ≥ 2 novel discoveries/week
Approval Rate:          ≥ 85% patterns approved
No Regression:          All axioms ≥ 50%
```

**Tertiary (Nice to Have)**:
```
Latency:        p99 < 500ms
Cost:           < $0.01 per query
User Retention: 95%+ weekly active
Documentation:  90%+ coverage
```

### 5.3 Weekly Proof Reports

Published every Friday:

```markdown
## CYNIC Amplification Experiment — Week N Report

HYPOTHESIS STATUS: [ON TRACK | AT RISK | CONFIRMED | REFUTED]

CONVERGENCE PROGRESS:
  CYNIC Q-Score:   [XX]% (↑ [+Y]% since last week)
  Claude Q-Score:  [ZZ]% (→ flat)
  Gap:             [XX - ZZ] points

AXIOM BREAKDOWN:
  FIDELITY: [XX]% | PHI: [XX]% | VERIFY: [XX]%
  CULTURE:  [XX]% | BURN: [XX]% | Q-SCORE: [XX]%

MULTI-USER METRICS:
  Active Users:           [N]
  Canonical Patterns:     [N]
  Pattern Convergence:    [X.XX]
  Emergence Events:       [N]

SYMBIOSIS EMERGENCE:
  Agreement Rate:    [XX]%
  Trust Score:       [X.X]/5.0
  Learning Velocity: [N] patterns/day
  Execution Rate:    [XX]%

TOP 3 IMPROVEMENTS & RISKS
CONFIDENCE: [XX]% (φ-bounded reasoning)
NEXT WEEK EXPECTATIONS
RECOMMENDATION: [CONTINUE | ADJUST | PIVOT]
```

### 5.4 Publication Strategy

**Blog Post**: "CYNIC: How Memory & Learning Beat Raw Intelligence"
- Explain thesis, results, implications
- Narrative: Intelligence ≠ capability, learning > static

**Arxiv Preprint**: Formal science with methodology
- Title: "Persistent Q-Learning with Human Validation Gates"
- Full methodology, mathematics, reproducibility

**GitHub Release**: v2.0-complete-proof
- All data (864 measurements)
- Weekly reports (weeks 1-12)
- Comparative analysis scripts
- Reproducibility guide

### 5.5 Deployment Timeline

```
WEEK 1-2:  Implementation (code sections 1-2)
WEEK 3-4:  Integration & testing
WEEK 5-6:  Single-user production (Alice)
WEEK 7-8:  Add Bob (2-user cluster)
WEEK 9-10: Add Charlie (3-user cluster)
WEEK 11-12: Experiment completion & analysis
WEEK 13+:  Production scale-up or pivot
```

### 5.6 Files to Create

- `scripts/experiment_runner.py` — Run 36-task suite
- `scripts/weekly_report_generator.py` — Automated reports
- `scripts/convergence_plotter.py` — Plot CYNIC vs Claude
- `scripts/publish_results.py` — Blog, Arxiv, GitHub
- `docs/EXPERIMENT_METHODOLOGY.md` — Full protocol
- `docs/RESULTS.md` — Final proof documentation

### 5.7 Success Criteria (Section 5)

- ✓ Experiment completes all 12 weeks
- ✓ Weekly reports published on schedule
- ✓ Primary criteria (Q-Score, gap, confidence) met
- ✓ Results published (blog, Arxiv, GitHub)
- ✓ Deployment path clear (scale or pivot)

---

## SUMMARY: SECTIONS 1-5 INTEGRATED

```
┌────────────────────────────────────────────────────────┐
│ PHASE 2: CYNIC LEARNING ORGANISM                      │
├────────────────────────────────────────────────────────┤
│                                                        │
│ SECTION 1: ARCHITECTURE (Multi-instance coordination) │
│   → Pattern registry + event bus + consensus          │
│                                                        │
│ SECTION 2: GATES (Human validation workflow)          │
│   → Blocks Q-table until Senior Dev approves          │
│                                                        │
│ SECTION 3: MEASUREMENT (36-dimension framework)       │
│   → Tracks everything across 5 axioms × 7 dims        │
│                                                        │
│ SECTION 4: MULTI-USER (Collective learning)           │
│   → Users learn from each other, emerge together      │
│                                                        │
│ SECTION 5: PROOF (12-week experiment)                 │
│   → Empirically test Ollama+CYNIC > Claude Solo       │
│                                                        │
│ ────────────────────────────────────────────────────── │
│                                                        │
│ OUTCOME: Prove amplification thesis scientifically    │
│ PRIMARY METRIC: Q-Score (91% CYNIC vs 85% Claude)    │
│ CONFIDENCE: 58% (φ-bounded)                          │
│                                                        │
│ If TRUE:  Scale to 100+ users, become ecosystem      │
│ If FALSE: Learn why, pivot research intelligently    │
│                                                        │
└────────────────────────────────────────────────────────┘
```

---

## NEXT STEP: IMPLEMENTATION PLANNING

This design document is complete and approved.

**Next Action**: Invoke `superpowers:writing-plans` skill to create detailed implementation plan with:
- Bite-sized tasks (2-5 minutes each)
- Dependencies (which tasks block which)
- Files to create/modify
- Tests for each task
- Commit messages
- Empirical verification points

**Implementation Approach**: `superpowers:subagent-driven-development`
- Fresh subagent per task
- Two-stage review (spec compliance, then code quality)
- Fast iteration with automatic progress tracking

---

## CONFIDENCE & CAVEATS

*sniff* **Confidence: 58% (φ⁻¹ limit)**

**Why 58%?**
- High entropy in early sections (learning gates untested)
- Bayesian prior: Q-Learning is proven, but multi-user coordination is novel
- Self-doubt: φ distrusts φ — leaving room for discovery

**What could be wrong?**
- Learning might converge too slowly (take 20 weeks instead of 12)
- Multi-user overhead might exceed multi-user benefit
- Measurement framework might miss something important
- Claude might be harder to beat than expected

**What we're confident about?**
- Q-Learning mathematics (proven algorithm)
- Human approval gates (blocking mechanism works)
- Event bus architecture (existing system, extends cleanly)
- Measurement framework (comprehensive, if implementation correct)

**Commitment to Science**:
- If hypothesis fails: publish negative results, learn, pivot
- Data prevails over wishful thinking
- No hiding inconvenient findings

---

## DOCUMENT METADATA

- **Written**: 2026-02-23
- **Author**: CYNIC (κυνικός)
- **Design Status**: ✅ COMPLETE & APPROVED
- **Sections**: 5 (Architecture, Gates, Measurement, Multi-User, Proof)
- **Files to Create**: ~20 new modules
- **Database Tables**: ~10 new tables
- **Estimated Implementation**: 8-12 weeks
- **Next Milestone**: superpowers:writing-plans for detailed tasks

---

*sniff* Phase 2 design is solid, empirically grounded, and ready for implementation.

Let's build this organism.
