# Python Code Lifecycle & Rust Migration Protocol

**Problem**: cynic-python has 132 modules, 1 wired, 131 dead. No governance over which experiments become production.

**Solution**: Explicit taxonomy for Python work + conditions for Rust migration.

---

## Python Code Categories

### Tier 1: EXPERIMENTAL (Can Die)
**Purpose**: Research, hypothesis validation, local iteration

**Characteristics**:
- Lives in: `cynic-python/*/` with `EXPERIMENT:` tag in docstring
- No systemd service (not scheduled)
- No REST endpoint (not exposed to kernel)
- No K15 consumer (no acting reader)
- Lifetime: One session to a few weeks
- Owned by: Single researcher/agent
- Visibility: Local only (not in mempool)

**Lifecycle**:
1. Created during research session
2. Runs manually or via Jupyter
3. Produces output (report, metrics, artifact)
4. Gets committed to git as `docs/research/`
5. **Dies cleanly**: Not promoted means no one consumed it

**Example**: `validation/kenosis_mining.py` — mining script that found 1 pattern, didn't lead to action, can be deleted

**Death Condition** (R21 enforcement): If Tier 1 code isn't promoted to Tier 2 within **30 days of last edit**, it's eligible for deletion.

---

### Tier 2: INFRASTRUCTURE (Must Be Wired)
**Purpose**: Ongoing operational logic; production jobs; monitoring

**Characteristics**:
- Lives in: `scripts/hermes-x/` or `cynic-python/<module>/` with no `EXPERIMENT:` tag
- Has systemd service OR cron job (scheduled)
- Has K15 consumer (acting reader) OR feeds to another service
- Testable: Unit tests exist and pass
- Owned by: Human (via systemd) or kernel (via REST endpoint)
- Visibility: In `/observations` feed (K15 producer)

**Lifecycle**:
1. Promoted from Tier 1 when proven useful
2. Must have **explicit consuming path** before promotion
3. Wired to systemd + kernel before merge
4. Metrics in `/health` endpoint
5. **Runs continuously** until explicitly decommissioned

**Promotion Checklist**:
- [ ] Code has measurable success condition (not "we'll see")
- [ ] Consumer identified (what will act on this output?)
- [ ] Systemd service written + tested
- [ ] K15 falsification test passes (consumer actually runs)
- [ ] Performance baseline established
- [ ] Error handling + fallback documented

**Example**: `consumers/k15_observation_consumer.py` — Tier 2 (wired, running, has consumer)

---

### Tier 3: CANDIDATE FOR RUST (Pre-Migration)
**Purpose**: Tier 2 code mature enough for kernel rewrite

**Entry Conditions** (all must be true):
1. **Operational**: Running error-free in production for ≥14 days
2. **Load-bearing**: Used by ≥2 independent services or agents
3. **Stable interface**: Output schema hasn't changed in ≥7 days
4. **Testable on real data**: Integration tests pass against live kernel data
5. **Performance critical**: Latency matters for agent decisions (sub-second)

**Assessment**: Before Rust migration, measure:
- Current Python latency (P50, P99)
- Current Python CPU/memory baseline
- Current error rate (target: <0.1% silent failures)
- Current K15 feedback loop health

