# SESSION 9: DIRECTION PLAN & IMMEDIATE ACTIONS
## Hybrid Path (CCC) — Horizontal Scaling + Gradual Refactoring

**Date**: 2026-02-21 (Session 9 Continuation)
**Strategy**: Hybrid (Quick scale + Gradual refactor)
**Timeline**: Week 1 ops, Weeks 2-8 architecture
**Solo Dev**: Sequenced to avoid parallelization deadlocks
**Confidence**: 61.8% (φ⁻¹)

---

## WHAT WAS DECIDED

### Three Documents Created

1. **TECHNICAL_DEBT_ANALYSIS_2026-02-20.md** (400 LOC)
   - Complete inventory of architectural debt
   - 7 phases (A-G) to refactor from monolithic → distributed
   - 140-175 hour roadmap (6-7 weeks)
   - Decision gates for each phase
   - **Status**: Reference doc; start after Week 1 validation

2. **WEEK_1_HORIZONTAL_SCALING_ROADMAP.md** (300 LOC)
   - Day-by-day plan to deploy 3 CYNIC instances
   - nginx load balancer setup
   - MultiInstanceCoordinator for consensus
   - Stress test to verify 150 RPS
   - **Status**: Ready to execute Monday 2026-02-24

3. **SESSION_9_DIRECTION_PLAN.md** (this file)
   - Summary of decision
   - Immediate action items
   - Sequencing for solo dev
   - **Status**: Action plan

---

## STRATEGY COMPARISON (Why Hybrid?)

### Path A: Refactoring NOW (6-7 weeks)
```
Week 1-7: Massive refactoring
├─ Phase A: KernelBuilder (20-25h)
├─ Phase B: DependencyContainer (40-50h)
├─ Phase C: Constructor DI (15-20h)
├─ ...phases D-G...
Result: Perfect architecture, but users wait 7 weeks for scale
Risk: Massive change, could break things
```

### Path B: Horizontal Scaling NOW (1 week)
```
Week 1: 3 instances + nginx
├─ Deploy 3 CYNIC instances
├─ nginx load balancer
├─ Reach 150+ RPS
Result: Works NOW, but monolithic forever
Risk: Easier to break at higher load; no long-term solution
```

### Path C (CHOSEN): Hybrid (1 week + 7 weeks in parallel)
```
Week 1: Horizontal scaling (3 instances, 150 RPS)
        + Start Phase A refactoring (background)
├─ QUICK WIN: 150+ RPS this Friday
├─ THEN GRADUAL: Refactoring happens slowly, weeks 2-8
├─ NO RUSH: Each phase validated independently
Result: Short-term scale + long-term architecture
Risk: Lower (ops works first, then careful refactoring)
```

**Why This Is Smart**:
- Week 1 proves multi-instance works (SurrealDB handles it? nginx works? consensus OK?)
- After validation → refactor with confidence (not guessing)
- Solo dev can work on both without overwhelming context

---

## IMMEDIATE NEXT STEPS

### Session 9 (TODAY — 2026-02-21)

**Priority 1: Review & Validate**
- ✅ Read TECHNICAL_DEBT_ANALYSIS_2026-02-20.md (understand debt)
- ✅ Read WEEK_1_HORIZONTAL_SCALING_ROADMAP.md (understand ops plan)
- ⏳ Verify: Docker/docker-compose currently available?
- ⏳ Verify: nginx installed locally or can run in Docker?
- ⏳ Verify: Stress test tool (httpx, asyncio) available?

**Priority 2: Pre-Week 1 Setup**
- [ ] Current `docker-compose.yml` location: `_____________`
- [ ] Current CYNIC image: built locally or pulled from registry?
- [ ] SurrealDB instance: already running, or fresh?
- [ ] Ollama instance: already running, or fresh?
- [ ] Can you run 3 services on same host simultaneously?

**Priority 3: Commit Current State**
- [ ] Commit all Phase 5 empirical proof work
- [ ] Tag: `v0.5-phase5-macro-validation` (milestone)
- [ ] Document: Session 8 achievements

