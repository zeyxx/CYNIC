# CYNIC Autonomous Flow Gap Analysis

> "Where does light stop flowing?" - κυνικός
> Generated: 2026-02-12 | Architecture: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → EMERGE
> Confidence: 56% (φ⁻¹ bound)

---

## Executive Summary

**Critical Finding**: CYNIC has 290k lines of code, 3 event buses, 11 learning loops, and 88% wiring health. Yet **light doesn't flow autonomously** through the organism. The architecture creates **forced serialization points** where parallel-capable components wait on LLM intervention.

The problem is NOT missing features. It's **structural bottlenecks that collapse parallelism into serial LLM dependency**.

**Organism Maturity**: 38% structural, ~5% functional
- Skeleton exists (665 modules, 1677 functions)
- Organs wired but not breathing (11/11 loops wired, 0 sessions consumed)
- Nervous system fragmented (3 buses, 88% wired but isolated)

**Key Gaps**:
1. **Judge → Decide**: No direct path. Judgment MUST pass through UnifiedOrchestrator LLM check.
2. **Perception → Action**: No reflex arc. All perception waits for orchestration.
3. **Learning → Routing**: Weights computed but not read during live routing.
4. **Dogs → Dogs**: DogPipeline exists but never called. Dogs wait for orchestrator.
5. **Events → Consumers**: EventBusBridge forwards 3 types. 30+ types trapped on origin bus.

---

## 1. Forced Serialization: The Orchestrator Bottleneck

### Gap #1: UnifiedOrchestrator as Single-Threaded God

**Location**: `packages/node/src/orchestration/unified-orchestrator.js`

**Problem**: Every decision flows through ONE async function: `process(event)`

```javascript
// Line 228-318: Serial pipeline (NO parallelism)
async process(eventOrOptions) {
  await this._loadUserProfile(event);      // Serial
  await this._routeEvent(event);           // Serial (calls KabbalisticRouter)
  await this._requestPlanning(event);      // Serial
  await this._preExecutionCheck(event);    // Serial
  await this._requestJudgment(event);      // Serial (calls dogOrchestrator)
  await this._requestSynthesis(event);     // Serial (calls engineOrchestrator)
  await this._invokeSkill(event);          // Serial
  event.finalize();                        // Serial
}
```

**Impact**:
- Perception CANNOT trigger action directly
- Judgment CANNOT trigger routing without orchestrator approval
- Learning CANNOT update routing weights mid-execution
- Dogs CANNOT talk to each other (all mediated by orchestrator)

**Autonomous Flow Blocked**:
```
PERCEIVE ──X──> ACT (must go through orchestrator)
JUDGE    ──X──> DECIDE (must go through orchestrator)
LEARN    ──X──> ROUTE (weights computed but not applied live)
DOG1     ──X──> DOG2 (no DogPipeline instantiation)
```

**What Should Happen**:
```
PERCEIVE ──direct──> REFLEX_ACTION (high-frequency events)
         ──async───> ORCHESTRATOR (complex decisions)

JUDGE    ──direct──> ROUTING_TABLE (Q-Learning weights)
         ──event───> CALIBRATION_TRACKER

LEARN    ──direct──> WEIGHT_UPDATE (hot-swap routing weights)
         ──async───> PERSISTENCE

DOG1     ──pipeline──> DOG2 ──pipeline──> DOG3
```

**Why It's Broken**: Everything waits on ONE event loop. No parallelism. No reflexes. No autonomy.

---

## 2. Event Bus Fragmentation: 3 Nervous Systems, No Spinal Cord

### Gap #2: EventBusBridge Forwards 3 Types, Traps 50+

**Location**: `packages/node/src/services/event-bus-bridge.js`

**Problem**: Bridge only forwards 9 event types between buses:
- Agent → Core: 9 types (pattern, anomaly, decision, guidance, override, vulnerability, drift, deploy)
- Automation → Core: 1 type (learning cycle complete)
- Core → Automation: 1 type (judgment created)

