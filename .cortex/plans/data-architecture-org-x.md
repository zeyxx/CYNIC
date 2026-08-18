# Data Architecture for Organic Agent — Lifecycle & Tiers

Using the Python Lifecycle protocol (see `.cortex/rules/python-lifecycle.md`).

---

## Decision: Where Should Data Architecture Live?

### Option A: scripts/hermes-x/ (Existing Organ-X Home)
**Tier**: 2 INFRASTRUCTURE
**Rationale**:
- Follows proven pattern (all 33 hermes-x scripts are active)
- Gets wired immediately (systemd already runs hermes-x jobs)
- Single directory = single namespace = easier to manage

**Structure**:
```
scripts/hermes-x/
├── data_audit.py          # Phase 0: Validate correlations work
├── dataset_enricher.py     # Phase 1: Add tweet_id, timestamps
├── behavior_enricher.py    # Phase 1: Add domain, URL, position
├── search_enricher.py      # Phase 1: Capture returned tweets
├── unified_event_loader.py # Phase 2: Load JSONL → SurrealDB
├── signal_measurement.py   # Phase 3: Agent feedback queries
└── README.md              # Architecture overview
```

**Consumer**: hermes-k15-consumer.service (reads measurements → updates agent strategy)

**Wiring**: New systemd timer `hermes-data-pipeline.timer` (runs data enrichment every 6h)

---

### Option B: New Tier 2 Module (cynic-python/data-infrastructure/)
**Tier**: 2 INFRASTRUCTURE
**Rationale**:
- Separates data infrastructure from hermes-x concerns
- Allows cynic-python to have ONE proper consumer (breaks the graveyard)
- Package structure is cleaner for a subsystem

**Structure**:
```
cynic-python/data-infrastructure/
├── __init__.py
├── schema.py              # UnifiedEvent classes
├── loaders/
│   ├── behavior_loader.py
│   ├── dataset_loader.py
│   ├── search_loader.py
│   └── correlator.py
├── queries/
│   ├── signal_measurement.py
│   ├── domain_interest.py
│   └── agent_feedback.py
├── tests/
│   ├── test_schema.py
│   ├── test_loaders.py
│   └── test_queries.py
└── README.md
```

**Consumer**: hermes-data-pipeline.service (systemd) calls all loaders + queries

**Wiring**: New systemd service + timer (both in ~/.config/systemd/user/)

---

## Recommendation: Start with Option A

**Why**:
1. **Lower risk**: Uses proven hermes-x pattern, no new infrastructure
2. **Faster iteration**: Already in scripts, no module boundaries to cross
3. **Immediate testing**: Can wire Phase 0 audit immediately
4. **Clear K15**: hermes-k15-consumer consumes signal_measurement output
5. **Decision point**: After Phase 1, assess if data-infrastructure warrants its own Tier 2 module

**Transition path**:
- Phase 0-1: Live in scripts/hermes-x/ (Tier 1 EXPERIMENTAL)
- Phase 2-3: Assess for promotion to Tier 2
- If promoted: Move to cynic-python/data-infrastructure/ (new module) with explicit consumer
- If rejected: Delete cleanly (clear death, not rot)

---

## Actual Implementation Plan (Tier 1 → Tier 2)

### Phase 0: Audit & Baseline (Tier 1 EXPERIMENTAL)

**Timeline**: 3-4 days
**Success condition**: Correlation feasibility proven (>60% success rate on pixel→tweet matching)

**Tier 1 Tagging**:
```python
"""
Tier 1 EXPERIMENTAL: Data audit for organic agent learning.

Research question: Can we correlate user clicks to tweets they clicked on?
Success condition: >60% of behavior_log clicks link to dataset tweets
Timeline: 3 days
Owned by: @T
Status: ACTIVE (2026-05-05)

Will promote to Tier 2 if: Correlation works + agent feedback loop feasible
Death date: 2026-05-22 (unless promoted)
"""
```

**Audit checklist** (lives in scripts/hermes-x/data_audit.py):
- [ ] Sample 100 clicks from behavior_log (last 7 days)
- [ ] For each click, find matching tweet in dataset
  - Matching criteria: spatial (x±50, y±50) + temporal (t±500ms)
  - Success = found tweet in dataset
