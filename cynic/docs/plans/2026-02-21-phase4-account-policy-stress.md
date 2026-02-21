# Phase 4: Account + Policy Endpoints + Stress Testing

> **For Next Session:** Execute with fresh subagent-driven-development

**Goal:** Extend Phase 3 read-only API with account tracking (balance/spend/learn) and policy queries, then stress-test the complete system.

**Current State (End of Session 2026-02-21):**
- Day 2 ✅: OrganismState (84 tests, Kani certified)
- Phase 3 ✅: 4 read-only endpoints (15 tests, observability proven)
- Ready for: Account endpoints, policy queries, stress testing

**Architecture:**
- Account endpoints: Query E-Score reputation system
- Policy endpoints: Query learned action patterns (Q-table analysis)
- Stress testing: Verify 100+ RPS single-instance capacity
- Integration: Full kernel startup with real organism

**Tech Stack:**
- FastAPI (existing)
- Pydantic (response models)
- pytest-httpx (async HTTP testing)
- pytest-benchmark (performance testing)
- OrganismState (from Day 2)

---

## Tasks for Phase 4

### Task 1: Create Account Response Models
- `AccountStatusResponse` (balance, spend, learn_rate, reputation)
- `EScoreResponse` (per-agent reputation scores)
- Frozen=True for immutability

### Task 2: Create GET /api/organism/account Endpoint
- Query E-Score system
- Return account balance + reputation
- Verify spending limits enforced

### Task 3: Create GET /api/organism/policy Endpoints
- Query learned Q-table patterns
- Return best actions per state
- Return policy statistics (coverage, confidence)

### Task 4: Add Stress Tests
- 100 concurrent requests to all endpoints
- Measure latency, throughput, errors
- Verify zero data corruption

### Task 5: Integration Tests
- Full kernel startup
- Real organism with 11 dogs
- Verify all endpoints return consistent reality

### Task 6: Load Profiling
- Profile memory usage
- Identify bottlenecks
- Document scaling limits

---

## Success Criteria

✅ Account endpoints working (balance, reputation)
✅ Policy endpoints working (learned patterns)
✅ Stress tests passing (100+ RPS)
✅ Integration tests passing (real organism)
✅ Documentation complete (API reference)
✅ Ready for production deployment

---

**Estimated Duration:** 2-3 hours (6 tasks × 20-30 min each)
**Test Count:** +30 tests (stress + integration)
**Total Session Tests:** 99 → 129+

---

**Branch:** `architecture/organism-v1-bootstrap` (continue)
**Base:** Phase 3 endpoints complete, ready to extend
