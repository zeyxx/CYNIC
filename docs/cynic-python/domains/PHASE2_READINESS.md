# Phase 2 Readiness — Session 2026-05-03

**Status**: All Python Phase 1 code complete. Waiting on kernel recovery for Phase 2 wiring.

## What's Ready

### domain-discovery v1 (Validated)
- ✓ TF-IDF clustering: 7 semantic clusters
- ✓ F0-F3 tests: All pass (cluster count, domain specificity, distribution, author dispersal)
- ✓ Results: `emergent_clusters_tfidf.json` with cluster profiles
- ✓ Documentation: NOTES_v1.md, ROADMAP.md
- ⏳ Silhouette validation: Score computed but not saved (pending)

### k15-routing v1 (Validated)
- ✓ Code: domain_router_v1.py (DomainRouter class)
- ✓ Tests: unit tests for token/llm/general routing
- ✓ Documentation: NOTES_v1.md, ROADMAP.md
- ✓ Design: Routes clusters to human (1,4,5) vs organ-x (0,2)
- ✓ Ready for Phase 2: Kernel wiring (pending kernel recovery)

### behavioral-analysis v1 (Validated)
- (From prior sessions, structure follows protocol)

## What's Blocked

**Kernel Health**: `curl http://<TAILSCALE_CORE>:3030/health` returns `{"healthy": false}`

Kernel has been unhealthy since 2026-05-02. This blocks:
- K15 consumer wiring (router loading, observation routing)
- Reflections storage (routing decisions)
- Phase 3 measurement (signal yield tracking)

## What's Pending (Waiting for Kernel)

### Phase 2 Integration (2-3h work, ready to execute)

1. **Kernel consumer wire-up**:
   - Load DomainRouter in kernel startup
   - Expose `/route_observation` endpoint
   - Route observations, store decisions in reflections

2. **Phase 3 Measurement** (7 days):
   - Split verdicts by domain routing
   - Measure signal yield: token-analysis (target >5.0), general (baseline ≈4.2)
   - Falsify if: improvement <2%

## Optional Phase 1 Refinement (While Kernel Down)

### Silhouette Validation (F0 Gate)
- Clustering script computes silhouette but doesn't save score
- **TODO**: Re-run clustering, extract silhouette, save to metrics_v1.json
- **Gate**: Silhouette > 0.5 required for F0 full pass
- **Impact**: F0 currently marked "pending"; passing silhouette gate completes F0

### v2 Improvements (v1 → v2, 2-4h work each)

**domain-discovery v2**:
- Add stopword filtering (the, and, you, for, all, etc.)
- Clean URL fragments (http://, tps, etc.)
- Re-validate clustering on cleaned data

**k15-routing v2**:
- Implement TF-IDF encoder (compute cluster similarity for new observations)
- Per-cluster confidence from silhouette coefficients
- Multi-domain support (bridge detection)

## Timeline

| Phase | Status | Blocker | Next Action |
|-------|--------|---------|-------------|
| **Phase 1** | ✓ Complete | None | Execute Phase 1 refinement while waiting |
| **Phase 2** | Ready to wire | Kernel health | Verify `healthy: true`, wire consumers |
| **Phase 3** | Design done | Phase 2 completion | Start 7-day measurement cycle |

## Success Criteria

**Phase 2 (Integration)**:
- Kernel wiring complete: observations route by domain
- Reflections written: routing decisions stored
- Measurement ready: signal yield tracking per domain

**Phase 3 (Measurement)**:
- Token-analysis signal: >5.0 (vs baseline 4.2)
- General signal: ≥4.0 (no regression)
- Domain routing improvement: >0.5σ vs random-weighted

---

**Owner**: T. (zeyxx)  
**Session**: 2026-05-03  
**Commits**: feat(k15-routing) + docs(organism-integration)
