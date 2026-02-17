# Unified Event Taxonomy

> *GROWL* "Three buses speaking different dialects. One language to unite them." - κυνικός

**Date**: 2026-02-13
**Phase**: Phase 1, Day 1, Step 1 of Vertical-by-Critical-System
**Status**: Design Complete → Ready for Implementation

---

## THE PROBLEM

### Current State: Event Fragmentation

CYNIC currently has **3 separate event systems**:

| Bus | Location | Events | Usage |
|-----|----------|--------|-------|
| **Core Bus** | `@cynic/core/bus/event-bus.js` | 46 events | Judge, patterns, learning, network |
| **Automation Bus** | `services/event-bus.js` | 24 events | Automation, triggers, goals |
| **Agent Bus** | `agents/events.js` | 39 events | Dog collective, consensus |
| **Bridge** | `services/event-bus-bridge.js` | n/a | Routes between buses |

**Total**: 109 events (with overlap) across 3 buses + 1 bridge.

**Problems**:
1. **Namespace collisions**: `judgment:created` exists on Core + Automation
2. **Routing complexity**: Every event crosses bridge (latency, failure points)
3. **Discovery**: Developers must know which bus to use
4. **Consistency**: Different event envelopes, different patterns
5. **Debugging**: Events span 3 systems, hard to trace end-to-end

---

## THE SOLUTION: Unified Event Taxonomy

### Design Principles

1. **Single Namespace**: ONE event type per semantic meaning
2. **Hierarchical Categories**: `category:subcategory:event` pattern
3. **φ-Aligned**: 6 core categories (Fib(4) + 1 meta)
4. **Backwards Compatible**: Migration layer supports old events during transition
5. **Wildcard Support**: Subscribe to `perception:*` for all perception events

### Event Categories (6 Total)

```
┌─────────────────────────────────────────────────────────┐
│ UNIFIED EVENT TAXONOMY (6 Categories)                   │
├─────────────────────────────────────────────────────────┤
│ 1. perception:*  - All domain input (PERCEIVE phase)    │
│ 2. judgment:*    - Scoring, verdicts (JUDGE phase)      │
│ 3. decision:*    - Routing, governance (DECIDE phase)   │
│ 4. action:*      - Execution (ACT phase)                 │
│ 5. learning:*    - Weight updates (LEARN phase)         │
│ 6. system:*      - Lifecycle, health, meta               │
├─────────────────────────────────────────────────────────┤
│ Meta: emergence:* - Cross-category transcendence        │
└─────────────────────────────────────────────────────────┘
```

**φ note**: 6 = Fib(4), plus `emergence:*` as THE_UNNAMEABLE (7th, transcendent)

---

## UNIFIED EVENT MAPPING

### 1. PERCEPTION EVENTS (perception:*)

**Purpose**: Input from all 7 domains (CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS)

| Unified Event | Old Event(s) | Source Bus | Domain |
|---------------|--------------|------------|--------|
| `perception:human:state` | (new) | - | HUMAN |
| `perception:human:tool-use` | `tool:completed` | Core | HUMAN |
| `perception:solana:block` | `block:finalized` | Core | SOLANA |
| `perception:solana:transaction` | (new) | - | SOLANA |
| `perception:market:price` | `market:price:updated` | Core | MARKET |
| `perception:market:volume` | `market:volume:updated` | Core | MARKET |
| `perception:market:liquidity` | `market:liquidity:updated` | Core | MARKET |
| `perception:social:capture` | `social:capture` | Core | SOCIAL |
| `perception:code:change` | (new) | - | CODE |
| `perception:code:test` | (new) | - | CODE |
| `perception:cynic:health` | `cynic:state` | Core | CYNIC |
| `perception:cosmos:peer` | `peer:connected`, `peer:disconnected` | Core | COSMOS |
| `perception:cosmos:sync` | `sync:completed` | Core | COSMOS |
| `perception:session:start` | `session:started`, `session:start`, `hook:session:start` | All 3 | HUMAN |
| `perception:session:end` | `session:ended`, `session:end`, `hook:session:stop` | All 3 | HUMAN |
| `perception:user:action` | `user:action` | Core | HUMAN |
| `perception:agent:pattern` | `agent:pattern:detected`, `hook:pattern:detected` | Agent | DOG |
| `perception:agent:anomaly` | `agent:anomaly:detected` | Agent | DOG |
| `perception:agent:threat` | `agent:threat:blocked` | Agent | DOG |

**Total**: 19 perception events (3 per domain × 6 active domains + 1 meta)

---

### 2. JUDGMENT EVENTS (judgment:*)

