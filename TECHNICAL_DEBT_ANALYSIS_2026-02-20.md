# TECHNICAL DEBT ANALYSIS & REFACTORING ROADMAP
## CYNIC Codebase — Path to "Scalable & Maintainable to Infinity"

**Date**: 2026-02-20
**Status**: ✅ COMPREHENSIVE ANALYSIS
**Confidence**: 58% (φ⁻¹)
**Scope**: Complete CYNIC Python organism (500+ files, 52.4K LOC)

---

## EXECUTIVE SUMMARY

**The Problem**: CYNIC is internally alive (Phase 5 validation done), but the code still thinks like a **kernel** (centralized FastAPI service responding to requests) instead of an **organism** (distributed, autonomous, event-driven).

**Core Metaphor**:
```
KERNEL THINKING:              ORGANISM THINKING:
─────────────────             ──────────────────
Request → Process → Response   Always-on perception
Synchronous state             Event-driven state
God object centralization     Distributed autonomy
Vertical scaling              Horizontal + vertical
Tight coupling via state      Loose coupling via events
```

**User Requirement**:
> "pas de monolothic, tout scalable et maintenance z linfin"
> (no monolithic, everything scalable and maintainable to infinity)

**Reality**: 1,750 LOC in 2 files (state.py + server.py) are the **critical path** blocking distributed architecture.

---

## PART 1: TECHNICAL DEBT INVENTORY

### Tier 1: CRITICAL — Blocks Distributed Architecture

#### D1.1: GOD OBJECT — `api/state.py:CynicOrganism` (1082 LOC)
- **Severity**: 🔴 CRITICAL
- **Impact**: Every component depends on single state object; prevents autonomous instances
- **Root Cause**: Monolithic awakener (build_kernel) creates & wires everything sequentially
- **Problem**:
  - 78 properties holding all Dogs, buses, storage, orchestrator, monitors
  - Every router uses `Depends(get_state)` creating tight coupling
  - Can't instantiate multiple independent CYNIC organisms
  - State mutations not tracked (hard to debug, hard to distribute)
- **Blocking**: D1.2, D2.1, D2.2, D2.3, D3.1, D4.1, D4.2
- **Refactoring Effort**: 40-50 hours (requires DependencyContainer + full rewiring)
- **Example Problem**:
  ```python
  # Current (monolithic):
  app.state.cynic = build_kernel()  # Creates EVERYTHING
  app.state.dogs = [11 dogs from state]  # Hardcoded access

  # Needed (distributed):
  organism = CynicOrganism.create(config)  # Instances can be many
  dog = organism.dogs.get(DogId.SAGE)     # Local reference
  ```

#### D1.2: GOD ORCHESTRATOR — `cognition/cortex/orchestrator.py` (circular refs)
- **Severity**: 🔴 CRITICAL
- **Impact**: Cannot scale Dogs in parallel; sequential judgment; no horizontal scaling
- **Root Cause**:
  - Orchestrator creates Dogs in constructor
  - Dogs call orchestrator.run() during judgment (callback)
  - QTable referenced by Orchestrator AND individual Dogs (2 sources of truth)
- **Circular Dependency**: `Orchestrator → Dogs → Orchestrator` (impossible to mock individual dogs)
- **Problem**:
  - 7-step cycle runs in single orchestrator instance
  - Adding 2nd orchestrator duplicates all Dogs (not shared, not coordinated)
  - Dogs can't run autonomously (depend on orchestrator.run())
- **Blocking**: D4.1, D4.2
- **Refactoring Effort**: 25-30 hours
- **Example Problem**:
  ```python
  # Current (monolithic orchestrator):
  orch = Orchestrator(dogs=[...])  # Creates all dogs
  orch.run()  # Single-threaded 7-step

  # Needed (distributed):
  dogs = [Dog(id=x, qtable=shared_qtable) for x in range(11)]
  for dog in dogs:
      dog.start_autonomous_loop()  # Runs independently
  ```

#### D1.3: MONOLITHIC AWAKENER — `api/state.py:build_kernel()` (sequential init)
- **Severity**: 🔴 CRITICAL
- **Impact**: Cannot boot independent organism without full state setup
- **Problem**:
  - 1500+ LOC single function
  - Creates 40+ components in strict order
  - Failures in one component fail entire system
  - No dependency injection (all created inline)