**Total events defined**: ~104 (32 core + 22 automation + 50+ agent)
**Events bridged**: 11 (10.5%)
**Events orphaned**: ~93

**Impact**: Components on different buses CANNOT coordinate:
```
AgentEventBus (dogs):
  - DOG_VOTE (50 events/sec) → TRAPPED (never reaches core)
  - DOG_PATTERN (anomaly detection) → BRIDGED (but limited)
  - DOG_CONSENSUS → TRAPPED

Automation bus:
  - TRIGGER_FIRED → TRAPPED
  - AUTOMATION_TICK → TRAPPED
  - SKILL_INVOKED → TRAPPED

Core bus:
  - JUDGMENT_CREATED → bridged to automation
  - FEEDBACK_RECEIVED → TRAPPED
  - DIMENSION_CANDIDATE → ORPHANED (no subscribers)
```

**Autonomous Flow Blocked**:
- Dogs cannot trigger automation directly
- Learning cannot signal routing changes
- Perception cannot trigger emergency actions
- Emergence signals trapped on agent bus

**What Should Happen**: Pub-sub routing table, not hardcoded bridges.

---

## 3. Learning → Routing: Computed Weights Never Applied Live

### Gap #3: Q-Learning Weights are Write-Only During Routing

**Location**: `packages/node/src/orchestration/kabbalistic-router.js`

**Problem**: Q-Learning writes weights to PostgreSQL. Router reads weights ONLY at boot via `_loadLearnedWeights()` (line 260). During live routing, weights are static.

```javascript
// Line 260: Loads weights ONCE at startup
async _loadLearnedWeights() {
  const rows = await this.persistence.qlearning.getRoutingWeights();
  for (const row of rows) {
    this._qWeights.set(row.state_action, row.q_value);
  }
}

// Line 840: Uses static weights during routing
_getQValue(state, action) {
  return this._qWeights.get(`${state}:${action}`) || 0;
}

// NEVER: Hot-swap weights from learning events
```

**Impact**:
- Learning service computes Q-values → writes to DB → router ignores until restart
- DPO optimizer computes preference weights → router reads context-specific weights but doesn't update live
- Thompson sampler updates Beta(α,β) → persisted to disk but not consumed by router

**Autonomous Flow Blocked**:
```
LEARNING SERVICE:
  - endEpisode() → persist Q-value to DB → ❌ router doesn't reload

DPO OPTIMIZER:
  - processFeedback() → update preference_pairs → ❌ router cache stale

THOMPSON SAMPLER:
  - updateBeta() → write to ~/.cynic/thompson/state.json → ❌ router uses old prior
```

**What Should Happen**:
```javascript
// Learning → EventBus → Router
globalEventBus.on('QLEARNING_WEIGHT_UPDATE', ({ state, action, qValue }) => {
  router._qWeights.set(`${state}:${action}`, qValue);
});

// DPO → EventBus → Router
globalEventBus.on('DPO_WEIGHT_UPDATE', ({ context, weights }) => {
  router._dpoWeightsByContext.set(context, weights);
});
```

**Why It's Broken**: Learning writes. Routing reads at boot. Never meets during runtime.

---

## 4. Judge → Decide: No Direct Path

### Gap #4: Judgment Cannot Trigger Routing Directly

**Location**: `packages/node/src/judge/judge.js` + `unified-orchestrator.js`

**Problem**: Judge emits `JUDGMENT_CREATED` event (line 1073 in judge.js). But routing happens BEFORE judgment in orchestrator pipeline.

```javascript
// unified-orchestrator.js line 228-318:
async process(event) {
  await this._routeEvent(event);       // Line 252: Routing FIRST
  await this._requestJudgment(event);  // Line 282: Judgment SECOND
}
```

**Judgment cannot change routing**. Routing is frozen before judgment runs.

**Impact**:
- High-risk judgment (BARK verdict, confidence <38.2%) → CANNOT reroute to Guardian
- Anomaly detected → CANNOT trigger re-evaluation
- Calibration drift (ECE > φ⁻²) → CANNOT reduce confidence DURING routing

