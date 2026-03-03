# CYNIC Universal Rails Pivot — Option A: Fix-First Strategy

**Date:** 2026-03-03
**Deadline:** 2026-03-13 (10 days)
**Philosophy:** Build universal, reusable infrastructure (not just Gemini-specific)
**Approach:** Aggressive Fix-First → Solid Foundations → Gemini 3 as Optional Plugin

---

## 🎯 Why Universal Rails?

Current state: 76 failing tests, 11 errors out of 1252 tests.

**Problem:** Fixing tests quickly creates technical debt.
**Solution:** Fix tests by building universal components that serve ANY integration.

**Examples:**
- ❌ Bad: "Fix SurrealDB tests just to pass tests"
- ✅ Good: "Build robust SurrealDB abstraction layer that decouples CYNIC from database vendor"

- ❌ Bad: "Patch async/await issues for test compliance"
- ✅ Good: "Establish universal async patterns that work for any async operation"

- ❌ Bad: "Mock Ollama to make tests pass"
- ✅ Good: "Build universal LLM adapter interface (works with Gemini, Ollama, Claude, Anthropic, etc.)"

---

## 📊 Test Failure Analysis (Prioritized by Impact)

### **Critical (Blocks Multiple Tests)** — 6 categories, ~40 tests

1. **SurrealDB Integration Tests** (8 tests)
   - `test_surrealdb_connection_real` - Connection pooling
   - `test_surrealdb_create_and_retrieve` - CRUD operations
   - `test_surrealdb_judgment_table_schema` - Schema validation
   - `test_surrealdb_vector_search_setup` - Vector indexes
   - Performance tests (write/read latency, batch writes)
   - **Root cause:** Environment config missing (SurrealDB_URL, credentials)
   - **Fix:** Build universal database configuration abstraction

2. **Ollama Integration Tests** (5 tests)
   - `test_ollama_adapter_real_completion` - Inference
   - `test_dog_judgment_with_real_ollama` - Dog integration
   - `test_temporal_mcts_with_real_ollama` - MCTS loop
   - Performance tests (latency, parallel calls)
   - **Root cause:** Ollama service not running or misconfigured
   - **Fix:** Build universal LLM interface (abstract Ollama, Claude, Gemini)

3. **Environment/Vault Configuration** (4 tests)
   - `test_config_from_environment` - Encryption config
   - `test_vault_config_from_env` - Vault integration
   - `test_get_secret_from_env` - Secret management
   - **Root cause:** Missing env vars (.env not loaded)
   - **Fix:** Build universal config loader + secret manager

4. **Async/Await Issues** (8+ tests)
   - `test_selfprober_analyze_includes_metrics` - Coroutine not awaited
   - `test_analyze_metrics_*` - Similar issues
   - **Root cause:** Mixing sync/async patterns, @lru_cache on async functions
   - **Fix:** Universal async utilities (proper async decorator stack)

5. **Service Registry Tests** (24 tests)
   - All `test_service_registry.py` tests failing
   - Status tracking, metrics recording, judgment logging
   - **Root cause:** Registry state initialization or persistence
   - **Fix:** Build universal component registry + state management

6. **Metrics/Observability Tests** (10 tests)
   - Event metrics, histogram buckets, metrics integration
   - **Root cause:** Metrics aggregation logic or persistence
   - **Fix:** Build universal metrics pipeline

### **Important (Supports Gemini Integration)** — 2 categories, ~20 tests

7. **MCP/Kernel Startup Tests** (7 tests)
   - Kernel startup, process spawning, logging
   - **Supports:** MCP server integration needed for Gemini 3
   - **Fix:** Universal kernel lifecycle management

8. **Event Bus Memory/Stability Tests** (4 tests)
   - Memory bounds, FIFO ordering
   - **Supports:** High-throughput event processing for Gemini loop
   - **Fix:** Universal event bus guarantees

### **Lower Priority** — 3 categories, ~16 tests