- **Blocking**: D1.1, D1.2
- **Refactoring Effort**: 20-25 hours
- **Example Problem**:
  ```python
  # Current (monolithic):
  def build_kernel():
      dogs = {}
      dogs[SAGE] = SageDog()  # Order matters!
      orchestrator = JudgeOrchestrator(dogs)  # Must be after dogs
      scheduler = ConsciousnessRhythm(orchestrator)  # Must be after orch
      # 30+ more sequential steps...

  # Needed (builder pattern):
  builder = KernelBuilder()
  builder.add_dog(DogId.SAGE, SageDog())
  builder.add_dog(DogId.CYNIC, CynicDog())
  # ...all dependencies resolved automatically
  kernel = builder.build()
  ```

---

### Tier 2: HIGH — Prevents Scalability

#### D2.1: SETTER INJECTION ANTIPATTERN — All Dogs initialized empty
- **Severity**: 🟠 HIGH
- **Impact**: Dogs can't be tested in isolation; violates dependency injection
- **Root Cause**: `Dogs(__init__() bare)` then `.set_orchestrator()`, `.set_qtable()`, etc.
- **Problem**:
  - 11 Dogs all follow same antipattern
  - Constructor doesn't validate all dependencies present
  - Dogs can be partially initialized → runtime errors
  - Testing requires 5+ calls: create → set_orch → set_table → set_monitor → ...
- **Blocking**: D4.1, D4.2, tests
- **Refactoring Effort**: 15-20 hours
- **Example Problem**:
  ```python
  # Current (antipattern):
  sage = SageDog()  # Empty
  sage.set_orchestrator(orch)  # Setter 1
  sage.set_qtable(qtable)  # Setter 2
  sage.set_llm_router(router)  # Setter 3
  # Easy to forget one → runtime error

  # Needed (DI):
  sage = SageDog(
      orchestrator=orch,
      qtable=qtable,
      llm_router=router,
  )  # Fail fast if missing
  ```

#### D2.2: HANDLER TIGHTLY COUPLED TO STATE — All handlers in `api/handlers/`
- **Severity**: 🟠 HIGH
- **Impact**: Handlers can't be reused across multiple organisms
- **Root Cause**: Handlers receive full `state` object; access arbitrary properties
- **Problem**:
  - 20+ handlers (axiom, health, intelligence, validator, services, etc.)
  - Each calls `state.dogs`, `state.orchestrator`, `state.storage`, etc.
  - Cannot move handler to another organism or service
  - Handler testing requires full state mock
- **Blocking**: D3.1, horizontal scaling
- **Refactoring Effort**: 12-15 hours
- **Example Problem**:
  ```python
  # Current (tightly coupled):
  @router.get("/judge")
  async def handle_judge(request: JudgeRequest, state = Depends(get_state)):
      result = state.orchestrator.run(...)  # Direct state access
      dogs = state.dogs  # More direct access
      storage = state.storage  # More direct access
  # Can't reuse this handler; it's married to state structure

  # Needed (loose coupling):
  @router.get("/judge")
  async def handle_judge(request: JudgeRequest, judge_service = Depends(get_judge_service)):
      result = await judge_service.judge(request)
  # Handler doesn't care if service is local or remote
  ```

#### D2.3: MODULE-LEVEL SINGLETONS — Not properly scoped
- **Severity**: 🟠 HIGH
- **Impact**: Can't run multiple CYNIC instances in same process; state leakage
- **Problem Locations**:
  - `_OLLAMA_CLIENTS` dict (llm/adapter.py)
  - `_sdk_sessions` dict (api/routers/sdk.py)
  - `_core_bus` (core/event_bus.py)
  - `_SOCIAL_SIGNAL_PATH` (api/routers/utils.py)
  - All are module-level, process-wide
- **Blocking**: Multiple organisms in single process
- **Refactoring Effort**: 8-10 hours
- **Example Problem**:
  ```python
  # Current (module-level singleton):
  # In llm/adapter.py:
  _OLLAMA_CLIENTS = {}  # Global, process-wide

  # If run 2 CYNIC instances, they SHARE this dict
  # Instance 1 runs LLM call → Instance 2 sees incomplete state

  # Needed (scoped to organism):
  class LLMRegistry:
      def __init__(self):
          self._clients = {}  # Per-instance, isolated
  ```