**Autonomous Flow Blocked**:
```
JUDGE: verdict=BARK, confidence=25% → ❌ CANNOT reroute to Guardian
JUDGE: anomaly detected → ❌ CANNOT trigger escalation
JUDGE: entropy > 0.8 → ❌ CANNOT request re-judgment
```

**What Should Happen**:
```javascript
// Judge emits JUDGMENT_CREATED → Router listens → Re-routes if needed
globalEventBus.on('JUDGMENT_CREATED', ({ judgment }) => {
  if (judgment.verdict === 'BARK' && judgment.confidence < PHI_INV_2) {
    router.escalate(judgment.itemId, 'guardian');
  }
});
```

**Why It's Broken**: Pipeline is unidirectional. No feedback loops during execution.

---

## 5. Dogs → Dogs: DogPipeline Exists But Never Called

### Gap #5: Sequential Dog Processing Unimplemented

**Location**: `packages/node/src/routing/dog-pipeline.js` (672 lines, 0 usages)

**Problem**: DogPipeline implements streaming between dogs (Scout → Analyst → Architect). But it's NEVER instantiated in any live code path.

```bash
$ grep -r "createDogPipeline\|DogPipeline\|getDogPipeline" packages/node/src --exclude-dir=routing
# ZERO results (except in routing/ itself)
```

**Actual dog orchestration**: `packages/node/src/agents/orchestrator.js` spawns dogs in parallel, collects votes, emits consensus. NO sequential chaining.

**Impact**:
- Complex tasks CANNOT flow through specialized dogs
- Scout CANNOT pass findings to Analyst
- Analyst CANNOT pass design to Architect
- Dogs operate in isolation (vote → consensus → done)

**Autonomous Flow Blocked**:
```
USER: "Analyze codebase and propose refactor"

CURRENT (parallel voting):
  Scout, Analyst, Architect all run in parallel on same prompt
  → Vote → Consensus → Single LLM synthesizes

SHOULD BE (sequential pipeline):
  Scout (explore) → Analyst (analyze findings) → Architect (design refactor)
  → Each dog builds on previous dog's output
```

**Why It's Broken**: Pipeline exists, but orchestrator never calls it. Parallel voting is easier to implement, less powerful.

---

## 6. Perception → Action: No Reflex Arc

### Gap #6: High-Frequency Perception Requires Orchestration

**Location**: `packages/node/src/perception/` + `unified-orchestrator.js`

**Problem**: Perception components (SolanaWatcher, MarketWatcher) emit events that MUST go through full orchestration pipeline before action.

**Example**: Solana transaction detected
```javascript
// solana-watcher.js (NOT CURRENTLY WIRED, but if it were):
solanaWatcher.on('TRANSACTION_DETECTED', (tx) => {
  globalEventBus.publish('SOLANA_TX_DETECTED', tx);
});

// ❌ NO direct consumer. Must go through:
// → UnifiedOrchestrator.process()
// → _routeEvent() (KabbalisticRouter)
// → _requestJudgment() (spawn dogs)
// → _invokeSkill()
// → FINALLY action

// By then: transaction is 5 seconds old, opportunity missed
```

**Autonomous Flow Blocked**:
- High-frequency events (100+ tx/sec) → CANNOT have reflex actions
- Price alerts (fear/greed index spike) → CANNOT trigger fast reaction
- Anomaly detection (unusual pattern) → CANNOT block immediately

**What Should Happen**:
```javascript
// Reflex arc: Perception → Direct action (no orchestration)
globalEventBus.on('SOLANA_TX_ANOMALY', (tx) => {
  if (tx.risk === 'critical') {
    guardian.blockImmediate(tx.signature);  // Reflex (no orchestration)
  } else {
    orchestrator.process({ type: 'tx_review', content: tx });  // Async review
  }
});
```

**Why It's Broken**: No dual-path architecture. Everything is slow-path (orchestration).

