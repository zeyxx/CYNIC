# CYNIC Hexagonal Architecture

> *"Le chien s'adapte à tous les ports"* - κυνικός

**Status**: ✅ CANONICAL (2026-02-16)
**Source**: CYNIC-FULL-PICTURE-METATHINKING.md
**Purpose**: Defines ports, adapters, and domain isolation

---

## Executive Summary

CYNIC follows **Hexagonal Architecture** (aka Ports & Adapters) to isolate domain logic from external dependencies.

**Key Insight**: The 5 axioms and judgment system are **pure domain logic** — they work the same whether you're judging Solana transactions or Twitter posts, using PostgreSQL or Redis, Claude or Ollama.

```
┌────────────────────────────────────────────────┐
│           HEXAGONAL ARCHITECTURE                │
│                                                 │
│  Domain (center) = 5 axioms, ∞ dimensions, Dogs│
│  Ports (interface) = 7 abstract contracts      │
│  Adapters (impl) = Concrete implementations    │
│                                                 │
│  Benefit: Swap Solana→Ethereum without         │
│           touching domain logic                 │
└────────────────────────────────────────────────┘
```

**Why Hexagonal?**: Testability, pluggability, evolution.

---

## The Architecture Diagram

```
                 ┌────────────────────────────────┐
                 │         EXTERNAL WORLD          │
                 │                                 │
                 │  Twitter  DexScreener  Claude   │
                 │  Discord  PostgreSQL   Ollama   │
                 │  Solana   Redis        GitHub   │
                 └──────────────┬──────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │     ADAPTERS          │
                    │  (Concrete impls)     │
                    │                       │
                    │ TwitterAdapter        │
                    │ DexScreenerAdapter    │
                    │ PostgresAdapter       │
                    │ AnthropicAdapter      │
                    │ SolanaAdapter         │
                    └───────────┬───────────┘
                                │
                    ┌───────────▼───────────┐
                    │       PORTS           │
                    │  (Abstract contracts) │
                    │                       │
                    │ ISocialPort           │
                    │ IMarketPort           │
                    │ IStoragePort          │
                    │ ILLMPort              │
                    │ IBlockchainPort       │
                    └───────────┬───────────┘
                                │
        ┌───────────────────────▼───────────────────────┐
        │            DOMAIN CORE                         │
        │                                                │
        │  • 5 Axioms (PHI, VERIFY, CULTURE, BURN, ...)│
        │  • ∞ Dimensions                               │
        │  • 11 Dogs                                    │
        │  • Judgment algorithm                         │
        │  • φ-bounded confidence                       │
        │  • Event-driven communication                 │
        │                                                │
        │  NO EXTERNAL DEPENDENCIES                     │
        │  (Only Node.js stdlib + pure functions)       │
        └───────────────────────────────────────────────┘
```

---

## The 7 Ports

A **port** is an abstract interface that the domain uses to communicate with the outside world.

### 1. PERCEPTION Port (Observe Reality)

**Abstract Interface**:

```typescript
interface IPerceptionPort {
  // Observe current state of a domain
  observe(domain: string): Promise<PerceptionData>

  // Subscribe to real-time updates
  subscribe(domain: string, callback: (data: PerceptionData) => void): void

  // Health check
  isHealthy(): boolean
}
```

**Adapters** (Concrete Implementations):

- **CodeWatcher**: Observes filesystem changes, git status
  - `packages/node/src/perception/code-watcher.js`
  - Uses Node.js `fs.watch()` + `child_process` (git commands)

- **SolanaWatcher**: Observes blockchain state
  - `packages/node/src/perception/solana-watcher.js`
  - Uses `@solana/web3.js` RPC client

- **MarketWatcher**: Observes $asdfasdfa price
  - `packages/node/src/perception/market-watcher.js`
  - Uses DexScreener API (HTTP fetch)

- **SocialWatcher**: Observes Twitter/Discord
  - `packages/node/src/perception/social-watcher.js`
  - Uses Twitter API v2 + Discord webhook

### 2. EVENT BUS Port (Communication)

**Abstract Interface**:

