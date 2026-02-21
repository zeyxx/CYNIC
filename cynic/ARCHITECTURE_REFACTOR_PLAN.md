# CYNIC Architecture Refactor — Complete Implementation Plan
## Bottom-Up, All The Way — 6-7 Days of Focused Work

---

## 🎯 VISION (After Refactor)

```
BEFORE (Current - BROKEN):
  API → Cognition (14k LOC monolith) → Organism (1k LOC wrapper)
  State: Fragmented across 5 systems
  Handlers: Registered in 4 places
  Events: Fire-and-forget, no persistence
  Deployment: Single service, single process

AFTER (Fixed - PRODUCTION-READY):
  API → Organism (coordinator)
       └─ Cognition (subsystem, 2k LOC)
       └─ State Manager (centralized)
       └─ Handler Registry (single source)
       └─ Event Store (persistent)

  Deployment: Multi-service, multi-instance ready
```

---

## 📋 IMPLEMENTATION ROADMAP

### PHASE 1: BLOCKING BUGS (3 days) ← START HERE
Fix low-level issues that break everything else.

#### DAY 1: State Consolidation
**Goal**: Merge 5 state systems into 1 organism-owned state manager

**Current reality**:
```python
# State lives in 5 places:
1. api/state.py::KernelState (9,925 LOC monster)
2. organism/conscious_state.py::ConsciousState (660 LOC)
3. senses/checkpoint.py::checkpoint() (function, no class)
4. core/topology/topology_mirror.py::TopologyMirror (class)
5. cognition/cortex/ (Q-table, residual state, dog state)
```

**After**:
```python
# Single source of truth:
organism/state_manager.py::OrganismState
  - memory_state (Q-table, dogs, residuals) → RAM + SurrealDB backup
  - persistent_state (consciousness, actions) → SurrealDB only
  - checkpoint_state (recovery data) → File only

All updates: Organism.state.update(change)
All queries: Organism.state.query(key)
Write-through consistency checks enabled
```

**Tasks**:
```
[ ] 1a. Create organism/state_manager.py (500 LOC)
        - Class OrganismState with 3 layers
        - Update/query/sync interface
        - Write-through consistency

[ ] 1b. Create organism/state_recovery.py (300 LOC)
        - Load from SurrealDB on startup
        - Load from checkpoint if DB missing
        - Merge recovery logic

[ ] 1c. Update organism/__init__.py
        - Export OrganismState
        - Lazy-load state on first use

[ ] 1d. Update all imports (100+ files)
        - api/state.py → organism/state_manager
        - senses/checkpoint.py → organism/state_manager
        - cognition/cortex → organism/state_manager
        - VERIFICATION: All tests pass

[ ] 1e. Integration tests (200 LOC)
        - test_state_consolidation.py
        - Verify memory + persistent sync
        - Verify checkpoint recovery
        - Verify write-through consistency
```

**Validation**:
- All 128 test files still pass
- No state corruption after 100 updates
- Checkpoint recovery works

**Risk**: HIGH - Every module touches state
**Mitigation**: Test thoroughly before next day

---

#### DAY 2: Handler Registration & Lifecycle
**Goal**: Single handler registry with clear lifecycle

**Current reality**:
```python
# Handlers registered in 4 places:
1. api/entry.py - Bootstrap phase
2. api/server.py - App startup
3. cognition/cortex/orchestrator.py - Runtime
4. api/handlers/{handler}.py - Individual files
```

**After**:
```python
# Single registry:
organism/handler_registry.py::HandlerRegistry
  - Discovery: Scan for handler classes
  - Initialization: Instantiate with dependencies
  - Activation: Subscribe to events
  - Runtime: Handle events
  - Deactivation: Cleanup

Registry is owned by Organism, not API or Cognition
```

**Tasks**:
```
[ ] 2a. Create organism/handler_registry.py (400 LOC)
        - HandlerRegistry class
        - Discovery/init/activate/deactivate lifecycle
        - Health checks per handler
        - Error isolation (one handler crash != all crash)

[ ] 2b. Create organism/handler_interface.py (100 LOC)
        - BaseHandler abstract class
        - Required methods: on_startup, handle(), on_shutdown
        - Dependency injection pattern

[ ] 2c. Update all handlers (20+ files)
        - Implement BaseHandler interface
        - Remove direct event_bus subscriptions
        - Remove api/state imports
        - Inject dependencies via __init__

[ ] 2d. Update api/entry.py
        - Remove handler registration
        - Call organism.initialize_handlers()

[ ] 2e. Update cognition/cortex/orchestrator.py
        - Remove handler registration calls
        - Query handler_registry if needed (read-only)

[ ] 2f. Integration tests (200 LOC)
        - test_handler_registry.py
        - Verify all handlers activate
        - Verify handler health checks work
        - Verify crash isolation
```