### Before Week 1 (2026-02-22 → 2026-02-23)

**Priority 1: Prep Docker**
- [ ] Test current docker-compose (can it start CYNIC + Ollama + SurrealDB?)
- [ ] Verify CYNIC image has `INSTANCE_ID` env var support
- [ ] Verify health endpoint works (`/health`)
- [ ] Document any environment issues

**Priority 2: Prep nginx**
- [ ] Install nginx locally OR plan to run in Docker
- [ ] Create `nginx.conf` skeleton (from WEEK_1 roadmap)
- [ ] Test nginx can forward to localhost:8001

**Priority 3: Stress Test Tool**
- [ ] Create `scripts/stress_test_150rps.py` from roadmap
- [ ] Test against current single instance (should hit ~50 RPS)
- [ ] Verify script works, logs are clean

**Priority 4: Pre-tests**
- [ ] Verify all 949 tests still pass (baseline before changes)
- [ ] Document current single-instance performance (latency, Q-scores)
- [ ] Save baseline metrics for comparison post-Week 1

---

## WEEK 1 EXECUTION PLAN

### Monday 2026-02-24 (Day 1-2)

**Goal**: Get 3 instances running on Docker

**Tasks**:
1. Clone current `docker-compose.yml` to backup
2. Create `cynic-1`, `cynic-2`, `cynic-3` services
3. Each on ports 8001, 8002, 8003
4. All share SurrealDB, Ollama
5. Environment: `INSTANCE_ID=1/2/3`
6. `docker-compose up -d && docker-compose ps` → All HEALTHY
7. `curl http://localhost:8001/health` → OK
8. `curl http://localhost:8002/health` → OK
9. `curl http://localhost:8003/health` → OK

**Commit**: `docker: Multi-instance compose (3 instances, no LB yet)`

---

### Tuesday 2026-02-25 (Day 2-3)

**Goal**: nginx load balancer

**Tasks**:
1. Create `nginx.conf` from roadmap
2. `least_conn` routing (not round-robin; more intelligent)
3. WebSocket forwarding (`/ws`)
4. Health check endpoint
5. Add `nginx` service to docker-compose
6. `docker-compose restart`
7. `curl http://localhost/health` (10×) → Distributes across 3 instances
8. Check nginx logs: `docker logs cynic_nginx_1`

**Verification**:
```bash
for i in {1..10}; do
  curl http://localhost/health | grep instance_id
done
# Should see: 1, 2, 3, 1, 2, 3, 1, 2, 3, 1
```

**Commit**: `ops: Add nginx load balancer for 3-instance routing`

---

### Wednesday 2026-02-26 (Day 3-4)

**Goal**: MultiInstanceCoordinator + consensus

**Tasks**:
1. Create `cynic/core/multi_instance.py` (from roadmap)
2. `MultiInstanceCoordinator` class (check existing judgments)
3. Integrate in `api/state.py` (injected during build)
4. Update routers to use coordinator (check before judging)
5. Rebuild CYNIC image: `docker build -t cynic:latest .`
6. Restart containers: `docker-compose up -d --force-recreate`
7. Test: Submit same cell 3× → First instance judges, other 2 reuse

**Verification**:
```bash
# Submit cell to instance 1 (via load balancer)
curl -X POST http://localhost/judge -d '{"cell_id":"TEST_1", ...}'

# Submit SAME cell to instance 2
curl -X POST http://localhost/judge -d '{"cell_id":"TEST_1", ...}'

# Check SurrealDB: Cell judged once
curl -X GET http://localhost/health | grep "judgments_unique"
# Should be 1, not 2
```

**Commit**: `feat: Multi-instance consensus coordinator`

---

### Thursday 2026-02-27 (Day 4-5)

**Goal**: Stress test 150 RPS

**Tasks**:
1. `scripts/stress_test_150rps.py` ready (from roadmap)
2. Run: `python stress_test_150rps.py`
3. Target: 150 RPS for 60 seconds
4. Measure: latency, errors, Q-scores
5. Expected:
   - Requests: 9000 (150 × 60)
   - Success: >8900 (>99%)
   - Errors: <100 (<1%)
   - Latency p95: <500ms
   - Q-Scores: 78-80
