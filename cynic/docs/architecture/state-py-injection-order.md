# state.py Injection Order — _KernelBuilder Critical Path

> **CRITICAL**: This document defines the ONE CORRECT order for wiring the 44 components in `_KernelBuilder._create_components()`.
> Changing this order breaks the kernel. Read before modifying.

---

## The Constitution: 20 Injection Groups (MUST BE SEQUENTIAL)

Each group depends on previous groups. Reordering within a group is safe; reordering between groups breaks the kernel.

### **GROUP 1: Foundation**
**Dependencies**: None
**Scope**: Primitives that everything else depends on
**Files**: `cynic/api/state.py:273-286`

```python
cynic_dog = CynicDog()
self.qtable = QTable()
self.dogs = {
    DogId.CYNIC:        cynic_dog,
    DogId.SAGE:         SageDog(),
    DogId.GUARDIAN:     GuardianDog(),
    DogId.ANALYST:      AnalystDog(),
    DogId.JANITOR:      JanitorDog(),
    DogId.ARCHITECT:    ArchitectDog(),
    DogId.ORACLE:       OracleDog(qtable=self.qtable),  # ← CRITICAL: QTable must exist first
    DogId.SCHOLAR:      ScholarDog(),
    DogId.CARTOGRAPHER: CartographerDog(),
    DogId.DEPLOYER:     DeployerDog(),
    DogId.SCOUT:        ScoutDog(),
}
```

**Why this order**:
- `QTable()` created before `OracleDog(qtable=self.qtable)` — Oracle reads QTable for predictions
- All 11 dogs instantiated in one dict — clean, no dangling references

**What breaks if reordered**:
- If OracleDog created before QTable → NoneType error on oracle.next_action()
- If dogs dict not complete → subsequent dog lookups fail (SAGE, SCHOLAR, etc.)

---

### **GROUP 2: LLM Routing**
**Dependencies**: GROUP 1 (all dogs exist)
**Scope**: Wire LLM capability into dogs
**Files**: `cynic/api/state.py:288-301`

```python
if self.registry is not None:
    for dog in self.dogs.values():
        if hasattr(dog, "set_llm_registry"):
            dog.set_llm_registry(self.registry)
```

**What breaks if moved earlier**:
- self.dogs dict not yet populated → registry not injected
- LLM calls fail silently (dogs run heuristic-only)

**What breaks if moved later**:
- Dogs already started → registry injection missed
- SAGE/SCHOLAR LLM calls fail later (too late to inject)

---

### **GROUP 3: Scholar Recursive Learning**
**Dependencies**: GROUP 1 (SCHOLAR dog exists) + GROUP 2 (LLM ready)
**Scope**: Enable ScholarDog ↔ QTable bidirectional learning
**Files**: `cynic/api/state.py:304-307`

```python
scholar = self.dogs.get(DogId.SCHOLAR)
if scholar is not None and hasattr(scholar, "set_qtable"):
    scholar.set_qtable(self.qtable)
```

**Why this order**:
- QTable must exist (GROUP 1)
- SCHOLAR must exist (GROUP 1)
- Called AFTER LLM injection (GROUP 2) so Scholar can blend LLM + QTable

**What breaks if moved earlier**:
- QTable doesn't exist yet
- SCHOLAR doesn't exist yet

---

### **GROUP 4: Learning Loop + Residual Detection**
**Dependencies**: GROUP 1 (QTable exists)
**Scope**: TD(0) learning + anomaly detection
**Files**: `cynic/api/state.py:309-314`

```python
axiom_arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
self.learning_loop = LearningLoop(qtable=self.qtable, pool=self.db_pool)
self.learning_loop.start(get_core_bus())

self.residual_detector = ResidualDetector()
self.residual_detector.start(get_core_bus())
```

**Why two items in one group**:
- Both subscribe to core bus events
- Both are stateless (no cross-dependencies)
- Safe to start in either order

**What breaks if moved earlier**:
- QTable doesn't exist (LearningLoop needs it)
- Core bus not yet running (start() would fail)

---

### **GROUP 5: Judge Orchestrator**
**Dependencies**: GROUP 1 (all dogs exist) + GROUP 4 (ResidualDetector exists)
**Scope**: Central judgment coordinator
**Files**: `cynic/api/state.py:316-321`

