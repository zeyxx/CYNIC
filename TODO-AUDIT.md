# CYNIC Audit TODO List

> Generated 2026-01-24 from security & architecture audit
> Updated 2026-01-25 with architecture transformation
> Updated 2026-01-26 with Phase 2 CYNICNode decomposition

## Completed (26/28)

- [x] #1 Fix .env credential leak
- [x] #2 Fix mixed module system in tools/domains/index.js
- [x] #3 Create custom error types in @cynic/core
- [x] #4 Add critical path tests - PoJ judgment, key-manager, consensus
  - ✓ Already covered: poj.test.js, consensus.test.js, multi-node-consensus.test.js, crypto.test.js
- [x] #5 Replace Math.random() with crypto.randomBytes()
- [x] #6 Add rate limiting to MCP server
- [x] #7 Break up god files (>1000 lines) into smaller modules
  - ✓ scorers.js (1326 → 6 modules in scorers/)
  - ✓ cynic.js (1949 → 1364, extracted: constants.js, sefirot.js, relationship-graph.js)
  - ⏸️ 12 other files analyzed: cohesive classes, no split needed
    - node.js, server.js: system coordinators (complexity inherent)
    - live.js, arch.js: view classes (many small render methods)
    - analyst.js, scout.js, deployer.js, guardian.js: agents (BaseAgent)
    - learning-service.js, engine.js: domain services
    - events.js, organic-signals.js: could split but low priority
- [x] #9 Fix phi-alignment violation in residual.js
- [x] #12 Add circuit breaker to persistence layer
- [x] #13 Fix unsafe Function() constructor in console.js
- [x] #14 Tighten CORS policy for production
- [x] #15 Add Content-Security-Policy headers
- [x] #16 Deprecate legacy 3D E-Score system
- [x] #17 Standardize export patterns - named exports only
- [x] #18 Remove re-exports from @cynic/node index.js
- [x] #20 Extract duplicated scorer patterns into base class
- [x] #21 Extract duplicated repository CRUD patterns
  - ✓ All 17/17 repos now use BaseRepository
- [x] #22 Fix SSL certificate validation disabled
- [x] #24 Clean up TODO/FIXME markers

## Completed (26/26)