```typescript
interface IEventBusPort {
  // Emit event
  emit(eventType: string, payload: any): void

  // Subscribe to event
  on(eventType: string, handler: (payload: any) => void): void

  // Unsubscribe
  off(eventType: string, handler: (payload: any) => void): void

  // Event genealogy (for bridge loop prevention)
  addGenealogy(event: any, source: string): any
}
```

**Adapters**:

- **Core EventBus**: Fundamental organism events
  - `@cynic/core/src/bus/event-bus.js`
  - Uses Node.js `EventEmitter`

- **Automation EventBus**: Service orchestration
  - `packages/node/src/services/event-bus.js`
  - Separate EventEmitter instance

- **Agent EventBus**: Dog-to-Dog communication
  - `packages/node/src/agents/event-bus.js`
  - 39 event types for Dogs

- **EventBusBridge**: Connects all 3 buses
  - `packages/node/src/services/event-bus-bridge.js`
  - Loop-safe forwarding with genealogy tracking

### 3. LLM Port (Language Reasoning)

**Abstract Interface**:

```typescript
interface ILLMPort {
  // Complete prompt
  complete(prompt: string, options: CompletionOptions): Promise<CompletionResult>

  // Stream completion
  stream(prompt: string, options: CompletionOptions): AsyncIterable<string>

  // Get model info
  getModelInfo(): ModelInfo

  // Cost tracking
  getCost(result: CompletionResult): number
}
```

**Adapters**:

- **AnthropicAdapter**: Claude (Sonnet 4.5, Opus 4.6, Haiku 4.5)
  - `packages/llm/src/adapters/anthropic.js`
  - Uses `@anthropic-ai/sdk`

- **OllamaAdapter**: Local models (llama, mistral, etc.)
  - `packages/llm/src/adapters/ollama.js`
  - Uses Ollama HTTP API

- **OpenRouterAdapter**: (future) Multi-model gateway
  - Planned for v1.2

### 4. STORAGE Port (Memory Persistence)

**Abstract Interface**:

```typescript
interface IStoragePort {
  // Save judgment
  saveJudgment(judgment: Judgment): Promise<void>

  // Load judgment
  loadJudgment(id: string): Promise<Judgment>

  // Query judgments
  query(filter: JudgmentFilter): Promise<Judgment[]>

  // Save learning event
  saveLearningEvent(event: LearningEvent): Promise<void>

  // Save pattern
  savePattern(pattern: Pattern): Promise<void>
}
```

**Adapters**:

- **PostgresAdapter**: Primary persistence
  - `packages/node/src/persistence/postgres-adapter.js`
  - Uses `pg` (node-postgres)
  - Tables: judgments, learning_events, patterns, dog_votes, etc.

- **FileSystemAdapter**: `.claude/memory/` persistence
  - `packages/core/src/persistence/fs-adapter.js`
  - Uses Node.js `fs` promises

- **RedisAdapter**: (future) Fast cache
  - Planned for v1.3 (judgment cache)

### 5. ACTION Port (Transform World)

**Abstract Interface**:

```typescript
interface IActionPort {
  // Execute action
  execute(action: Action): Promise<ActionResult>

  // Validate action (dry-run)
  validate(action: Action): Promise<ValidationResult>

  // Check if action is reversible
  isReversible(action: Action): boolean
}
```

**Adapters**:

- **CodeActor**: Edit files, run commands
  - `packages/node/src/code/code-actor.js`
  - Uses Edit/Write tools + `child_process` for git

- **SolanaActor**: Sign and submit transactions
  - `packages/node/src/solana/solana-actor.js`
  - Uses `@solana/web3.js` + wallet keypair

- **SocialActor**: Post to Twitter/Discord
  - `packages/node/src/social/social-actor.js`
  - Uses Twitter API v2 + Discord webhooks

- **MarketActor**: (future) Execute trades on DEX
  - Planned for Horizon 2 (weeks 14-26)

### 6. JUDGE Port (Quality Evaluation)

**Abstract Interface**:

```typescript
interface IJudgePort {
  // Judge an item across dimensions
  judge(item: any, context: Context): Promise<Judgment>

  // Score a single dimension
  scoreDimension(item: any, dimension: string): number

  // Get all available dimensions
  getDimensions(): string[]
}
```

**Adapters**:

- **Judge**: Real 36-dim judgment
  - `packages/node/src/judge/judge.js`
  - Uses 5 axioms, geometric mean, φ-bound