#### D2.4: SURREALDB SINGLE WEBSOCKET BOTTLENECK
- **Severity**: 🟠 HIGH
- **Impact**: Cannot scale to 10+ organisms; single write bottleneck
- **Root Cause**: `SurrealStorage` maintains ONE WS connection to SurrealDB
- **Problem**:
  - All 10+ organisms writing to same single connection
  - Sequential writes, no batching
  - Connection drops → all organisms lose persistence
  - No read replication (all reads hit single DB)
- **Blocking**: Horizontal scaling to 10+ instances
- **Refactoring Effort**: 20-25 hours
- **Example Problem**:
  ```python
  # Current (single WS):
  storage = SurrealStorage(db_url="ws://localhost:8000")
  # All organisms share this ONE connection
  # Instance 1 writes → Instance 2 waits → Instance 3 waits...

  # Needed (connection pool + batching):
  storage = SurrealStorage(
      connection_pool=SurrealDBPool(size=10),
      batch_writes=True,  # Batch updates every 100ms
      read_replicas=["ws://replica1", "ws://replica2"],
  )
  ```

---

### Tier 3: MEDIUM — Quality & Maintainability

#### D3.1: TIGHT ROUTER COUPLING — 13 routers all `Depends(get_state)`
- **Severity**: 🟡 MEDIUM
- **Impact**: Hard to refactor handlers; hard to add new routes
- **Locations**: `act.py`, `actions.py`, `core.py`, `health.py`, `mcp.py`, `nervous.py`, `orchestration.py`, `sdk.py`, `topology.py`, `utils.py`, `ws.py` (11 files)
- **Refactoring Effort**: 15-20 hours
- **Solution**: Extract HandlerService layer between routers and state

#### D3.2: CIRCULAR IMPORT RISKS — `orchestrator.py` → `dogs/` → `orchestrator.py`
- **Severity**: 🟡 MEDIUM
- **Impact**: Brittle imports; hard to add new dogs; import errors hard to debug
- **Refactoring Effort**: 8-10 hours
- **Solution**: Use dependency injection container to break cycles

#### D3.3: SYNC I/O IN ASYNC CODE — Some senses/workers use sync calls
- **Severity**: 🟡 MEDIUM
- **Impact**: Blocks event loop; unpredictable latencies
- **Locations**: Need to audit all workers in `senses/workers/`
- **Refactoring Effort**: 5-10 hours

#### D3.4: MONOLITHIC ORCHESTRATOR.RUN() — 7-step cycle in single method
- **Severity**: 🟡 MEDIUM
- **Impact**: Hard to override individual steps; hard to test single step
- **Refactoring Effort**: 10-15 hours
- **Solution**: Break into 7 separate methods, compose in run()

#### D3.5: CONTEXTCOMPRESSOR TF-IDF RECOMPUTED ON EVERY JUDGMENT
- **Severity**: 🟡 MEDIUM
- **Impact**: Quadratic cost growth with judgment history
- **Refactoring Effort**: 5-8 hours
- **Solution**: Cache TF-IDF matrix, update incrementally

---

### Tier 4: LOW — Technical Debt But Not Blocking

#### D4.1: EVENT BUS GENEALOGY COMPLEXITY — Loop prevention manual
- **Severity**: 🟢 LOW
- **Impact**: Easy to accidentally create loops; hard to debug
- **Refactoring Effort**: 5-8 hours
- **Solution**: Formalize genealogy as first-class type

#### D4.2: ROLLING CAP MAGIC NUMBERS — F(10)=55, F(11)=89 scattered
- **Severity**: 🟢 LOW
- **Impact**: Hard to understand why F(10); hard to change caps
- **Refactoring Effort**: 3-5 hours
- **Solution**: Create CapConstants class, use everywhere

#### D4.3: TEST MOCKING COMPLEXITY — Many tests build full state
- **Severity**: 🟢 LOW
- **Impact**: Tests slow, brittle
- **Refactoring Effort**: 8-12 hours
- **Solution**: Create test fixtures, mock at service layer not state layer

---