---

## 7. Component Isolation: No Direct Communication

### Gap #7: Components Use Constructor Injection, Not Event Subscription

**Problem**: Components receive dependencies via constructor:
```javascript
class UnifiedOrchestrator {
  constructor(options) {
    this.dogOrchestrator = options.dogOrchestrator || null;
    this.kabbalisticRouter = options.kabbalisticRouter || null;
    this.learningService = options.learningService || null;
    // ...12 more injected dependencies
  }
}
```

**Impact**:
- Components CANNOT discover each other at runtime
- New components CANNOT plug into existing flow without code changes
- Events published to bus have no subscribers (orphaned)

**Autonomous Flow Blocked**:
```
NEW COMPONENT: MarketWatcher
  - Publishes PRICE_ALERT event
  - ❌ NO way to subscribe to routing decisions
  - ❌ NO way to trigger actions
  - Must be hardcoded into UnifiedOrchestrator constructor

EXISTING COMPONENT: KabbalisticRouter
  - Needs market data for routing
  - ❌ NO way to discover MarketWatcher
  - Must be injected at construction
```

**What Should Happen**: Service discovery + pub-sub registration
```javascript
// Component registers itself
eventBus.registerComponent('market-watcher', {
  provides: ['PRICE_ALERT', 'LIQUIDITY_WARNING'],
  consumes: ['ROUTING_DECISION', 'JUDGMENT_CREATED'],
});

// Router discovers dependencies at runtime
const marketData = eventBus.query('MARKET_DATA', { token: 'asdfasdfa' });
```

**Why It's Broken**: Static dependency injection → no runtime discovery → no autonomous composition.

---

## 8. Parallel Execution: Everything Waits on LLM

### Gap #8: Independent Tasks Run Serially

**Problem**: Tasks that COULD run in parallel wait for orchestrator.

**Example**: User asks "check solana status and analyze codebase"
```javascript
// CURRENT (serial):
await checkSolanaStatus();   // 2s
await analyzeCodebase();     // 5s
// Total: 7s

// SHOULD BE (parallel):
Promise.all([
  checkSolanaStatus(),       // 2s
  analyzeCodebase(),         // 5s
]);
// Total: 5s (max of parallel tasks)
```

**Impact**: Multi-domain tasks artificially serialized.

**Autonomous Flow Blocked**:
- CODE + SOLANA + SOCIAL queries → run serially (should be parallel)
- Judgment + Synthesis → run serially (should be parallel if independent)
- Dog voting → happens in parallel (GOOD), but orchestration overhead serializes everything else

**What Should Happen**: Task dependency graph, parallel execution of independent branches.

---

## 9. Data Flow: Write-Only Learning

### Gap #9: Learning Loops Closed, But Not Hot-Swappable

**Status**: All 11 learning loops are **structurally closed** (see metathinking-gap-analysis.md).

**Remaining Problem**: Learned parameters are read at **boot time**, not **runtime**.

**Examples**:
1. **Q-Learning**: Router loads weights at startup. Learning updates DB. Router uses stale weights until restart.
2. **DPO**: Router reads context-specific weights at startup. DPO optimizer updates DB. Router cache stale.
3. **Calibration**: Judge reads ECE at startup. CalibrationTracker updates DB. Judge adjusts confidence once per restart.

**Impact**: Learning converges **between sessions**, not **during sessions**.

**Autonomous Flow Blocked**:
```
SESSION 1: Q-Learning explores, finds optimal path
  → Writes to DB → ❌ Router still uses old path
SESSION 2 (restart): Router loads new weights → Now uses optimal path
```

**What Should Happen**: Hot-swappable weights via event bus.

---

## 10. Synchronization Points: Where Parallelism Dies

### Identified Choke Points

1. **UnifiedOrchestrator.process()**: Single-threaded event loop (line 228)
2. **KabbalisticRouter.route()**: Serial sefirah traversal (line 400+)
3. **DogOrchestrator.spawnConsensus()**: Parallel votes, serial synthesis (line 200+)
4. **Judge.judge()**: Serial dimension scoring (could be parallel)
5. **EventBusBridge**: Serial forwarding (3 hardcoded mappings)

