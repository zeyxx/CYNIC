# CYNIC Remediation Action Items

**Status:** Phase 1-2B complete. Remaining work to finish architectural hardening.

## Completed (5 commits, 23 files modified)

- [x] Phase 1: Async Safety (asyncio.run, bare except, race conditions)
- [x] Phase 2A: State Management (thread locks, singletons)
- [x] Phase 2B-1: Config System (consolidation, validation)

## Remaining High-Priority Fixes

### Phase 2B-2: Credential Migration (2 files)

Must move `os.getenv()` calls into config system:

1. **cynic/kernel/organism/brain/dialogue/llm_bridge.py:15**
   ```python
   # Current
   api_key = os.getenv("ANTHROPIC_API_KEY")

   # Should be
   from cynic.kernel.core.config import CynicConfig
   config = CynicConfig.from_env()
   api_key = config.anthropic_api_key
   ```
   **Task:** Update constructor to accept config parameter

2. **cynic/interfaces/mcp/__main__.py:69**
   ```python
   # Should migrate token read to config
   config = CynicConfig.from_env()
   token = config.mcp_token  # Add field if needed
   ```

**Rule Violated:** Rule 3 (No direct os.getenv outside config)

---

### Phase 2B-3: API Input Validation (2 endpoints)

Add Pydantic models for API safety:

1. **cynic/interfaces/api/routers/governance.py:83**
   ```python
   # Current: req: dict
   # Should be: req: RegisterCommunityRequest (Pydantic model)
   ```
   **Impact:** Prevent injection attacks

2. **cynic/interfaces/api/routers/empirical.py:65**
   ```python
   # Current: count: int = 1000
   # Should be: count: int = Query(default=1000, ge=1, le=100_000)
   ```
   **Impact:** Prevent resource exhaustion

**Rule Violated:** Not a core rule, but security best practice

---

### Phase 3A: Resource Lifecycle (6 files)

Register cleanup handlers for event listeners:

- [ ] cynic/kernel/organism/state_manager.py - add stop() method
- [ ] cynic/kernel/organism/brain/learning/loops.py - add stop() method
- [ ] cynic/kernel/organism/brain/cognition/cortex/residual.py - add stop() method
- [ ] cynic/kernel/organism/brain/cognition/cortex/account.py - add stop() method
- [ ] cynic/kernel/organism/perception/conduits.py - add stop() method for websocket listeners
- [ ] cynic/kernel/organism/factory.py - call stop() on all components during shutdown

**Rule Violated:** Rule 4 (Handlers must be unregistered)

---

### Phase 3B: Database Connection Management

- [ ] cynic/kernel/infrastructure/db_pool.py:134 - wrap get_connection in context manager
- [ ] Add tests for connection pool exhaustion scenarios

**Rule Violated:** Implicit (no lock protection on pool)

---

### Ongoing: CLI Code Cleanup (11 instances)

These files still use `asyncio.run()` in async contexts (low priority, CLI only):

1. cynic/kernel/organism/brain/dna/examples.py:230
2. cynic/kernel/organism/brain/cognition/cortex/fractal_cost_benchmark.py:362
3. cynic/interfaces/cli/dashboard.py:378
4. cynic/interfaces/cli/deploy.py:167
5-11. ... (6 more files)

**Note:** These are development/CLI tools, not prod. Lower priority than core fixes.

**Rule Violated:** Rule 1 (No asyncio.run in async)

---

## Phase 4+ Work (Future Sessions)

- **Phase 3B:** Input validation Pydantic models
- **Phase 4A:** Observability (Prometheus metrics, StructuredLogger wiring)
- **Phase 4B:** Type safety (replace top 10 `Any` usages)
- **Phase 5:** Architecture (fix kernel→interface imports)
- **Phase 6:** Integration tests (error injection, concurrent scenarios)

---

## Testing Strategy

After each fix:

1. Run pre-commit gates: `pytest tests/ -q --tb=no`
2. Check for pattern regressions: `grep -r "asyncio.run(" cynic/ | wc -l`
3. Validate credential isolation: `grep -r "os.getenv" cynic/ | grep -v config.py | wc -l`

---

## Documentation Updates

- [x] Create ARCHITECTURAL_RULES.md
- [ ] Update CLAUDE.md with rules enforcement section
- [ ] Add pre-commit hook implementation guide
- [ ] Document credential migration path

---

## Timeline Estimate

- **Phase 2B-2:** 30 min (credential migration)
- **Phase 2B-3:** 20 min (API validation)
- **Phase 3A:** 45 min (cleanup handlers)
- **Phase 3B:** 30 min (connection pooling)
- **CLI cleanup:** 45 min (deferred, lower priority)

**Total:** ~3 hours for core fixes (before Phase 4+)

---

**Last Updated:** 2026-03-02
