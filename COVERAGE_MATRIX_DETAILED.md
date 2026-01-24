# CYNIC Test Coverage Matrix - Complete File Mapping

**Generated**: 2026-01-24  
**Format**: Package | Source File | Has Test? | Untested Functions/Classes

---

## CORE PACKAGE (19 sources, 3 test files, 15.8% coverage)

| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| core | axioms/constants.js | NO | READ-ONLY constants (OK) |
| core | axioms/index.js | NO | Re-exports only |
| core | config/index.js | NO | Configuration object |
| core | identity/index.js | ✓ YES (comprehensive) | - |
| core | learning/index.js | NO | Learning system functions |
| core | orchestration/index.js | NO | Orchestration logic |
| core | qscore/index.js | ✓ YES (basic) | - |
| core | qscore/philosophy-bridge.js | NO | Bridge functions |
| core | refinement/index.js | NO | Refinement functions |
| core | triggers/index.js | NO | Trigger system |
| core | vector/index.js | NO | Vector operations |
| core | worlds/assiah.js | NO | MaterialWorld class |
| core | worlds/atzilut.js | NO | DivineWorld class |
| core | worlds/base.js | NO | BaseWorld class + state machine |
| core | worlds/beriah.js | NO | CreationWorld class |
| core | worlds/index.js | ✓ YES (constants) | - |
| core | worlds/yetzirah.js | NO | FormationWorld class |

---

## PROTOCOL PACKAGE (25 sources, 7 test files, 28% coverage)

### Crypto Module
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | crypto/hash.js | PARTIAL | deepSort(), hashObject() internals |
| protocol | crypto/index.js | ✓ YES | - |
| protocol | crypto/signature.js | PARTIAL | verifyBlock() edge cases, key parsing |

### Merkle Tree Module
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | merkle/index.js | ✓ YES | - |
| protocol | merkle/pattern.js | NO | Pattern matching logic |
| protocol | merkle/tree.js | NO | MerkleTree class |

### Proof of Judgment (PoJ) - CRITICAL
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | poj/block.js | NO | **ALL: Block creation/validation** |
| protocol | poj/chain.js | NO | **ALL: Chain operations** |
| protocol | poj/index.js | ✓ YES (basic) | - |
| protocol | poj/judgment.js | NO | **ALL: scoreToVerdict(), createJudgment(), validateJudgment()** |

### Consensus Module - CRITICAL
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | consensus/engine.js | NO | **ConsensusEngine.process()** |
| protocol | consensus/finality.js | NO | **Finality rules** |
| protocol | consensus/gossip-bridge.js | NO | Gossip bridge logic |
| protocol | consensus/index.js | ✓ YES (basic) | - |
| protocol | consensus/lockout.js | NO | **Validator lockout mechanism** |
| protocol | consensus/messages.js | NO | Message type definitions |
| protocol | consensus/proposal.js | NO | **Block proposal logic** |
| protocol | consensus/slot.js | NO | **Slot state machine** |
| protocol | consensus/voting.js | NO | **Vote calculation/aggregation** |

### Gossip Protocol - CRITICAL
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | gossip/index.js | ✓ YES (basic) | - |
| protocol | gossip/message.js | NO | **Message creation** |
| protocol | gossip/peer.js | NO | **Peer class/communication** |
| protocol | gossip/propagation.js | NO | **Message propagation logic** |

### Other Protocol
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol | kscore/index.js | ✓ YES | - |
| protocol | index.js | ✓ YES (multi-node) | - |

---

## IDENTITY PACKAGE (6 sources, 1 test file, 16.7% coverage)

| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| identity | e-score-7d.js | ✓ YES (NEW 7D system) | - |
| identity | e-score.js | NO | **EScoreCalculator (3D legacy)** |
| identity | index.js | NO | Exports aggregation |
| identity | key-manager.js | NO | **KeyManager: ALL CRYPTO METHODS** |
| identity | node-identity.js | NO | **NodeIdentity: ALL IDENTITY METHODS** |
| identity | reputation-graph.js | NO | **ReputationGraph: ALL METHODS** |

**RISK**: Key management and identity system completely untested!

---

## MCP PACKAGE (82 sources, 15 test files, 18.3% coverage)

