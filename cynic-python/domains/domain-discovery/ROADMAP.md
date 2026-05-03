# Domain Discovery Roadmap (v0 → v∞)

**Current Status**: v1 LOCKED, v2 DEFERRED (2026-05-03)

## Phase Progression

### v1: TF-IDF Semantic Clustering (LOCKED for Phase 2-3)
- **Status**: ✓ Complete, validated, behavioral grounding φ⁻³ confidence
- **Clusters**: 7 semantic clusters from TF-IDF on 6,361 tweets
- **Quality**: HHI=0.266 (well-balanced), F0-F3 tests pass
- **Consumer**: K15 router (Phase 2 integration)
- **Locked Until**: Phase 3 measurement (if signal yield <2%, consider alternative)
- **Files**: `v1/emergent_clustering_tfidf_v1.py`, `results_v1/`

**Falsification Criteria**:
- Phase 3 signal yield <2% improvement → v1 hypothesis FALSIFIED, promote v2 or alternative

---

### v2: Stopword Filtering (DEFERRED — Conditional on Phase 3 Result)

**Hypothesis**: Removing 60+ English stopwords reduces noise, improves silhouette >5%

**Status**: 
- Code written, tested locally ✓
- Execution hung during iteration 9/15 (k-means timeout)
- Deferral decision: v1 sufficient for Phase 2, no critical path blocker

**Conditional Activation**:
| Phase 3 Result | Action |
|---|---|
| Signal yield >5% | Keep v1 (optimization unnecessary) |
| Signal yield 2-5% | Activate v2 optimization (Phase 4) |
| Signal yield <2% | Inspect alternative approaches (v2 alone may not fix) |

**Files**: `v2/emergent_clustering_tfidf_v2.py`, `v2/NOTES_v2.md`, `v2/validate_improvement_v1_v2.py`

**Cleanup Plan**: If Phase 3 shows >5% yield, delete v2/ directory entirely (decision finalized, no need to keep alternative)

---

### v3: Semantic Bridges / Emergent Domains (CONCEPTUAL)

**Idea**: Detect multi-cluster signals (tweets that bridge domains) to identify cross-domain patterns

**Motivation**: v1 assumes hard cluster boundaries; but real content often spans multiple domains (e.g., "Solana security" = token_analysis + security)

**Status**: Research phase only (not scoped for Phase 1-3)

**Dependencies**: v1/v2 validated first

---

### v4: Rust Crystallization (OPTIONAL)

**Idea**: If Phase 3 validates domain routing, port Python clustering logic to Rust for production speed

**Prerequisites**: 
- Phase 3 signal yield confirmed >2%
- v1 or v2 fully specified
- K15 consumer contract locked

**Timeline**: Post-Phase-3, if viability proven

---

## Decision Tree (Phase 3 → Phase 4)

```
Phase 3: Measure signal_yield_per_domain[token_analysis]
  │
  ├─ yield <2%
  │  └─→ FALSIFIED: v1 routing doesn't improve signal
  │      Action: Investigate why (domain boundaries wrong? routing logic?)
  │              Consider alternative (e.g., time-based routing, author-based)
  │              Delete v2/ (won't solve root problem)
  │
  ├─ 2% ≤ yield <5%
  │  └─→ VALIDATED but suboptimal
  │      Action: Activate v2 (Phase 4)
  │              Deploy v2 clustering, re-measure
  │              Keep both v1/ and v2/ for comparison
  │
  └─ yield ≥5%
     └─→ VALIDATED and sufficient
         Action: Keep v1 (no v2 needed)
                 Delete v2/ (decision finalized, unnecessary artifact)
                 Move to Phase 4 Rust crystallization
```

---

## Artifact Lifecycle

| Artifact | Phase 1 | Phase 2 | Phase 3 | Phase 4 | After |
|---|---|---|---|---|---|
| v1/ | ✓ Locked | ✓ Live | ✓ Measure | ✓ Crystallize | Archived |
| v2/ | Deferred | — | Decision | Conditional | ✗ Deleted or ✓ Promoted |
| behavioral_grounding_* | ✓ Created | ✗ Deleted | — | — | — |
| PHASE1_STRUCTURE.md | ✓ Created | — | Reviewed | Updated | Archived |

**Deletion Events**:
- End of Phase 2: Delete `behavioral_grounding_fast.py`, `behavioral_topic_analysis.py` (use case complete)
- End of Phase 3: Delete OR promote v2/ (decision made)

---

## Success Criteria (Phase 1 → Phase 2)

✓ v1 clustering reproducible and validated (HHI, F-tests)  
✓ K15 router design complete, unit tested  
✓ Consumer contract documented (Phase 2 kernel integration)  
✓ v2 hypothesis documented, deferral justified (φ⁻² confidence sufficient)  
✓ Behavioral grounding confirms routing approach sound (φ⁻¹ Phase 3 >2% signal)  

**Phase 1 Complete**: 2026-05-03 ✓

---

## Notes

- **v1 → v2**: Not a "minor update" (v1.5). If v2 ships, v1 is archived as historical artifact.
- **Avoid iteration hell**: Decision tree prevents "maybe we'll optimize later" debt. v2 gets a binary decision at Phase 3 completion.
- **Consumer focus**: Every version's success is measured by Phase 3 signal yield, not academic metrics (silhouette >0.5 doesn't matter if Phase 3 shows <2% improvement).