**Validation**:
- All 128 test files still pass
- All handlers activate in correct order
- Handler crash doesn't crash organism

**Risk**: HIGH - Handlers are critical path
**Mitigation**: Run tests continuously

---

#### DAY 3: Event Persistence & Replay
**Goal**: Event sourcing - events written before handlers run

**Current reality**:
```python
# Events are fire-and-forget:
await bus.emit(Event(...))  # Handler gets it now or never

# Problems:
- No history
- Late handlers miss startup events
- Crash = events lost
```

**After**:
```python
# Event sourcing:
await event_store.persist(Event(...))  # Write to DB first
await bus.emit(Event(...))              # Then notify handlers

On startup:
  event_store.replay_since(last_checkpoint)
  Handlers can query history
  Can trace entire organism timeline
```

**Tasks**:
```
[ ] 3a. Create organism/event_store.py (400 LOC)
        - EventStore class
        - Append-only event log in SurrealDB
        - Persist before emit
        - Replay from checkpoint
        - Query interface for history

[ ] 3b. Update core/event_bus.py
        - Hook: await event_store.persist() before emit
        - Make it configurable (can disable in tests)

[ ] 3c. Update organism/bootstrap.py
        - On startup: event_store.replay_since(checkpoint)

[ ] 3d. Integration tests (200 LOC)
        - test_event_store.py
        - Verify events persist
        - Verify replay works
        - Verify handler gets old events on startup

[ ] 3e. Performance check
        - Measure impact of persist (should be <10ms)
        - If slow, add batching
```

**Validation**:
- Events persist to SurrealDB
- Replay gives same events in same order
- No performance degradation

**Risk**: MEDIUM - Event bus used everywhere
**Mitigation**: Feature-flag to disable during tests

---

### PHASE 2: ARCHITECTURE (2-3 days)
Reorganize modules so organism is central.

#### DAY 4: Organism-First Architecture
**Goal**: Make organism/ the root coordinator, cognition/ a subsystem

**Current reality**:
```
cynic/
├── api/
│   ├── server.py (creates everything)
│   ├── state.py (state holder)
│   ├── handlers/ (scattered)
│   └── ...
├── cognition/ (14k LOC, imports api!)
│   ├── cortex/orchestrator.py (imports from api)
│   └── ...
└── organism/ (1k LOC, wrapper)
    └── conscious_state.py
```

**After**:
```
cynic/
├── organism/ (ROOT COORDINATOR)
│   ├── __init__.py (exports Organism)
│   ├── awakener.py (startup)
│   ├── hibernator.py (shutdown)
│   ├── state_manager.py (state - from PHASE 1)
│   ├── handler_registry.py (handlers - from PHASE 1)
│   ├── event_store.py (events - from PHASE 1)
│   ├── subsystems.py (compose cognition + senses)
│   └── ...
├── cognition/ (SUBSYSTEM)
│   ├── __init__.py (exports CognitionSubsystem)
│   ├── orchestrator.py (NO api imports!)
│   └── ...
├── api/ (HTTP LAYER)
│   ├── server.py (delegates to organism)
│   ├── routers/ (thin wrappers)
│   └── ...
└── senses/ (SUBSYSTEM, like cognition)
```

**Tasks**:
```
[ ] 4a. Create organism/subsystems.py (300 LOC)
        - class CognitionSubsystem
        - class SensesSubsystem
        - Dependency injection pattern
        - Clear initialization order

[ ] 4b. Reorganize organism/
        - Move state_manager, handler_registry, event_store here
        - Create Organism class that composes subsystems
        - Make Organism the single entry point

[ ] 4c. Move cognition to subsystem model
        - Extract CognitionSubsystem from cognition/cortex/orchestrator
        - Remove api imports
        - Inject dependencies

[ ] 4d. Update api/server.py
        - Initialize Organism first
        - API wraps Organism, doesn't manage it
        - api/routers delegate to organism.handle(request)

[ ] 4e. Update api/routers/
        - All routers import from organism, not cognition
        - routers are thin HTTP wrappers only
        - business logic in organism

[ ] 4f. Integration tests (300 LOC)
        - test_organism_architecture.py
        - Verify organism starts without api
        - Verify api wraps organism
        - Verify subsystems initialize in order
```

**Validation**:
- Organism can start independently
- API can be mocked, organism still works
- All 128 tests still pass

**Risk**: VERY HIGH - Reorganizes entire codebase
**Mitigation**: Do this in a feature branch, careful testing

---