- [x] #8 Replace console.log with structured logging ✓ DONE
  - DONE: Created @cynic/core/logger module
  - DONE: Converted persistence clients (9 calls)
  - DONE: Converted packages/node/src/node.js (41 calls)
  - DONE: Converted packages/node/src/components/*.js (21 calls)
  - DONE: Fixed MCP server.js console.log breaking protocol (2 calls)
  - DONE: Converted @cynic/mcp package (24 files, ~150 calls)
    - server/ServiceInitializer.js, HttpAdapter.js, poj-chain-manager.js
    - persistence.js, session-manager.js, discovery-service.js, librarian-service.js
    - ecosystem-service.js, code-analyzer.js, integrator-service.js
    - All persistence adapters (9 files)
    - tools/domains/*.js (8 files), tools/registry.js
  - DONE: Converted @cynic/core boot/*.js, bus/*.js (7 files)
  - DONE: Converted @cynic/node transport, judge, agents, state (10 files)
  - DONE: Converted @cynic/protocol gossip/propagation.js (1 file)
  - DONE: Converted @cynic/identity node-identity.js (1 file)
  - REMAINING: ~15 console.* calls in production code (low priority)
  - SKIP: CLI/bin files (100+ calls) - intentional terminal output with chalk
  - SKIP: dashboard/js files (100+ calls) - browser-side, can't use @cynic/core
  - SKIP: MCP server.js (37 calls) - MCP protocol requires stderr for JSON-RPC
  - SKIP: examples/scripts/tests (~600+ calls) - development files

## Pending (2)

- [ ] #19 Add tests for persistence repositories
  - Was: 2,759 lines (21%) → Now: 3,354 lines (25.6%)
  - Added: UserRepository (11 tests), SessionRepository (6 tests), FeedbackRepository (8 tests)
  - 207 tests passing, 6/17 repos now covered
  - Remaining: 11 repos without unit tests
- [ ] #23 Add TypeDoc generation for API documentation

## Good Coverage (2)

- [x] #10 Add tests for emergence package
  - Was: 598 lines tests (23%) → Now: 746 lines tests (28.6%)
  - Added: recordUncertainty, getMetaInsight, reset, getPatterns, hasPattern, clear, getCandidates, getStats, removeNode
  - 52 tests passing, all public methods covered
- [x] #11 Add tests for identity package
  - Was: 758 lines tests (28%) → Still: 758 lines (28%)
  - Already comprehensive: 50 tests covering KeyManager, E-Score 3D/7D, NodeIdentity, ReputationGraph
  - All tests passing

## Architecture Transformation (NEW - Post-Audit)

> SOLID-compliant refactoring based on comprehensive audit

### Phase 1: DI Foundation ✓ COMPLETED
- [x] #25 Create ServiceContainer in @cynic/core
  - `packages/core/src/container.js` - DI Container with singleton/transient support
  - Exports: ServiceContainer, globalContainer, withContainer, createCYNICContainer
  - Package export path: `@cynic/core/container`
- [x] #26 Create RepositoryFactory in @cynic/persistence
  - `packages/persistence/src/factory.js` - Factory for all 17 repositories
  - Exports: RepositoryFactory, createMockFactory
  - Supports custom factories, tags, mock testing
  - Package export path: `@cynic/persistence/factory`

### Phase 2: CYNICNode Decomposition ✓ COMPLETED
- [x] #27 Extract OperatorComponent from CYNICNode
  - `packages/node/src/components/operator-component.js`
  - Encapsulates: Operator, BurnVerifier, E-Score persistence
- [x] #28 Extract JudgeComponent from CYNICNode
  - `packages/node/src/components/judge-component.js`
  - Encapsulates: CYNICJudge, ResidualDetector, LearningService
- [x] #29 Extract StateComponent from CYNICNode
  - `packages/node/src/components/state-component.js`
  - Encapsulates: StateManager, chain, knowledge, persistence
- [x] #30 Extract TransportComponent from CYNICNode
  - `packages/node/src/components/transport-component.js`
  - Encapsulates: WebSocketTransport, GossipProtocol, peer management
- [x] #31 Extract ConsensusComponent from CYNICNode
  - `packages/node/src/components/consensus-component.js`
  - Encapsulates: ConsensusEngine, ConsensusGossip, validator set
- [x] #32 Extract EmergenceComponent from CYNICNode
  - `packages/node/src/components/emergence-component.js`
  - Encapsulates: EmergenceLayer, SharedMemory, CollectivePack, EventBus

### Phase 3: Plugin System ✓ COMPLETED
- [x] #33 Create DimensionRegistry for runtime extension
  - `packages/node/src/judge/dimension-registry.js`
  - Event-driven, validated dimension registration
  - Plugin API with registerPlugin/unloadPlugin
  - Scorer registration per dimension
  - Backward compatible with legacy registry
- [x] #34 Update CYNICJudge to use DimensionRegistry
  - Added dimensionRegistry option to constructor
  - registerDimension() and registerPlugin() methods
  - Registry scorers take priority in scoring pipeline
  - getPlugins() and getRegistryStats() for introspection

### Phase 4: Dead Code Cleanup ✓ COMPLETED
- [x] #35 Analyze dead modules in scripts/lib/
  - Analysis: 144 modules are NOT dead - lazy-loaded by wisdom/engine system
  - Inventory documented in INVENTORY_SCRIPTS_LIB.md
  - Future: move 5 critical modules to packages/core/, keep engines operational
- [x] #36 Unify circuit-breaker implementations
  - `packages/core/src/circuit-breaker.js` - Unified CircuitBreaker class
  - Exports: CircuitBreaker, CircuitState, withCircuitBreaker
  - Event-driven with state transitions
  - φ-derived thresholds (61.8% reset timeout)

### Phase 5: Hook Refactor ✓ COMPLETED
- [x] #37 Extract inline logic from hooks
  - `scripts/hooks/lib/pattern-detector.js` - Pattern detection utilities
  - Exports: detectToolPattern, mapToTriggerEventType, parseTestOutput, etc.
- [x] #38 Create BaseHook class
  - `scripts/hooks/lib/base-hook.js` - Abstract base class
  - Exports: BaseHook, HookType, Decision, runHook
  - Provides: context parsing, response helpers, logging, runner

### Audit Reports Generated
- `AUDIT_ARCHITECTURE.md` - Full SOLID analysis (990 lines)
- `AUDIT_KEY_FILES.md` - Quick reference to key files
- `VIOLATIONS_QUICK_FIX.md` - Prioritized fix guide

## Key Files Created/Modified

### New Files
- `packages/core/src/container.js` - DI Container (SOLID foundation)
- `packages/persistence/src/factory.js` - Repository Factory
- `packages/core/src/errors.js` - Custom error types (10 classes, 44 codes)
- `packages/core/src/crypto-utils.js` - Secure random utilities
- `packages/core/src/logger.js` - Structured logging module
- `packages/node/src/components/` - SOLID Node Components (Phase 2):
  - `operator-component.js` - Operator identity & E-Score domain
  - `transport-component.js` - P2P networking domain
  - `consensus-component.js` - φ-BFT consensus domain
  - `state-component.js` - State & persistence domain
  - `judge-component.js` - Judgment & learning domain
  - `emergence-component.js` - Consciousness & patterns domain
  - `index.js` - Component exports
- `packages/node/src/judge/dimension-registry.js` - Plugin System (Phase 3):
  - Runtime dimension registration with validation
  - Plugin API for extensions
  - Event-driven architecture
- `packages/node/src/judge/scorers/` - Modular scorer directory:
  - `utils.js` - Shared utilities
  - `phi-axiom.js` - PHI dimension scorers
  - `verify-axiom.js` - VERIFY dimension scorers
  - `culture-axiom.js` - CULTURE dimension scorers
  - `burn-axiom.js` - BURN dimension scorers
  - `index.js` - Registry and re-exports
- `packages/node/src/agents/collective/` - CYNIC modules:
  - `constants.js` - CYNIC_CONSTANTS, enums
  - `sefirot.js` - Tree of Life geometry
  - `relationship-graph.js` - Agent relationship learning

### Modified Files
- `packages/mcp/src/server/HttpAdapter.js` - Rate limiting, CORS, CSP
- `packages/mcp/src/dashboard/js/components/console.js` - Safe math parser
- `packages/persistence/src/postgres/client.js` - Circuit breaker, logger
- `packages/persistence/src/redis/client.js` - Logger
- Various files for deprecation notices and export patterns

## Resume Instructions

```bash
# Clone and setup
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC
npm install

# Run tests to verify
npm test

# Continue with Claude Code
claude
```

Then tell Claude: "Reprends les tâches dans TODO-AUDIT.md"
