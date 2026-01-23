# CYNIC SOLID Refactoring Plan

> "φ distrusts φ" - Refactoring with discipline

## Progress Overview

| Principle | Target | Status | Priority |
|-----------|--------|--------|----------|
| **ISP** | persistence.js | ✅ DONE (1437→444 lines) | - |
| **OCP** | tools/index.js | 🔄 Structure ready | P1 |
| **SRP** | server.js | ✅ HttpAdapter (1706→1453) | - |
| **DIP** | MCPServer._initialize() | ✅ ServiceInitializer (1453→1339) | - |
| **SRP** | MetricsService | ⏳ Pending | P3 |
| **LSP** | Repository signatures | ⏳ Pending | P3 |

---

## Phase 1: Complete Server Migration (SRP + DIP)

### 1.1 Integrate HttpAdapter into MCPServer

**Goal:** MCPServer delegates all HTTP concerns to HttpAdapter

**Changes:**
```javascript
// BEFORE (in server.js)
this._httpServer = createServer((req, res) => this._handleHttpRequest(req, res));

// AFTER
this._httpAdapter = new HttpAdapter({
  port: this.port,
  dashboardPath: join(__dirname, 'dashboard'),
  auth: this.auth
});
this._httpAdapter.setRoute('mcp', (req, res) => this._handleMcpRequest(req, res));
this._httpAdapter.setRoute('api', (req, res, url) => this._handleApiRequest(req, res, url));
await this._httpAdapter.start();
```

**Files to modify:**
- `packages/mcp/src/server.js` - Use HttpAdapter
- Remove: `_startHttpServer()`, `_handleHttpRequest()`, `_setCorsHeaders()`, `_handleSSE()`

**Estimated reduction:** ~300 lines from server.js

---

### 1.2 Integrate ServiceInitializer into MCPServer

**Goal:** MCPServer uses ServiceInitializer instead of direct `new` calls

**Changes:**
```javascript
// BEFORE (in _initialize)
if (!this.persistence) {
  this.persistence = new PersistenceManager({ dataDir: this.dataDir });
  await this.persistence.initialize();
}
if (!this.sessionManager) {
  this.sessionManager = new SessionManager(this.persistence);
}
// ... 10+ more direct instantiations

// AFTER
const initializer = new ServiceInitializer({
  dataDir: this.dataDir,
  onBlockCreated: (block) => this._broadcastSSEEvent('block', block),
  onJudgment: (j) => this._broadcastSSEEvent('judgment', j),
});

const services = await initializer.initialize({
  judge: this.judge,        // Use provided if available
  persistence: this.persistence,
});

// Assign to this
Object.assign(this, services);
```

**Files to modify:**
- `packages/mcp/src/server.js` - Use ServiceInitializer in `_initialize()`

**Estimated reduction:** ~150 lines from server.js

---

## Phase 2: Complete Tools Migration (OCP)

### 2.1 Move Tool Implementations to Domain Files

**Current state:**
- `tools/index.js` = 5004 lines with all tool functions
- `tools/domains/*.js` = Factories that import from index.js

**Target state:**
- `tools/index.js` = ~500 lines (exports + createAllTools)
- `tools/domains/*.js` = Full implementations

**Migration order (by dependency):**

1. **judgment.js** (~500 lines)
   - Move: `createJudgeTool`, `createRefineTool`, `createFeedbackTool`, `createLearningTool`
   - Dependencies: enrichItem, PHI_INV, THRESHOLDS

2. **knowledge.js** (~300 lines)
   - Move: `createSearchTool`, `createDigestTool`, `createDocsTool`
   - Dependencies: persistence

3. **blockchain.js** (~400 lines)
   - Move: `createPoJChainTool`, `createTraceTool`
   - Dependencies: pojChainManager, persistence

4. **consciousness.js** (~600 lines)
   - Move: `createEmergenceTool`, `createSelfModTool`, `createMilestoneHistoryTool`, `createPatternsTool`
   - Dependencies: judge, persistence, EMERGENCE constants

5. **session.js** (~400 lines)
   - Move: `createSessionStartTool`, `createSessionEndTool`, `createProfileSyncTool`, `createProfileLoadTool`, `createPsychologyTool`
   - Dependencies: sessionManager, persistence