#### DAY 5: Lifecycle Management
**Goal**: Graceful startup/shutdown with signal handling

**Current reality**:
```python
# Startup: Unclear order, things initialize randomly
# Shutdown: Container SIGTERM → process dies, no cleanup

# Problems:
- In-flight requests abandoned
- Q-table not saved
- Checkpoint not written
- Events not flushed
```

**After**:
```python
# Startup sequence (in order):
1. Organism.bootstrap() - Core initialization
2. Organism.initialize_subsystems() - Cognition, Senses
3. Organism.activate_handlers() - Start listening
4. API.start() - Accept requests

# Shutdown sequence (reverse):
1. API.stop() - Stop accepting requests
2. Organism.deactivate_handlers() - Stop listening
3. Organism.flush_state() - Save Q-table, checkpoint
4. Organism.hibernate() - Cleanup
```

**Tasks**:
```
[ ] 5a. Create organism/lifecycle.py (400 LOC)
        - LifecycleManager class
        - Hook system: on_startup, on_shutdown
        - Ordered execution
        - Timeout handling

[ ] 5b. Update organism/awakener.py
        - Call LifecycleManager.startup() with hooks
        - Register subsystems as hooks
        - Register API as hook

[ ] 5c. Create organism/hibernator.py (200 LOC)
        - Graceful shutdown logic
        - Flush state to SurrealDB
        - Save checkpoint
        - Cleanup resources

[ ] 5d. Update api/server.py
        - Add signal handlers (SIGTERM, SIGINT)
        - Call organism.hibernate() on signal
        - Wait for in-flight requests to finish

[ ] 5e. Update Dockerfile
        - Change CMD to handle signals
        - Set shutdown timeout to 30s
        - Verify graceful shutdown in compose

[ ] 5f. Integration tests (200 LOC)
        - test_lifecycle.py
        - Verify startup order
        - Verify shutdown order
        - Verify state saved on shutdown
        - Verify signal handling works
```

**Validation**:
- Startup: Subsystems initialize in order
- Shutdown: SIGTERM results in graceful shutdown
- State: Q-table persisted after shutdown
- Checkpoint: Recovery data saved

**Risk**: MEDIUM - Affects startup/shutdown paths
**Mitigation**: Test thoroughly with docker-compose

---

### PHASE 3: DEPLOYMENT (2-3 days)
Make CYNIC ready for multi-instance.

#### DAY 6: Multi-Service Architecture
**Goal**: Split into independent services (organism, cognition, handlers)

**After**:
```
docker-compose.yml:
  cynic-organism:        # Core coordinator
  cynic-cognition:       # Heavy lifting (optional, scale independently)
  cynic-api:             # HTTP layer
  cynic-handlers:        # Background tasks (optional, scale independently)
  surrealdb:             # Shared state
  ollama:                # Shared LLM
```

**Tasks**:
```
[ ] 6a. Create cynic-organism Dockerfile
        - Minimal, fast startup
        - No HTTP, just gRPC or socket
        - Health checks: /health/organism

[ ] 6b. Create cynic-cognition Dockerfile
        - Pulls from organism (gRPC)
        - Implements reasoning
        - Optional (defaults to in-process)

[ ] 6c. Create cynic-api Dockerfile
        - Minimal, just HTTP wrapping
        - Calls organism service
        - Health checks: /health/api

[ ] 6d. Create cynic-handlers Dockerfile
        - Optional, scales independently
        - Subscribes to organism events
        - Health checks: /health/handlers

[ ] 6e. Update docker-compose.yml
        - 4 services + shared backends
        - Service discovery (DNS)
        - Volume sharing for state
        - Network definitions

[ ] 6f. Integration tests (300 LOC)
        - test_multi_service.py
        - Verify services communicate
        - Verify state sync
        - Verify handler scaling
```

**Validation**:
- Each service starts independently
- State synchronized across services
- Can scale handlers from 1 to N

**Risk**: VERY HIGH - Distributed system complexity
**Mitigation**: Start with 2-service, then scale

---

#### DAY 7: Kubernetes-Ready & Documentation
**Goal**: Make CYNIC production-deployable

**Tasks**:
```
[ ] 7a. Create k8s/ directory
        - deployment.yaml (organism)
        - service.yaml (organism)
        - configmap.yaml (config)
        - persistentvolume.yaml (state)

[ ] 7b. Create helm/ directory (optional but recommended)
        - helm chart for CYNIC
        - Values for dev/staging/prod
        - Easy upgrade path

[ ] 7c. Update TESTING.md
        - Document new architecture
        - Test strategy for multi-service
        - Performance benchmarks

[ ] 7d. Create DEPLOYMENT.md
        - Local: docker-compose up
        - Cloud: kubectl apply -k k8s/
        - Monitoring: Prometheus metrics

[ ] 7e. Add Prometheus metrics
        - Handler performance
        - Event latency
        - State sync times
        - Error rates

[ ] 7f. Final integration tests (200 LOC)
        - test_production_readiness.py
        - Simulate cloud deployment
        - Verify all failure modes handled
```

