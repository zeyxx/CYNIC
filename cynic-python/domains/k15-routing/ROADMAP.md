# K15 Routing — Roadmap

## v1 (Phase 2 Ready, pending kernel recovery)

**Status**: Code complete, blocked on kernel health  
**Timeline**: 2026-05-03 design, 2026-05-?? deployment (when kernel recovers)

### Deliverables
- ✓ DomainRouter class (keyword heuristics phase)
- ✓ CLI interface (--route, --route-stdin)
- ✓ Unit tests (token/llm/general classification)
- ✓ NOTES_v1.md (design decisions, falsification criteria)

### Design
- 7 semantic clusters from TF-IDF discovery (domain-discovery v1)
- 83% human engagement → clusters 1,4,5 (general/LLM)
- 17% specialist → clusters 0,2 (token-analysis)
- Keyword heuristics route observations; TF-IDF inference deferred to v2

## v2 (Phase 2 Optimization, 2-3h work)

### Goals
- Add TF-IDF encoder to compute cluster similarity for new observations
- Per-cluster confidence from silhouette coefficients
- Multi-domain support (observations that bridge clusters)
- Measurement integration (log routing decisions to /reflections)

### Changes
- Load vocab + IDF weights from clustering results
- Implement `compute_tfidf_vector(text) → sparse_dict`
- Implement `find_nearest_cluster(vector) → (cluster_id, distance)`
- Update confidence from keyword heuristic (0.6-0.8) to silhouette-based
- Add --measure flag to write decisions to observations log

### Dependencies
- Silhouette coefficient from domain-discovery v1 (in progress)
- K15 consumer wiring in kernel (need kernel health first)

## v3 (Phase 3 Learning, 2-4w work)

### Goals
- Embed routing in feedback loop
- Measure phase 3 signal yield per domain (target: token > 5.0, general ≈ 4.2)
- Auto-tune cluster assignments if signal diverges

### Changes
- Kernel integration: router receives observation, records domain + confidence
- Measurement: track routing distribution, signal yield per domain
- Feedback: if signal improves with domain-aware routing, update weights in v4

### Success Criteria
- Token-analysis signal > 5.0 (vs baseline 4.2, 19% improvement)
- General signal maintains ≥ 4.0 (no regression)
- Domain-aware routing beats random-weighted routing by >0.5σ

## v4 (Crystallization, Rust port)

**Decision gate**: Phase 3 measurement shows signal improvement > 2%.

Once v3 validates domain routing, port to Rust:
- Move logic to `cynic-kernel/src/domain/router.rs`
- Remove Python consumer (archive old version)
- Kernel owns routing decision, no cross-process latency

---

## Known Limitations

1. **Keyword heuristics in v1 may misclassify** (target: >80% accuracy)
   - Falsify: Manual audit of 100 diverse observations, compute precision/recall
   - Mitigation: Low confidence threshold (0.6) until TF-IDF in v2

2. **No cross-domain support** (multi-domain tweets)
   - Affects: ~5% of tweets per bridge_detector results
   - Mitigation: Highest-confidence cluster assignment sufficient for Phase 1

3. **Confidence scores not probabilistic**
   - Reason: Phase 1 uses heuristics, not Bayesian model
   - Upgrade: Use silhouette as proxy in v2, proper calibration in v3

## Integration Points

| Phase | Consumer | Input | Output | Blocker |
|-------|----------|-------|--------|---------|
| v1 | (standalone) | text | {domain, target, confidence} | None (ready) |
| v2 | kernel /route | observation | reflections.domain_route | Kernel recovery |
| v3 | kernel feedback loop | routing + signal | Per-domain metrics | Phase 2 → Phase 3 transition |

---

**Next session**: Verify kernel health, wire v1 consumer, start Phase 2 measurement.