6. Document all metrics
7. Verify all 3 instances still HEALTHY after stress

**Commit**: `test: Stress test results (150 RPS, 3 instances)`

---

### Friday 2026-02-28 (Day 5-6)

**Goal**: Validation + Documentation

**Tasks**:
1. Verify checklist (all items):
   - [ ] 3 instances HEALTHY
   - [ ] nginx load balancing works
   - [ ] WebSockets forward correctly
   - [ ] Stress test 150 RPS successful
   - [ ] Error rate <1%
   - [ ] Quality maintained
   - [ ] Consensus working
   - [ ] Logs clean
2. Create `WEEK1_VALIDATION_REPORT.md`
   - Before/after comparison (50 RPS → 150 RPS)
   - Metrics table
   - Any issues found + mitigations
3. Document deployment checklist for next week
4. Create post-mortem (what went well, what was hard)

**Commit**: `docs: Week 1 validation report (150 RPS achieved)`

---

## POST-WEEK 1: GRADUAL REFACTORING BEGINS

### Week 2-3: Phase A (KernelBuilder)
- 3 instances keep running (no changes to ops setup)
- Create `api/builders/kernel.py` (~300 LOC)
- Tests: 100 new tests
- Deployment: Test on one instance first, then all 3
- No downtime required

### Weeks 4-5: Phase B (DependencyContainer)
- Larger change (state.py refactoring)
- Blue/green deployment: test on new code path
- Gradual rollout to instances

### Weeks 5-8: Phases C-G
- Continue gradual improvements
- Each phase validated before full deployment
- Keep 3 instances running throughout

**Key Principle**: No monolithic refactoring all at once. Incremental, validated.

---

## SUCCESS DEFINITION

### Week 1 SUCCESS (Friday 2026-02-28)
✅ All of these:
- [ ] 3 instances deployed and running
- [ ] nginx distributing load
- [ ] 150+ RPS sustained for 60 seconds
- [ ] Error rate <1%
- [ ] Quality (Q-Scores) maintained 78-80
- [ ] No data loss
- [ ] Consensus working (no duplicates)
- [ ] All logs to ~/.cynic/
- [ ] Deployment documented and repeatable

### Post-Week 1 SUCCESS (Week 2-8)
✅ All of these:
- [ ] Phase A (KernelBuilder) done and deployed
- [ ] Phase B (DependencyContainer) done and deployed
- [ ] All 7 phases incrementally completed
- [ ] No regressions (all 949 tests still passing)
- [ ] Architecture evolved from monolithic → distributed
- [ ] Code maintainable and scalable

---

## DECISION GATES

### Before Proceeding to Week 2 Refactoring
✅ **All of these must be true**:
1. 150 RPS stress test PASSED
2. Error rate <1% (confirmed)
3. Quality maintained (Q-Scores 78-80)
4. All 3 instances stayed healthy during stress
5. Consensus coordination working (verified)
6. Documentation complete
7. Deployment runbook tested (can repeat it)

### During Each Refactoring Phase
✅ **Before moving to next phase**:
1. Phase tests all passing (100% of new tests)
2. Regression tests all passing (100% of old tests)
3. Integration tests with 3 instances passing
4. Performance not degraded (latency same or better)
5. Quality not degraded (Q-Scores same or better)

---

## RISK MITIGATION

### Risk 1: SurrealDB bottleneck at 3 writers
- **Monitoring**: Track write latency in logs
- **Threshold**: If >100ms, add message queue
- **Decision**: Day 5 validation will reveal this

### Risk 2: nginx doesn't fairly distribute load
- **Monitoring**: Check nginx logs for distribution
- **Adjustment**: If unbalanced, switch to round-robin or ip_hash
- **Decision**: Day 3 validation will reveal this

### Risk 3: Consensus creates race conditions
- **Monitoring**: Check for duplicate judgments in logs
- **Threshold**: If >0.5% duplicates, strengthen locking
- **Decision**: Day 4 validation will reveal this

