# CYNIC Audit TODO List

> Generated 2026-01-24 from security & architecture audit

## Completed (17/24)

- [x] #1 Fix .env credential leak
- [x] #2 Fix mixed module system in tools/domains/index.js
- [x] #3 Create custom error types in @cynic/core
- [x] #5 Replace Math.random() with crypto.randomBytes()
- [x] #6 Add rate limiting to MCP server
- [x] #9 Fix phi-alignment violation in residual.js
- [x] #12 Add circuit breaker to persistence layer
- [x] #13 Fix unsafe Function() constructor in console.js
- [x] #14 Tighten CORS policy for production
- [x] #15 Add Content-Security-Policy headers
- [x] #16 Deprecate legacy 3D E-Score system
- [x] #17 Standardize export patterns - named exports only
- [x] #18 Remove re-exports from @cynic/node index.js
- [x] #20 Extract duplicated scorer patterns into base class
- [x] #22 Fix SSL certificate validation disabled
- [x] #24 Clean up TODO/FIXME markers

## In Progress (2)

- [ ] #7 Break up god files (>1000 lines) into smaller modules
  - DONE: scorers.js (1326 lines → 6 modules in scorers/)
  - REMAINING: 13 files >1000 lines:
    - cynic.js (1949)
    - node.js (1642)
    - live.js (1419)
    - server.js (1339)
    - learning-service.js (1266)
    - analyst.js (1252)
    - scout.js (1175)
    - deployer.js (1145)
    - arch.js (1128)
    - events.js (1124)
    - engine.js (1096)
    - organic-signals.js (1081)
    - guardian.js (1016)

- [ ] #8 Replace console.log with structured logging
  - DONE: Created @cynic/core/logger module
  - DONE: Converted persistence clients (9 calls)
  - REMAINING: ~900 console.log calls across codebase
  - Pattern established - can be done incrementally

## Pending (5)

- [ ] #4 Add critical path tests - PoJ judgment, key-manager, consensus
- [ ] #10 Add tests for emergence package (20% → 60%)
- [ ] #11 Add tests for identity package (17% → 60%)
- [ ] #19 Add tests for persistence repositories (13% → 50%)
- [ ] #21 Extract duplicated repository CRUD patterns
- [ ] #23 Add TypeDoc generation for API documentation

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
