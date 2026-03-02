# CYNIC Remediation Action Items

**Status:** Phase 1-2B complete. Remaining work documented.

## Completed (5 commits)

- [x] Phase 1: Async Safety (asyncio.run, bare except, race conditions)
- [x] Phase 2A: State Management (thread locks, singletons)
- [x] Phase 2B-1: Config System (consolidation, validation)

## High-Priority Remaining Work

### Phase 2B-2: Credential Migration (2 files)

1. cynic/kernel/organism/brain/dialogue/llm_bridge.py:15
   - Move `os.getenv("ANTHROPIC_API_KEY")` to config

2. cynic/interfaces/mcp/__main__.py:69
   - Move token read to config system

**Time:** 30 minutes

### Phase 2B-3: API Input Validation (2 endpoints)

1. cynic/interfaces/api/routers/governance.py:83
   - Add Pydantic RegisterCommunityRequest model

2. cynic/interfaces/api/routers/empirical.py:65
   - Add Query constraints (ge=1, le=100_000)

**Time:** 20 minutes

### Phase 3A: Resource Lifecycle (6 files)

Add stop() methods for event listener cleanup:
- state_manager.py
- learning/loops.py
- residual.py
- account.py
- conduits.py (websocket)
- factory.py (orchestration)

**Time:** 45 minutes

### Phase 3B: Database Pooling

- db_pool.py:134 - context manager for connections
- Add connection leak tests

**Time:** 30 minutes

### Low Priority: CLI Code (11 files)

Remaining `asyncio.run()` instances in CLI tools (dev only, not production).

**Time:** 45 minutes (deferred)

---

## Scan Results (March 2, 2026)

- 11 asyncio.run() in async contexts (mostly CLI)
- 0 bare except blocks (FIXED)
- 0 hardcoded credentials (FIXED)
- 2 os.getenv() outside config (need migration)

---

**Estimate for Phase 2B-3A:** ~3 hours