6. **ecosystem.js** (~500 lines)
   - Move: `createEcosystemTool`, `createEcosystemMonitorTool`, `createIntegratorTool`, `createDiscoveryTool`
   - Dependencies: ecosystem, integrator, discovery

7. **system.js** (~400 lines)
   - Move: `createHealthTool`, `createMetricsTool`, `createCollectiveStatusTool`, `createAgentsStatusTool`, `createAgentDiagnosticTool`
   - Dependencies: node, judge, metrics, collective

8. **automation.js** (~400 lines)
   - Move: `createTriggersTool`, `createOrchestrationTool`
   - Dependencies: persistence, collective, scheduler

9. **code.js** (~300 lines)
   - Move: `createCodebaseTool`, `createVectorSearchTool`
   - Dependencies: codebaseOptions

**After migration:**
- `tools/index.js` becomes thin re-export + `createAllTools` using registry
- New tools added via registry pattern (OCP)

---

## Phase 3: Repository Consistency (LSP)

### 3.1 Standardize Repository Interfaces

**Problem:** Inconsistent method signatures across repositories

```javascript
// JudgmentRepository
async search(query, options = {})

// DiscoveryRepository
async search(repoOwner, repoName, options = {})  // Different signature!

// PsychologyRepository
async search()  // No parameters!
```

**Solution:** Define base interface, adapt implementations

```javascript
// packages/persistence/src/interfaces/ISearchable.js
export interface ISearchable {
  search(query: string, options?: SearchOptions): Promise<any[]>;
}

// DiscoveryRepository adapts
async search(query, options = {}) {
  // Parse query for repo info
  const { repoOwner, repoName } = this._parseQuery(query);
  return this._searchInternal(repoOwner, repoName, options);
}
```

**Files to create:**
- `packages/persistence/src/interfaces/ISearchable.js`
- `packages/persistence/src/interfaces/IRepository.js`

**Files to modify:**
- All repository files to implement consistent interface

---

## Phase 4: Additional SRP Extractions

### 4.1 MetricsService (757 lines → ~300 lines)

**Current responsibilities:**
1. Collect metrics from 8 sources
2. Generate Prometheus format
3. Generate HTML reports
4. Manage alerts
5. EventEmitter inheritance

**Extract:**
- `MetricsCollector` - Collects from all sources
- `PrometheusFormatter` - Formats for Prometheus
- `MetricsReporter` - HTML reports
- `AlertManager` - Alert handling

### 4.2 TriggerRepository (507 lines)

**Current responsibilities:**
1. Trigger CRUD
2. Execution history
3. Event storage
4. Rate limiting

**Extract:**
- `TriggerStore` - CRUD operations
- `ExecutionHistory` - History tracking
- `TriggerEvents` - Event storage
- `RateLimiter` - Rate limiting logic

---

## Execution Order

```
Week 1: Phase 1 (Server)
├── Day 1-2: Integrate HttpAdapter
├── Day 3-4: Integrate ServiceInitializer
└── Day 5: Test & verify

Week 2: Phase 2 (Tools) - Part 1
├── Day 1: judgment.js
├── Day 2: knowledge.js, blockchain.js
├── Day 3: consciousness.js
├── Day 4: session.js
└── Day 5: Test & verify

Week 3: Phase 2 (Tools) - Part 2
├── Day 1: ecosystem.js
├── Day 2: system.js
├── Day 3: automation.js, code.js
├── Day 4: Update createAllTools to use registry
└── Day 5: Test & verify

Week 4: Phase 3 & 4
├── Day 1-2: LSP Repository interfaces
├── Day 3-4: MetricsService extraction
└── Day 5: Final testing
```

---

## Success Metrics

| Metric | Before | Current | Target |
|--------|--------|---------|--------|
| server.js lines | 1706 | 1339 ✅ | < 800 |
| tools/index.js lines | 5004 | 5004 | < 500 |
| persistence.js lines | 1437 | 444 ✅ | < 500 |
| Max file size | 5004 | 5004 | < 500 |
| Direct `new` in MCPServer | 10+ | 3 ✅ | 0 |
| Test coverage | ? | ? | > 80% |

---

## Notes

- Each phase should be committed separately
- Test on production after each phase
- Backward compatibility required (legacy API works)
- No breaking changes to MCP protocol

*"φ distrusts φ" - Max confidence 61.8%*