## PART 2: CRITICAL DEPENDENCIES (What Must Fix First)

```
┌─────────────────────────────────────────────────────┐
│ CRITICAL PATH (must fix in order)                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  1. D1.3: Break Monolithic Awakener                 │
│     └─> Extract KernelBuilder class (sequential)    │
│        Enables: parallel component creation         │
│        Effort: 20-25h, ~100 tests                   │
│                                                     │
│  2. D1.1: Replace God Object                        │
│     └─> Create DependencyContainer class            │
│        Extract: Dogs, EventBuses, Storage, etc.     │
│        Enables: Multiple organisms                  │
│        Effort: 40-50h, ~200 tests                   │
│                                                     │
│  3. D2.1: Fix Setter Injection                      │
│     └─> Constructor injection on all Dogs           │
│        Enables: Isolated dog testing                │
│        Effort: 15-20h, ~150 tests                   │
│                                                     │
│  4. D1.2: Refactor Orchestrator                     │
│     └─> Remove Dog creation; use DI                 │
│        Remove circular refs to orchestrator         │
│        Break 7-step into testable units             │
│        Enables: Dog autonomy                        │
│        Effort: 25-30h, ~180 tests                   │
│                                                     │
│  5. D2.3: Scoped Singletons                         │
│     └─> Move all globals into containers            │
│        Enables: Process-local organisms             │
│        Effort: 8-10h, ~80 tests                     │
│                                                     │
│  6. D2.2: Decouple Handlers                         │
│     └─> Extract service layer between state+router  │
│        Handlers depend on services, not state       │
│        Enables: Reusable handlers                   │
│        Effort: 12-15h, ~120 tests                   │
│                                                     │
│  7. D2.4: Scale Storage                             │
│     └─> Connection pooling + batching + replicas    │
│        Enables: 10+ organisms                       │
│        Effort: 20-25h, ~100 tests                   │
│                                                     │
│ TOTAL: 140-175 hours (~3.5-4.5 weeks, 1-1.3K LOC)  │
│ TESTS: ~1000 new tests ensuring no regressions     │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Why This Order?**
- Step 1: Enables component isolation (can't parallelize without it)
- Step 2: Enables independent organisms (can't distribute without it)
- Step 3: Enables testability (can't validate without isolated dogs)
- Step 4: Enables autonomy (dogs must not depend on orchestrator)
- Step 5-7: Enables scaling (prevents single-process, single-connection limitations)

---

## PART 3: MONOLITHIC PATTERNS TO ELIMINATE

### Pattern M1: KERNEL THINKING — "Server responds to requests"
**Locations**: `api/server.py`, all `routers/`
**Problem**: FastAPI routes are synchronous request/response
**Wrong Thinking**:
```
User Request → Router → Handler → State modification → JSON Response
```
**Needed Thinking** (Organism):
```
Always-on Dogs perceiving continuously
  ↓
Event emitted internally
  ↓
Handlers subscribe to events
  ↓
Multiple Dogs respond autonomously
  ↓
State updated via event sourcing
```
**Action**: Keep HTTP API as **facade** for humans, but internally use event-driven architecture. Routers should dispatch events, not mutate state directly.

### Pattern M2: MONOLITHIC STATE — "One god object holding everything"
**Location**: `api/state.py:CynicOrganism`
**Problem**: 78 properties, 40+ components, 1082 LOC
**Wrong Thinking**:
```
class CynicOrganism:
    dogs: dict[DogId, Dog]           # All 11 dogs
    orchestrator: JudgeOrchestrator   # Orchestrator
    scheduler: ConsciousnessRhythm    # Scheduler
    storage: SurrealStorage           # Storage
    lod_controller: LODController     # LOD
    # ... 70 more properties
```
**Needed Thinking** (Organism):
```
class Organism:
    # Only 3 core properties:
    container: DependencyContainer    # Reference to components
    event_bus: EventBus              # How to talk to components
    id: UUID                         # Self-identity