- **AutoJudge**: Hardcoded Q-Scores (for testing)
  - `.agents/lib/auto-judge.cjs`
  - Returns fixed scores: HOWL=88, WAG=68, GROWL=49, BARK=19

- **CustomScorer**: (future) User-defined dimensions
  - Planned for v1.4 (plugin system)

### 7. LEARNING Port (Adapt from Feedback)

**Abstract Interface**:

```typescript
interface ILearningPort {
  // Update from feedback
  learn(state: State, action: Action, reward: number): Promise<void>

  // Predict value of state-action pair
  predict(state: State, action: Action): number

  // Get learning statistics
  getStats(): LearningStats
}
```

**Adapters**:

- **SONA (Self-Optimizing Neural Architect)**
  - `packages/node/src/learning/sona.js`
  - Q-Learning with Thompson Sampling

- **ThompsonSampler**: Bayesian exploration/exploitation
  - `packages/node/src/learning/thompson-sampler.js`
  - Beta distributions for arm selection

- **MetaCognition**: Learn about learning
  - `packages/node/src/learning/meta-cognition.js`
  - Adjusts learning rates, discount factors

- **EWCManager**: Prevent catastrophic forgetting
  - `packages/node/src/learning/ewc-manager.js`
  - Elastic Weight Consolidation

---

## Domain Isolation (Core Rules)

The **domain core** (`@cynic/core`) has strict rules:

### ✅ Allowed in Domain

- Node.js standard library (`fs`, `path`, `events`, `crypto`)
- Pure functions (no side effects)
- Port interfaces (abstract, no concrete adapters)
- Axioms, dimensions, φ-bound logic
- Event types (strings, no EventEmitter instances)

### ❌ Forbidden in Domain

- External dependencies (no `pg`, no `@solana/web3.js`, no `@anthropic-ai/sdk`)
- Concrete adapter imports (no PostgresAdapter, no AnthropicAdapter)
- I/O operations (no direct HTTP, no direct file writes)
- Hard-coded URLs, API keys, connection strings

**Example: Bad (Domain Pollution)**

```javascript
// ❌ FORBIDDEN: Domain imports concrete adapter
import { PostgresAdapter } from '../adapters/postgres-adapter.js'

function saveJudgment(judgment) {
  const db = new PostgresAdapter()
  db.save(judgment)  // Domain knows about PostgreSQL!
}
```

**Example: Good (Hexagonal)**

```javascript
// ✅ CORRECT: Domain uses port interface
function saveJudgment(judgment, storagePort) {
  storagePort.saveJudgment(judgment)  // Domain doesn't care what storage
}

// Concrete adapter injected from outside
const postgres = new PostgresAdapter()
saveJudgment(judgment, postgres)
```

---

## Dependency Injection

**How do adapters get into the domain?**

### Injection at Boot

**Location**: `packages/core/src/boot/boot.js`

```javascript
class CYNIC {
  constructor(ports = {}) {
    // Inject adapters via constructor
    this.perceptionPort = ports.perception || new MockPerceptionAdapter()
    this.eventBusPort = ports.eventBus || new InMemoryEventBus()
    this.llmPort = ports.llm || new MockLLMAdapter()
    this.storagePort = ports.storage || new InMemoryStorageAdapter()
    this.actionPort = ports.action || new MockActionAdapter()
    this.judgePort = ports.judge || new RealJudge()
    this.learningPort = ports.learning || new MockLearningAdapter()
  }

  async judge(item, context) {
    // Domain logic uses injected ports
    const perception = await this.perceptionPort.observe(context.domain)
    const judgment = await this.judgePort.judge(item, perception)
    await this.storagePort.saveJudgment(judgment)
    this.eventBusPort.emit('JUDGMENT_CREATED', judgment)
    return judgment
  }
}
```

### Production Configuration