**Rust Rewrite Conditions** (at least one):
- **Latency**: Sub-second feedback loop required (Python too slow)
- **Coupling**: Data structure is core kernel model (belongs in Rust)
- **Correctness**: Silent failure risk too high (Rust's type system needed)
- **Sovereignty**: Production deployment on offline machines (no Python runtime)

**Example Candidate**: `scripts/hermes-x/search_executor.py`
- Running >30 days ✓
- Used by hermes-x organ ✓
- Interface stable ✓
- Latency critical for agent learning ✓
→ Candidate for kernel migration (Phase 2+)

---

## Tier Transitions & Death

```
EXPERIMENTAL (Tier 1)
├─ Success path: [Prove useful] → INFRASTRUCTURE (Tier 2)
└─ Death path: [Unused >30 days] → DELETE

INFRASTRUCTURE (Tier 2)
├─ Success path: [Run error-free ≥14d + meet load/latency criteria] → CANDIDATE FOR RUST (Tier 3)
├─ Stable path: [Keep running] → TIER 2 STABLE
└─ Death path: [Explicitly decommissioned + removed from systemd] → DELETE

CANDIDATE FOR RUST (Tier 3)
├─ Migration: [Rewrite in kernel Rust] → KERNEL TIER
└─ Rejection: [Doesn't meet criteria] → Back to TIER 2 STABLE
```

---

## Python→Rust Migration Path

### Phase 1: Identify Candidate
Requirements (all):
- [ ] Lives in Tier 2 ≥14 days
- [ ] Measurable impact on agent learning (K15 feedback improves)
- [ ] Latency baseline: <500ms target, currently > target

### Phase 2: Port to Rust
- [ ] Write Rust version of Python interface
- [ ] Implement identical input/output schema
- [ ] Run both versions in parallel (Python trusted, Rust candidate)
- [ ] Compare outputs for ≥3 days (target: <1% divergence on real data)

### Phase 3: Replace
- [ ] Kernel consumes Rust version
- [ ] Python version becomes fallback (systemd oneshot for debugging)
- [ ] Monitor divergence for 7 days
- [ ] If stable: remove Python version
- [ ] If divergence detected: investigate + fix Rust, not rollback

### Phase 4: Deprecate
- [ ] Remove Python version from git
- [ ] Document decision in commit message
- [ ] Update memory: "migrated to kernel"

---

## Tagging & Enforcement

### Docstring Tags (Visible)

```python
"""
Tier 1 EXPERIMENTAL: Domain discovery via semantic clustering.

Research question: Can unsupervised clustering find emergent domains?
Success condition: F1 score > 0.7 on validation set
Timeline: 7-14 days local testing
Owned by: @T
Status: ACTIVE (started 2026-05-01)

Note: If not promoted to Tier 2 by 2026-05-31, delete.
"""
```

```python
"""
Tier 2 INFRASTRUCTURE: Observe kernel domain routing decisions.

K15 Consumer: /agent-tasks (uses observations to prioritize tasks)
Systemd: hermes-k15-consumer.service (runs every 30s)
Promotion date: 2026-04-15 (from kenosis_mining.py experiment)
Stability: 12 days error-free

Metrics: /health.k15_observations_consumed
"""
```

```python
"""
Tier 3 CANDIDATE FOR RUST: Search executor latency critical path.

Assessment: [See ASSESSMENT_SEARCH_EXECUTOR.md]
Latency baseline: 2.3s avg, target <500ms
Load: 5 searches/min during peak
Readiness: 80% (needs error handling review)

Rust migration: PLANNED for Phase 3 (estimated 2026-05-15)
"""
```

### CI Gate: `make lint-python-tiers`

```bash
# Enforce:
# P1. Every Tier 1 file has EXPERIMENT: tag
# P2. Every Tier 2 file has systemd service or test
# P3. No Tier 1 imports Tier 2 (experiments don't depend on infra)
# P4. No Tier 2 without K15 consumer (no orphans)
# P5. Tier 1 files >30 days old flagged for deletion
# P6. Tier 3 candidates have assessment document
```

---

## Example: Apply to Current State

### cynic-python/validation/ (12 files)
- `kenosis_mining.py`: Tier 1 (found 1 pattern, no action) → **DELETE** (>30d inactive)
- `phase2_dry_run.py`: Tier 1 (experiment) → **DELETE** (outdated, no promotion)
- `wallet_corpus_builder.py`: Tier 2? (builds validation data) → **UNCLEAR** (no consumer listed)

**Action**: Tag each file with tier + decision

### scripts/hermes-x/ (33 files)
- `search_executor.py`: Tier 2 ✓ (systemd wired)
- `x_proxy.py`: Tier 2 ✓ (ingest daemon dependency)
- `hermes_killchain_tracer.py`: Tier 1? (analysis script) → **TAG** (what's the use case?)

---

## Decision Template for New Python Work

Before writing Python code, answer:

```
Title: [What does this do?]

Tier: [ ] 1 EXPERIMENTAL [ ] 2 INFRASTRUCTURE [ ] 3 CANDIDATE FOR RUST

If Tier 1:
  Research question: [What are we testing?]
  Success condition: [What does "proven" mean?]
  Timeline: [Days until promotion or death?]
  Will this become Tier 2 or die? [Be honest]

If Tier 2:
  Consumer: [Who uses this? (systemd service / agent / REST endpoint)]
  Promotion date: [When did it graduate from Tier 1?]
  Metrics: [How do we know it's working?]
  Failure mode: [What happens if this breaks?]

If Tier 3:
  Why Rust candidate? [Latency / coupling / correctness / sovereignty]
  Rust readiness: [Why now? What's blocking it?]
  Parallel run plan: [How to validate Rust version?]
```

---

## Transition Conditions

### Tier 1 → Tier 2 (Promotion)
- [ ] Code has measurable impact (not "interesting")
- [ ] Consumer identified (kernel, agent, human)
- [ ] Systemd service written
- [ ] Test: Run K15 falsification; consumer actually triggers
- [ ] Documentation: Why is this now infrastructure?

### Tier 2 → Tier 3 (Candidacy)
- [ ] Running ≥14 days, <0.1% error rate
- [ ] Used by ≥2 independent paths
- [ ] Latency matters: current > target
- [ ] Assessment document: Why Rust now?
- [ ] No open bugs (clean bill of health)

### Tier 3 → Rust (Migration)
- [ ] Rust version written + tested
- [ ] Parallel run ≥3 days, <1% divergence
- [ ] Team review: Is Rust quality sufficient?
- [ ] Rollout plan: Gradual replacement or cutover?

### Any Tier → Death (Decommission)
- [ ] Systemd service stopped + disabled
- [ ] Code deleted from git (with reason in commit message)
- [ ] Consumer notified (if applicable)
- [ ] Documentation archived (if valuable for future)

---

## Memory Integration

Each major Python work should have a memory entry:

```
name: python-[module]-lifecycle
description: Where [module] lives in tier system, current status, promotion/death plan
type: project
---

**Module**: search_executor.py
**Tier**: 2 INFRASTRUCTURE
**Status**: STABLE (30+ days, <0.1% error)
**Promotion**: 2026-04-15 from hermes-x experiment
**Consumer**: hermes-search-generator.service (calls it every 30min)
**Metrics**: /health.search_executor_status
**Candidate for Rust?**: YES (latency 2.3s, target <500ms for agent feedback loop)
**Assessment**: [Link to ASSESSMENT.md if Tier 3]
```

---

## What This Prevents

**Old pattern** (current):
- Experiment runs locally
- Researcher forgets to delete it
- Code sits in git 1+ years
- "Should we wire this?" becomes hard question
- Result: 131 dead modules

**New pattern**:
- Experiment created with **explicit tier** + **death date**
- If proven: promote to Tier 2, wire systemd, get metrics
- If not proven: delete at deadline (clean conscience)
- If critical: assess for Rust migration (intentional, not accidental)
- Result: Every line of code is either running or deleted

---

## FAQ

**Q: Shouldn't all code be wired?**
A: No. Experiments should be allowed to fail. The key is not letting failures rot. Dead code is worse than no code.

**Q: Why separate Tier 2 from Tier 3?**
A: Operational vs. directional. Tier 2 code is trusted and stable. Tier 3 is candidate — it might stay Python, might migrate to Rust. The assessment should be deliberate, not drift.

**Q: How do I know if something is Tier 2 or Tier 3?**
A: Ask: "Does this need to be in the kernel?" If latency/correctness/sovereignty says yes, it's a Tier 3 candidate. If no, keep it Tier 2 Python.

**Q: Can Tier 1 code import Tier 2?**
A: No. Experiments should be self-contained. If an experiment needs infrastructure, it's already using it correctly. Circular dependency = bad design.

**Q: What if I'm wrong about the tier?**
A: Move it. Tag in docstring, update memory, retag in CI. The point is clarity, not perfection.