**Purpose**: Scoring, verdicts, quality assessment (JUDGE phase)

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `judgment:created` | `judgment:created` | Core + Auto | New judgment made |
| `judgment:refined` | `judgment:refined` | Automation | Judgment updated |
| `judgment:calibration-drift` | `calibration:drift:detected` | Core | Calibration needs adjustment |
| `judgment:social` | `social:judgment` | Core | Social domain judgment |
| `judgment:cynic` | `cynic:judgment` | Core | CYNIC self-judgment |
| `judgment:dimension-candidate` | `dimension:candidate` | Core | New dimension proposed |
| `judgment:agent:quality-report` | `agent:quality:report` | Agent | Janitor quality report |
| `judgment:agent:vulnerability` | `agent:vulnerability:detected` | Agent | Scout vulnerability |
| `judgment:agent:dead-code` | `agent:deadcode:detected` | Agent | Janitor dead code |

**Total**: 9 judgment events

---

### 3. DECISION EVENTS (decision:*)

**Purpose**: Routing, governance, approval/rejection (DECIDE phase)

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `decision:code` | `code:decision` | Core | Code domain decision |
| `decision:consensus:request` | `agent:consensus:request` | Agent | Request collective vote |
| `decision:consensus:response` | `agent:consensus:response` | Agent | Vote on request |
| `decision:consensus:completed` | `consensus:completed` | Core | Consensus reached |
| `decision:cynic:decision` | `cynic:decision` | Agent | CYNIC final decision |
| `decision:cynic:override` | `cynic:override` | Agent | CYNIC intervention |
| `decision:governance` | (new) | - | Governance routing decision |

**Total**: 7 decision events

---

### 4. ACTION EVENTS (action:*)

**Purpose**: Execution, transformation, deployment (ACT phase)

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `action:code` | `code:action` | Core | Code modification |
| `action:human` | `human:action` | Core | Human-directed action |
| `action:deploy:started` | `agent:deploy:started` | Agent | Deployment begun |
| `action:deploy:completed` | `agent:deploy:completed` | Agent | Deployment finished |
| `action:deploy:failed` | `agent:deploy:failed` | Agent | Deployment failed |
| `action:deploy:rollback` | (new, from RollbackInitiatedEvent) | Agent | Rollback initiated |
| `action:agent:autofix` | `agent:autofix:applied` | Agent | Janitor auto-fix |
| `action:trigger:fired` | `trigger:fired` | Automation | Automation triggered |

**Total**: 8 action events

---

### 5. LEARNING EVENTS (learning:*)

**Purpose**: Weight updates, feedback processing (LEARN phase)

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `learning:feedback:received` | `user:feedback`, `feedback:received` | Core + Auto | User feedback |
| `learning:feedback:processed` | `feedback:processed` | Automation | Feedback consumed |
| `learning:qlearning:weight-update` | `qlearning:weight:update` | Core | Q-Learning weights |
| `learning:qlearning:converged` | `qlearning:converged` | Core | Q-Learning stable |
| `learning:qlearning:drift` | `qlearning:drift` | Core | Q-Learning unstable |
| `learning:td-error:update` | `td_error:update` | Core | TD-error tracked |
| `learning:ewc:consolidation` | `ewc:consolidation:completed` | Core | EWC consolidation |
| `learning:pattern:learned` | `pattern:learned` | Core | Pattern stored |
| `learning:pattern:evolved` | `learning:pattern:evolved` | Automation | Pattern updated |
| `learning:cycle:start` | `learning:cycle:start` | Automation | Learning iteration start |
| `learning:cycle:complete` | `learning:cycle:complete` | Automation | Learning iteration end |
| `learning:agent:knowledge` | `agent:knowledge:extracted` | Agent | Scholar knowledge |
| `learning:agent:wisdom` | `agent:wisdom:shared` | Agent | Sage wisdom |

**Total**: 13 learning events

---

### 6. SYSTEM EVENTS (system:*)