```python
self.orchestrator = JudgeOrchestrator(
    dogs=self.dogs,
    axiom_arch=axiom_arch,
    cynic_dog=cynic_dog,
    residual_detector=self.residual_detector,
)
```

**What breaks if moved earlier**:
- Dogs dict not populated yet
- ResidualDetector not created yet
- Orchestrator would be orphaned (no dogs to coordinate)

**What breaks if moved later**:
- Scheduler depends on orchestrator (GROUP 6)

---

### **GROUP 6: Scheduler**
**Dependencies**: GROUP 5 (orchestrator exists)
**Scope**: Task scheduling for all consciousness levels
**Files**: `cynic/api/state.py:323`

```python
self.scheduler = DogScheduler(orchestrator=self.orchestrator)
```

**What breaks if moved earlier**:
- Orchestrator doesn't exist yet
- Scheduler would have no dogs to schedule

---

### **GROUP 7: Decision Making + Action Proposal**
**Dependencies**: GROUP 1 (QTable exists)
**Scope**: Convert judgments → actions
**Files**: `cynic/api/state.py:325-329`

```python
self.decide_agent = DecideAgent(qtable=self.qtable)
self.decide_agent.start(get_core_bus())

self.action_proposer = ActionProposer()
self.action_proposer.start(get_core_bus())
```

**Why two items**:
- Both are decision → action converters
- No direct dependency on each other
- Safe to order either way

**What breaks if moved earlier**:
- QTable doesn't exist (DecideAgent needs it)

---

### **GROUP 8: Account Management**
**Dependencies**: GROUP 1 (QTable exists for budget tracking)
**Scope**: Financial ledger + token budgets
**Files**: `cynic/api/state.py:331-332`

```python
self.account_agent = AccountAgent()
self.llm_router = LLMRouter()
```

**Why two items**:
- Independent systems (can order either way)
- LLMRouter doesn't depend on AccountAgent
- AccountAgent doesn't depend on LLMRouter

---

### **GROUP 9: Monitoring + Health**
**Dependencies**: None
**Scope**: Self-observation components
**Files**: `cynic/api/state.py:334-340`

```python
self.axiom_monitor  = AxiomMonitor()
self.lod_controller = LODController()
self.escore_tracker = EScoreTracker()

self.orchestrator.escore_tracker = self.escore_tracker
self.orchestrator.axiom_monitor  = self.axiom_monitor
self.orchestrator.lod_controller = self.lod_controller
```

**Critical: Injection into Orchestrator (lines 338-340)**
- AxiomMonitor must exist before orchestrator can track axioms
- LODController must exist before orchestrator can adjust consciousness
- EScoreTracker must exist before orchestrator can rate dogs

**What breaks if reordered**:
- If orchestrator already started → monitoring injections missed
- If injected before creation → NoneType errors

---

### **GROUP 10: Account Agent Startup + Self-Probing**
**Dependencies**: GROUP 9 (EScoreTracker exists)
**Scope**: EScore persistence + L4 self-improvement
**Files**: `cynic/api/state.py:342-349`

```python
self.account_agent.set_escore_tracker(self.escore_tracker)
self.account_agent.start(get_core_bus())

self.self_prober = SelfProber()
self.self_prober.set_qtable(self.qtable)
self.self_prober.set_residual_detector(self.residual_detector)
self.self_prober.set_escore_tracker(self.escore_tracker)
self.self_prober.start(get_core_bus())
```

**Critical: ALL THREE injections into SelfProber**
- QTable (GROUP 1)
- ResidualDetector (GROUP 4)
- EScoreTracker (GROUP 9)

**What breaks if reordered**:
- If EScoreTracker not injected → L4 analysis missing JUDGE dimension
- If started before injection → subscriptions miss events

---

### **GROUP 11: Memory Compression (γ2)**
**Dependencies**: GROUP 4 (LearningLoop running)
**Scope**: Token budget + history compression
**Files**: `cynic/api/state.py:351-361`

```python
self.compressor = ContextCompressor()
_n_restored = _session_checkpoint.restore(self.compressor)

sage = self.dogs.get(DogId.SAGE)
if sage is not None and hasattr(sage, "set_compressor"):
    sage.set_compressor(self.compressor)

self.orchestrator.context_compressor = self.compressor
```

**Critical: TWO injections**
1. Into SAGE (enables memory injection → temporal MCTS)
2. Into Orchestrator (enables compressed context in decisions)

