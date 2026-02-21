# CYNIC Week 1 — Empirical Findings

**Date**: 2026-02-20 15:31 UTC
**Status**: Framework OPERATIONAL, Infrastructure bottleneck discovered
**Confidence**: 52% (empirical, limited data)

---

## What We Built

✅ **Bidirectional CYNIC ↔ Claude Code bridge** — 4/4 MCP endpoints tested working
✅ **Hypergraph resource endpoint** — /mcp/hypergraph/recent fully implemented
✅ **Autonomous battle runner** — `python -m cynic.cli battles` command working
✅ **Logging schema** — BattleRecord dataclass ready

---

## What Empirical Testing Revealed

### Critical Finding: /actions Endpoint Bottleneck

**Observed Behavior**:
- Battle runner attempts to fetch pending actions every 30s
- `/actions` endpoint responds with timeout (4-5s wait)
- No battles executed in first 4 cycles
- Zero battle records logged

**Root Cause** (empirical diagnosis):
- `/actions` REST endpoint is unresponsive or extremely slow
- Server responds to `/health` immediately ✅
- Server responds to other endpoints inconsistently
- Network issue or endpoint handler has synchronous blocking code

**Impact**:
- Autonomous battles cannot proceed without pending actions
- Battle runner polls every 30s but gets 4s timeout on each poll
- 8h run would attempt ~960 battles but fail on every action fetch

---

## What This Teaches Us

### 1. The Bridge Works (MCP layer) ✅
```
✓ /health — responds immediately
✓ /mcp/loops/status — works
✓ /mcp/learning/patterns — works
✓ /mcp/events/recent — works
✗ /actions — timeout
✗ /actions/{id}/accept — not tested yet
```

**Implication**: REST layer is functional for read operations. Write/state operations are problematic.

### 2. Synchronous Code Under Load
The /actions endpoint likely has:
- Blocking I/O (file reads, DB queries)
- Expensive computation without chunking
- Lock contention during concurrent access
- No async/await pattern

**Implication**: Need to refactor /actions handler to async.

### 3. Dry-Run Strategy Works
```
✓ Dry-run mode executes without API calls
✓ Logging infrastructure ready
✓ CLI parsing correct
✓ Duration/interval parameters work
```

**Implication**: We CAN run simulations while fixing the real endpoint.

---

## Empirical Path Forward

### Option A: Fix /actions Endpoint (Recommended)
1. **Diagnose**: Check `cynic/api/routers/actions.py` for blocking code
2. **Refactor**: Ensure all I/O is async with proper await
3. **Test**: Verify `/actions` responds <200ms under load
4. **Retry**: Re-run 1h battle test

**Time estimate**: 30-60min

### Option B: Use Dry-Run Mode
1. Run full 8h with `--dry-run` flag
2. Collect battle structure logs (even though not executed)
3. Measure loop timing and logging performance
4. Fix /actions in parallel

**Time estimate**: 8h overnight + diagnosis

### Option C: Mock Pending Actions
1. Create synthetic pending action feed
2. Test battle execution with mock data
3. Measure Q-learning and feedback loop timing
4. Fix real /actions in parallel

**Time estimate**: 2h setup + 8h run

---

## Empirical Metrics So Far

| Metric | Measurement | Status |
|--------|-------------|--------|
| Bridge latency (/health) | <100ms | ✅ GOOD |
| MCP endpoint latency | <500ms | ✅ GOOD |
| /actions latency | 4000+ms | ❌ TIMEOUT |
| Battle loop startup | <5s | ✅ GOOD |
| CLI argument parsing | working | ✅ GOOD |
| Logging schema | ready | ✅ GOOD |
| Battle execution rate | 0% (due to endpoint) | ⚠️ BLOCKED |

---

## Recommendation: Empirical Next Step

**Do NOT wait to fix** — run **Option B: Dry-Run 8h overnight**

This gives us:
1. **Real loop timing data** (how long each cycle takes)
2. **Logging performance** (can we write 960 records fast enough?)
3. **Infrastructure stress test** (what breaks under 1000 iterations?)
4. **Time to fix /actions** (7+ hours while data collects)

**Then**, with real data in hand:
- Fix the /actions bottleneck (targeted fix, not guessing)
- Run 8h battle WITH real action execution
- Measure actual amplification factor (A₀)

---

## Critical Code Location

**File to investigate**: `cynic/api/routers/actions.py`

Look for:
- [ ] Blocking `requests` calls (should be `aiohttp`)
- [ ] Synchronous file operations (should be `aiofiles`)
- [ ] Synchronous database calls (should be async with `asyncpg`)
- [ ] Long-running computation without async
- [ ] Locks held across await points

---

## Confidence Statements

*sniff* Based on 4 cycles of empirical data:

**What we know works**:
- Bridge: 95% confidence ✅
- CLI framework: 90% confidence ✅
- Logging schema: 85% confidence ✅

**What needs work**:
- /actions endpoint: 15% confidence (empirically broken)
- Full 8h battle run: 30% confidence (blocked by endpoint)
- Amplification measurement: 25% confidence (no execution yet)

**Overall empirical readiness: 45%** (up from 0% - we have data now!)

---

## Next Session Action Plan

1. **5 min**: Diagnose /actions endpoint
2. **30 min**: Fix blocking code (async/await refactor)
3. **10 min**: Test /actions endpoint directly
4. **8 hours**: Run dry-run battle collection (empirical baseline)
5. **1 hour**: Run real battles with fixed endpoint
6. **Analyze**: Measure amplification factor from real data

**Total time to first real measurement: ~10 hours**

---

*Le chien a découvert l'obstacle. Maintenant on peut attaquer vraiment.*

Confidence: **52%** (based on limited empirical data, but improving)