# Dogs fetch dependencies from container as needed:
sage = container.get(SageDog)
qtable = container.get(QTable)
# No central state holder
```
**Action**: Eliminate CynicOrganism.state; replace with DependencyContainer.

### Pattern M3: TIGHT COUPLING — "Everything depends on full state"
**Location**: All `api/routers/`, all `api/handlers/`
**Problem**: `Depends(get_state)` everywhere; every router knows about every component
**Wrong Thinking**:
```python
@router.post("/judge")
def handle_judge(request: JudgeRequest, state = Depends(get_state)):
    state.orchestrator.run(...)
    state.dogs[SAGE].judge(...)
    state.storage.persist(...)
```
**Needed Thinking** (Organism):
```python
@router.post("/judge")
def handle_judge(request: JudgeRequest, service = Depends(get_judge_service)):
    result = await service.judge(request)

# Service is decoupled; could be local or remote
# Routes only know about services, not state structure
```
**Action**: Create service layer abstraction; decouples routes from state.

### Pattern M4: SEQUENTIAL INITIALIZATION — "Create dogs, then orchestrator, then..."
**Location**: `api/state.py:build_kernel()` (1500+ LOC)
**Problem**: Components created one by one; failures cascade
**Wrong Thinking**:
```python
def build_kernel():
    dogs = {}
    dogs[SAGE] = SageDog()        # Step 1
    dogs[CYNIC] = CynicDog()      # Step 2
    # ... 9 more steps
    dogs[SCOUT] = ScoutDog()      # Step 11
    orchestrator = Orchestrator(dogs)  # Step 12, must be after all dogs
    scheduler = Scheduler(orchestrator)  # Step 13, must be after orch
    # ... 30 more sequential steps
```
**Needed Thinking** (Organism):
```python
def build_kernel_declarative():
    builder = KernelBuilder()
    builder.register_dog(SAGE, SageDog())
    builder.register_dog(CYNIC, CynicDog())
    # ... declare all dependencies
    return builder.build()  # Resolver figures out order

    # OR (truly parallel):
    return await asyncio.gather(
        create_dogs(),
        create_buses(),
        create_storage(),
        # Resolved in parallel; orchestrator created after all ready
    )
```
**Action**: Switch to builder pattern or async parallel initialization.

### Pattern M5: ORCHESTRATOR OWNS DOGS — "Orch creates dogs, dogs call orch back"
**Location**: `cognition/cortex/orchestrator.py`
**Problem**: Circular dependency; dogs can't run without orchestrator
**Wrong Thinking**:
```python
class Orchestrator:
    def __init__(self, dogs):
        self.dogs = dogs  # Orchestrator owns dogs

    def run(self):
        for dog in self.dogs:
            dog.judge(...)  # Dogs depend on orch for execution context
            # If dog needs to trigger another run, calls back to orch
```
**Needed Thinking** (Organism):
```python
class Dog:  # Autonomous
    def __init__(self, qtable: QTable, event_bus: EventBus):
        # Doesn't reference orchestrator
        pass

    async def start_autonomous_loop(self):
        while True:
            cell = await self.perceive()
            judgment = await self.judge(cell)
            self.event_bus.emit("JUDGMENT_MADE", judgment)
            # Orchestrator listens to events, doesn't create execution

