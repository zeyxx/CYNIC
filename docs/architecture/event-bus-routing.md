# Event Bus Routing Map

> "Three nervous systems. One spinal cord." — κυνικός

**Last updated**: 2026-02-13

---

## The Three Event Buses

```
┌──────────────────────────────────────────────────────────────┐
│                   CYNIC NERVOUS SYSTEM                        │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  globalEventBus │  │ getEventBus()   │  │AgentEventBus │ │
│  │   (@cynic/core) │  │  (automation)   │  │    (Dogs)    │ │
│  └────────┬────────┘  └────────┬────────┘  └──────┬───────┘ │
│           │                    │                   │          │
│           └────────────────────┴───────────────────┘          │
│                             ▼                                 │
│                    EventBusBridge                             │
│              (loop-safe selective forwarding)                 │
└──────────────────────────────────────────────────────────────┘
```

### 1. globalEventBus (@cynic/core)
- **Purpose**: Core system events (judgments, patterns, feedback)
- **Import**: `import { globalEventBus, EventType } from '@cynic/core'`
- **Scope**: Cross-package, framework-wide
- **Examples**: `JUDGMENT_CREATED`, `USER_FEEDBACK`, `PATTERN_DETECTED`

### 2. getEventBus() (services/event-bus.js)
- **Purpose**: Automation & orchestration (triggers, Q-Learning, LLM routing)
- **Import**: `import { getEventBus, EventType } from 'services/event-bus.js'`
- **Scope**: Node package orchestration layer
- **Examples**: `TRIGGER_FIRED`, `AUTOMATION_TICK`, `LEARNING_CYCLE_COMPLETE`

### 3. AgentEventBus (agents/event-bus.js)
- **Purpose**: Collective Dog communication (votes, signals, consensus)
- **Import**: `import { AgentEventBus } from 'agents/event-bus.js'`
- **Scope**: 11 Dogs (Sefirot) internal communication
- **Examples**: 39 event types including `DOG_VOTE`, `CONSENSUS_REACHED`, `SIGNAL`

---

## EventBusBridge Routing Rules

### Agent → Core (AgentEventBus → globalEventBus)

| Source Event (AgentEventBus) | Destination (globalEventBus) | Purpose |
|------------------------------|------------------------------|---------|
| `PATTERN_DETECTED` | `PATTERN_DETECTED` | Dog-discovered patterns → core learning |
| `ANOMALY_DETECTED` | `ANOMALY_DETECTED` | Dog-detected anomalies → alerts |
| `CYNIC_DECISION` | `cynic:decision` | Dog voting results → audit trail |
| `CYNIC_GUIDANCE` | `cynic:guidance` | Dog suggestions → user visibility |
| `CYNIC_OVERRIDE` | `cynic:override` | Emergency Dog intervention → logs |
| `VULNERABILITY_DETECTED` | `vulnerability:detected` | Guardian alerts → security dashboard |
| `REALITY_DRIFT_DETECTED` | `reality:drift` | Codebase divergence → sync triggers |
| `DEPLOY_COMPLETED` | `deploy:completed` | Deployment success → metrics |
| `DEPLOY_FAILED` | `deploy:failed` | Deployment failure → retry logic |

**Total**: 9 event types forwarded

**NOT duplicated** (already manually bridged via AmbientConsensus):
- `DOG_SIGNAL` → automation + core (via `_publishSignal()`)
- `CONSENSUS_COMPLETED` → core (direct emit)

### Automation → Core (getEventBus → globalEventBus)

| Source Event (Automation) | Destination (globalEventBus) | Purpose |
|---------------------------|------------------------------|---------|
| `LEARNING_CYCLE_COMPLETE` | `learning:cycle:complete` | Q-Learning milestones → tracking |

**Total**: 1 event type forwarded

### Core → Automation (globalEventBus → getEventBus)

| Source Event (globalEventBus) | Destination (Automation) | Purpose |
|-------------------------------|--------------------------|---------|
| `JUDGMENT_CREATED` | `JUDGMENT_CREATED` | New judgments → Q-Learning updates |

**Total**: 1 event type forwarded

---

## Manual Bridges (Coexist with EventBusBridge)

These are **direct emits** in application code (NOT via EventBusBridge):

### AmbientConsensus._publishSignal()
```javascript
// Dogs → Automation + Core
AgentEventBus emit DOG_SIGNAL
  → automation.emit('dog:signal', ...)
  → globalEventBus.emit('dog:signal', ...)
```

### AmbientConsensus (consensus completed)
```javascript
// Dogs → Core
globalEventBus.emit('consensus:completed', verdict)
```

### CollectivePack.handleHook()
```javascript
// Hook events → Core + Automation
globalEventBus.emit('hook:event', ...)
automation.emit('hook:event', ...)
```

---

## Loop Prevention