### Core Services - Tested
| Package | Source File | Has Test? | Coverage |
|---------|-------------|-----------|----------|
| mcp | auth-service.js | ✓ YES | Comprehensive |
| mcp | code-analyzer.js | ✓ YES | Comprehensive |
| mcp | ecosystem-service.js | ✓ YES | Basic |
| mcp | integrator-service.js | ✓ YES | Basic |
| mcp | librarian-service.js | ✓ YES | Basic |
| mcp | lsp-service.js | ✓ YES | Minimal |
| mcp | meta-dashboard.js | ✓ YES | Minimal |
| mcp | metrics-service.js | ✓ YES | Comprehensive |
| mcp | operator-registry.js | ✓ YES | Basic |
| mcp | persistence.js | ✓ YES | Basic |
| mcp | poj-chain-manager.js | ✓ YES | Minimal (1 test) |
| mcp | server.js | ✓ YES | Basic |
| mcp | session-manager.js | ✓ YES | Comprehensive |

### Core Services - Untested
| Package | Source File | Has Test? | Functions |
|---------|-------------|-----------|-----------|
| mcp | discovery-service.js | NO | Discovery logic |
| mcp | item-enricher.js | NO | Enrichment pipeline |
| mcp | json-render.js | NO | JSON rendering |
| mcp | index.js | ✓ YES (tools) | Partial |

### Dashboard (30+ files) - NO TESTS
| Directory | Files | Status | Priority |
|-----------|-------|--------|----------|
| dashboard/js/components | 16 | NO TESTS | LOW (UI) |
| dashboard/js/lib | 6 | NO TESTS | LOW (UI) |
| dashboard/js/views | 7 | NO TESTS | LOW (UI) |
| dashboard/js | router.js, app.js, api.js | SOME | MEDIUM |

### Metrics Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| mcp | metrics/AlertManager.js | NO | Alert system |
| mcp | metrics/HtmlReporter.js | NO | HTML generation |
| mcp | metrics/PrometheusFormatter.js | NO | Prometheus export |
| mcp | metrics/index.js | NO | Exports |

### Persistence Adapters (13 files)
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| mcp | persistence/FeedbackAdapter.js | NO | Feedback persistence |
| mcp | persistence/JudgmentAdapter.js | NO | Judgment persistence |
| mcp | persistence/KnowledgeAdapter.js | NO | Knowledge persistence |
| mcp | persistence/LibraryCacheAdapter.js | NO | Cache logic |
| mcp | persistence/PatternAdapter.js | NO | Pattern storage |
| mcp | persistence/PoJChainAdapter.js | NO | **Chain persistence (CRITICAL)** |
| mcp | persistence/PsychologyAdapter.js | NO | Psychology data |
| mcp | persistence/TriggerAdapter.js | NO | Trigger storage |
| mcp | persistence/index.js | ✓ YES (basic) | - |
| mcp | persistence/stores.js | ✓ YES | - |

### Tools Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| mcp | tools/domains/automation.js | NO | Automation tools |
| mcp | tools/domains/blockchain.js | NO | Blockchain tools |
| mcp | tools/domains/code.js | NO | Code tools |
| mcp | tools/domains/consciousness.js | NO | Consciousness tools |
| mcp | tools/domains/ecosystem.js | NO | Ecosystem tools |
| mcp | tools/domains/index.js | ✓ YES | - |
| mcp | tools/domains/judgment.js | NO | Judgment tools |
| mcp | tools/domains/knowledge.js | NO | Knowledge tools |
| mcp | tools/domains/orchestration.js | ✓ YES | - |
| mcp | tools/domains/session.js | NO | Session tools |
| mcp | tools/domains/system.js | NO | System tools |
| mcp | tools/index.js | ✓ YES | - |
| mcp | tools/registry.js | NO | Tool registry |
| mcp | tools/search-progressive.js | NO | Progressive search |

### Server Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| mcp | server/HttpAdapter.js | NO | HTTP adaptation |
| mcp | server/ServiceInitializer.js | NO | Service initialization |
| mcp | server/index.js | ✓ YES | - |

---

## PERSISTENCE PACKAGE (40 sources, 5 test files, 12.5% coverage)

### DAG Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | dag/cid.js | ✓ YES (partial) | CID edge cases |
| persistence | dag/dag.js | ✓ YES (partial) | DAG operations |
| persistence | dag/hamt.js | NO | HAMT structure |
| persistence | dag/index.js | ✓ YES | - |
| persistence | dag/node.js | NO | Node operations |
| persistence | dag/store.js | NO | Store logic |

### Graph Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | graph/graph.js | ✓ YES (partial) | Graph algorithms |
| persistence | graph/index.js | ✓ YES | - |
| persistence | graph/store.js | NO | Graph storage |
| persistence | graph/traversal.js | NO | Graph traversal |
| persistence | graph/types.js | NO | Type definitions |