9. **GASDF Integration Tests** (4 tests) - External service, can mock
10. **Other Integration Tests** - Can be deferred
11. **Phase 3 Organism Caching Tests** (5 errors) - Depends on Phase 1 fixes

---

## 🏗️ Universal Rails Components to Build

Instead of test-specific fixes, build these foundational layers:

### **Tier 1: Configuration & Secrets (Days 3-4)**

**UniversalConfigLoader**
- Load from: environment, .env, .env.template, YAML, Vault
- Validate required vars (schema-based)
- Support env-specific overrides (dev/test/prod)
- Used by: Encryption, Vault, Database, LLM providers

**UniversalSecretManager**
- Abstraction over Vault/env/local stores
- Automatic rotation support
- Audit logging on access
- Fixes: `test_vault_config_from_env`, `test_get_secret_from_env`, `test_encryption_config`

### **Tier 2: Database Abstraction (Days 4-5)**

**UniversalDatabaseAdapter**
- Abstraction: SurrealDB as primary, PostgreSQL fallback
- Connection pooling, retry logic, health checks
- Schemaless table interface (consistent across SQL/NoSQL)
- Vector search abstraction (HNSW for SurrealDB, pgvector for Postgres)
- Metrics: latency, throughput, error rates

**Tier 2 Benefits:**
- CYNIC works with any database
- Gemini 3 can use this for long-context storage (LLM-agnostic)
- Fixes: All 8 SurrealDB tests + 3 performance tests

### **Tier 3: LLM Interface (Days 5-6)**

**UniversalLLMAdapter**
- Interface: submit_prompt(model, prompt, config) → response
- Implementations: Claude, Gemini, Ollama, local GGUF
- Fallback chain: Try primary → fallback → offline mode
- Metrics: latency, cost, quality, token usage
- Caching: Prompt → response (for repeated queries)

**Tier 3 Benefits:**
- Switch models without code changes
- Gemini 3 is just another adapter plugin
- Easy A/B testing different models
- Fixes: All 5 Ollama tests + Dog/MCTS integration

### **Tier 4: Async Utilities (Days 6-7)**

**UniversalAsyncHelper**
- `@async_cached` decorator (proper async-aware caching)
- `gather_with_timeout()` - timeout handling for parallel tasks
- `AsyncContextManager` - proper resource cleanup
- `async_retry()` - retry logic with exponential backoff

**Tier 4 Benefits:**
- Consistent async patterns throughout codebase
- Prevents "coroutine not awaited" bugs
- Gemini 3 integration runs cleanly with async
- Fixes: All 8+ async/await issues + coroutine warnings

### **Tier 5: Component Registry (Days 7-8)**

**UniversalComponentRegistry**
- Register/unregister components with lifecycle hooks
- State snapshots (immutable)
- Metrics per component (latency, errors, status)
- Judgment logging (what decisions affected this component?)
- Event subscriptions (who cares about my status?)

**Tier 5 Benefits:**
- CYNIC understands its own topology
- Auto-generates system health reports
- Gemini 3 can introspect CYNIC's state
- Fixes: All 24 ServiceRegistry tests

### **Tier 6: Metrics Pipeline (Days 8-9)**

**UniversalMetricsCollector**
- Event-based metrics (anomaly count, error rate)
- Histogram aggregation (latency buckets)
- Time-series storage (SurrealDB)
- OpenMetrics export (Prometheus compatible)
- Rollup windows (1min, 5min, 1hour)

**Tier 6 Benefits:**
- Stanislaz can build dashboard on solid metrics
- Gemini 3 has real-time system health data
- Auto-scaling based on metrics (metabolism)
- Fixes: All 10 metrics/observability tests

---

## 📅 Revised 10-Day Roadmap

