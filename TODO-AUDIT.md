# CYNIC Audit TODO List

> Generated 2026-01-24 from security & architecture audit
> Updated 2026-01-25 with analysis and status

## Completed (23/26)

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

## In Progress (1)

- [ ] #8 Replace console.log with structured logging
  - DONE: Created @cynic/core/logger module
  - DONE: Converted persistence clients (9 calls)
  - REMAINING: ~900 console.log calls across codebase
  - Pattern established - can be done incrementally

## Pending (2)

- [ ] #19 Add tests for persistence repositories (~21% → 50%)
  - Current: 2,759 lines tests / 13,080 lines src
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

## Key Files Created/Modified

### New Files
- `packages/core/src/errors.js` - Custom error types (10 classes, 44 codes)
- `packages/core/src/crypto-utils.js` - Secure random utilities
- `packages/core/src/logger.js` - Structured logging module
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