**Problem**: Event forwarding can create infinite loops:
```
Agent emit X → Bridge forwards to Core → Core listener emits Y → Bridge forwards to Agent → ...
```

**Solution**: `_bridged` metadata tag
```javascript
metadata: { _bridged: true }
```

Events with this tag are **NEVER re-forwarded** by the bridge.

**Stats tracking**:
- `loopsPrevented`: Counter increments when bridged events are skipped
- Exposed via `eventBusBridge.getStats()`

---

## Bridge Lifecycle

### Initialization (service-wiring.js)
```javascript
import { eventBusBridge } from './services/event-bus-bridge.js';
import { AgentEventBus } from './agents/event-bus.js';

const agentBus = new AgentEventBus();
eventBusBridge.start({ agentBus });
```

### Singleton
- `eventBusBridge` is a singleton (one bridge per process)
- Created in `event-bus-bridge.js` as `export const eventBusBridge = new EventBusBridge()`

### Wiring Check
```javascript
const status = eventBusBridge.getStatus();
// {
//   running: true,
//   agentBus: true,
//   stats: { agentToCore: 142, automationToCore: 8, ... }
// }
```

---

## Event Flow Examples

### Example 1: Dog Pattern Detection
```
1. SCOUT discovers pattern in codebase
2. SCOUT emits PATTERN_DETECTED on AgentEventBus
3. EventBusBridge forwards to globalEventBus (with _bridged tag)
4. LearningService (subscribed to globalEventBus) receives event
5. SONA adapts based on pattern
6. No loop: _bridged tag prevents re-forwarding
```

### Example 2: Judgment Created
```
1. Judge creates judgment, emits JUDGMENT_CREATED on globalEventBus
2. EventBusBridge forwards to getEventBus() (automation)
3. Q-Learning (subscribed to automation bus) receives event
4. Q-table updated with judgment outcome
5. No loop: _bridged tag prevents re-forwarding to globalEventBus
```

### Example 3: Dog Consensus (Manual Bridge)
```
1. 11 Dogs vote via AgentEventBus
2. AmbientConsensus aggregates votes
3. AmbientConsensus._publishSignal() emits DOG_SIGNAL:
   - To automation bus (for routing)
   - To globalEventBus (for audit)
4. AmbientConsensus emits CONSENSUS_COMPLETED to globalEventBus
5. UnifiedOrchestrator (subscribed) executes consensus verdict
```

---

## Full Event Type Inventory

### globalEventBus (Core)
```javascript
JUDGMENT_CREATED
USER_FEEDBACK
PATTERN_DETECTED
ANOMALY_DETECTED
IDENTITY_VIOLATION
PERMISSION_VIOLATION
COST_ALERT
BUDGET_EXHAUSTED
MODEL_RECOMMENDATION
MEMORY_RESTORED
CONTEXT_COMPRESSED
// + forwarded from Agent + Automation buses
```

### getEventBus() (Automation)
```javascript
TRIGGER_FIRED
AUTOMATION_TICK
LEARNING_CYCLE_COMPLETE
JUDGMENT_CREATED  // forwarded from Core
DOG_SIGNAL        // manual from AmbientConsensus
// + others
```

### AgentEventBus (Dogs)
```javascript
// 39 event types (see agents/events.js)
DOG_SIGNAL
DOG_VOTE
CONSENSUS_REACHED
PATTERN_DETECTED
ANOMALY_DETECTED
CYNIC_DECISION
CYNIC_GUIDANCE
VULNERABILITY_DETECTED
REALITY_DRIFT_DETECTED
DEPLOY_COMPLETED
DEPLOY_FAILED
// + 28 more
```

---

## Files

| File | Purpose |
|------|---------|
| `packages/core/src/bus/event-bus.js` | globalEventBus + EventType enum |
| `packages/node/src/services/event-bus.js` | getEventBus() + EventType enum |
| `packages/node/src/agents/event-bus.js` | AgentEventBus class |
| `packages/node/src/agents/events.js` | AgentEvent enum (39 types) |
| `packages/node/src/services/event-bus-bridge.js` | EventBusBridge singleton |
| `packages/node/src/agents/collective/ambient-consensus.js` | Manual Dog signal bridging |

---

## φ Wisdom

**Why Three Buses?**
- **Separation of concerns**: Core / Automation / Dogs have different lifecycles
- **Performance**: Dogs emit 1000s of micro-events → isolated from core
- **Historical**: Grew organically as CYNIC evolved

**Why a Bridge?**
- **Before bridge**: Manual emits everywhere (brittle, error-prone)
- **After bridge**: Declarative routing rules (AGENT_TO_CORE, etc.)
- **Loop safety**: Automatic prevention of infinite forwarding

**Future Evolution**:
- Single unified bus with topic namespaces?
- Keep 3 buses but simplify bridge rules?
- **Current state**: 88% wiring complete, bridge works, don't fix what ain't broke

---

*"Le chien coordonne ses propres nerfs."*