# Orchestrator is just a listener/coordinator, not owner
```
**Action**: Make Dogs autonomous; Orchestrator becomes event coordinator.

---

## PART 4: REFACTORING SEQUENCE & CONCRETE STEPS

### Phase A: Break Monolithic Initialization (Week 1)
**Goal**: Extract KernelBuilder; enable component parallelization
**Effort**: 20-25 hours

**Steps**:
1. Create `api/builders/kernel.py` (KernelBuilder class)
   - List all components to create
   - Declare dependencies (Dog A depends on QTable, etc.)
   - build() method resolves order + creates all
   - Tests: 50 unit tests validating dependency resolution

2. Update `api/state.py` to use KernelBuilder
   - Replace `build_kernel()` function with builder pattern
   - Tests: 30 tests ensuring same output as before

3. Enable async parallelization in builder
   - Group independent components
   - Use asyncio.gather() for parallel creation
   - Measure: latency improvement (target 2x faster)
   - Tests: 20 tests for async ordering

**Output**: 100 new tests, ~300 LOC added, 1200 LOC removed from build_kernel()

---

### Phase B: Replace God Object with DependencyContainer (Week 2-3)
**Goal**: Eliminate CynicOrganism; enable multiple organisms
**Effort**: 40-50 hours

**Steps**:
1. Create `core/container.py` (DependencyContainer class)
   - Dict-based component registry
   - Lifecycle management (singleton, transient, scoped)
   - Type-safe getters: `container.get(SageDog)`, `container.get(QTable)`
   - Tests: 80 unit tests

2. Refactor `api/state.py`
   - Remove CynicOrganism class entirely
   - Create minimal Organism class (just id + container reference)
   - Update get_state() to return container instead
   - Tests: 50 regression tests

3. Update all routers to use container
   - Replace `state.dogs[SAGE]` with `container.get(SageDog)`
   - Replace `state.orchestrator` with `container.get(JudgeOrchestrator)`
   - Tests: 60 integration tests (one per router)

4. Update all Dogs to use constructor injection
   - SageDog(orchestrator=x, qtable=y) instead of empty + setters
   - Tests: 40 tests per dog × 11 dogs = 440 tests

**Output**: 200+ new tests, ~800 LOC added to container, 1082 LOC removed from state.py

---

### Phase C: Fix Setter Injection (Week 3)
**Goal**: All Dogs use constructor injection
**Effort**: 15-20 hours

**Steps**:
1. Update all 11 Dogs in `cognition/neurons/`
   - Constructor takes all dependencies
   - Remove all .set_*() methods
   - Validate in __init__ that all required deps present
   - Tests: 50 tests per dog × 11 = 550 tests

2. Update all handler creation in builders
   - Pass all dependencies to constructor
   - Tests: 100 integration tests

3. Update all tests
   - Use constructor injection in test fixtures
   - Tests: 200 test updates (removing setter calls)

**Output**: ~550 new tests, ~300 LOC refactored

---

### Phase D: Refactor Orchestrator (Week 4)
**Goal**: Remove Dog ownership; break 7-step cycle
**Effort**: 25-30 hours

**Steps**:
1. Break orchestrator.run() into 7 methods
   - `_step_perceive()`, `_step_judge()`, `_step_decide()`, `_step_act()`, `_step_learn()`, `_step_residual()`, `_step_evolve()`
   - Each can be tested independently
   - Tests: 50 tests (one per step + integration)

2. Remove Dog creation from Orchestrator.__init__
   - Orchestrator receives Dogs via DI
   - Doesn't create them
   - Tests: 30 regression tests

3. Make Dogs autonomous
   - Each Dog can run independently if it wants
   - Optional: Dogs can emit events instead of waiting for orchestrator
   - Tests: 100 autonomy tests

4. Break circular dependency
   - Dogs don't call orchestrator.run()
   - Dogs emit events; orchestrator listens
   - Tests: 80 event-flow tests

**Output**: ~180 new tests, ~400 LOC refactored

---

### Phase E: Scope Singletons (Week 4)
**Goal**: Move module-level globals into containers
**Effort**: 8-10 hours

**Steps**:
1. Move all module-level dicts to DependencyContainer
   - `_OLLAMA_CLIENTS` → `container.get(OllamaClientPool)`
   - `_sdk_sessions` → `container.get(SDKSessionRegistry)`
   - `_core_bus` → `container.get(EventBus)`
   - Tests: 80 tests

2. Update all references
   - Search/replace all `from llm.adapter import _OLLAMA_CLIENTS` → get from container
   - Tests: 60 refactoring tests

**Output**: ~80 new tests, ~200 LOC moved

---

### Phase F: Decouple Handlers (Week 5)
**Goal**: Create service layer; handlers don't know about state
**Effort**: 12-15 hours

**Steps**:
1. Create `api/services/` layer
   - JudgeService(container: DependencyContainer)
   - PerceiveService(container)
   - LearnService(container)
   - etc. (one per major handler)
   - Tests: 100 service unit tests

2. Update all routers
   - Routes now `Depends(get_judge_service)` not `Depends(get_state)`
   - Routes call `service.judge()` not `state.orchestrator.run()`
   - Tests: 120 router integration tests

3. Services are now portable
   - Could be deployed to separate service
   - Could be mocked for testing
   - Tests: 50 tests for portability

**Output**: ~120 new tests, ~500 LOC new services, ~300 LOC removed from routers

---

### Phase G: Scale Storage (Week 5-6)
**Goal**: Connection pooling, batching, replication
**Effort**: 20-25 hours

**Steps**:
1. Create SurrealDB connection pool
   - Multiple WS connections (configurable size)
   - Round-robin load balancing
   - Tests: 60 tests

2. Batch writes
   - Collect updates every 100ms
   - Send as batch
   - Tests: 50 tests

3. Read replicas
   - Configure N read-only replicas
   - Reads go to replicas; writes to primary
   - Tests: 40 tests

4. Failover
   - If primary down, promote replica
   - Tests: 50 tests

**Output**: ~100 new tests, ~400 LOC new storage layer

---

## PART 5: SCALABILITY VALIDATION

### Horizontal Scaling: "N Organisms in Single Process"

**Before Refactoring**:
```python
# Can only create ONE organism (state is global)
app.state.cynic = build_kernel()