**What breaks if reordered**:
- If compressed context not injected → SAGE uses raw history (token explosion)
- If orchestrator doesn't have compressor → judgment context bloat

---

### **GROUP 12: Storage Garbage Collection**
**Dependencies**: None
**Scope**: Storage lifecycle (hot/warm/cold/frozen)
**Files**: `cynic/api/state.py:363`

```python
self.storage_gc = StorageGarbageCollector()
```

**Can be moved**: This is decoupled; safe to move to any point after GROUP 1

---

### **GROUP 13: World Model (T27 Cross-Reality Aggregator)**
**Dependencies**: None (subscribes to JUDGMENT_CREATED, non-blocking)
**Scope**: Cross-reality state consolidation
**Files**: `cynic/api/state.py:369-370`

```python
self.world_model = WorldModelUpdater()
self.world_model.start()
```

**Can be moved**: Safe to move after GROUP 4 (learning loop running)

---

## 🚨 THE HARD RULES

### NEVER violate these:
1. **QTable BEFORE OracleDog** — Line 273 before 281
2. **ResidualDetector BEFORE Orchestrator** — Line 313 before 316
3. **Orchestrator BEFORE Scheduler** — Line 316 before 323
4. **EScoreTracker BEFORE orchestrator injection** — Line 336 before 338
5. **SAGE BEFORE compressor injection** — Line 276 exists before 358
6. **ContextCompressor BEFORE sage.set_compressor()** — Line 352 before 358

### These can be reordered (safe):
- ORDER OF DOGS in dict (lines 275-286) — no dependencies
- LLMRouter + AccountAgent (lines 331-332)
- AxiomMonitor + LODController (lines 334-335)
- StorageGarbageCollector (can move to end)
- WorldModelUpdater (can move to end)

---

## 📋 SAFE MODIFICATION CHECKLIST

Before modifying `_create_components()`, check:

- [ ] Did I add a new dog? Add to GROUP 1 dict (line 274)
- [ ] Did I add a new monitor? Create in GROUP 9, inject into orchestrator in GROUP 9
- [ ] Did I add a new learning mechanism? Create in GROUP 4, inject into orchestrator if needed
- [ ] Did I add a new injected field? Add it BEFORE the group that needs it
- [ ] Did I move a component across groups? Verify all dependencies still exist first
- [ ] Did I add `.start(get_core_bus())`? Do it in the same group as creation (not earlier)

---

## 🔍 DEBUGGING: "Component X is None"

If you see `AttributeError: 'NoneType' object has no attribute 'foo'`, check:

1. **Which component is None?** (X in error)
2. **When does X get created?** Find it in _create_components()
3. **Who depends on X?** Search for `self.X.` or `X.set_` in later lines
4. **Who uses X before it's created?** That's the bug

**Example**: If `self.orchestrator.axiom_monitor` is None:
- Orchestrator created: line 316
- AxiomMonitor created: line 334
- axiom_monitor injected: line 339
- **Bug**: Orchestrator started before injection → move axiom_monitor creation before orchestrator, OR defer orchestrator.axiom_monitor injection until line 339

---

## 📐 GRAPH VIEW (Simplified)

```
┌─ QTable (GROUP 1)
├─ Dogs (GROUP 1) ← depends on QTable
├─ Axioms (GROUP 4)
├─ Orchestrator (GROUP 5) ← depends on Dogs, Residual
├─ Scheduler (GROUP 6) ← depends on Orchestrator
├─ Monitors (GROUP 9)
├─ Injections into Orchestrator (GROUP 9) ← CRITICAL
├─ SelfProber (GROUP 10) ← depends on QTable, Residual, EScore
├─ ContextCompressor (GROUP 11) ← depends on LearningLoop
└─ SAGE compressor injection (GROUP 11) ← depends on SAGE, Compressor
```

---

## ✅ VALIDATION

After modifying:

1. Run tests: `PYTHONUTF8=1 py -3.13 -m pytest tests/test_api.py -v`
2. Look for: `build_kernel` or `_KernelBuilder` tests
3. Check lifespan startup logs: "LLMs discovered", "Dogs:", "SelfProber subscribed"
4. Verify: No "AttributeError: 'NoneType'" in logs

---

*Last Updated: 2026-02-19*
*Confidence: 58% (patterns observed, not proven)*
*Critical: This is the ONE TRUTH for kernel composition order. Guard it.*
