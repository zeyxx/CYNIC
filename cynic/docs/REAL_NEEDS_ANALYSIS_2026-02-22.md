# CYNIC Real Documentation Needs Analysis

> **Focus**: What the REAL organism needs NOW (Feb 2026)
> **NOT**: Old Claude plugin docs, aspirational visions, or chaos
>
> *🐕 κυνικός | "What do we ACTUALLY need?"*

---

## Executive Summary

After analyzing 3,152 .md files, here's what CYNIC **ACTUALLY** needs right now:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    REAL NEEDS vs FAKE NEEDS                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   REAL NEEDS (Feb 2026):                                            │
│   1. Kernel Python v2.0 bootstrap docs                              │
│   2. One scientific paper draft (ICML submission)                   │
│   3. One-pager marketing (for users/investors)                      │
│   4. KPI dashboard (measure real progress)                          │
│   5. Runbook (for 24/7 daemon ops)                                  │
│                                                                      │
│   FAKE NEEDS (don't create):                                        │
│   ❌ More vision docs (we have 25k words already)                   │
│   ❌ JS plugin docs (archived, v1.0 legacy)                         │
│   ❌ Duplicate architecture docs (8 versions exist)                 │
│   ❌ Session reports (auto-generated, 600+ files)                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## I. Current State (Empirical)

### A. What EXISTS

| Category | Files | Status | Needed? |
|----------|-------|--------|---------|
| Vision docs | ~15 | Redundant (UNIFIED-VISION = 25k words) | ❌ No |
| Architecture | 8 versions | Conflicting | ⚠️ Consolidate |
| Session logs | 600+ | Auto-generated noise | ❌ Archive |
| Research drafts | 3 | Incomplete | ✅ Complete 1 |
| Marketing | 0 | MISSING | ✅ Create |
| Kernel docs | 5 | Partial | ✅ Complete |

### B. What's MISSING

| Doc Type | Status | Priority | Audience |
|----------|--------|----------|----------|
| **One-pager** | MISSING | P0 | Users/Investors |
| **Paper draft** | 50% | P0 | Academic |
| **Kernel spec** | 30% | P1 | Developers |
| **API reference** | MISSING | P1 | Developers |
| **Comparison doc** | MISSING | P2 | Marketing |

---

## II. What We ACTUALLY Need (Priority Order)

### P0: Critical (Now)

#### 1. ONE-PAGER (Marketing)
**Purpose**: Explain CYNIC in 1 page, 3 minutes
**Audience**: Users, investors, journalists
**Status**: MISSING
**Content needed**:
- What is CYNIC? (1 paragraph)
- Why different from Copilot/Cursor? (2-3 bullets)
- The 5 axioms (quick list)
- How to start (1 command)

#### 2. PAPER DRAFT (Scientific)
**Purpose**: ICML 2026 submission
**Status**: `x-article-draft.md` exists but incomplete
**Content needed**:
- Abstract
- Introduction (φ-bounded AI)
- Method (multi-LLM orchestration)
- Experiments (benchmark results)
- Conclusion

#### 3. KPI DASHBOARD (Technical)
**Purpose**: Measure REAL progress, not aspirational
**Status**: CREATED (`kpis.md`)
**Content**:
- Tier 1: alive_check, uptime, tests
- Tier 2: learning loops, judgments
- Tier 3: matrix completion, calibration

---

### P1: Important (Week 1-2)

#### 4. KERNEL SPEC (Technical)
**Purpose**: Python v2.0 architecture
**Status**: Partial in `todolist.md`
**Content needed**:
- 9 kernel components (3000 LOC)
- φ-bound implementation
- Dogs (11) mapping
- Learning loops (11)

#### 5. API REFERENCE (Technical)
**Purpose**: For developers integrating CYNIC
**Status**: MISSING
**Content needed**:
- REST API endpoints
- MCP tools list
- Python SDK methods

---

### P2: Nice-to-have (Month 1)

#### 6. COMPARISON DOC (Marketing)
**Purpose**: vs Copilot, Cursor, Windsurf, Aider
**Status**: MISSING
**Content needed**:
- Feature comparison table
- Differentiators (φ-bound, multi-LLM, learning)
- Cost comparison

#### 7. BENCHMARK DOC (Research)
**Purpose**: CYNIC-1000 benchmark definition
**Status**: Partial
**Content needed**:
- 1000 test cases
- Evaluation metrics
- Leaderboard format

---

## III. Anti-Patterns to BREAK

### Anti-Pattern 1: Documentation Sprawl

**Current**: 3,152 .md files
**Problem**: Nobody knows where to look
**Solution**: Consolidate to <150 files

### Anti-Pattern 2: Version Confusion

**Current**: 5 "status" docs, 8 "architecture" docs
**Problem**: Which one is current?
**Solution**: ONE source of truth per topic

### Anti-Pattern 3: Aspirational vs Real

**Current**: VISION docs describe future, not present
**Problem**: Users expect features that don't exist
**Solution**: Label clearly [CURRENT] vs [ASPIRATIONAL]

### Anti-Pattern 4: Auto-generated Noise

**Current**: 600+ digest files from sessions
**Problem**: Clutters search, hides signal
**Solution**: Archive all auto-generated content

---

## IV. Concrete Action Plan

### This Week (Feb 22-28)

```
Day 1: Create ONE-PAGER.md (1 page)
Day 2: Complete PAPER-DRAFT.md (scientific)
Day 3: Clean up root .md files (92 → 10)
Day 4: Create API-REFERENCE.md
Day 5: Archive all session/digest files
```

### Next Week (Mar 1-7)

```
Day 1-2: Create COMPARISON.md
Day 3-4: Create BENCHMARK.md
Day 5: Final consolidation
```

---

## V. KPIs to Measure Documentation Health

| KPI | Current | Target | Formula |
|-----|---------|--------|---------|
| Total .md files | 3,152 | <150 | Count |
| Root .md files | 92 | <10 | Count |
| Docs per topic | 5+ (duplicates) | 1 | Unique |
| Signal/Noise | 15% | >70% | Useful / Total |
| LLM-indexable | No | Yes | _index.json |
| One-pager exists | No | Yes | Boolean |
| Paper draft % | 50% | 100% | Completion |

---

## VI. Files to CREATE (Priority Order)

1. `cynic/docs/08-marketing/one-pager.md` - **MISSING - P0**
2. `cynic/docs/06-research/paper-draft.md` - **50% - P0**
3. `cynic/docs/03-reference/api.md` - **MISSING - P1**
4. `cynic/docs/03-reference/kernel.md` - **30% - P1**
5. `cynic/docs/08-marketing/comparison.md` - **MISSING - P2**

---

## VII. Files to ARCHIVE (Immediate)

```
.digests/                → 600 files → .archive/auto-generated/
*-2026-02-*.md           → 30 files → .archive/dated-reports/
SESSION_*.md             → 10 files → .archive/sessions/
PHASE_*_*.md             → 15 files → .archive/sessions/
CYNIC-v2-*.md            → 5 files → .archive/legacy-plans/
CYNIC-v3-*.md            → 5 files → .archive/legacy-plans/
```

---

## VIII. Scientific Research Needs

### What's Missing for Academic Credibility

1. **Benchmark**: CYNIC-1000 needs to be defined and published
2. **Paper**: Formal write-up of φ-bounded multi-LLM approach
3. **Experiments**: Real data from production usage
4. **Baselines**: Comparison vs single-LLM approaches

### Research Questions to Answer

1. Does multi-LLM orchestration beat single LLM? (Evidence: +2.6 Q-Score)
2. Does φ-bounding improve calibration? (Measure: ECE)
3. Does continual learning improve routing? (Measure: accuracy over time)

---

## IX. Marketing Needs

### What's Missing for Growth

1. **One-pager**: Explain CYNIC in 30 seconds
2. **Comparison**: Why choose CYNIC over Copilot?
3. **Demo video**: Show a real session
4. **Twitter thread**: Launch announcement

### Target Metrics

| Metric | Current | Target (Month 1) |
|--------|---------|------------------|
| GitHub stars | ~100 | 500+ |
| Active users | ~10 | 100+ |
| Paper citations | 0 | 1+ |

---

## X. Conclusion

**We don't need more vision docs. We need:**

1. ✅ KPI framework (created)
2. ✅ Runbook (created)
3. ❌ One-pager (MISSING - create now)
4. ❌ Complete paper (50% - finish now)
5. ❌ API reference (MISSING - create soon)

**Total new docs needed**: 5 files
**Total docs to archive**: 665+ files
**Signal/Noise improvement**: 15% → 70%

---

*🐕 κυνικός | "Focus on what's REAL, burn the rest"*