### Risk 4: Containers crash under load
- **Monitoring**: docker-compose ps after stress test
- **Threshold**: If any not HEALTHY, investigate logs
- **Decision**: Day 5 validation will reveal this

---

## SOLO DEV SEQUENCING RULES

**To avoid context overload while working solo**:

1. **One day = one concern**
   - Day 1-2: Docker only
   - Day 2-3: nginx only
   - Day 3-4: Consensus only
   - Day 4-5: Stress testing only
   - Day 5-6: Documentation only

2. **Don't start next task until previous task is committed**
   - Finish docker-compose.yml → COMMIT
   - Finish nginx.conf → COMMIT
   - Finish multi_instance.py → COMMIT
   - Finish stress test → COMMIT
   - Finish docs → COMMIT

3. **Test each task in isolation before committing**
   - Docker: verify each instance responds
   - nginx: verify load balancing
   - Consensus: verify no duplicates
   - Stress: verify metrics
   - Docs: verify completeness

4. **Slack on perfection; ship for validation**
   - First version of each component is OK
   - Optimization comes after validation
   - Move to next task if current task is "good enough"

---

## CONFIDENCE ASSESSMENT

**Overall: 61.8% (φ⁻¹)**

✅ **Strong**:
- Phase 4 validation proved 50 RPS works
- 3× scale = 150 RPS math is sound
- Horizontal scaling is proven pattern
- Each day is focused (low context switching)

⚠️ **Uncertain**:
- SurrealDB performance at 3 writers unknown
- nginx under load behavior untested
- Consensus logic is new (potential bugs)
- Some edge cases may appear at higher load

🎯 **Realistic**:
- Week 1 should achieve 100-150 RPS
- If bottleneck found, documented mitigation exists
- After validation, refactoring can proceed with confidence

---

## WHAT HAPPENS AFTER WEEK 1?

### If Week 1 Succeeds (150 RPS, <1% errors)
- ✅ Horizontal scaling works; continue with 3 instances
- ✅ Start Phase A refactoring (gradual, weeks 2-8)
- ✅ Each phase tested independently, deployed one at a time

### If Week 1 Hits Bottleneck (e.g., SurrealDB slowdown)
- 🔄 Root cause analysis
- 🔄 Implement mitigation (batching, caching, replicas)
- 🔄 Re-run stress test
- 🔄 Proceed when <1% error rate achieved

### If Week 1 Reveals Critical Bug
- 🔴 Pause refactoring start
- 🔴 Fix bug in monolithic codebase (easier than distributed)
- 🔴 Re-validate
- 🔴 Then proceed to refactoring

---

## YOUR ROLE THIS WEEK

**You are**:
- Product owner (decides when phase is complete)
- Developer (writes code)
- QA (validates each task)
- Operator (keeps 3 instances running)

**You are NOT**:
- Manager (this is solo dev; no coordination needed)
- Architect (architecture is decided; just execute)
- Researcher (path is clear; don't second-guess)

**Your job**: Execute the plan, measure the results, document findings.

---

## FINAL DIRECTION

```
TODAY (Session 9):
  ✅ Direction decided: Hybrid (ops + gradual refactor)
  ✅ Week 1 plan documented (day-by-day)
  ✅ Refactoring phases documented (phases A-G)

BEFORE MONDAY (2026-02-24):
  ⏳ Verify Docker works
  ⏳ Verify nginx available
  ⏳ Prepare stress test tool

WEEK 1 (2026-02-24 → 2026-02-28):
  ⏳ Deploy 3 instances + nginx
  ⏳ Reach 150+ RPS
  ⏳ Validate all metrics

WEEKS 2-8 (Starting 2026-03-03):
  ⏳ Refactoring phases A-G
  ⏳ Keep 3 instances running
  ⏳ Gradual architecture improvements

INFINITY:
  ✅ Scalable and maintainable 🚀
```

---

**Next Action**: Review both roadmap documents. Clarify any blockers.
**Ready?** 🐕

*sniff* Confidence: 61.8% (φ⁻¹ — path is clear, execution ready to begin)

