# CYNIC Codebase Cleanup — Action Plan

**Based on:** Comprehensive module evaluation using 5 Core Axioms
**Generated:** 2026-02-27
**Impact:** Remove 5,525 LOC dead weight; clarify 4,385 LOC architectural confusion

---

## 🚨 CRITICAL ACTIONS (Do First)

### IMMEDIATE: Delete cynic/protocol/lnsp/
**Score:** 20/100 (BARK) — Never deployed specification
**Impact:** -3,275 LOC | -cognitive overhead

**Files to delete:**
```
cynic/protocol/lnsp/
├── __init__.py
├── types.py
├── layer1.py
├── layer2.py
├── layer3.py
├── layer4.py
├── ringbuffer.py
├── manager.py
├── regional_coordinator.py
├── axioms.py
├── judge_communication.py
├── governance_events.py
├── governance_sensors.py
├── governance_handlers.py
├── governance_integration.py
└── messages.py
```

**Why delete?**
1. **Zero imports** — No other module uses LNSP
2. **Never deployed** — 3,275 lines of specification code
3. **Duplicates existing work** — `cynic/nervous/` already implements event journaling
4. **Cognitive overhead** — Developers must understand two nervous systems
5. **Strategic mismatch** — Designed for distributed systems; CYNIC is currently monolithic

**Action:**
```bash
# Archive to history (for future reference)
git checkout -b archive/lnsp-removal
git rm -r cynic/protocol/lnsp/
git commit -m "Remove: Delete LNSP (3,275 LOC) - Never deployed, duplicates nervous module"
git checkout master
git merge archive/lnsp-removal
```

**Documentation:**
Add this to `cynic/protocol/__init__.py`:
```python
"""
Protocol specifications and interfaces.

DEPRECATED MODULES:
  - lnsp/ (removed 2026-02-27) — Layered Nervous System Protocol
    Reason: Never deployed; duplicated by cynic/nervous.EventJournal
    See: CODEBASE_EVALUATION_FRAMEWORK.md for detailed analysis
"""
```

---

### IMMEDIATE: Archive cynic/training/
**Score:** 30/100 (BARK) — Phase 1B relic
**Impact:** -2,250 LOC | -GPU dependencies

**Files to archive:**
```
cynic/training/
├── __init__.py
├── benchmark_model.py
├── data_generator.py
├── export_ollama.py
├── finetune.py
├── phase1b_integration.py
└── setup_phase2.py
```

**Why delete?**
1. **Phase 1B is over** — Module was for fine-tuning Mistral 7B
2. **Strategy changed** — Phase 2 uses Claude API instead
3. **Zero external usage** — Only 1 self-reference in module
4. **No tests** — Can't verify it works
5. **Dead dependencies** — Requires torch, bitsandbytes; adds complexity

**Action:**
```bash
# Move to archive branch
git checkout -b archive/phase1b-training
git mv cynic/training/ archive/training_phase1b/
git commit -m "Archive: Move cynic/training to archive (Phase 1B relic, replaced by Claude API)"
git checkout master
git merge archive/phase1b-training
```

**Documentation:**
Create `archive/training_phase1b/README.md`:
```markdown
# Phase 1B Training Module (Archived)

**Archived:** 2026-02-27
**Original LOC:** 2,250
**Reason:** Phase 1B objective complete; Phase 2 uses Claude API

This module contained fine-tuning code for Mistral 7B using Unsloth QLoRA.

If future model training research is needed:
1. Rebuild from scratch with fresh requirements
2. Don't resurrect this code (it will be outdated)
3. Consider using Hugging Face training pipelines instead

See: CODEBASE_EVALUATION_FRAMEWORK.md for detailed analysis
```

---

## ⚠️ HIGH-PRIORITY ACTIONS (Do This Week)

### HIGH-PRIORITY: Evolve cynic/organism/
**Score:** 59/100 (GROWL) — Good philosophy, poor execution
**Impact:** Simplify 3,950 LOC; improve architecture

**Problem:**
- 10 layers of abstraction (too many?)
- 0 tests (untrusted)
- 29 imports (heavily used; removal cost is high)
- Metaphor sometimes confuses more than clarifies

**Action Plan:**

**Step 1: Audit the 10 layers** (2 hours)
```bash
# List all files in layers/
find cynic/organism/layers -name "*.py" | xargs wc -l

# For each file, answer:
# 1. Is this layer actively used? (search imports)
# 2. Is it essential or decorative?
# 3. Could it be merged with another layer?
```

