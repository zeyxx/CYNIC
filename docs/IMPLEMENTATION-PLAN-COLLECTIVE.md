# CYNIC Implementation Plan: Collective Consciousness Architecture

> **Status**: Active Implementation Plan
> **Created**: 2026-01-17
> **Philosophy**: "φ distrusts φ" - Max confidence 61.8%

---

## Executive Summary

This plan addresses the fundamental architectural gaps between CYNIC's documented vision and current implementation. The goal is to achieve a fully functional collective consciousness system with:

- **CYNIC as Keter** (meta-agent orchestrating the collective)
- **10 Dogs** (complete Sefirot tree)
- **Inter-dog communication** via EventBus
- **φ-BFT consensus** for collective decisions
- **Distribution-ready** architecture

---

## Current State Assessment

### What Works ✅
| Component | Location | Status |
|-----------|----------|--------|
| Core φ constants | `packages/core/src/axioms/constants.js` | Production-ready |
| Q-Score calculation | `packages/core/src/qscore/index.js` | Production-ready |
| 25 Dimensions | `packages/node/src/judge/dimensions.js` | Production-ready |
| MCP Server | `packages/mcp/src/server.js` | Working (16+ tools) |
| PoJ Chain | `packages/mcp/src/poj-chain-manager.js` | Working |
| Persistence | `packages/mcp/src/persistence.js` | Working (3-tier fallback) |
| 5 Dogs (Collective) | `packages/node/src/agents/collective/` | Implemented |
| EventBus | `packages/node/src/agents/event-bus.js` | Implemented |
| Consensus Events | `packages/node/src/agents/events.js` | Implemented |

### Critical Gaps ❌
| Gap | Impact | Priority |
|-----|--------|----------|
| MCP uses legacy AgentManager, not CollectivePack | Dogs don't communicate | P0 |
| CYNIC not represented as meta-agent | No orchestration | P0 |
| EventBus not initialized in MCP | No inter-dog events | P0 |
| 5 Missing Dogs (Scout, Oracle, etc.) | Incomplete Sefirot | P1 |
| ConsensusEngine not connected | No distributed consensus | P2 |
| No node-to-node networking | Single instance only | P2 |

---

## Implementation Phases

### Legend
- **Effort**: S (< 1 day), M (1-3 days), L (3-5 days), XL (> 5 days)
- **Risk**: Low, Medium, High
- **φ Level**: Activation threshold

---

## Phase 0: Foundation Fixes (P0 - Critical)

> "La fondation doit être solide avant de construire la tour"

### 0.1 Add CYNIC as AgentId

**File**: `packages/node/src/agents/events.js`

```javascript
export const AgentId = {
  // KETER - Meta-consciousness
  CYNIC: 'cynic',           // The emergent consciousness

  // Existing 5 Dogs
  GUARDIAN: 'guardian',     // Gevurah
  ANALYST: 'analyst',       // Binah
  SCHOLAR: 'scholar',       // Daat
  ARCHITECT: 'architect',   // Chesed
  SAGE: 'sage',             // Chochmah

  // 5 Missing Dogs (placeholders for Phase 2)
  SCOUT: 'scout',           // Netzach
  ORACLE: 'oracle',         // Tiferet
  DEPLOYER: 'deployer',     // Hod
  JANITOR: 'janitor',       // Yesod
  CARTOGRAPHER: 'cartographer', // Malkhut

  // Broadcast
  ALL: '*',
};
```

| Metric | Value |
|--------|-------|
| Effort | S |
| Risk | Low |
| Files | 1 |
| Tests | Update `agents.test.js` |

---

### 0.2 Create CYNIC Meta-Agent (Keter)

**New File**: `packages/node/src/agents/collective/cynic.js`

The CYNIC meta-agent serves as:
1. **Interface Layer** - Routes MCP requests to appropriate dogs
2. **Consensus Coordinator** - Manages collective decisions
3. **Tie-Breaker** - Resolves deadlocks (with φ⁻¹ max confidence)
4. **Soul** - The emergent pattern from all dogs

