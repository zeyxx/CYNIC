# K15 Domain Router v1 — Design Notes

**Session**: 2026-05-03  
**Status**: READY FOR PHASE 2 (pending kernel recovery)

## What Was Done

1. **Domain Router Skeleton** (domain_router_v1.py)
   - Loads pre-computed TF-IDF clustering results (7 clusters)
   - Routes observations by keyword heuristics (Phase 1) → TF-IDF inference (Phase 2)
   - Returns: {domain, target, confidence, cluster_id}

2. **Routing Logic** (based on measured engagement)
   - **Human-bound clusters** (83%): 1, 4, 5 (General + LLM tech)
   - **Organ-X-bound clusters** (17%): 0, 2 (Token-specialist)
   - Default: Route to human if unknown

3. **K15 Consumer Wire-Up** (design, not yet wired)
   - Kernel calls: `router.route_observation(observation_text)`
   - Consumer updates: `/reflections/{observation_id}` with domain + target
   - Feedback loop: K15 producer observes routing decisions, measures signal yield per domain

## Key Decisions

1. **Keyword heuristics Phase 1, TF-IDF inference Phase 2**
   - Decision: Use simple keyword patterns now, add TF-IDF encoder when kernel recovers
   - Reason: Reduces blocker surface (router works immediately), TF-IDF is optimization
   - Falsify if: Keyword routing misclassifies >20% of observations

2. **Target inference from cluster profiling**
   - Decision: Map cluster_id directly to target (no probabilistic model)
   - Reason: Data shows clean 83/17 split (general vs specialist)
   - Falsify if: Phase 3 measurement shows signal yield diverges by >0.5σ per domain

3. **Confidence scores are heuristic (not probabilities)**
   - Decision: Confidence = keyword match strength + cluster cohesion estimate
   - Reason: Phase 1 doesn't have silhouette per-cluster, use composite
   - Upgrade: When silhouette passes, use per-cluster silhouette as confidence base

## Blockers

- **Kernel unreachable** (2026-05-02 → ?): Cannot wire K15 consumer until kernel recovers
  - Mitigation: Router code is ready; kernel integration is <30min once kernel live

## Next Steps (Phase 2 Integration)

1. Verify kernel health: `curl http://cynic-core:3030/health`
2. Wire K15 consumer in kernel:
   - Kernel loads DomainRouter on startup
   - `/route_observation` endpoint calls router, stores decision in reflections
3. Enable Phase 3 measurement:
   - Split verdicts by domain_route
   - Measure signal yield per domain (target: token_analysis > 5.0, general ≈ 4.2)

## What Would Falsify This Work

1. **Keyword routing accuracy < 70%**: Heuristics insufficient; need TF-IDF encoder
2. **Phase 3 signal yield < 2% improvement**: Routing not the bottleneck
3. **Domain inference misses cross-cluster authors**: Need multi-domain support (Phase 2 improvement)

## Confidence

**Phase 1 design: φ⁻¹ (0.618)**
- Cluster structure proven (domain-discovery v1 validated)
- Routing logic grounded in measured engagement (82% human browsing)
- Code ready, kernel blocker orthogonal

---

**Next session**: Check kernel status, wire K15 consumer, measure Phase 3.