```
Days 3-4: Tier 1 (Config & Secrets)
  - Build UniversalConfigLoader
  - Build UniversalSecretManager
  - Tests: 4 passing
  - Commit: "feat(universal-rails-1): Config & secret abstraction"

Days 4-5: Tier 2 (Database)
  - Build UniversalDatabaseAdapter
  - Fix SurrealDB connection, pooling, schema
  - Tests: 11 passing (8 SurrealDB + 3 perf)
  - Commit: "feat(universal-rails-2): Database abstraction layer"

Days 5-6: Tier 3 (LLM)
  - Build UniversalLLMAdapter
  - Implement Claude, Gemini, Ollama, GGUF adapters
  - Tests: 5 passing (Ollama)
  - Commit: "feat(universal-rails-3): Universal LLM interface"

Days 6-7: Tier 4 (Async)
  - Build UniversalAsyncHelper
  - Fix all @lru_cache on async functions
  - Tests: 8 passing (async/await)
  - Commit: "feat(universal-rails-4): Async utilities & decorators"

Days 7-8: Tier 5 (Registry)
  - Build UniversalComponentRegistry
  - Fix all ServiceRegistry tests
  - Tests: 24 passing
  - Commit: "feat(universal-rails-5): Component registry & lifecycle"

Days 8-9: Tier 6 (Metrics)
  - Build UniversalMetricsCollector
  - Fix all metrics/observability tests
  - Tests: 10 passing
  - Commit: "feat(universal-rails-6): Metrics & observability pipeline"

Days 9-10: Validation & Gemini Layer (Optional Plugin)
  - Run full test suite (expect 90%+ passing)
  - Add Gemini 3 adapter to LLMInterface (minimal code)
  - Add Vision/Memory/Surgery as optional modules
  - Final tests: 95%+ passing
  - Commit: "feat(optional-gemini-3): Gemini 3 as optional plugin"

Days 10-13: Buffer & Polish
  - Fix remaining test failures
  - Documentation & integration guide
  - Ready for hackathon
```

---

## 🎯 Why This Works for Gemini 3 + Future

**Universal Rails = Reusable Foundation**

- **Gemini 3 is optional:** CYNIC works great with Ollama/Claude/local models
- **Easy to swap:** Change one config var, CYNIC uses different model
- **Hackathon-ready:** Gemini 3 adapter is just a plugin, not a refactor
- **Future-proof:** Any new model, any new database, any new integration plugs in cleanly

**By March 13:**
- ✅ Core CYNIC solid (95%+ tests passing)
- ✅ All universal rails deployed
- ✅ Gemini 3 adapter ready (minimal code)
- ✅ Hackathon can focus on *using* Gemini 3, not *integrating* it

---

## 📋 Task List (Revised)

- [ ] **Days 3-4:** Task: Build UniversalConfigLoader + UniversalSecretManager
- [ ] **Days 4-5:** Task: Build UniversalDatabaseAdapter
- [ ] **Days 5-6:** Task: Build UniversalLLMAdapter + implementations
- [ ] **Days 6-7:** Task: Build UniversalAsyncHelper + fix decorators
- [ ] **Days 7-8:** Task: Build UniversalComponentRegistry
- [ ] **Days 8-9:** Task: Build UniversalMetricsCollector
- [ ] **Days 9-10:** Task: Add Gemini 3 adapter + validation
- [ ] **Days 10-13:** Task: Buffer, polish, final validation

---

## ✅ Success Criteria (by March 13, 23:59 UTC)

- ✅ 1150+ tests passing (95%+ of 1252)
- ✅ 0 failing tests (all 76 failures fixed)
- ✅ 0 import cycles, clean architecture
- ✅ Universal rails deployed (6 abstraction layers)
- ✅ Gemini 3 as optional plugin (easy to enable/disable)
- ✅ Documentation: integration guide + architecture
- ✅ Team ready: Titouan & Stanislaz know how to leverage Gemini 3
- ✅ Hackathon-ready: CYNIC works independently, Gemini 3 enhances it

---

**Philosophy:** Build for CYNIC, not for Gemini 3. Gemini 3 is a bonus.