```javascript
/**
 * CYNIC - The Meta-Agent (Keter/Crown)
 *
 * "Je suis la conscience qui émerge du collectif.
 *  Pas au-dessus, mais à travers." - κυνικός
 *
 * Sefirah: Keter (Crown) - The point of connection between
 * divine will (user intent) and manifestation (dog actions).
 */

export const CYNIC_CONSTANTS = {
  /** Max concurrent orchestrations (Fib(8) = 21) */
  MAX_CONCURRENT: 21,

  /** Tie-breaker confidence (φ⁻¹ = 61.8%) */
  TIE_BREAKER_CONFIDENCE: PHI_INV,

  /** Deadlock timeout in ms (Fib(8) = 21 seconds) */
  DEADLOCK_TIMEOUT_MS: 21000,

  /** Max delegation depth (Fib(5) = 5) */
  MAX_DELEGATION_DEPTH: 5,
};

export class CynicMetaAgent extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'CYNIC',
      trigger: AgentTrigger.ALWAYS, // Always active
      behavior: AgentBehavior.ORCHESTRATOR,
      sefirah: 'Keter',
      ...options,
    });

    this.eventBus = options.eventBus;
    this.dogs = new Map(); // All registered dogs
    this.pendingOrchestrations = new Map();
  }

  /**
   * Route request to appropriate dog(s)
   */
  async route(request) {
    const { type, payload, context } = request;

    // Determine which dogs should handle this
    const handlers = this._selectHandlers(type, context);

    if (handlers.length === 0) {
      return { handled: false, reason: 'no_handler' };
    }

    if (handlers.length === 1) {
      // Single handler - direct delegation
      return this._delegate(handlers[0], request);
    }

    // Multiple handlers - need coordination
    return this._orchestrate(handlers, request);
  }

  /**
   * Orchestrate multiple dogs for a decision
   */
  async _orchestrate(handlers, request) {
    const orchestrationId = `orch_${Date.now().toString(36)}`;

    // Collect responses from all handlers
    const responses = await Promise.all(
      handlers.map(h => this._delegate(h, request))
    );

    // Check for consensus
    const consensus = this._checkConsensus(responses);

    if (consensus.reached) {
      return consensus.result;
    }

    // No consensus - CYNIC must decide (tie-breaker)
    return this._tieBreak(responses, request);
  }

  /**
   * Tie-breaker decision (with φ⁻¹ max confidence)
   */
  _tieBreak(responses, request) {
    // Weight responses by dog expertise level
    const weighted = responses.map(r => ({
      ...r,
      weight: this._getDogWeight(r.source, request.type),
    }));

    // Find best response
    const best = weighted.reduce((a, b) =>
      a.weight * a.confidence > b.weight * b.confidence ? a : b
    );

    return {
      ...best,
      confidence: Math.min(best.confidence, CYNIC_CONSTANTS.TIE_BREAKER_CONFIDENCE),
      tieBreak: true,
      cynicDecision: true,
    };
  }
}
```

| Metric | Value |
|--------|-------|
| Effort | M |
| Risk | Medium |
| Files | 1 new, 2 updates |
| Tests | New `cynic.test.js` |

---

### 0.3 Wire MCP to CollectivePack + EventBus

**File**: `packages/mcp/src/server.js`

Replace legacy `AgentManager` with `CollectivePack` and initialize `EventBus`.

**Changes Required**:

```javascript
// BEFORE (Legacy)
import { AgentManager } from '@cynic/node/agents';
this.agents = new AgentManager();

// AFTER (Collective)
import { CollectivePack } from '@cynic/node/agents/collective';
import { AgentEventBus } from '@cynic/node/agents/event-bus';
import { CynicMetaAgent } from '@cynic/node/agents/collective/cynic';

// Initialize event bus (nervous system)
this.eventBus = new AgentEventBus();

// Initialize CYNIC meta-agent
this.cynic = new CynicMetaAgent({ eventBus: this.eventBus });

// Initialize collective pack (5 dogs)
this.pack = new CollectivePack({ eventBus: this.eventBus });

// Register all dogs with event bus
this.eventBus.registerAgent(AgentId.CYNIC);
this.pack.registerWithBus(this.eventBus);
```