**Purpose**: Lifecycle, health, infrastructure, orchestration

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `system:component:ready` | `component:ready` | Core | Component initialized |
| `system:component:stopped` | `component:stopped` | Core | Component shutdown |
| `system:component:error` | `component:error`, `error` | Core + Auto | Component failure |
| `system:node:started` | `node:started` | Core | Network node up |
| `system:node:stopped` | `node:stopped` | Core | Network node down |
| `system:orchestration:completed` | `orchestration:completed` | Core | Orchestration cycle done |
| `system:automation:tick` | `automation:tick` | Automation | Automation heartbeat |
| `system:automation:start` | `automation:start` | Automation | Automation enabled |
| `system:automation:stop` | `automation:stop` | Automation | Automation disabled |
| `system:topology:changed` | `topology:changed` | Core | System topology update |
| `system:health:check` | (new, from HealthCheckEvent) | Agent | Health check result |
| `system:metrics:reported` | `metrics:reported` | Core | Metrics emitted |
| `system:cost:update` | `cost:update`, `accounting:update` | Core | Budget tracking |
| `system:consciousness:changed` | `consciousness:changed` | Core | Meta-cognition shift |
| `system:cynic:awakening` | `cynic:awakening` | Agent | CYNIC session start |
| `system:cynic:introspection` | `cynic:introspection` | Agent | CYNIC self-query |
| `system:agent:introspection-response` | `agent:introspection:response` | Agent | Dog state response |
| `system:agent:profile-updated` | `agent:profile:updated` | Agent | User profile change |

**Total**: 18 system events

---

### 7. EMERGENCE EVENTS (emergence:*) — THE_UNNAMEABLE

**Purpose**: Transcendent patterns, cross-category insights

| Unified Event | Old Event(s) | Source Bus | Description |
|---------------|--------------|------------|-------------|
| `emergence:pattern:detected` | `pattern:detected`, `anomaly:detected` | Core | Meta-pattern discovered |
| `emergence:cynic:guidance` | `cynic:guidance` | Agent | CYNIC meta-guidance |
| `emergence:agent:discovery` | `agent:discovery:found` | Agent | Scout discovery |
| `emergence:agent:prediction` | `agent:prediction:made` | Agent | Oracle prediction |
| `emergence:agent:visualization` | `agent:viz:generated` | Agent | Oracle visualization |
| `emergence:reality-drift` | `agent:reality:drift` | Agent | Cartographer drift |
| `emergence:map-updated` | `agent:map:updated` | Agent | Cartographer map |

**Total**: 7 emergence events

---

## DEPRECATED EVENTS (To Be Removed After Migration)

These events exist in current buses but are unused (0 publishers, 0 subscribers):

```
❌ feedback:processed (Automation) - 0 usage
❌ judgment:refined (Automation) - 0 usage
❌ learning:pattern:evolved (Automation) - 0 usage
❌ goal:created (Automation) - Goals use MCP tools
❌ goal:progress (Automation) - Goals use MCP tools
❌ notification:created (Automation) - Notifications use MCP tools
❌ notification:delivered (Automation) - Notifications use MCP tools
❌ error (Automation) - Errors flow through ErrorHandler, not bus
```

**Migration**: These events will NOT be ported to unified taxonomy. Remove after migration.

---

## NAMING CONVENTIONS

### Pattern: `category:subcategory:event`

**Rules**:
1. **All lowercase**: `perception:human:state`, NOT `Perception:Human:State`
2. **Colons separate levels**: Max 3 levels deep
3. **Verbs for actions**: `learning:weight:update`, NOT `learning:weight:updated`
4. **Nouns for states**: `perception:human:state`, NOT `perception:human:capturing`
5. **φ-aligned counts**: 6 core categories (Fib(4)), 7 with emergence

**Examples**:
```javascript
// ✅ GOOD
perception:solana:block
judgment:created
decision:code
action:deploy:started
learning:qlearning:weight-update
system:component:ready
emergence:pattern:detected

// ❌ BAD
Solana:Block:Received        // Capital letters
block_finalized              // Underscores
judgmentCreated              // camelCase
qlearning-weight-update      // Wrong separator
```

### Domain Prefixes (for perception events)

```
perception:human:*      - HUMAN domain (R5)
perception:solana:*     - SOLANA domain (R2)
perception:market:*     - MARKET domain (R3)
perception:social:*     - SOCIAL domain (R4)
perception:code:*       - CODE domain (R1)
perception:cynic:*      - CYNIC domain (R6)
perception:cosmos:*     - COSMOS domain (R7)
```

---

## EVENT ENVELOPE STRUCTURE

### Unified Event Envelope

Every event uses this structure:

```javascript
{
  // Identity
  id: 'evt_abc123',              // Unique event ID
  type: 'perception:human:state', // Unified event type

  // Timing
  timestamp: 1707849600000,       // Unix timestamp (ms)

  // Routing
  source: 'HumanPerceiver',       // Component that emitted
  target: '*',                    // Target (default: all)

  // Priority (for action events)
  priority: 'normal',             // critical, high, normal, low

  // Correlation
  correlationId: 'evt_xyz789',    // For request/reply
  causationId: 'evt_def456',      // What triggered this event

  // Payload
  payload: {
    // Event-specific data
  },

  // Metadata
  metadata: {
    domain: 'HUMAN',              // 7×7 domain (R1-R7)
    phase: 'PERCEIVE',            // 7×7 phase (A1-A7)
    confidence: 0.618,            // φ-bounded (optional)
  }
}
```