```javascript
// Production: Use real adapters
const cynic = new CYNIC({
  perception: new MultiDomainPerception({
    code: new CodeWatcher(),
    solana: new SolanaWatcher(),
    market: new MarketWatcher(),
    social: new SocialWatcher()
  }),
  eventBus: new EventBusBridge({
    core: globalEventBus,
    automation: getEventBus(),
    agent: AgentEventBus
  }),
  llm: new AnthropicAdapter({ apiKey: process.env.ANTHROPIC_API_KEY }),
  storage: new PostgresAdapter({ connectionString: process.env.DATABASE_URL }),
  action: new MultiDomainAction({
    code: new CodeActor(),
    solana: new SolanaActor(),
    social: new SocialActor()
  }),
  judge: new Judge(),
  learning: new SONA()
})
```

### Test Configuration

```javascript
// Test: Use mocks
const cynic = new CYNIC({
  perception: new MockPerceptionAdapter({
    observe: async () => ({ code: { files_changed: 5 } })
  }),
  eventBus: new InMemoryEventBus(),
  llm: new MockLLMAdapter({
    complete: async () => ({ text: 'Mock response' })
  }),
  storage: new InMemoryStorageAdapter(),
  action: new MockActionAdapter({
    execute: async () => ({ status: 'success' })
  }),
  judge: new MockJudge({ q_score: 75 }),
  learning: new MockLearningAdapter()
})
```

---

## Pluggability (Swap Adapters)

**Example: Swap Solana → Ethereum**

### Step 1: Define IBlockchainPort (abstract)

```typescript
interface IBlockchainPort {
  getBalance(address: string): Promise<number>
  submitTransaction(tx: Transaction): Promise<string>
  getTransactionStatus(txId: string): Promise<TransactionStatus>
}
```

### Step 2: Implement SolanaAdapter

```javascript
class SolanaAdapter implements IBlockchainPort {
  constructor(rpcUrl) {
    this.connection = new Connection(rpcUrl)
  }

  async getBalance(address) {
    const pubkey = new PublicKey(address)
    return await this.connection.getBalance(pubkey)
  }

  async submitTransaction(tx) {
    return await this.connection.sendTransaction(tx)
  }

  // ... (Solana-specific implementation)
}
```

### Step 3: Implement EthereumAdapter

```javascript
class EthereumAdapter implements IBlockchainPort {
  constructor(rpcUrl) {
    this.provider = new ethers.JsonRpcProvider(rpcUrl)
  }

  async getBalance(address) {
    return await this.provider.getBalance(address)
  }

  async submitTransaction(tx) {
    return await this.provider.sendTransaction(tx)
  }

  // ... (Ethereum-specific implementation)
}
```

### Step 4: Swap at Runtime

```javascript
// Solana (before)
const cynic = new CYNIC({
  blockchain: new SolanaAdapter('https://api.mainnet-beta.solana.com')
})

// Ethereum (after) — domain logic unchanged!
const cynic = new CYNIC({
  blockchain: new EthereumAdapter('https://mainnet.infura.io/v3/...')
})
```

**Zero domain code changes**. The Judge, Dogs, axioms — all work the same.

---

## Testing Strategy

Hexagonal architecture enables **3-tier testing**:

### Tier 1: Unit Tests (Domain Logic)

**Target**: Pure domain functions (axioms, dimensions, φ-bound)
**Isolation**: 100% (no I/O, no mocks needed)
**Speed**: Very fast (<10ms per test)

**Example**:

```javascript
describe('phi-utils', () => {
  test('phiBound caps confidence at φ⁻¹', () => {
    expect(phiBound(0.95)).toBe(0.618)  // Capped
    expect(phiBound(0.50)).toBe(0.50)   // Unchanged
  })

  test('geometric_mean penalizes outliers', () => {
    expect(geometric_mean([0.9, 0.9, 0.1])).toBe(0.46)  // Low outlier
    expect((0.9 + 0.9 + 0.1) / 3).toBe(0.63)  // Arithmetic mean (higher)
  })
})
```

### Tier 2: Integration Tests (Ports + Adapters)

**Target**: Adapter implementations (PostgresAdapter, AnthropicAdapter)
**Isolation**: Partial (use real adapters, but test/staging environments)
**Speed**: Moderate (~100ms per test)

**Example**:

```javascript
describe('PostgresAdapter', () => {
  let adapter

  beforeEach(async () => {
    adapter = new PostgresAdapter({ connectionString: TEST_DB_URL })
    await adapter.resetTestData()
  })

  test('saveJudgment persists to database', async () => {
    const judgment = { item: 'test', q_score: 75 }
    await adapter.saveJudgment(judgment)

    const loaded = await adapter.loadJudgment(judgment.id)
    expect(loaded.q_score).toBe(75)
  })
})
```