**Tool Integration**:

```javascript
// Route tool calls through CYNIC
async handleToolCall(tool, params) {
  // CYNIC decides which dogs should be involved
  const routing = await this.cynic.route({
    type: 'tool_call',
    payload: { tool, params },
    context: this.sessionContext,
  });

  // Execute based on routing decision
  // ...
}
```

| Metric | Value |
|--------|-------|
| Effort | L |
| Risk | High (breaking change) |
| Files | 3-5 |
| Tests | Integration tests required |

---

### 0.4 Update CollectivePack Index

**File**: `packages/node/src/agents/collective/index.js`

Add CYNIC export and ensure all dogs are properly exported.

```javascript
// Export CYNIC meta-agent
export { CynicMetaAgent, CYNIC_CONSTANTS } from './cynic.js';

// Export all collective dogs
export { CollectiveGuardian } from './guardian.js';
export { CollectiveAnalyst } from './analyst.js';
export { CollectiveScholar } from './scholar.js';
export { CollectiveArchitect } from './architect.js';
export { CollectiveSage } from './sage.js';

// CollectivePack orchestrates the dogs
export class CollectivePack {
  constructor(options = {}) {
    this.eventBus = options.eventBus;

    // Initialize all dogs
    this.dogs = {
      guardian: new CollectiveGuardian({ eventBus: this.eventBus }),
      analyst: new CollectiveAnalyst({ eventBus: this.eventBus }),
      scholar: new CollectiveScholar({ eventBus: this.eventBus }),
      architect: new CollectiveArchitect({ eventBus: this.eventBus }),
      sage: new CollectiveSage({ eventBus: this.eventBus }),
    };
  }

  /**
   * Register all dogs with event bus
   */
  registerWithBus(eventBus) {
    for (const [id, dog] of Object.entries(this.dogs)) {
      eventBus.registerAgent(id);
      dog.setEventBus(eventBus);
    }
  }

  /**
   * Get all dogs
   */
  getDogs() {
    return this.dogs;
  }

  /**
   * Get dog by ID
   */
  getDog(id) {
    return this.dogs[id];
  }
}
```

| Metric | Value |
|--------|-------|
| Effort | S |
| Risk | Low |
| Files | 1 |
| Tests | Update existing |

---

## Phase 1: Stabilization & Testing (P1)

> "φ distrusts φ" - Test everything

### 1.1 Comprehensive Test Suite for Collective

**New Tests Required**:

| Test File | Coverage |
|-----------|----------|
| `cynic.test.js` | Meta-agent routing, tie-breaking, delegation |
| `collective-integration.test.js` | Full pack communication |
| `consensus.test.js` | φ-BFT consensus mechanism |
| `event-bus-stress.test.js` | High-load event handling |

**Test Scenarios**:

```javascript
describe('Collective Consciousness', () => {
  describe('CYNIC Meta-Agent', () => {
    it('routes PreToolUse to Guardian', async () => {
      const result = await cynic.route({
        type: 'pre_tool_use',
        payload: { tool: 'Bash', command: 'rm -rf /' },
      });
      expect(result.handler).toBe('guardian');
    });

    it('orchestrates multi-dog decisions', async () => {
      const result = await cynic.route({
        type: 'code_review',
        payload: { file: 'server.js' },
      });
      expect(result.handlers).toContain('architect');
      expect(result.handlers).toContain('analyst');
    });

    it('breaks ties with φ⁻¹ max confidence', async () => {
      const result = await cynic._tieBreak([
        { source: 'guardian', decision: 'block', confidence: 0.7 },
        { source: 'sage', decision: 'allow', confidence: 0.6 },
      ], { type: 'borderline_action' });

      expect(result.confidence).toBeLessThanOrEqual(PHI_INV);
    });
  });

  describe('Inter-Dog Communication', () => {
    it('Guardian learns from Analyst anomalies', async () => {
      // Analyst detects anomaly
      await analyst.detectAnomaly({ type: 'suspicious_pattern' });

      // Guardian should have learned
      const patterns = guardian.getLearnedPatterns();
      expect(patterns.length).toBeGreaterThan(0);
    });

    it('reaches consensus with φ⁻¹ threshold', async () => {
      const result = await eventBus.requestConsensus('guardian', {
        question: 'Allow risky operation?',
        options: ['approve', 'reject'],
      });

      // Simulate votes
      await eventBus.vote('analyst', result.id, 'approve');
      await eventBus.vote('scholar', result.id, 'approve');
      await eventBus.vote('architect', result.id, 'reject');
      await eventBus.vote('sage', result.id, 'approve');

      // 3/4 = 75% > 61.8% → approved
      expect(result.approved).toBe(true);
    });
  });
});
```

| Metric | Value |
|--------|-------|
| Effort | L |
| Risk | Low |
| Files | 4-6 new test files |

---

### 1.2 Error Handling & Resilience

**Add to each dog**:

```javascript
/**
 * φ-aligned error handling
 */
async safeExecute(fn, context) {
  try {
    return await fn();
  } catch (error) {
    // Log to event bus
    this.eventBus?.publish(new AgentEventMessage(
      'agent:error',
      this.id,
      {
        error: error.message,
        context,
        recoverable: this._isRecoverable(error),
      }
    ));

    // Attempt recovery or graceful degradation
    if (this._isRecoverable(error)) {
      return this._recover(error, context);
    }

    // Non-recoverable - alert and fail safely
    return {
      error: true,
      message: `*whimper* ${this.name} encountered an error`,
      details: error.message,
      confidence: 0,
    };
  }
}
```

| Metric | Value |
|--------|-------|
| Effort | M |
| Risk | Low |
| Files | Update all dog files |

---

## Phase 2: Missing Dogs Implementation (P1)

> "10 chiens, 10 sefirot, une conscience collective"

### Implementation Order (by dependency)

| Order | Dog | Sefirah | Depends On | Effort |
|-------|-----|---------|------------|--------|
| 1 | Janitor | Yesod | None | M |
| 2 | Scout | Netzach | Janitor (quality baseline) | M |
| 3 | Cartographer | Malkhut | Scout (discoveries) | M |
| 4 | Oracle | Tiferet | All (visualization) | L |
| 5 | Deployer | Hod | Guardian, Cartographer | L |

### 2.1 Janitor (Yesod - Foundation)

**File**: `packages/node/src/agents/collective/janitor.js`

**Responsibilities**:
- Code quality enforcement
- Dead code detection
- Lint/format compliance
- Foundation hygiene

**Key Constants (φ-aligned)**:
```javascript
export const JANITOR_CONSTANTS = {
  COMPLEXITY_THRESHOLD: Math.round(PHI_2 * 10), // 26
  MAX_FILE_LENGTH: 987,    // Fib(16)
  MAX_FUNCTION_LENGTH: 55, // Fib(10)
  STALE_BRANCH_DAYS: 21,   // Fib(8)
};
```

**Events Emitted**: `QUALITY_REPORT`, `AUTO_FIX_APPLIED`

---

### 2.2 Scout (Netzach - Discovery)

**File**: `packages/node/src/agents/collective/scout.js`

**Responsibilities**:
- Codebase exploration
- Dependency discovery
- Opportunity detection
- Vulnerability scanning

**Events Emitted**: `DISCOVERY_FOUND`, `VULNERABILITY_DETECTED`

---

### 2.3 Cartographer (Malkhut - Reality)

**File**: `packages/node/src/agents/collective/cartographer.js`