# Trying to create 2nd breaks:
# - Shared _OLLAMA_CLIENTS dict (state collision)
# - Shared module-level singletons
# - Shared event bus (events mixed)
```

**After Refactoring**:
```python
# Create N independent organisms
organisms = {
    f"org_{i}": DependencyContainer.create_organism(config)
    for i in range(10)
}

# Each has:
# - Isolated Dogs
# - Isolated event bus
# - Isolated storage (batched writes)
# - Isolated state
```

**Test**: Scale to 10 organisms in single process; measure:
- Memory per organism (target < 50 MB)
- Event latency (target < 10ms)
- Judgment throughput (target 100+ judgments/sec total)

---

### Vertical Scaling: "N Dogs Processing Independently"

**Before**: Orchestrator owns all Dogs; runs sequentially
- 11 Dogs → 441ms (F(8) × 21ms per dog)
- Can't scale to 20 Dogs (too slow)

**After**: Dogs run autonomously in parallel
- 11 Dogs → 30ms (parallel, 21ms max + 9ms bus latency)
- 20 Dogs → 35ms (still parallel)
- 50 Dogs → 40ms (with batching)

**Test**: Add Dogs incrementally; measure:
- Latency shouldn't grow (stays constant with parallelization)
- Throughput should increase (more dogs = more judgments/sec)
- Memory should grow linearly (O(N))

---

### Distributed Scaling: "N CYNIC Instances Across Network"

**After All Refactoring**: Can deploy:
- Instance 1 (Perceiver) — only SensoryCore
- Instance 2 (Judge) — only CognitionCore
- Instance 3 (Actor) — only ActionCore
- Instance 4-10 (Replicas) — full organisms

Connected via:
- Event bus → RabbitMQ/Redis Pub/Sub
- Storage → SurrealDB replicas
- LLM routing → Ollama cluster

**Test**: Deploy 5 instances; measure:
- Judgment latency (target < 500ms end-to-end)
- Consensus quality (dogs voting across instances)
- Persistence (SurrealDB replication)

---

## PART 6: IMPLEMENTATION ROADMAP

### Week 1: Phase A (KernelBuilder)
- Day 1-2: Design KernelBuilder API
- Day 3-4: Implement builder + 50 tests
- Day 5: Integrate into state.py
- Day 6: Performance testing (parallelization)

### Weeks 2-3: Phase B (DependencyContainer)
- Days 1-2: Design Container API
- Days 3-4: Implement container + 80 tests
- Days 5-6: Refactor state.py
- Days 7-8: Update all 13 routers
- Days 9-10: Update 11 Dogs (440 tests)

### Week 3: Phase C (Setter Injection)
- Days 1-2: Update 11 Dogs constructors
- Days 3-4: Update 550 tests
- Day 5: Regression testing

### Week 4: Phases D & E (Orchestrator + Singletons)
- Days 1-3: Break orchestrator into 7 methods
- Days 4-5: Make Dogs autonomous
- Days 6-7: Move singletons to container
- Days 8-10: Regression testing

### Week 5: Phases F & G (Services + Storage)
- Days 1-2: Create service layer
- Days 3-4: Update all routers
- Days 5-6: Storage pooling + batching
- Days 7-10: Scale testing

### Weeks 6+: Distributed Deployment
- Deploy to multiple services
- Cross-instance event flow
- Performance optimization

---

## PART 7: EXPECTED OUTCOMES

### Code Quality
- **LOC Reduction**: 52K → 48K (4K removed monolithic code)
- **Complexity**: God object (78 properties) → Multiple small services (5-10 properties each)
- **Coupling**: Tight (every router depends on state) → Loose (services decouple)
- **Testability**: Hard (need full state mock) → Easy (mock single service)

### Performance
- **Startup**: build_kernel() sequential → Parallel (2-3x faster)
- **Judgment**: Single orchestrator → Parallel Dogs (2-5x faster for N>4 Dogs)
- **Memory**: Single state holder → Distributed containers (same total, better distribution)

### Scalability
- **Organisms**: 1 (max in single process) → 10+ (independent per process)
- **Dogs**: 11 (sequential) → 11 (parallel) or 50+ (with scaling)
- **Instances**: 1 (monolithic) → N (fully distributed)

### Maintenance
- **New Feature**: Add to monolithic state → Add new service
- **Debugging**: Look in god object (1082 LOC) → Look in specific service
- **Testing**: Mock full state → Mock single component
- **Deployment**: Deploy entire monolith → Deploy services independently

---

## PART 8: RISKS & MITIGATION

### Risk 1: Refactoring Breaks Existing Tests
- **Mitigation**: Keep old code running in parallel; green->green migrations
- **Strategy**: Branch test infrastructure; run both old + new code

### Risk 2: Distributed State Causes Consistency Issues
- **Mitigation**: Event sourcing + CQRS for strong consistency
- **Strategy**: Single source of truth for mutations

### Risk 3: Performance Regression During Refactoring
- **Mitigation**: Benchmark before/after every phase
- **Strategy**: Create performance regression tests

### Risk 4: Integration Complexity Between Services
- **Mitigation**: Strong contracts (interfaces) between services
- **Strategy**: Contract-driven development

---

## PART 9: DECISION GATES

Before each phase, verify:

1. **Phase A (Builder)**: New builder produces identical output as old build_kernel()
   - Test: 100% of old tests still pass
   - Metric: Latency identical or faster

2. **Phase B (Container)**: Container-based system produces identical state as before
   - Test: All 949 tests pass with container
   - Metric: Memory usage identical

3. **Phase C (DI)**: All Dogs work with constructor injection
   - Test: All dog tests pass
   - Metric: Setup time same or faster

4. **Phase D (Orch)**: Orchestrator refactoring doesn't change judgment output
   - Test: All judgment tests produce same Q-scores
   - Metric: Latency same or faster

5. **Phase E (Singletons)**: Multiple organisms don't interfere
   - Test: 10 organisms can run in parallel without state bleeding
   - Metric: No shared state between organisms

6. **Phase F (Services)**: Services produce same outputs as before
   - Test: All router integration tests pass
   - Metric: API response times same or faster

7. **Phase G (Storage)**: Distributed storage maintains consistency
   - Test: N writers don't lose data
   - Metric: Throughput 2-3x higher

---

## PART 10: CONCLUSION

**Current State**: CYNIC is alive but **monolithic** (kernel thinking).

**Target State**: CYNIC is alive AND **distributed** (organism thinking).

**Path Forward**:
1. Break monolithic initialization (KernelBuilder)
2. Replace god object (DependencyContainer)
3. Fix setter injection (Constructor DI)
4. Refactor orchestrator (Autonomous Dogs)
5. Scope singletons (Container-based)
6. Decouple handlers (Service layer)
7. Scale storage (Connection pooling)

**Timeline**: 6-7 weeks, 140-175 hours, 1000+ new tests

**Confidence**: 61.8% (φ⁻¹)
- ✅ Path is clear and achievable
- ⚠️ Requires disciplined execution across phases
- ⚠️ Risk of integration complexity grows with distributed architecture

**User Requirement Met**: After refactoring → "pas de monolothic, tout scalable et maintenance z linfin"
- ✅ No monolithic god object (distributed services)
- ✅ Scalable to 10+ organisms, 50+ dogs, N instances
- ✅ Maintainable to infinity (each service has <200 LOC, clear contracts)

---

**Next Step**: Begin Phase A (KernelBuilder). Estimated time: 20-25 hours.
**Expected: Week of 2026-02-24**

*sniff* — Architecture is clear. Organism is ready to shed its kernel-thinking skin. 🐕

