# Python Code Lifecycle in CYNIC Organism

**Question**: At what state/time horizon does Python research code become part of the production organism?

## Timeline Model

### Phase 0: Research (1-3 sessions)
**State**: Exploratory notebooks, zero commitment  
**Time horizon**: Proof-of-concept, can be abandoned  
**Storage**: `.gitignore`, not committed  
**Example**: `test_embedding_clustering.ipynb`, `debug_helius_v0.py`  
**Lifecycle end**: If falsification fails OR superseded, stays exploratory

### Phase 1: Validated (Multi-session, proven)
**State**: Versioned scripts, reproducible results, F0-F3+ tests pass  
**Time horizon**: 1-4 weeks, feedback loop active  
**Storage**: `domains/{domain}/v{N}/`, committed but NOT integrated  
**Example**: `domains/domain-discovery/v1/emergent_clustering_tfidf_v1.py`  
**Lifecycle end**: Ready for Phase 2 (production adoption) OR archived (v2 replaces v1)  
**Requirement**: P1-P6 compliance (types, tests, logging, entry points)

### Phase 2: Integrated (Active K15 consumer)
**State**: Python code called by Rust kernel OR MCP agents  
**Time horizon**: 1-12 weeks, actively in feedback loop  
**Storage**: `shared/` (utilities) + `domains/{domain}/v{N}/` (domain logic)  
**Example**: `domain_router.py` (K15 consumer, called by kernel on each observation)  
**Lifecycle requirement**: 
  - K15 consumer exists (kernel calls it)
  - Metrics tracked (execution count, latency, error rate)
  - Feedback loop active (kernel/agent updates behavior based on Python output)
**Lifecycle end**: Crystallized (v3, Rust) OR archived (deprecated)

### Phase 3: Crystallized (Permanent, Rust)
**State**: Python logic ported to Rust, living in kernel  
**Time horizon**: Permanent, no further evolution in Python  
**Storage**: Deleted from `cynic-python/`, logic in `cynic-kernel/src/`  
**Example**: Heuristic scoring (was `domain_verdict_builder.py`, now in `src/domain/heuristics.rs`)  
**Requirement**: 
  - Phase 2 metrics prove value (signal improvement, reliability > 95%)
  - Latency < tolerance (no blocking calls from kernel)
  - Invariants expressible in type system (Rust enums/traits)

### Retirement (Archive)
**State**: No longer used, historical reference  
**Time horizon**: Indefinite, read-only  
**Storage**: `domains/archive/v{N}_{reason}.py`  
**Example**: `domains/archive/emergent_clustering_numpy_v0.py` (superseded by TF-IDF)  
**Requirement**: `ARCHIVE_MANIFEST.md` entry explaining why + what replaced it

## Transitions & Conditions

### Research → Validated
**Requires**:
- Falsification tests pass (F0-F3 or domain-specific gates)
- Reproducible across 2+ sessions
- P1-P6 compliance (types, tests, deps, logging, entry points, observability)
- NOTES_v{N}.md documents decisions, blockers, next steps

**Blocker**: Test failures, non-reproducible, missing P1-P6 compliance

### Validated → Integrated
**Requires**:
- K15 consumer identified and wired (kernel/agent calls Python code)
- Metrics instrumentation deployed (execution count, latency, signal quality)
- Feedback loop active (organism behavior changes based on Python output)
- 1-4 weeks active use, zero silent failures

**Blocker**: No K15 consumer OR silent failures detected OR signal yield < baseline