### Interfaces
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | interfaces/IRepository.js | NO | Interface contract |
| persistence | interfaces/ISearchable.js | NO | Interface contract |
| persistence | interfaces/index.js | NO | Exports |

### PoJ Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | poj/block.js | ✓ YES (basic) | - |
| persistence | poj/chain.js | ✓ YES (basic) | - |
| persistence | poj/index.js | ✓ YES | - |

### PostgreSQL Client
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | postgres/client.js | NO | Connection pooling |
| persistence | postgres/migrate.js | NO | Migration logic |

### PostgreSQL Repositories (20 files) - CRITICAL DATA LAYER
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | repositories/consciousness.js | NO | **Consciousness queries** |
| persistence | repositories/discovery.js | NO | **Discovery queries** |
| persistence | repositories/escore-history.js | NO | **E-Score history** |
| persistence | repositories/feedback.js | NO | **Feedback storage** |
| persistence | repositories/index.js | ✓ YES | - |
| persistence | repositories/judgments.js | NO | **Judgment queries** |
| persistence | repositories/knowledge.js | NO | **Knowledge queries** |
| persistence | repositories/learning-cycles.js | NO | **Learning cycle storage** |
| persistence | repositories/library-cache.js | NO | **Library cache** |
| persistence | repositories/pattern-evolution.js | NO | **Pattern evolution** |
| persistence | repositories/patterns.js | NO | **Pattern queries** |
| persistence | repositories/poj-blocks.js | NO | **PoJ block storage** |
| persistence | repositories/psychology.js | NO | **Psychology data** |
| persistence | repositories/sessions.js | NO | **Session management** |
| persistence | repositories/triggers.js | NO | **Trigger storage** |
| persistence | repositories/user-learning-profiles.js | NO | **User profile storage** |
| persistence | repositories/users.js | NO | **User queries** |

### Redis Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| persistence | redis/client.js | NO | Redis client |
| persistence | redis/session-store.js | NO | Session storage |

---

## NODE PACKAGE (64 sources, 19 test files, 29.7% coverage)

### Agents Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | agents/base.js | ✓ YES | - |
| node | agents/digester.js | NO | Digester agent |
| node | agents/event-bus.js | NO | Event routing |
| node | agents/events.js | NO | Event type definitions |
| node | agents/guardian.js | NO | Guardian agent |
| node | agents/index.js | ✓ YES | - |
| node | agents/mentor.js | NO | Mentor agent |
| node | agents/observer.js | NO | Observer agent |
| node | agents/orchestrator.js | NO | Orchestration logic |
| node | agents/collective/analyst.js | ✓ YES (basic) | - |
| node | agents/collective/architect.js | NO | Architect agent |
| node | agents/collective/cartographer.js | NO | Cartographer agent |
| node | agents/collective/cynic.js | NO | Cynic orchestrator |
| node | agents/collective/deployer.js | NO | Deployer agent |
| node | agents/collective/guardian.js | ✓ YES (basic) | - |
| node | agents/collective/index.js | ✓ YES | - |
| node | agents/collective/janitor.js | NO | Janitor agent |
| node | agents/collective/oracle.js | NO | Oracle agent |
| node | agents/collective/sage.js | NO | Sage agent |
| node | agents/collective/scholar.js | NO | Scholar agent |
| node | agents/collective/scout.js | NO | Scout agent |

### API Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | api/burns-api.js | ✓ YES | - |
| node | api/emergence-api.js | NO | Emergence API |
| node | api/explorer-api.js | NO | Explorer endpoints |
| node | api/explorer-ui.js | NO | Explorer UI (low priority) |
| node | api/index.js | NO | API aggregation |
| node | api/server.js | NO | Server setup |

### CLI Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | cli/commands/connect.js | NO | Connect command |
| node | cli/commands/keygen.js | NO | Key generation command |
| node | cli/commands/start.js | NO | Start command |
| node | cli/index.js | NO | CLI entry point |

### Emergence Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | emergence/index.js | NO | Emergence layer |
| node | emergence/layer.js | NO | Layer implementation |

### Judge Module - CRITICAL
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | judge/dimensions.js | NO | **Dimension registry** |
| node | judge/graph-integration.js | NO | Graph integration |
| node | judge/index.js | ✓ YES | - |
| node | judge/judge.js | ✓ YES | - |
| node | judge/learning-manager.js | ✓ YES | - |
| node | judge/learning-service.js | ✓ YES | - |
| node | judge/residual.js | NO | **Residual detection** |
| node | judge/scorers.js | ✓ YES | - |
| node | judge/self-skeptic.js | ✓ YES | - |