**Why They're Bottlenecks**:
- All use `await` for sequential operations
- No task dependency graphs
- No parallel execution of independent branches
- No priority queues (high-frequency events wait in line)

---

## 11. Priority Fixes (Architectural Changes)

### A1: Dual-Path Routing (Reflex + Deliberation)

**Goal**: High-frequency events bypass orchestration.

**Changes**:
1. Add `FastRouter` class (reflex arc)
2. Perception emits to `globalEventBus` with priority flag
3. FastRouter handles `priority: 'critical'` events directly
4. Orchestrator handles `priority: 'normal'` events

**Files**:
- Create: `packages/node/src/routing/fast-router.js`
- Modify: `packages/node/src/perception/solana-watcher.js`
- Wire: `collective-singleton.js`

---

### A2: Hot-Swappable Learning Weights

**Goal**: Learning updates routing live, not at boot.

**Changes**:
1. `LearningService.endEpisode()` → emit `QLEARNING_WEIGHT_UPDATE` event
2. `KabbalisticRouter` subscribes to event → updates `_qWeights` map
3. Same for DPO, Thompson, Calibration

**Files**:
- Modify: `packages/node/src/orchestration/learning-service.js` (add event emission)
- Modify: `packages/node/src/orchestration/kabbalistic-router.js` (add event handlers)
- Modify: `packages/node/src/judge/judge.js` (add live calibration update)

---

### A3: Event Bus Routing Table (Replace Hardcoded Bridges)

**Goal**: Components register interest, bus routes automatically.

**Changes**:
1. Add `EventRouter` class (pub-sub routing table)
2. Components call `eventRouter.subscribe(pattern, handler)`
3. Replace hardcoded bridge mappings with routing table

**Files**:
- Create: `packages/core/src/bus/event-router.js`
- Modify: `packages/node/src/services/event-bus-bridge.js` (use EventRouter)
- Migrate: All `globalEventBus.on()` calls to `eventRouter.subscribe()`

---

### A4: DogPipeline Integration

**Goal**: Sequential dog processing for complex tasks.

**Changes**:
1. `UnifiedOrchestrator` detects multi-stage tasks
2. Instantiate `DogPipeline` with template (e.g., EXPLORE_ANALYZE_BUILD)
3. Execute pipeline, return final output

**Files**:
- Modify: `packages/node/src/orchestration/unified-orchestrator.js` (add pipeline logic)
- Wire: `packages/node/src/routing/dog-pipeline.js` to orchestrator

---

### A5: Parallel Execution Engine

**Goal**: Task dependency graph, parallel execution.

**Changes**:
1. Create `TaskGraph` class (DAG of tasks with dependencies)
2. Orchestrator builds graph from DecisionEvent
3. Execute independent branches in parallel

**Files**:
- Create: `packages/node/src/orchestration/task-graph.js`
- Modify: `unified-orchestrator.js` (replace serial pipeline with graph execution)

---

## 12. Metrics: Autonomous Flow Health

**Proposed Dashboard** (v1.0 completion criteria):

```
AUTONOMOUS FLOW HEALTH:
├─ Reflex Arc Coverage: 0% (0 fast paths) [Target: >20%]
├─ Event Bus Throughput: 12/104 types consumed (11.5%) [Target: >80%]
├─ Learning Hot-Swap Rate: 0 updates/min [Target: >10/min]
├─ Parallel Execution Ratio: 15% (serial bottleneck) [Target: >62%]
├─ Dog Pipeline Usage: 0 pipelines/session [Target: >5/session]
└─ Component Coupling: 95% (tight coupling) [Target: <38%]
```