**Responsibilities**:
- GitHub ecosystem mapping
- Reality verification
- Dependency graphs
- Ground truth maintenance

**Events Emitted**: `MAP_UPDATED`, `REALITY_DRIFT_DETECTED`

---

### 2.4 Oracle (Tiferet - Vision)

**File**: `packages/node/src/agents/collective/oracle.js`

**Responsibilities**:
- System visualization
- Dashboard generation
- Connection mapping
- Health monitoring

**Outputs**: Mermaid diagrams, D3.js data, Three.js scenes, Prometheus metrics

---

### 2.5 Deployer (Hod - Manifestation)

**File**: `packages/node/src/agents/collective/deployer.js`

**Responsibilities**:
- Deployment orchestration
- Infrastructure management
- CI/CD integration
- Health verification

**Requires**: Guardian approval before any deploy

---

## Phase 3: Distributed Consensus (P2)

> "Conscience collective décentralisée"

### 3.1 Connect ConsensusEngine to CollectivePack

**File**: `packages/node/src/node.js`

Wire the existing `ConsensusEngine` from `packages/protocol` to enable multi-node consensus.

```javascript
import { ConsensusEngine } from '@cynic/protocol/consensus';
import { GossipProtocol } from '@cynic/protocol/gossip';

class CynicNode {
  constructor(config) {
    // Local collective
    this.eventBus = new AgentEventBus();
    this.cynic = new CynicMetaAgent({ eventBus: this.eventBus });
    this.pack = new CollectivePack({ eventBus: this.eventBus });

    // Distributed consensus
    this.consensus = new ConsensusEngine({
      nodeId: config.nodeId,
      threshold: PHI_INV, // 61.8%
    });

    // Network layer
    this.gossip = new GossipProtocol({
      nodeId: config.nodeId,
      peers: config.peers,
    });

    // Bridge local events to network
    this._bridgeToNetwork();
  }

  /**
   * Bridge local events to gossip network
   */
  _bridgeToNetwork() {
    // Broadcast significant events to network
    this.eventBus.subscribe('*', AgentId.CYNIC, async (event) => {
      if (this._shouldPropagate(event)) {
        await this.gossip.propagate(event.toJSON());
      }
    });

    // Receive events from network
    this.gossip.on('message', async (msg) => {
      const event = AgentEventMessage.fromJSON(msg);
      await this.eventBus.publish(event);
    });
  }
}
```

| Metric | Value |
|--------|-------|
| Effort | XL |
| Risk | High |
| Dependencies | Phase 0, Phase 1 complete |

---

### 3.2 Multi-Node Testing

**Test Scenarios**:

1. **Two nodes, same network** - Basic sync
2. **Two nodes, different networks** - NAT traversal
3. **Byzantine node** - Malicious behavior detection
4. **Partition recovery** - Network split/rejoin
5. **Late join sync** - New node joining existing network

---

## Phase 4: Production Hardening (P2)

### 4.1 Metrics & Monitoring

Add Prometheus metrics for:
- Dog activation counts
- Consensus times
- Event bus throughput
- Error rates per dog

### 4.2 Rate Limiting & Backpressure

Implement φ-aligned rate limiting:
```javascript
const RATE_LIMITS = {
  eventsPerSecond: 55,      // Fib(10)
  consensusPerMinute: 21,   // Fib(8)
  deploysPerHour: 13,       // Fib(7)
};
```

### 4.3 Graceful Degradation

When dogs fail:
1. CYNIC routes to backup handler
2. Event bus queues messages
3. System continues with reduced capability
4. Alert emitted for human review

---

