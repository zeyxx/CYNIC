# BATCH 4: PERCEIVE — Wire PerceptionRouter + Multi-Dimensional Awareness

## Date: 2026-02-04
## Status: PLANNED (implement in next session)
## Depends on: BATCH 1-3 (BURN + JUDGE + STEAL) — COMPLETED

---

## Context

### Meta-Analysis Results (this session)

CYNIC's awareness is **38.2% (phi-2)** — half blind. The fractal analysis reveals:

```
LEVEL              PHI     VERIFY   CULTURE   BURN     SCORE
1. Tool Call       ok      ok       ok        ok       92%
2. Session         ok      ok       ok        ok       85%
7. Meta/Self       ok      ok       ok        partial  75%
3. Ecosystem       ok      partial  partial   partial  40%
6. Blockchain      partial MISSING  partial   MISSING  10%
4. Infrastructure  MISSING MISSING  MISSING   MISSING   0%
5. Social/Team     MISSING MISSING  MISSING   MISSING   0%
```

The same perception-judgment-memory cycle that works at levels 1-2 (code/session)
does NOT exist for infrastructure, blockchain, or ecosystem monitoring.

### Key Finding: PerceptionRouter is Created but Never Consumed

- `packages/mcp/src/server.js` creates PerceptionRouter singleton
- Registers all 90 MCP tools with it
- **Never passes it to UnifiedOrchestrator**
- **Never consulted during tool selection or decision-making**
- Only 10 of 90 tools have explicit routing patterns
- No feedback loop (success/failure/latency tracking)

### Research (2025-2026 State of the Art)

- **EverMemOS** (2026): 4-layer brain architecture maps to Sefirot
- **Global Workspace Theory**: perception-broadcast-action cycle
- **Active Perception**: GAP in ecosystem — nobody has implemented autonomous MCP polling
- **MCP Nov 2025 spec**: Resource subscriptions exist but poorly adopted
- **Decision**: Active Perception Loop = separate research plan (after this batch)

---

## BATCH 4A: BURN solana-agent

### What
Remove `solana-agent` from `.mcp.json`. It's a dead MCP (0 code references) and a
functional duplicate of `solana-dev` (which IS referenced via PerceptionRouter).

### Files
- `.mcp.json` — Remove the `solana-agent` entry

### Verification
```bash
grep -r "solana-agent\|solana-mcp\|DEPLOY_TOKEN\|MINT_NFT" packages/ scripts/ --include="*.js" | grep -v node_modules
# Should return empty (0 references)
```

---

## BATCH 4B: Wire PerceptionRouter to UnifiedOrchestrator

### What
Pass the PerceptionRouter singleton to UnifiedOrchestrator so the orchestration
pipeline can consult it for data source routing decisions.

### Files to Modify

**1. `packages/node/src/orchestration/unified-orchestrator.js`**

In constructor, add:
```javascript
this.perceptionRouter = options.perceptionRouter || null;
```

Add method:
```javascript
/**
 * Consult PerceptionRouter for optimal data source
 * @param {string} target - URL, path, or keyword
 * @param {Object} options - { intent, preferStructured, preferFast }
 * @returns {Object|null} Routing decision
 */
_requestPerception(target, options = {}) {
  if (!this.perceptionRouter) return null;
  try {
    return this.perceptionRouter.route({
      target,
      intent: options.intent || 'read',
      preferStructured: options.preferStructured !== false,
      preferFast: options.preferFast || false,
    });
  } catch {
    return null;
  }
}
```

**2. `packages/mcp/src/server.js`**

In `_initialize()`, pass perceptionRouter to UnifiedOrchestrator:
```javascript
// Where UnifiedOrchestrator is created, add:
perceptionRouter: this.perceptionRouter,
```

**3. `packages/node/src/collective-singleton.js`**

Add perceptionRouter to the singleton options so the collective knows about it:
```javascript
// In getCollectivePack() or getOrchestrator()
perceptionRouter: options.perceptionRouter || null,
```

### Verification
```bash
# Check PerceptionRouter is passed through the chain
grep -n "perceptionRouter" packages/node/src/orchestration/unified-orchestrator.js
grep -n "perceptionRouter" packages/mcp/src/server.js
grep -n "perceptionRouter" packages/node/src/collective-singleton.js
```

---

## BATCH 4C: Add Missing MCP Routes to PerceptionRouter

### What
The PerceptionRouter only has 3 MCP route groups (Solana, GitHub, Render) covering
~10 tools. Add routes for the other MCPs so they're discoverable.

### Files to Modify

**`packages/llm/src/perception-router.js`**

