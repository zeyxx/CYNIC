# Phase 1 Domain Discovery Structure (2026-05-03)

**Goal**: Establish clear hierarchy and consumer contracts to avoid code orphans, dangling scripts, and iteration debt.

## Artifact Hierarchy (Production → Exploratory)

### Phase 2 Consumer (LOCKED)
These artifacts are **consumed by K15 router** in Phase 2. Immutable until Phase 3 measurement conflicts force revision.

```
cynic-python/domains/
├── k15-routing/v1/
│   ├── domain_router_v1.py         ← LOCKED (K15 consumer, wired in kernel)
│   ├── test_domain_router_v1.py    ← LOCKED (validation, no changes until Phase 3)
│   ├── NOTES_v1.md                 ← Routing design locked
│   └── ROADMAP.md                  ← v2/v3/v4 path documented
│
└── domain-discovery/v1/
    ├── emergent_clustering_tfidf_v1.py  ← LOCKED (produces canonical clusters)
    ├── results_v1/
    │   ├── emergent_clusters_tfidf.json  ← IMMUTABLE (canonical clustering)
    │   └── metrics_v1.json               ← IMMUTABLE (cluster quality baseline)
    ├── NOTES_v1.md                  ← v1 design frozen
    └── ROADMAP.md                   ← v2/v3/v4 path documented
```

**Locked = No changes unless Phase 3 measurement falsifies it (>20% underperformance).**

### Phase 3 Measurement (EPHEMERAL)
These scripts **measure Phase 3 outcomes**. Deleted after Phase 3 completes.

```
(To be created during Phase 2 → Phase 3 transition)
├── measure_signal_yield_per_domain.py
├── validate_phase3_hypothesis.py
└── phase3_results/
    └── signal_yield_by_domain.json (temporary, archived at phase-end)
```

### Exploratory / Deferred (QUARANTINE)
These are **experiments or optimizations** that may or may not ship. Kept separate, explicitly non-binding.

```
cynic-python/domains/domain-discovery/
├── v2/
│   ├── emergent_clustering_tfidf_v2.py  ← EXPLORATORY (stopword filtering)
│   ├── NOTES_v2.md                      ← Hypothesis: Δsilhouette > 0.05
│   └── validate_improvement_v1_v2.py    ← Comparison script (unused if v2 hangs)
│
├── behavioral_grounding_fast.py         ← EXPLORATORY (author-based check)
├── behavioral_topic_analysis.py         ← EXPLORATORY (keyword overlap)
└── BEHAVIORAL_GROUNDING.md              ← Findings: φ⁻³ confidence
```

**Exploratory = May be deleted without regression if hypothesis fails or approach changes.**

---

## Consumer Contract (K15 Rule)

### Producer: `domain_router_v1.py`
```
Input: observation text (string)
Output: {
  domain: str,                    # "general" | "llm_tech" | "token_analysis" | "social" | ...
  target: str,                    # "human_target" | "organ_x_target"
  confidence: float,              # heuristic score (Phase 1), inference (Phase 2+)
  cluster_id: int                 # 0-6 (v1 clusters)
}
```

### Consumer: Kernel K15 Integration (Phase 2)
- **Location**: cynic-kernel/src/api/routes/k15_router.rs (to be wired)
- **Call**: `domain_router.route_observation(text)` → decision → store in reflections
- **Measurement**: Phase 3 measures signal yield per domain from routing decisions

### Feedback Loop (Phase 3 → Phase 4)
```
Phase 3 measures: signal_yield_per_domain[domain] > baseline[domain]
  ↓
If yield[token_analysis] < baseline + 2%:
  → Hypothesis falsified, consider v2 (stopword filtering)
  → Or abandon domain routing, try alternative approach
  
If yield[token_analysis] > baseline + 2%:
  → Hypothesis validated, lock v1, move to Phase 4 (crystallization)
```

---