### Integrated → Crystallized
**Requires**:
- Phase 2 metrics show value (signal improvement > φ⁻¹ confidence, reliability > 95%)
- Invariants stable (thresholds, domains, rules don't change week-to-week)
- Latency allows Rust port (no blocking calls, no external API waits)
- Rust implementation complete + tested

**Blocker**: Metrics don't improve signal, frequent threshold tuning, latency too high

## Current Python Code States

| Domain | Version | Phase | Timeline | K15Consumer | Blocker |
|--------|---------|-------|----------|------------|---------|
| domain-discovery | v1 | Validated | d3a300ad-now (2026-05-03) | domain_router_v1.py (designed) | Kernel unreachable |
| k15-routing | v1 | Validated | 2026-05-03-now | (ready for Phase 2 wiring) | Kernel unhealthy |
| behavioral-analysis | v1 | Validated | 2026-04-27-now | (pending Phase 2) | None identified |
| token-calibration | v1 | Integrated | 2026-04-29-now | phase2_token_scorer | Kernel unreachable |
| inference-lab | v1 | Integrated | 2026-04-27-now | hermes_agent | Works (llama offline) |
| — | — | Archive | 2026-04-* | — | Superseded |

## Rules

1. **No Python code lives in organism without a K15 consumer**
   - Research (Phase 0) doesn't count (notebooks, exploratory)
   - Validated code (Phase 1) with K15 pending is OK (waiting for integration)
   - Integrated code (Phase 2) with no consumer = dead code (archive)
   - Falsify: `grep -r "import.*cynic_python" cynic-kernel/` — every import needs a callsite

2. **No Python code changes in production without Phase 2 validation**
   - Changes in `domains/{domain}/v{N}/` require F-tests or signal measurement
   - Threshold tuning counts as "change" (requires re-measurement)
   - Exception: Bug fixes in Python (non-logic changes) can be backported to Phase 2

3. **Crystallized logic is Rust logic**
   - Once ported to Rust, Python version is archived, not updated
   - Reason: Dual maintenance risk, divergence
   - If Python logic needs to change post-crystallization, fork a new domain (v2)

4. **Every Phase 2 code needs metrics**
   - Execution count (how often called)
   - Latency (time per call)
   - Quality metric (signal, precision, recall, or domain-specific)
   - Tracked in kernel `/health` endpoint OR MCP observer

## Time Horizons

| Phase | Typical Duration | Decision Point |
|-------|------------------|-----------------|
| **Research** | 1-3 sessions | "Does this approach work?" |
| **Validated** | 1-4 weeks | "Ready for organism integration?" |
| **Integrated** | 1-12 weeks | "Is this valuable enough to crystallize?" |
| **Crystallized** | Permanent | "Rust port stable + tested" |

## Current Blocker: Kernel Unhealthy

Domain-discovery v1, k15-routing v1, behavioral-analysis v1 are all **Validated** and ready for Phase 2 (K15 integration), but blocked by kernel health:

```
Status (2026-05-03):
  Kernel health: {"healthy": false}
  Last probe: curl -s http://<TAILSCALE_CORE>:3030/health
  Status since: 2026-05-02 (degraded → unhealthy over 24h)
  
Phase 2 Readiness:
  ✓ domain-discovery v1: Clustering validated (F0-F3 pass)
  ✓ k15-routing v1: Domain router designed + coded (domain_router_v1.py)
  ✓ behavioral-analysis v1: Analysis complete (pending measurement phase)
  
Next step: Verify kernel recovery
  curl http://<TAILSCALE_CORE>:3030/health
  If {"healthy": true}: Wire K15 consumers immediately (all code ready)
  If {"healthy": false}: Continue Phase 1 refinement (silhouette validation, v2 planning)
```

## Approval Gate (Org-Level Decision)

Before ANY Python code enters Phase 2 (integrated):

- [ ] Falsification tests pass (F0-F5 or equivalent)
- [ ] P1-P6 compliance verified (types, tests, observability, entry points)
- [ ] K15 consumer designed + planned (what will call this code?)
- [ ] Metrics instrumentation clear (what will we measure?)
- [ ] Measurement plan documented (how do we falsify Phase 2?)

**For domain-discovery v1**: 
- ✓ F0-F3 pass
- ✓ P1-P6 compliant
- ✓ K15 consumer: `domain_router.rs` (designed, pending kernel recovery)
- ✓ Metrics: Observation routing alignment, signal yield per cluster
- ✓ Measurement: Phase 3 (7-day cycle, target > 5.0 avg signal)

→ **Ready to enter Phase 2 once kernel recovers**