**Validation**:
- `docker-compose up` works
- `kubectl apply` works
- Prometheus metrics collected
- Documentation complete

**Risk**: LOW (documentation, no code changes)
**Mitigation**: Use templates

---

## 📊 EFFORT ESTIMATE

```
PHASE 1 (Blocking Bugs):
  Day 1: State Consolidation    → 1-2 days (HIGH IMPACT, HIGH RISK)
  Day 2: Handler Registry       → 1 day (MEDIUM IMPACT, HIGH RISK)
  Day 3: Event Persistence      → 1 day (MEDIUM IMPACT, MEDIUM RISK)
  Subtotal: 3 days

PHASE 2 (Architecture):
  Day 4: Organism-First         → 1-2 days (VERY HIGH IMPACT, VERY HIGH RISK)
  Day 5: Lifecycle Management   → 1 day (MEDIUM IMPACT, MEDIUM RISK)
  Subtotal: 2-3 days

PHASE 3 (Deployment):
  Day 6: Multi-Service          → 1-2 days (HIGH IMPACT, HIGH RISK)
  Day 7: Kubernetes + Docs      → 1 day (LOW RISK)
  Subtotal: 2-3 days

TOTAL: 6-7 days of focused work
```

---

## ⚠️ RISK MANAGEMENT

### HIGH RISK POINTS
1. **State consolidation** - Every module depends on it
   - Mitigation: Test thoroughly before moving on
   - Rollback: Keep old state system in parallel for 1 day

2. **Handler registration** - Critical path for all events
   - Mitigation: Feature-flag old handler system
   - Rollback: Can switch back easily if needed

3. **Organism-first architecture** - Reorganizes entire codebase
   - Mitigation: Do in feature branch, merge carefully
   - Rollback: Keep original in git history

### TESTING STRATEGY
```
After each day:
  1. Run full test suite (pytest --tb=short)
  2. Run integration tests (test_phase_X.py)
  3. Manual smoke test (docker-compose up)
  4. Check memory usage (no leaks)
  5. Check startup time (should not increase)
```

### BRANCH STRATEGY
```
main (current, stable)
└─ architecture/organism-v1-bootstrap (current branch)
   ├─ arch/phase1-state-consolidation (DAY 1)
   ├─ arch/phase1-handler-registry (DAY 2)
   ├─ arch/phase1-event-persistence (DAY 3)
   ├─ arch/phase2-organism-first (DAY 4)
   ├─ arch/phase2-lifecycle (DAY 5)
   ├─ arch/phase3-multi-service (DAY 6)
   └─ arch/phase3-kubernetes (DAY 7)

Each PR reviewed, tested, merged before next day starts.
```

---

## 🎯 WHAT HAPPENS NEXT (After Refactor)

### Immediately After (Week 2):
- **Phase 3 Tier 2-3**: Account/Policy endpoints (easy now!)
- **Integration tests**: Full cycle tests (easy now!)

### Week 3:
- **Phase 4 Foundation**: Multi-instance consensus
- **Deploy to staging**: 2 instances, verify state sync
- **Load testing**: Hammer it with concurrent requests

### Week 4:
- **Production deployment**: CYNIC v1 live
- **Monitoring**: Prometheus + alerting
- **Documentation**: Runbooks, playbooks

---

## ✅ SUCCESS CRITERIA

After all 7 days:
- ✅ Organism is the root coordinator
- ✅ State consolidated into 1 system
- ✅ Handlers registered in 1 place
- ✅ Events persist to DB
- ✅ Graceful startup/shutdown works
- ✅ Multi-service architecture verified
- ✅ Kubernetes-ready (basic)
- ✅ All 128 tests pass
- ✅ No regressions
- ✅ Performance maintained or improved

**Confidence**: 62% (φ⁻¹) before start, ~78% (HIGH) after Phase 1

---

## 📝 DECISION POINT

**Do we start immediately on DAY 1 (State Consolidation)?**
- YES: Major refactor, high effort but high reward
- NO: Continue with Phase 3 Tier 2-3 (easier path)

**Recommendation**: START NOW
- Phase 4 WILL fail without this
- Better to fix now than in emergency mode later
- 7 days is reasonable investment for production readiness

---

Created: 2026-02-21
Confidence: 62% (φ⁻¹)
Status: READY TO IMPLEMENT