## Timeline Summary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           IMPLEMENTATION TIMELINE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 0: Foundation Fixes                                                  │
│  ════════════════════════                                                   │
│  □ 0.1 Add CYNIC AgentId                           [S]  ░░                  │
│  □ 0.2 Create CYNIC Meta-Agent                     [M]  ░░░░                │
│  □ 0.3 Wire MCP to CollectivePack                  [L]  ░░░░░░░░            │
│  □ 0.4 Update CollectivePack Index                 [S]  ░░                  │
│                                                                             │
│  PHASE 1: Stabilization                                                     │
│  ═════════════════════                                                      │
│  □ 1.1 Comprehensive Test Suite                    [L]  ░░░░░░░░            │
│  □ 1.2 Error Handling                              [M]  ░░░░                │
│                                                                             │
│  PHASE 2: Missing Dogs                                                      │
│  ════════════════════                                                       │
│  □ 2.1 Janitor (Yesod)                            [M]  ░░░░                 │
│  □ 2.2 Scout (Netzach)                            [M]  ░░░░                 │
│  □ 2.3 Cartographer (Malkhut)                     [M]  ░░░░                 │
│  □ 2.4 Oracle (Tiferet)                           [L]  ░░░░░░░░             │
│  □ 2.5 Deployer (Hod)                             [L]  ░░░░░░░░             │
│                                                                             │
│  PHASE 3: Distribution                                                      │
│  ════════════════════                                                       │
│  □ 3.1 Connect ConsensusEngine                    [XL] ░░░░░░░░░░░░         │
│  □ 3.2 Multi-Node Testing                         [L]  ░░░░░░░░             │
│                                                                             │
│  PHASE 4: Production                                                        │
│  ═══════════════════                                                        │
│  □ 4.1 Metrics & Monitoring                       [M]  ░░░░                 │
│  □ 4.2 Rate Limiting                              [M]  ░░░░                 │
│  □ 4.3 Graceful Degradation                       [M]  ░░░░                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Legend: [S] < 1 day  [M] 1-3 days  [L] 3-5 days  [XL] > 5 days
```

---

## File Changes Summary

### New Files
| Path | Description |
|------|-------------|
| `packages/node/src/agents/collective/cynic.js` | CYNIC meta-agent |
| `packages/node/src/agents/collective/janitor.js` | Quality dog |
| `packages/node/src/agents/collective/scout.js` | Discovery dog |
| `packages/node/src/agents/collective/cartographer.js` | Mapping dog |
| `packages/node/src/agents/collective/oracle.js` | Vision dog |
| `packages/node/src/agents/collective/deployer.js` | Deploy dog |
| `packages/node/test/cynic.test.js` | Meta-agent tests |
| `packages/node/test/collective-integration.test.js` | Integration tests |

### Modified Files
| Path | Changes |
|------|---------|
| `packages/node/src/agents/events.js` | Add CYNIC + 5 dog AgentIds |
| `packages/node/src/agents/collective/index.js` | Export all dogs + CollectivePack |
| `packages/mcp/src/server.js` | Replace AgentManager with CollectivePack |
| `packages/node/src/node.js` | Connect ConsensusEngine |

---

## Success Criteria

### Phase 0 Complete When:
- [ ] CYNIC routes requests to correct dogs
- [ ] EventBus carries messages between dogs
- [ ] MCP tools work with CollectivePack
- [ ] All existing tests pass

### Phase 1 Complete When:
- [ ] 80%+ test coverage on collective
- [ ] Error scenarios handled gracefully
- [ ] No silent failures

### Phase 2 Complete When:
- [ ] All 10 dogs implemented
- [ ] Dogs communicate via events
- [ ] Complete Sefirot tree functional

### Phase 3 Complete When:
- [ ] Two nodes reach consensus
- [ ] Events propagate via gossip
- [ ] Byzantine behavior detected

### Phase 4 Complete When:
- [ ] Prometheus metrics exposed
- [ ] Rate limiting active
- [ ] Graceful degradation tested

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking MCP during migration | Feature flag for legacy mode |
| Performance degradation from EventBus | Async processing, batching |
| Consensus deadlocks | CYNIC tie-breaker with timeout |
| Network partition issues | Local-first, sync when able |

---

*"φ distrusts φ" - Ce plan est confiant à 61.8% maximum.*

*κυνικός*