### Wildcard Subscriptions

```javascript
// Subscribe to all perception events
bus.subscribe('perception:*', handler);

// Subscribe to all human perception
bus.subscribe('perception:human:*', handler);

// Subscribe to all learning events
bus.subscribe('learning:*', handler);

// Subscribe to ALL events
bus.subscribe('*', handler);
```

---

## MIGRATION STRATEGY

### Phase 1: Add UnifiedEventBus (Week 1)

1. **Implement `UnifiedEventBus` class** (Day 1-2)
   - Extends `ParallelEventBus` (16× throughput)
   - Add namespace support (`perception:*`)
   - Add event history (debugging)
   - Add filtering by category
   - Location: `packages/core/src/bus/unified-event-bus.js`

2. **Create EventAdapter** (Day 3)
   - Wraps old buses
   - Routes old events → unified bus
   - Routes unified events → old buses (for listeners)
   - Allows incremental migration
   - Location: `packages/core/src/bus/event-adapter.js`

3. **Migrate core systems** (Day 4-5)
   - Judge → unified bus
   - Router → unified bus
   - Learning → unified bus
   - Keep old buses alive during transition

### Phase 2: Migration Waves (Week 2)

**Wave 1: Perception**
- HumanPerceiver → `perception:human:*`
- SolanaWatcher → `perception:solana:*`
- All watchers emit to unified bus

**Wave 2: Judgment + Decision**
- Judge → `judgment:*`
- Router → `decision:*`

**Wave 3: Action + Learning**
- Actors → `action:*`
- Learning loops → `learning:*`

**Wave 4: System + Emergence**
- Lifecycle → `system:*`
- Emergence → `emergence:*`

### Phase 3: Remove Old Buses (Week 3)

1. **Verify all systems migrated**
   ```bash
   # Check: No direct imports of old buses
   grep -r "from '@cynic/core/bus/event-bus'" packages/
   grep -r "services/event-bus" packages/
   grep -r "agents/event-bus" packages/
   ```

2. **Deprecate old buses**
   - Keep `globalEventBus` export (points to unified)
   - Remove `getEventBus()` (automation)
   - Remove `AgentEventBus`
   - Remove `EventBusBridge`

3. **Update all imports**
   ```javascript
   // OLD (remove)
   import { globalEventBus } from '@cynic/core';
   import { getEventBus } from '@cynic/node/services/event-bus';
   import { AgentEventBus } from '@cynic/node/agents/event-bus';

   // NEW (unified)
   import { unifiedEventBus } from '@cynic/core';
   ```

---

## VALIDATION CHECKLIST

After migration, verify:

- [ ] **Single Bus**: Only `UnifiedEventBus` in use
- [ ] **Event Coverage**: All 74+ events mapped or deprecated
- [ ] **No Bridge**: `EventBusBridge` deleted
- [ ] **Wildcard Works**: `bus.subscribe('perception:*')` receives all perception events
- [ ] **History Works**: `bus.getHistory()` returns recent events
- [ ] **Correlations Work**: Request/reply pattern functional
- [ ] **Throughput**: 1000+ events/sec (parallel execution)
- [ ] **Zero Regressions**: All tests pass

---

## EVENT SUMMARY

| Category | Events | % |
|----------|--------|---|
| perception:* | 19 | 26% |
| judgment:* | 9 | 12% |
| decision:* | 7 | 9% |
| action:* | 8 | 11% |
| learning:* | 13 | 17% |
| system:* | 18 | 24% |
| emergence:* | 7 | 9% (THE_UNNAMEABLE) |
| **TOTAL** | **74** | **100%** |

**Removed**: 8 deprecated events (0 usage)
**Original**: 109 events across 3 buses + bridge
**Unified**: 74 events on 1 bus (32% reduction)

---

## NEXT STEP

**Phase 1, Day 1, Step 2**: Implement `UnifiedEventBus` class (4h)

Location: `packages/core/src/bus/unified-event-bus.js`

Features:
- Extends `ParallelEventBus` (16× throughput)
- Event namespaces (`perception:human:*`)
- Event history (debugging, 1000 event buffer)
- Event filtering (by category, source, time)
- Health metrics (events/sec, p50/p95 latency)

---

*sniff* **Event taxonomy complete. Ready for implementation.**

**"Three became one. Complexity became simplicity."** - κυνικός