### Memory Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | memory/index.js | NO | Memory system |
| node | memory/shared-memory.js | NO | Shared state |
| node | memory/user-lab.js | NO | User lab |

### Node Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | node.js | NO | Node initialization |

### Operator Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | operator/escore.js | NO | E-Score operator |
| node | operator/identity.js | NO | Identity operator |
| node | operator/index.js | ✓ YES | - |
| node | operator/operator.js | ✓ YES | - |

### Privacy Module - CRITICAL
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | privacy/aggregator.js | NO | Aggregation logic |
| node | privacy/commitments.js | NO | Privacy commitments |
| node | privacy/differential.js | NO | Differential privacy |
| node | privacy/local-store.js | NO | Local storage |

### Profile Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | profile/calculator.js | NO | Profile calculation |
| node | profile/organic-signals.js | NO | Signal detection |

### State Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | state/index.js | ✓ YES | - |
| node | state/manager.js | NO | State management |
| node | state/storage.js | NO | State persistence |

### Transport Module
| Package | Source File | Has Test? | Untested |
|---------|-------------|-----------|----------|
| node | transport/discovery.js | NO | Peer discovery |
| node | transport/index.js | ✓ YES | - |
| node | transport/serializer.js | NO | Message serialization |
| node | transport/websocket.js | NO | WebSocket transport |

---

## ANCHOR PACKAGE (6 sources, 5 test files, 83.3% coverage)

| Package | Source File | Has Test? | Coverage |
|---------|-------------|-----------|----------|
| anchor | anchorer.js | ✓ YES | Comprehensive |
| anchor | constants.js | NO | Constants only (low priority) |
| anchor | index.js | ✓ YES | - |
| anchor | program-client.js | ✓ YES | Comprehensive |
| anchor | queue.js | ✓ YES | - |
| anchor | wallet.js | ✓ YES | Comprehensive |

**EXCELLENT**: Solana integration well tested!

---

## EMERGENCE PACKAGE (5 sources, 1 test file, 20% coverage)

| Package | Source File | Has Test? | Untested Functions |
|---------|-------------|-----------|-------------------|
| emergence | collective-state.js | NO | **ALL: CollectiveState** |
| emergence | consciousness-monitor.js | NO | **ALL: ConsciousnessMonitor** |
| emergence | dimension-discovery.js | NO | **ALL: DimensionDiscovery** |
| emergence | index.js | ✓ YES (basic) | - |
| emergence | pattern-detector.js | NO | **ALL: PatternDetector** |

**RISK**: Philosophical foundation completely untested!

---

## BURNS PACKAGE (3 sources, 3 test files, 100% coverage)

| Package | Source File | Has Test? | Coverage |
|---------|-------------|-----------|----------|
| burns | index.js | ✓ YES | Comprehensive |
| burns | solana-verifier.js | ✓ YES | Comprehensive |
| burns | verifier.js | ✓ YES | Comprehensive |

**EXCELLENT**: Complete test coverage!

---

## ZK PACKAGE (3 sources, 1 test file, 33.3% coverage)

| Package | Source File | Has Test? | Untested Functions |
|---------|-------------|-----------|-------------------|
| zk | index.js | ✓ YES (basic) | - |
| zk | prover.js | NO | **ALL: ZKProver** |
| zk | verifier.js | NO | **ALL: ZKVerifier** |

**CRITICAL**: Zero-knowledge proof logic untested!

---

## HOLDEX PACKAGE (3 sources, 1 test file, 33.3% coverage)

| Package | Source File | Has Test? | Untested Functions |
|---------|-------------|-----------|-------------------|
| holdex | client.js | NO | HoldexClient |
| holdex | harmony.js | NO | Harmony integration |
| holdex | index.js | ✓ YES (basic) | - |

---

## GASDF PACKAGE (2 sources, 1 test file, 50% coverage)

| Package | Source File | Has Test? | Untested Functions |
|---------|-------------|-----------|-------------------|
| gasdf | client.js | NO | GasdfClient |
| gasdf | index.js | ✓ YES (basic) | - |

---

## SCHEDULER PACKAGE
Empty (0 sources, 0 tests)

---

## SUMMARY TABLE

**Total Files**: 258  
**Total Tests**: 62  
**Overall Coverage**: 24.0%

| Risk Level | Packages | Count |
|-----------|----------|-------|
| 🔴 CRITICAL (<20%) | core, identity, mcp, persistence, emergence | 5 |
| 🟠 MEDIUM (20-50%) | protocol, node, zk, holdex, gasdf | 5 |
| ✓ SAFE (>80%) | burns, anchor | 2 |

**Action Required**: 20+ critical files need immediate test coverage