Add to MCP_ROUTES:
```javascript
// Context7 (Documentation)
{
  pattern: /docs?|documentation|library|api\s*ref|reference/i,
  tools: [
    'mcp__plugin_context7_context7__resolve-library-id',
    'mcp__plugin_context7_context7__query-docs',
  ],
},

// Oracle (Token Scoring) — new from BATCH 2
{
  pattern: /token.*scor|oracle|mint.*judge|verdict.*token/i,
  tools: [
    'brain_oracle_score',
    'brain_oracle_watchlist',
    'brain_oracle_stats',
  ],
},

// X/Twitter (Social Intelligence)
{
  pattern: /tweet|twitter|x\.com|social.*media|trending/i,
  tools: [
    'brain_x_feed',
    'brain_x_search',
    'brain_x_analyze',
    'brain_x_trends',
  ],
},

// Extended GitHub (PRs, Issues, Code Search)
{
  pattern: /pull.*request|pr\s*#|issue\s*#|github.*search|code.*search/i,
  tools: [
    'mcp__github__get_pull_request',
    'mcp__github__list_pull_requests',
    'mcp__github__get_issue',
    'mcp__github__list_issues',
    'mcp__github__search_code',
    'mcp__github__get_pull_request_files',
    'mcp__github__get_pull_request_status',
  ],
},

// Extended Render (Deploys, Logs, Metrics)
{
  pattern: /deploy|render.*log|service.*status|infra.*health/i,
  tools: [
    'mcp__render__list_deploys',
    'mcp__render__get_deploy',
    'mcp__render__list_logs',
    'mcp__render__get_metrics',
    'mcp__render__get_service',
  ],
},

// CYNIC Memory & Knowledge
{
  pattern: /remember|recall|past.*session|history|memory.*search/i,
  tools: [
    'brain_memory_search',
    'brain_search',
    'brain_patterns',
  ],
},

// CYNIC Judgment
{
  pattern: /judge|evaluate|assess|score|verdict/i,
  tools: [
    'brain_cynic_judge',
    'brain_cynic_refine',
    'brain_consensus',
  ],
},
```

### Add Feedback Tracking

Add to PerceptionRouter class:
```javascript
/**
 * Record routing outcome for learning
 * @param {string} layer - api|mcp|browser|filesystem
 * @param {string} tool - Tool name used
 * @param {boolean} success - Did the route succeed?
 * @param {number} latency - Execution time in ms
 */
recordOutcome(layer, tool, success, latency) {
  if (!this._outcomes) this._outcomes = [];
  this._outcomes.push({
    layer, tool, success, latency,
    ts: Date.now(),
  });
  // Keep last 100 outcomes
  if (this._outcomes.length > 100) this._outcomes.shift();

  // Update stats
  if (!this._stats.byTool) this._stats.byTool = {};
  if (!this._stats.byTool[tool]) {
    this._stats.byTool[tool] = { success: 0, failure: 0, avgLatency: 0 };
  }
  const s = this._stats.byTool[tool];
  if (success) s.success++; else s.failure++;
  s.avgLatency = (s.avgLatency * (s.success + s.failure - 1) + latency) / (s.success + s.failure);
}
```

### Verification
```bash
# Count routes before and after
grep -c "pattern:" packages/llm/src/perception-router.js
# Before: ~5, After: ~12

# Test that route() returns tools for new patterns
node -e "
  import { getPerceptionRouter } from '@cynic/llm';
  const r = getPerceptionRouter();
  console.log(r.route({ target: 'documentation for React', intent: 'read' }));
  console.log(r.route({ target: 'twitter trends', intent: 'read' }));
  console.log(r.route({ target: 'deploy status on render', intent: 'read' }));
"
```

---

## BATCH 4D: Smart Conditional Infra Scan at SessionStart

### What
At SessionStart, check infrastructure health ONLY if the previous session had errors
or if the last deploy failed. Zero-cost when everything is fine.

### Files to Modify

**`scripts/hooks/awaken.js`**

Add after PHASE 4 (MEMORY_MOUNT), new section:

```javascript
// =========================================================================
// PHASE 4.5: MULTI-DIMENSIONAL AWARENESS (Smart Conditional)
// "Le chien surveille la meute" - Check infra only when needed
// =========================================================================

// Condition: Only check if previous session had issues
const previousHadErrors = lastSessionData?.handoff?.unresolvedErrors?.length > 0;
const previousHadInfraErrors = (lastSessionData?.handoff?.summary || '').includes('ECONNREFUSED') ||
                                (lastSessionData?.handoff?.summary || '').includes('timeout');

if (previousHadErrors || previousHadInfraErrors) {
  const awareness = {};

  // A. Render Deploy Status (if Render MCP available)
  try {
    const renderResult = await Promise.race([
      callBrainTool('brain_health', {}),
      new Promise(resolve => setTimeout(() => resolve(null), 3000)),
    ]);
    if (renderResult) {
      awareness.infrastructure = {
        status: renderResult.status || 'unknown',
        services: renderResult.services?.length || 0,
        uptime: renderResult.uptime,
      };
    }
  } catch { /* infra check is optional */ }

  // B. Database Health (quick connection test)
  try {
    const { getPool } = await import('@cynic/persistence');
    const pool = getPool();
    if (pool) {
      const dbResult = await Promise.race([
        pool.query('SELECT 1 as ok, now() as ts'),
        new Promise(resolve => setTimeout(() => resolve(null), 2000)),
      ]);
      awareness.database = {
        connected: !!dbResult?.rows?.[0]?.ok,
        timestamp: dbResult?.rows?.[0]?.ts,
      };
    }
  } catch {
    awareness.database = { connected: false, error: 'connection_failed' };
  }

  // C. Solana Balance (if HELIUS_API_KEY available)
  // Only if Oracle is configured (from BATCH 2)
  if (process.env.HELIUS_API_KEY) {
    try {
      const balanceResult = await Promise.race([
        callBrainTool('brain_health', { subsystem: 'solana' }),
        new Promise(resolve => setTimeout(() => resolve(null), 3000)),
      ]);
      if (balanceResult?.solana) {
        awareness.blockchain = {
          cluster: balanceResult.solana.cluster || 'devnet',
          connected: balanceResult.solana.connected || false,
        };
      }
    } catch { /* blockchain check optional */ }
  }

  if (Object.keys(awareness).length > 0) {
    contextInjections.push({
      type: 'awareness',
      title: 'Multi-Dimensional Awareness (triggered by previous session errors)',
      content: JSON.stringify(awareness, null, 2),
      dimensions: Object.keys(awareness),
    });
  }
}
```

### Verification
```bash
# Simulate: Create a last-session.json with unresolvedErrors
echo '{"sessionEndTime": 1234, "handoff": {"unresolvedErrors": [{"tool":"Bash"}]}}' > ~/.cynic/last-session.json
# Next session start should trigger infra check
# Check output contains awareness section
```

---

## BATCH 4E: Rendre les External MCPs Routables

### What
Ensure all external MCPs (Render, GitHub, Playwright, Context7, Solana-dev) have
proper routing patterns in PerceptionRouter. This is part of BATCH 4C but listed
separately for clarity.

**Covered by BATCH 4C above.** No additional work needed.

---

## Sequencing

```
BATCH 4A (5 min)  → Burn solana-agent from .mcp.json
BATCH 4B (30 min) → Wire PerceptionRouter to Orchestrator
BATCH 4C (30 min) → Add 7+ new MCP route groups + feedback tracking
BATCH 4D (20 min) → Smart conditional infra scan in awaken.js
─────────────────
Total: 4 concrete changes, ~10 files modified
```

---

## Post-Implementation: Research Plan Queue

After BATCH 4, these topics need a dedicated research session:

1. **Active Perception Loop (GWT-style)**
   - Autonomous polling cycle for all MCPs
   - Attention mechanism (which signal gets broadcast?)
   - Integration with Dogs as specialized modules
   - Reference: EverMemOS 4-layer architecture, LIDA cognitive cycle

2. **Open Source Model Training**
   - Export CYNIC judgment data (Q-scores, dimensions, verdicts) as training set
   - Fine-tune small model (Phi-3, Llama-3, Mistral) on CYNIC judgment patterns
   - Goal: Local model that can judge independently (reduce API dependency)
   - Reference: LangMem SDK memory extraction patterns

3. **Real-Time Infrastructure Monitoring**
   - WebSocket/SSE connection to Render deploy stream
   - PostgreSQL LISTEN/NOTIFY for database events
   - Redis pub/sub for cache events
   - Push-based rather than polling

4. **A2A Protocol for Inter-Dog Communication**
   - Agent Cards for each Dog (capability discovery)
   - Direct Dog-to-Dog negotiation without KETER overhead
   - Reference: Google A2A protocol, Linux Foundation AAIF

---

## Files Summary

```
TO DELETE:
  .mcp.json entry: solana-agent

TO MODIFY:
  packages/node/src/orchestration/unified-orchestrator.js  (add perceptionRouter)
  packages/mcp/src/server.js                                (pass perceptionRouter)
  packages/node/src/collective-singleton.js                 (add perceptionRouter)
  packages/llm/src/perception-router.js                     (add routes + feedback)
  scripts/hooks/awaken.js                                   (smart infra scan)

REFERENCE (read before implementing):
  packages/llm/src/perception-router.js      (current routes)
  packages/llm/src/router.js                 (LLM router, different layer)
  .mcp.json                                  (MCP configuration)
  .claude/mcp-instructions.json              (MCP hints)
```