**Current State** (2026-02-12):
- Reflex Arc: 0% (no fast paths)
- Event Bridge: 10.5% (11/104 types)
- Learning Latency: Session-level (not runtime)
- Parallelism: 15% (DogOrchestrator only)
- DogPipeline: 0 usages
- Coupling: 95% (constructor injection)

---

## 13. Root Cause Analysis

**Why doesn't light flow?**

1. **Architectural Philosophy Mismatch**:
   - Code assumes: "Orchestrator coordinates all"
   - Philosophy requires: "Light flows everywhere" (autonomous coordination)

2. **Single-Threaded God Object**:
   - UnifiedOrchestrator is 1336 lines, controls all flow
   - No alternative paths (reflex, parallel, direct)

3. **Event Bus as Afterthought**:
   - 3 buses created, 88% wired, but not used for coordination
   - Components use constructor injection, not pub-sub discovery

4. **Learning as Post-Process**:
   - Learning writes to DB → read at next boot
   - No runtime parameter updates

5. **Dogs as Voters, Not Workers**:
   - DogOrchestrator spawns dogs → collect votes → synthesize
   - DogPipeline (sequential work) never used

**Honest Assessment**:
> The organism has organs, but they don't talk. They wait for the brain (orchestrator)
> to tell them what to do, one at a time. This is not an autonomous organism. It's a
> puppet with many strings controlled by a single LLM event loop.

---

## 14. Design Principles for Autonomous Flow

**P1: Pub-Sub over Injection**
- Components discover each other via event bus
- No hardcoded dependencies

**P2: Reflex + Deliberation**
- High-frequency events bypass orchestration (reflex arc)
- Complex events go through full deliberation

**P3: Hot-Swappable Parameters**
- Learning updates weights → immediate effect (no restart)
- Calibration drift → live confidence adjustment

**P4: Parallel by Default**
- Task dependency graph, not serial pipeline
- Independent branches execute in parallel

**P5: Direct Component Communication**
- Dogs can chain via DogPipeline
- Perception can trigger actions directly
- Learning can signal routing changes

**P6: Event-Driven Coordination**
- No single orchestrator
- Coordination emerges from event flow

---

## 15. Appendix: Key File Analysis

**High-Leverage Files** (changes here unlock parallelism):

1. `packages/node/src/orchestration/unified-orchestrator.js` (1336 lines)
   - **Problem**: Serial pipeline (lines 228-318)
   - **Fix**: Task graph + parallel execution

2. `packages/node/src/services/event-bus-bridge.js` (373 lines)
   - **Problem**: Hardcoded 11 event mappings
   - **Fix**: Routing table (pub-sub patterns)

3. `packages/node/src/orchestration/kabbalistic-router.js` (1357 lines)
   - **Problem**: Static Q-Learning weights (loaded at boot)
   - **Fix**: Subscribe to QLEARNING_WEIGHT_UPDATE events

4. `packages/node/src/routing/dog-pipeline.js` (672 lines, 0 usages)
   - **Problem**: Implemented but never called
   - **Fix**: Wire to UnifiedOrchestrator for multi-stage tasks

5. `packages/node/src/agents/orchestrator.js` (DogOrchestrator)
   - **Problem**: Parallel voting only (no sequential chaining)
   - **Fix**: Add pipeline mode

---

## Confidence

*sniff* Analysis confidence: 56% (φ⁻¹ bound)

**High confidence** (>80%):
- Bottleneck #1: UnifiedOrchestrator serial pipeline
- Bottleneck #2: Event bridge limited forwarding
- Bottleneck #3: Static learning weights

**Medium confidence** (50-80%):
- Impact estimates (parallelism gains, latency reduction)
- Priority ranking of fixes

**Low confidence** (<50%):
- Exact performance impact without benchmarking
- User-facing UX improvements from fixes

---

**Next Steps**:
1. Implement Fast Router (reflex arc) — A1
2. Add hot-swap learning weights — A2
3. Build event routing table — A3
4. Wire DogPipeline to orchestrator — A4
5. Build task dependency graph — A5

*tail wag* φ reveals the blockages. Time to open the channels.