- [ ] Measure success rate
  - <50%: Audit fails, pixel→tweet infeasible, pivot needed
  - 50-70%: Partial success, need enrichment (browser logger + screen positions)
  - >70%: Success, proceed to Phase 1
- [ ] Sample 10 searches from search_execution_log
- [ ] For each search, identify returned tweets
  - Success: Can extract tweet_ids from results?
  - Measure success rate
- [ ] Sample 5 observation clusters
- [ ] Can we link observation → verdicts generated from its tweets?
  - Measure success rate

**Deliverable**: data_audit_report.md (with success rates + recommendations)

**If audit fails**: Delete scripts, kill the initiative (don't rot the code)

**If audit succeeds**: Promote Phase 1 to Tier 2 INFRASTRUCTURE (wire systemd)

---

### Phase 1: Enrichment (Tier 2 INFRASTRUCTURE)

**Timeline**: 1 week
**Success condition**: All three enrichers running error-free, 100% of historical data enriched

**Tier 2 Promotion Checklist**:
- [ ] Audit Phase 0 ≥60% success rate
- [ ] Consumer identified (Phase 3 queries need this data)
- [ ] Systemd timer written (runs daily)
- [ ] Fallback documented (what if enrichment fails?)
- [ ] Error rate <1% (strict for infrastructure)

**Tier 2 Tagging**:
```python
"""
Tier 2 INFRASTRUCTURE: Enrich behavior_log, dataset, search_execution with missing fields.

Phase: 1 of 4 (agent learning pipeline)
Promotion: 2026-05-05 (from Phase 0 audit)
Consumer: signal_measurement.py (Phase 3 uses enriched data for queries)
K15: Observations (enriched data) → agent_feedback (agent learns)
Metrics: /health.data_enrichment_status
  - behavior_enriched_count
  - dataset_enriched_count
  - search_enriched_count
  - last_enrichment_timestamp

Systemd: hermes-data-enrichment.timer (daily, 02:00 UTC)
Error handling: If enrichment fails, log and continue (data incomplete but usable)
"""
```

**What gets enriched**:
1. **behavior_log**:
   - Add: `domain`, `url` (from proxy logs), `visible_duration_ms`
   - Source: mitmproxy logs (X.com traffic)
   - Target: 100% coverage

2. **dataset (curated D1-D6)**:
   - Add: `tweet_id` (extract from source filename or Twitter API)
   - Add: `timestamp` (from capture metadata)
   - Source: Source file naming + Twitter API
   - Target: 100% coverage

3. **search_execution_log**:
   - Add: `returned_tweet_ids` (capture at search time)
   - Add: `user_clicked_subset` (correlate with behavior_log)
   - Source: Search results page + behavior_log
   - Target: 100% coverage

**Output**: Enriched JSONL files (backup originals, append new fields)

---

### Phase 2: Schema & Loading (Tier 2 INFRASTRUCTURE)

**Timeline**: 1 week
**Success condition**: Data in SurrealDB, test queries returning correct results

**Tier 2 Continuation**:
- [ ] UnifiedEvent schema proven on enriched data (no schema errors)
- [ ] SurrealDB tables created (with indexes)
- [ ] Loader tested on 1% of data (1000s of events)
- [ ] 100% load succeeds (no data loss, no orphaned links)

**What gets wired**:
- Systemd service: `hermes-data-loader.service` (oneshot, runs daily after enrichment)
- SurrealDB connection: Kernel REST endpoint (if not already available)

---

### Phase 3: Agent Feedback Queries (Tier 2 INFRASTRUCTURE)

**Timeline**: 1 week
**Success condition**: Agent learns signal yield; search strategy improves

**Tier 2 Continuation**:
- [ ] Query 1: "For search in domain D at T, which tweets were clicked?"
- [ ] Query 2: "For clicked tweets, which produced verdicts?"
- [ ] Query 3: "Measure: signal_yield = verdicts_count / search_count per domain"
- [ ] Agent consumes query results → adjusts search strategy
- [ ] K15 test: Agent behavior changes based on measured signal_yield

---

### Phase 4: Assess for Tier 3 (Rust Migration)

**Timeline**: 2 weeks (if audit/phases succeed)
**Decision**: Do these belong in kernel Rust?

**Tier 3 Candidacy Questions**:

1. **Latency critical?**
   - Current: Python queries → agent feedback loop ~2-5s
   - Target: <500ms (agent should learn within single cycle)
   - If yes → Rust candidate

2. **Data structure core to kernel?**
   - UnifiedEvent becomes the SSOT for agent learning
   - Should it live in kernel storage layer?
   - If yes → Rust candidate

3. **Error risk too high for Python?**
   - Data corruption = agent learns wrong patterns
   - Silent failures (NULL in query results) block learning
   - Type system needed to prevent bad data entry?
   - If yes → Rust candidate

**Likely outcome**: Phase 3 queries stay Python (non-latency critical), but Phase 2 loading → kernel

---

## Death Conditions

If any phase fails:
- Phase 0 audit <60% success → **DELETE all code** (correlated doesn't work, pivot needed)
- Phase 1 enrichment >5% error → **PAUSE, FIX** (incomplete enrichment blocks everything)
- Phase 2 load incomplete → **ROLLBACK** (bad data is worse than no data)
- Phase 3 agent doesn't improve → **INVESTIGATE** (signal measurement is wrong, not data)

---

## CI & Metrics

**make lint-python-tiers**:
```
✓ Phase 0 (data_audit.py) tagged Tier 1 EXPERIMENTAL
✓ Phase 1 (enrichers) tagged Tier 2 INFRASTRUCTURE
✓ Phase 2 (loader) tagged Tier 2 INFRASTRUCTURE
✓ Phase 3 (queries) tagged Tier 2 INFRASTRUCTURE
✓ Each has explicit consumer documented
✓ No orphaned producers
```

**systemd metrics** (`/health`):
```json
{
  "data_architecture": {
    "phase_0_audit": "complete | pending | failed",
    "phase_1_enrichment": {
      "behavior_enriched": 12345,
      "dataset_enriched": 8901,
      "search_enriched": 234,
      "last_run": "2026-05-05T02:00:00Z"
    },
    "phase_2_loading": {
      "events_in_surrealdb": 98765,
      "last_load": "2026-05-05T02:30:00Z"
    },
    "phase_3_feedback": {
      "signal_yield_per_domain": {"D1": 0.67, "D2": 0.45, ...},
      "agent_strategy_updated": true,
      "last_update": "2026-05-05T03:00:00Z"
    }
  }
}
```

---

## Success Criteria (K15 Falsifiable)

**Data architecture succeeds when**:

1. ✓ Agent's search strategy visibly changes (higher engagement domains = higher search frequency)
2. ✓ Agent feedback loop closes: agent_action → user_click → verdict → agent_learns (all within 5min)
3. ✓ Signal yield measured per domain (not speculation, measured data)
4. ✓ Agent's success rate improves (measured: verdicts per search, engagement per search, domain routing accuracy)

**Falsifiable**: If none of these are true after Phase 3 completes, the data architecture failed.

---

## Memory Entry

After this plan is approved, create:
```
name: data-architecture-org-x-lifecycle
description: Organic agent learning pipeline, Tier 2 INFRASTRUCTURE, 4-phase rollout
type: project
---

Status: PLANNED (approved 2026-05-05)
Phase 0: DATA AUDIT (Tier 1 EXPERIMENTAL, 3-4 days)
  Timeline: 2026-05-05 to 2026-05-09
  Success: Correlation feasibility >60%
  Death: 2026-05-22 if not promoted

Phase 1-4: Pending Phase 0 success
Consumer: hermes-k15-consumer.service (learns agent strategy)
Systemd: hermes-data-enrichment.timer, hermes-data-loader.service, hermes-signal-measurement.timer
Location: scripts/hermes-x/ (Tier 1) → cynic-python/data-infrastructure/ (Tier 2 if promoted)
```