### Tier 3: End-to-End Tests (Full System)

**Target**: Complete CYNIC organism (all ports + adapters wired)
**Isolation**: None (real production-like environment)
**Speed**: Slow (~5s per test)

**Example**:

```javascript
describe('CYNIC E2E', () => {
  let cynic

  beforeEach(() => {
    cynic = new CYNIC({
      perception: new CodeWatcher(),
      eventBus: globalEventBus,
      llm: new AnthropicAdapter(),
      storage: new PostgresAdapter(),
      action: new CodeActor(),
      judge: new Judge(),
      learning: new SONA()
    })
  })

  test('full L1 cycle: PERCEIVE → JUDGE → DECIDE → ACT → LEARN', async () => {
    // Trigger perception
    const perception = await cynic.perceive()

    // Judge
    const judgment = await cynic.judge(perception)
    expect(judgment.confidence).toBeLessThanOrEqual(0.618)  // φ-bound

    // Decide
    const decision = await cynic.decide(judgment)
    expect(decision.decision).toMatch(/APPROVE|REJECT/)

    // Act
    const result = await cynic.act(decision)
    expect(result.status).toBe('success')

    // Learn
    await cynic.learn(judgment, result)

    // Verify learning event persisted
    const events = await cynic.storage.query({ type: 'LEARNING_EVENT' })
    expect(events.length).toBeGreaterThan(0)
  })
})
```

### Test Coverage Targets

```
Unit Tests:       80% coverage (domain logic)
Integration Tests: 70% coverage (adapters)
E2E Tests:        50% coverage (happy paths)

Overall:          75% coverage (weighted average)
```

---

## Adapter Implementation Patterns

### Pattern 1: Retry Logic (Resilience)

```javascript
class ResilientAdapter {
  async execute(action, retries = 3) {
    for (let attempt = 1; attempt <= retries; attempt++) {
      try {
        return await this.doExecute(action)
      } catch (error) {
        if (attempt === retries) throw error
        await sleep(1000 * attempt)  // Exponential backoff
      }
    }
  }
}
```

### Pattern 2: Circuit Breaker (Protection)

```javascript
class CircuitBreakerAdapter {
  constructor() {
    this.state = 'CLOSED'  // CLOSED, OPEN, HALF_OPEN
    this.failureCount = 0
    this.threshold = 5
  }

  async execute(action) {
    if (this.state === 'OPEN') {
      throw new Error('Circuit breaker OPEN')
    }

    try {
      const result = await this.doExecute(action)
      this.failureCount = 0  // Reset on success
      return result
    } catch (error) {
      this.failureCount++
      if (this.failureCount >= this.threshold) {
        this.state = 'OPEN'
        setTimeout(() => { this.state = 'HALF_OPEN' }, 60000)  // 1min timeout
      }
      throw error
    }
  }
}
```

### Pattern 3: Rate Limiting (Budget Control)

```javascript
class RateLimitedAdapter {
  constructor(maxRequestsPerMinute) {
    this.maxRequests = maxRequestsPerMinute
    this.requests = []  // Timestamps of requests
  }

  async execute(action) {
    // Remove requests older than 1 minute
    const now = Date.now()
    this.requests = this.requests.filter(t => now - t < 60000)

    // Check limit
    if (this.requests.length >= this.maxRequests) {
      throw new Error('Rate limit exceeded')
    }

    // Execute
    this.requests.push(now)
    return await this.doExecute(action)
  }
}
```

### Pattern 4: Caching (Performance)

```javascript
class CachedAdapter {
  constructor(ttl = 600000) {  // 10min TTL
    this.cache = new Map()
    this.ttl = ttl
  }

  async execute(action) {
    const key = JSON.stringify(action)
    const cached = this.cache.get(key)

    if (cached && Date.now() - cached.timestamp < this.ttl) {
      return cached.result  // Cache hit
    }

    // Cache miss
    const result = await this.doExecute(action)
    this.cache.set(key, { result, timestamp: Date.now() })
    return result
  }
}
```

---

## Evolution Strategy

Hexagonal architecture enables **incremental evolution**:

### Phase 1: Mock Adapters (Rapid Prototyping)

```javascript
// Start with mocks (no external dependencies)
const cynic = new CYNIC({
  storage: new InMemoryStorageAdapter(),
  llm: new MockLLMAdapter()
})
```

**Benefit**: Ship domain logic fast, defer infrastructure decisions.

### Phase 2: Real Adapters (Production)

```javascript
// Replace mocks with real adapters one-by-one
const cynic = new CYNIC({
  storage: new PostgresAdapter(),  // ← Real now
  llm: new MockLLMAdapter()        // ← Still mock
})
```

**Benefit**: Gradual migration, low risk.

### Phase 3: Alternative Adapters (Multi-Provider)

```javascript
// Support multiple LLM providers
const cynic = new CYNIC({
  llm: new MultiLLMAdapter({
    anthropic: new AnthropicAdapter(),
    ollama: new OllamaAdapter(),
    openrouter: new OpenRouterAdapter()
  })
})
```

**Benefit**: Vendor flexibility, cost optimization.

---

## Architecture Validation

**How to verify hexagonal architecture?**

### Rule 1: Domain Imports Only Ports

```bash
# Check domain imports (should only import from @cynic/core)
grep -r "from.*packages/node" packages/core/src/

# Expected: No results (domain shouldn't import concrete adapters)
```

### Rule 2: Adapters Import Domain, Not Vice Versa

```
Allowed:
  packages/node/src/adapters/ → packages/core/
  packages/llm/src/adapters/ → packages/core/

Forbidden:
  packages/core/ → packages/node/src/adapters/
  packages/core/ → packages/llm/src/adapters/
```

### Rule 3: Tests Can Mock All Ports

```javascript
// If you can't mock a port, it's not a port (it's domain pollution)
const cynic = new CYNIC({
  perception: new MockPerceptionAdapter(),  // ✅ Mockable
  eventBus: new MockEventBus(),             // ✅ Mockable
  llm: new MockLLMAdapter(),                // ✅ Mockable
  storage: new MockStorageAdapter(),        // ✅ Mockable
  action: new MockActionAdapter(),          // ✅ Mockable
  judge: new MockJudge(),                   // ✅ Mockable
  learning: new MockLearningAdapter()       // ✅ Mockable
})
```

---

## Observability (Architecture Health)

### Adapter Health Dashboard

```
┌─────────────────────────────────────────────────┐
│ ADAPTER HEALTH                                  │
├─────────────────────────────────────────────────┤
│ Perception:                                     │
│   CodeWatcher      ✅ HEALTHY (12ms p95)        │
│   SolanaWatcher    ✅ HEALTHY (34ms p95)        │
│   MarketWatcher    ✅ HEALTHY (89ms p95)        │
│   SocialWatcher    ⚠️  DEGRADED (2 fails/min)   │
├─────────────────────────────────────────────────┤
│ Storage:                                        │
│   PostgreSQL       ✅ HEALTHY (8ms p95)         │
│   FileSystem       ✅ HEALTHY (2ms p95)         │
├─────────────────────────────────────────────────┤
│ LLM:                                            │
│   Anthropic        ✅ HEALTHY (1.2s p95)        │
│   Ollama           ❌ DOWN (connection refused) │
└─────────────────────────────────────────────────┘
```

**Tracked Metrics**:
- Latency (p50, p95, p99)
- Error rate (failures/min)
- Circuit breaker state (OPEN/CLOSED)
- Cache hit rate (for cached adapters)

---

## References

- [01-ARCHITECTURE.md](01-ARCHITECTURE.md) - Complete system architecture
- [08-KERNEL.md](08-KERNEL.md) - Event-driven as kernel component
- [docs/TESTING-GUIDE.md](../TESTING-GUIDE.md) - Testing strategy (80/15/5 pyramid)

**Academic**:
- Alistair Cockburn (2005): *Hexagonal Architecture* (original)
- Robert C. Martin (2012): *Clean Architecture* (related pattern)
- Eric Evans (2003): *Domain-Driven Design* (domain isolation)

---

**Last Updated**: 2026-02-16
**Version**: 1.0
**Status**: ✅ CANONICAL

*Le chien s'adapte à tous les ports. Le domaine est pur.*