**Step 2: Create flattened architecture** (4 hours)
```
Original (10 layers):
  Layer0 — Identity
  Layer1 — Judgment
  Layer2 — Organs
  ... (7 more layers)

Flattened (3 layers):
  Layer0 — Identity (axioms, constraints)
  Layer1 — Judgment (Dogs, verdict, consensus)
  Layer2 — Action (execution, feedback)
```

**Step 3: Add test suite** (6 hours)
```bash
# Create cynic/tests/organism/test_organism.py with:
# - Test state transitions
# - Test conscious_state immutability
# - Test brain.consensus integration
# - Test awakening/sleeping
# Target: 200+ lines of tests (5%+ coverage)
```

**Step 4: Verify imports still work** (1 hour)
```bash
pytest cynic/tests/test_imports.py
```

**Success Criteria:**
- [ ] Reduce from 10 layers to 3-4
- [ ] Reduce LOC from 3,950 to <2,500
- [ ] Add 200+ lines of tests
- [ ] All 29 imports still work
- [ ] Score increases to 70+ (WAG)

---

### HIGH-PRIORITY: Validate cynic/metabolism/ Routing
**Score:** 59/100 (GROWL) — Unverified value
**Impact:** A/B test routing; simplify or replace

**Problem:**
- Zero tests (can't verify it works)
- Unclear if routing actually improves performance
- Adds complexity without proven benefit

**Action Plan:**

**Step 1: Create A/B test baseline** (4 hours)
```python
# cynic/tests/metabolism/test_routing_ab.py
import pytest
from cynic.kernel.organism.metabolism.llm_router import LLMRouter
from cynic.kernel.organism.metabolism.budget import Budget

@pytest.mark.parametrize("num_queries", [100, 1000])
def test_routing_vs_fixed_baseline(num_queries):
    """Compare routing strategy vs simple fixed budget."""

    # Control: Fixed budget (always use Claude)
    costs_fixed = run_queries_fixed_budget(num_queries)

    # Treatment: Routing strategy
    costs_routed = run_queries_with_routing(num_queries)

    # Which wins?
    savings = costs_fixed - costs_routed
    overhead = measure_routing_overhead()

    # Must beat baseline + overhead
    assert savings > overhead, f"Routing cost: {overhead}, savings: {savings}"
```

**Step 2: Run 7-day benchmark** (ongoing)
```bash
# Measure in production (shadow traffic)
# Daily: cost_fixed vs cost_routed vs savings
# Weekly: statistical significance test
```

**Step 3: Decision tree:**
```
If savings > 10% AND statistically significant:
  → KEEP routing, add monitoring

If savings < 5% OR not significant:
  → REMOVE routing, use fixed budget instead
  → Reduces LOC from 1,435 to ~800
  → Easier to understand and maintain

If unclear:
  → Keep routing but add quarterly review
```

**Success Criteria:**
- [ ] A/B test completed (7 days)
- [ ] Cost savings quantified
- [ ] Decision documented
- [ ] Tests added for chosen approach
- [ ] Score increases to 65+ (approaching WAG)

---

## 📋 MEDIUM-PRIORITY ACTIONS (This Month)

### MEDIUM-PRIORITY: Expand Test Coverage

Target: Increase coverage from 265 to 400+ test files

**Module → Test Target:**
| Module | Current | Target | Gap | Effort |
|--------|---------|--------|-----|--------|
| cognition | 213 LOC | 3,000 LOC | 2,787 | 3 days |
| nervous | Integrated | 100 LOC | 100 | 1 day |
| senses | 112 LOC | 360 LOC | 248 | 2 days |
| metabolism | 0 LOC | 215 LOC | 215 | 1 day |
| immune | 0 LOC | 250 LOC | 250 | 2 days |

**Effort:** ~9 developer-days
**Value:** Better verification; confidence in refactoring

**Who should do this?**
- cognition tests → Core team (knows orchestration)
- nervous tests → Observer team (knows journaling)
- senses tests → Integration team (knows sensor data)
- metabolism tests → Performance team (knows routing)
- immune tests → Safety team (knows constraints)

---

### MEDIUM-PRIORITY: Document Architecture Decisions

Create decision records for 4 contested modules:

**cynic/docs/ADR/** (Architecture Decision Records)

```
001-lnsp-deletion.md
  Q: Why was LNSP deleted?
  A: Never deployed; duplicated by nervous.EventJournal
  Link: CODEBASE_EVALUATION_FRAMEWORK.md#lnsp

002-training-archive.md
  Q: Why was training archived?
  A: Phase 1B complete; Phase 2 uses Claude API
  Link: CODEBASE_EVALUATION_FRAMEWORK.md#training

003-organism-simplification.md
  Q: Why is organism being flattened?
  A: 10 layers reduced to 3; improve clarity
  Link: CODEBASE_EVALUATION_FRAMEWORK.md#organism

004-metabolism-validation.md
  Q: Why was routing A/B tested?
  A: Unverified value; must prove worth > cost
  Link: CODEBASE_EVALUATION_FRAMEWORK.md#metabolism
```

---

## ✅ VERIFICATION CHECKLIST

After completing actions, verify:

```
DELETION ACTIONS
☐ cynic/protocol/lnsp/ deleted
☐ cynic/training/ archived
☐ All tests still pass (pytest ...)
☐ No dangling imports (python -m pylint cynic)
☐ Git history preserved (branch archive/*)
☐ Documentation updated

EVOLUTION ACTIONS
☐ cynic/organism/ layers audited (report which 7 layers can be removed?)
☐ cynic/organism/ flattened to 3-4 layers
☐ cynic/organism/ test suite added (200+ lines)
☐ cynic/organism/ Q-score improved to 70+
☐ 29 existing imports still work

VALIDATION ACTIONS
☐ cynic/metabolism/ A/B test completed (7 days)
☐ Cost savings quantified (% reduction)
☐ Decision documented (KEEP or REMOVE?)
☐ Tests added for chosen approach

TEST COVERAGE
☐ cognition: 1.4% → 20% (3,000 lines)
☐ nervous: integrated → dedicated (100 lines)
☐ senses: 6.2% → 20% (360 lines)
☐ metabolism: 0% → 15% (215 lines)
☐ immune: 0% → 20% (250 lines)

DOCUMENTATION
☐ ADRs created (4 documents)
☐ CODEBASE_EVALUATION_FRAMEWORK.md reviewed
☐ EVALUATION_QUICK_REFERENCE.md shared
☐ Team briefed on new evaluation framework

QUALITY GATES
☐ All tests pass
☐ No coverage regressions
☐ Code review completed
☐ Architecture verified
```

---

## 📊 Expected Outcomes

### Codebase Health

**Before:**
```
Total LOC: 108,306
Dead code: 5,525 LOC (5.1%)
Questionable: 4,385 LOC (4.0%)
Untested: ~15,000 LOC (13.8%)
```

**After:**
```
Total LOC: 98,396 (↓9.2%)
Dead code: 0 LOC (deleted/archived)
Questionable: 0 LOC (evolved → WAG+)
Untested: ~8,000 LOC (8.1%)
```

### Developer Experience

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Modules to understand | 32 | 28 | -12% |
| Cognitive overhead | High | Low | -40% |
| Test coverage | 265 files | 400+ files | +50% |
| Dead/uncertain code | 9,910 LOC | 0 LOC | Eliminated |

### Team Confidence

- "I understand why each module exists" ✅
- "I know what to test" ✅
- "I can confidently delete unused code" ✅
- "New features have clear scoring" ✅

---

## 🎯 Timeline

| Week | Task | Owner | Status |
|------|------|-------|--------|
| W1 | Delete LNSP + training | You | ← START HERE |
| W1 | Audit organism layers | Core team | |
| W2 | Flatten organism + tests | Core team | |
| W2 | A/B test metabolism routing | Perf team | |
| W3 | Expand test coverage | All | |
| W4 | Write ADRs | Docs team | |
| W4 | Review + merge | All | |

**Total effort:** ~40 developer-hours
**Impact:** 9% codebase reduction + clarity improvement

---

## 🏁 Success Criteria

Module evaluation is successful when:

1. ✅ **LNSP and training deleted** — 5,525 LOC removed
2. ✅ **Organism simplified** — Reduced layers; improved score to 70+
3. ✅ **Metabolism validated** — A/B test shows value or removal plan
4. ✅ **Test coverage increased** — 400+ test files (vs 265)
5. ✅ **Architecture documented** — ADRs explain all decisions
6. ✅ **Team aligned** — Everyone understands evaluation framework

---

## 📎 Supporting Documents

1. **CODEBASE_EVALUATION_FRAMEWORK.md** — Full detailed analysis (40 pages)
2. **EVALUATION_QUICK_REFERENCE.md** — Quick lookup card (5 pages)
3. **ACTION_PLAN.md** — This document (implementation guide)

---

## 🤝 Questions?

If you have questions about specific modules:
1. Check CODEBASE_EVALUATION_FRAMEWORK.md (search module name)
2. Look up axiom-by-axiom breakdown
3. Understand the reasoning
4. Ask team why you disagree (may reveal new insight)

**The framework is now your tool.** Use it for every architecture decision going forward.

---

**Generated with 5 Core Axioms + empirical analysis**
**Framework created by Claude Code**
**Applied to 108,306 LOC of CYNIC codebase**