## Deletion Policy (Prevent Orphans)

**Rule**: Exploratory code is deleted **immediately after use case is resolved**.

| Script | Use Case | Keep Until | Action |
|--------|----------|-----------|--------|
| `behavioral_grounding_fast.py` | Pre-Phase-2 validation | Phase 1 complete ✓ | Delete after Phase 2 starts (no longer needed) |
| `behavioral_topic_analysis.py` | Pre-Phase-2 analysis | Phase 1 complete ✓ | Delete after Phase 2 starts |
| `emergent_clustering_tfidf_v2.py` | v2 optimization hypothesis | Phase 3 yield measured | Keep if Phase 3 underperforms; delete if validated |
| `validate_improvement_v1_v2.py` | v1/v2 comparison | v2 completion/deferral ✓ | Delete (v2 won't run, comparison invalid) |

**Anti-pattern**: Leaving `behavioral_grounding_fast.py` in the repo indefinitely "just in case" = dead code, confuses future developers.

---

## Phase Readiness Gate (R20 FOGC Test)

Before moving to Phase 2, verify:

```
□ K15 router has a Phase 2 consumer (kernel K15 integration) ✓
□ Domain discovery v1 has a Phase 3 consumer (signal yield measurement) (Phase 2 → 3 to define)
□ All exploratory scripts have a "delete after X" date (documented above)
□ Protocol.md prevents iteration chaos (lifecycle stages, versioning) ✓
□ No scripts are left as "someday-might-be-useful" orphans
```

---

## Code Organization Example

**Current (acceptable)**:
```
domain-discovery/
├── v1/
│   └── emergent_clustering_tfidf_v1.py  (LOCKED, Phase 2 consumer)
├── v2/
│   └── emergent_clustering_tfidf_v2.py  (DEFERRED, Phase 3 decision point)
├── behavioral_grounding_fast.py         (EPHEMERAL, delete after Phase 2 starts)
└── BEHAVIORAL_GROUNDING.md              (EPHEMERAL, archive to memory, delete from repo)
```

**Future (if v2 approved after Phase 3)**:
```
domain-discovery/
├── v1/                        (superseded, keep for history)
│   └── emergent_clustering_tfidf_v1.py
├── v2/                        (NEW LOCKED after Phase 3)
│   ├── emergent_clustering_tfidf_v2.py  (promoted to LOCKED)
│   ├── results_v2/
│   │   └── emergent_clusters_tfidf_v2.json  (canonical, replaces v1)
│   └── NOTES_v2.md
└── ROADMAP.md                 (updated to v3/v4 path)
```

**Future (if v2 rejected after Phase 3)**:
```
domain-discovery/
├── v1/
│   ├── emergent_clustering_tfidf_v1.py  (remains LOCKED)
│   └── results_v1/  
│       └── emergent_clusters_tfidf.json  (remains canonical)
└── ROADMAP.md                 (documents v2 rejection, next path)

# v2/, behavioral_grounding_fast.py, etc. deleted completely
```

---

## Action: Archive & Cleanup

**Commit 1** (current): Lock Phase 1 work, document structure
- Commit: PHASE1_STRUCTURE.md
- Mark v1 scripts as LOCKED in comments
- Update ROADMAP.md to show v2 deferral path

**Commit 2** (post-Phase-3): Archive or delete exploratory work
- If v2 approved: promote v2/ to locked, v1/ to history
- If v2 rejected: delete v2/, delete behavioral_grounding_* scripts
- Update PHASE1_STRUCTURE.md with actual outcome

---

**This structure ensures**:
✓ v1 is immutable until Phase 3 measurement validates/falsifies it  
✓ v2 is clearly deferred, not "maybe someday"  
✓ Behavioral grounding scripts have an explicit deletion date (end of Phase 1)  
✓ K15 consumer relationship is documented (Phase 2 kernel integration)  
✓ Future developers see the decision tree, not orphan code
