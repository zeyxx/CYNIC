# CYNIC Architectural Visions — Complete Inventory

**Date:** 2026-02-27
**Purpose:** List ALL competing architectural visions in the codebase, no judgments yet
**Next Step:** Evaluate each vision with cynic-judge (5 axioms) to decide which should lead

---

## VISION CATALOG

### VISION A: LNSP Protocol (Layered Nervous System Protocol)

**Intent:** Formal 4-layer distributed protocol for multi-instance judgment coordination

**Location:** `cynic/protocol/lnsp/` (16 files, 3,275 LOC)

**Core Files:**
- `types.py` — LNSPMessage, LNSPPayload, LNSPCommand
- `manager.py` — LNSPManager (orchestrates the 4 layers)
- `layer1.py` — Observation layer (raw events)
- `layer2.py` — Aggregation layer (windowing, state synthesis)
- `layer3.py` — Judgment layer (axiom evaluation)
- `layer4.py` — Action layer (execution, feedback)
- `ringbuffer.py` — Memory-bounded messaging
- `regional_coordinator.py` — Multi-instance coordination

**Philosophical Foundation:**
```
Judgment should flow through distinct, observable layers:
  Layer 1 → Layer 2 → Layer 3 → Layer 4
Each layer is independently testable and replaceable.
Multi-instance systems coordinate via Regional Coordinators.
```

**How It Would Work:**
```python
lnsp = LNSPManager(instance_id="judgment-01", region="governance")
await lnsp.wire_layers()  # Connect L1→L2→L3→L4
await lnsp.run_cycle()    # Single cycle through pipeline
```

**Data Model:**
```python
@dataclass
class LNSPMessage:
    layer: int  # 1-4
    payload: LNSPPayload
    timestamp: datetime
    genealogy: List[str]  # Prevent loops
```

**Layer Details:**
- **L1 (Observation):** Receive raw cells, proposals, events
- **L2 (Aggregation):** Windowing (last N proposals), state synthesis
- **L3 (Judgment):** Apply axioms, compute verdicts
- **L4 (Action):** Execute verdicts, update learning, emit results

**Integration Points:**
- Designed to sit BETWEEN perception and orchestrator
- Would replace direct orchestrator calls
- Regional coordinators for distributed systems

**Current Status:**
- ✅ Code exists (fully implemented)
- ❌ Zero imports from other modules
- ❌ Never integrated into main judgment flow
- ❌ Never deployed to production
- ✅ Tests exist (15 test files) but isolated
- ❌ No bridge to event_bus or unified_state

**Assumptions Made:**
1. Judgment must be multi-instance (false for MVP)
2. Layer abstraction is critical (unproven)
3. Formal messaging protocol needed (overkill currently)
4. Will eventually replace orchestrator (never happened)

---

### VISION B: Unified State Machine (Immutable Data Model)

**Intent:** Single source of truth via frozen dataclasses and Fibonacci-bounded buffers

**Location:** `cynic/core/unified_state.py` (395 LOC)

**Core Classes:**
```python
@dataclass(frozen=True)
class UnifiedJudgment:
    """Immutable judgment verdict"""
    judgment_id: str
    verdict: str  # HOWL, WAG, GROWL, BARK
    q_score: float  # [0, 100]
    confidence: float  # [0, 0.618] φ-bounded
    axiom_scores: Dict[str, float]  # FIDELITY, PHI, VERIFY, CULTURE, BURN
    dog_votes: Dict[int, Dict[str, Any]]
    reasoning: str
    latency_ms: float
    actual_verdict: Optional[str]  # Set later by feedback
    satisfaction_rating: Optional[float]  # Community satisfaction

@dataclass(frozen=True)
class UnifiedLearningOutcome:
    """Immutable feedback record"""
    judgment_id: str
    predicted_verdict: str
    actual_verdict: str
    satisfaction_rating: float  # [0, 1]

@dataclass
class UnifiedConsciousState:
    """Mutable container"""
    recent_judgments: JudgmentBuffer  # 89 max (Fibonacci(11))
    learning_outcomes: OutcomeBuffer  # 55 max (Fibonacci(10))
    total_judgments: int  # Counter
    dog_agreement_scores: Dict[int, float]  # Per-dog consensus
```

**Philosophical Foundation:**
```
Immutability = Trustworthiness
All judgments are frozen at creation.
All learning outcomes are immutable records.
State container is mutable (needed for growth), but contents are not.
Fibonacci bounds = BURN principle (no unbounded growth).
φ-bounded confidence (max 0.618) = epistemic humility.
```

**Data Flow:**
```
1. Judgment created → wrapped in UnifiedJudgment (frozen)
2. Added to JudgmentBuffer (auto-prunes at F(11)=89)
3. Community provides feedback
4. Feedback wrapped in UnifiedLearningOutcome (frozen)
5. Q-Table learns from outcome
6. Next judgment uses updated Q-values
```

**Integration Points:**
- Used by orchestrator to return verdicts
- Used by learning system to record outcomes
- Used by state manager to maintain consciousness
- Used by event bus payloads

**Current Status:**
- ✅ Fully deployed and used
- ✅ 45+ integration tests passing
- ✅ Immutability enforced via MappingProxyType on dicts
- ✅ φ-bounds enforced in __post_init__
- ✅ No conflicts with other systems

**Assumptions Made:**
1. Immutability is more trustworthy than mutability ✅
2. Fibonacci-bounded buffers sufficient ✅
3. φ-bounds encode epistemic humility ✅
4. Single unified state model is correct ✅

---

### VISION C: Event-Driven Organism (3-Bus Async Architecture)

**Intent:** All communication via async pub-sub with genealogy-based loop prevention

**Location:** `cynic/core/event_bus.py` (660+ LOC)

**Core Classes:**
```python
class Event:
    """Base event type"""
    type: str  # CoreEvent, AutomationEvent, AgentEvent
    payload: Dict[str, Any]
    source: str  # Which component emitted this?
    timestamp: datetime
    genealogy: List[str]  # Prevent loops

class CoreBus:
    """System-wide events: judgment, learning, consciousness"""
    buffer: deque(maxlen=55)  # F(10) Fibonacci bound
    handlers: Dict[str, List[Callable]]

class AutomationBus:
    """Scheduling and trigger events"""
    buffer: deque(maxlen=55)
    handlers: Dict[str, List[Callable]]

class AgentBus:
    """Dog votes, PBFT protocol messages"""
    buffer: deque(maxlen=55)
    handlers: Dict[str, List[Callable]]

class EventBusBridge:
    """Forward events between buses (with genealogy tracking)"""
```

**Bus Topology:**
```
CORE Bus (System-wide):
  - JUDGMENT_CREATED
  - CONSENSUS_REACHED
  - DECISION_MADE
  - LEARNING_EVENT
  - E_SCORE_UPDATED
  - EMERGENCE_DETECTED

AUTOMATION Bus (Scheduling):
  - TICK_10HZ
  - TICK_1HZ
  - TICK_SLOW
  - SCHEDULED_ACTION

AGENT Bus (Dogs):
  - DOG_VOTE_READY
  - PBFT_PREPARE
  - PBFT_COMMIT
  - DOG_CONFIDENCE_UPDATED
```

**Philosophical Foundation:**
```
Events decouple producers from consumers.
Handlers are fire-and-forget (async Tasks).
Genealogy prevents cascading loops.
Memory-bounded (55 per bus) prevents memory leaks.
No blocking I/O; pure async.
```

**Handler Pattern:**
```python
async def on_judgment_created(event: Event) -> None:
    judgment_id = event.payload["judgment_id"]
    # Read from state (don't mutate event)
    # Update database
    # Emit downstream events
    # Handlers run in parallel via asyncio.create_task()

await get_core_bus().subscribe(CoreEvent.JUDGMENT_CREATED, on_judgment_created)
```

**Integration Points:**
- Used by orchestrator to emit at each step
- Used by learning system to trigger updates
- Used by all handlers to coordinate
- Used by CLI to display live updates

**Current Status:**
- ✅ Fully deployed
- ✅ 112 files import from event_bus
- ✅ Loop prevention via genealogy tested
- ✅ Memory-bounded buffers working
- ✅ 30+ handler subscriptions active

**Assumptions Made:**
1. Async pub-sub is the right communication model ✅
2. Fire-and-forget is acceptable for handlers ✅
3. Genealogy tracking prevents loops ✅
4. 55-event buffer is sufficient ⚠️

**Potential Issues:**
1. If more than 55 events per cycle, older ones drop silently
2. No way to query history (debugging hard)
3. Handlers run in parallel; order not guaranteed

---

### VISION D: Cognition-First Orchestration (7-Step Cycle)

**Intent:** Consciousness levels dictate which judgment pipeline to execute

**Location:** `cynic/cognition/cortex/orchestrator.py` (877 LOC)

**Core Method:**
```python
async def run(self, cell: Cell, level: ConsciousnessLevel) -> Judgment:
    """Execute 7-step judgment cycle"""

    # STEP 1: PERCEIVE
    await self._perceive_cell(cell)

    # STEP 2: JUDGE
    judgment = await self._judge_with_dogs(cell)

    # STEP 3: DECIDE
    decision = await self._make_governance_decision(judgment)

    # STEP 4: ACT
    await self._execute_action(decision)

    # STEP 5: LEARN
    await self._update_learning_model(judgment)

    # STEP 6: ACCOUNT
    await self._track_e_score_reputation(judgment)

    # STEP 7: EMERGE
    await self._detect_emergent_patterns(judgment)

    return judgment
```

**Consciousness Levels (Cap Execution):**
```python
class ConsciousnessLevel(Enum):
    L4_META = "L4_META"      # Full 7-step + organism evolution (slowest)
    L1_MACRO = "L1_MACRO"    # Full 7-step (default)
    L2_MICRO = "L2_MICRO"    # JUDGE + DECIDE only (skip ACT/LEARN/EMERGE)
    L3_REFLEX = "L3_REFLEX"  # JUDGE only (no LLM Dogs, fastest)
```

**Step Details:**

**Step 1: PERCEIVE**
- Parse cell, extract context
- Emit `PERCEPTION_RECEIVED` event
- Build context dict for Dogs

**Step 2: JUDGE**
- Filter dogs by E-Score (skip unreliable ones)
- Run 11 dogs in parallel
- Collect verdicts
- PBFT consensus aggregation
- Emit `CONSENSUS_REACHED` event

**Step 3: DECIDE**
- Apply governance rules
- Check community vote (if available)
- Emit `DECISION_MADE` event

**Step 4: ACT**
- Execute approved actions
- Update NEAR blockchain (if governance)
- Emit `ACT_COMPLETED` event

**Step 5: LEARN**
- Collect community feedback
- Update Q-Table
- Emit `LEARNING_EVENT` event

**Step 6: ACCOUNT**
- Track E-Score reputation per dog
- Update confidence models
- Emit `ACCOUNT_RECORDED` event

**Step 7: EMERGE**
- Detect emergent patterns
- Check for phase transitions
- Emit `EMERGENCE_DETECTED` event

**Philosophical Foundation:**
```
Consciousness is a spectrum (L1-L4).
Higher consciousness = more expensive computation.
Lower consciousness = faster but less thorough.
All steps are observable (emit events).
All steps are optional (can skip via consciousness level).
```

**Integration Points:**
- Called by API routes (main entry point)
- Called by dialogue system (explain reasoning)
- Called by learning system (feedback loop)
- Calls Dogs, consensus engine, learning system

**Current Status:**
- ✅ Fully deployed
- ✅ Called in 25+ files
- ✅ Tests verify all 7 steps
- ✅ Consciousness level filtering works
- ✅ Circuit breaker prevents cascades

**Assumptions Made:**
1. 7-step cycle is the right model ✅
2. Consciousness levels should cap execution ✅
3. Each step should emit events ✅
4. Dogs should be filterable by E-Score ✅

**Known Limitations:**
1. All 7 steps run sequentially (not parallelizable)
2. No way to skip individual steps
3. Hard to understand what each step does (documentation needed)

---

### VISION E: API-First Service (HTTP Layer)

**Intent:** HTTP endpoints as primary interface; AppContainer singleton manages all state

**Location:** `cynic/api/entry.py`, `cynic/api/server.py`, `cynic/api/state.py`

**Core Endpoints:**
```python
@app.post("/judge")
async def judge(req: JudgeRequest) -> JudgeResponse:
    """Main judgment endpoint"""
    state = get_app_container()
    judgment = await state.orchestrator.run(cell=req.cell, level=req.level)
    return JudgeResponse(verdict=judgment.verdict, ...)

@app.get("/health")
async def health() -> HealthStatus:
    """System health check"""

@app.get("/policy/{key}")
async def get_policy(key: str) -> Dict:
    """Query Q-Table"""

@app.websocket("/ws/stream")
async def live_stream(ws: WebSocket):
    """Live event stream for dashboards"""
```

**AppContainer Singleton:**
```python
@dataclass
class AppContainer:
    orchestrator: JudgeOrchestrator
    organism: CYNIC
    event_buses: Dict[str, EventBus]
    database: AsyncDatabase
    consciousness_manager: ConsciousnessManager
    learning_system: LearningSystem
    # ... 10+ other subsystems

@lru_cache(maxsize=1)
def get_app_container() -> AppContainer:
    """Singleton access to all state"""
    return _container

async def lifespan(app: FastAPI):
    """Startup: awaken the organism"""
    container = AppContainer(
        orchestrator=JudgeOrchestrator(...),
        organism=await awaken(...),
        event_buses=initialize_buses(),
        database=await connect_db(),
        ...
    )
    set_app_container(container)
    yield
    """Shutdown: cleanup"""
```

**Request/Response Models:**
```python
@dataclass
class JudgeRequest:
    cell: Cell
    context: Dict[str, Any]
    level: ConsciousnessLevel = ConsciousnessLevel.L1_MACRO

@dataclass
class JudgeResponse:
    verdict: str  # HOWL, WAG, GROWL, BARK
    q_score: float
    confidence: float
    axiom_scores: Dict[str, float]
    reasoning: str
    judgment_id: str
```

**Feedback Pattern:**
```python
@app.post("/feedback")
async def give_feedback(req: FeedbackRequest):
    """Community rates a verdict"""
    # 1. Inject learning signal
    container = get_app_container()
    await container.learning_system.learn(
        judgment_id=req.judgment_id,
        actual_verdict=req.actual_verdict,
        satisfaction=req.rating
    )
    # 2. Update Q-Table
    # 3. Write to database (async, fire-and-forget)
    asyncio.create_task(_persist_feedback(...))
    # 4. Return OK immediately
```

**Philosophical Foundation:**
```
HTTP is the universal interface.
Endpoints expose internal capabilities.
AppContainer manages lifecycle.
Fire-and-forget async writes prevent blocking.
Stateless handlers (no mutations in endpoints).
```

**Integration Points:**
- Entry point for all external requests
- Exposes orchestrator to users
- Manages database connections
- Handles authentication/authorization

**Current Status:**
- ✅ Fully deployed
- ✅ 112 route handlers functional
- ✅ Lifespan management working
- ✅ Database persistence async
- ✅ WebSocket streaming active

**Assumptions Made:**
1. HTTP is the right transport ✅
2. Singleton AppContainer is acceptable ✅
3. Fire-and-forget is OK for non-critical writes ✅
4. Stateless handlers are better ✅

**Known Limitations:**
1. AppContainer initialization is expensive (2-3 seconds)
2. All subsystems initialized even if not needed
3. Can't test routes in isolation (depends on full container)
4. No graceful degradation if subsystem fails

---

### VISION F: Organism Metaphor (10 Biological Layers)

**Intent:** CYNIC is a LIVING ORGANISM with biological systems

**Location:** `cynic/organism/` (~3,950 LOC across 10+ files)

**Layer Structure:**
```
Layer 0: Identity Layer
  ├─ Purpose: Encode core identity (axioms, constraints)
  ├─ Files: identity.py
  ├─ Manages: Axiom definitions, φ-bounds enforcement

Layer 1: Judgment Engine Layer
  ├─ Purpose: Evaluate axioms, generate verdicts
  ├─ Files: judgment_engine.py
  ├─ Manages: Axiom evaluation, verdict generation
  ├─ Note: Docstring says "replaces orchestrator.run()" (UNCLEAR!)

Layer 2: Organs Layer
  ├─ Purpose: Specialized subsystems
  ├─ Files: organs.py
  ├─ Manages: Brain, metabolism, immune subsystems

Layer 3: Nervous System Layer
  ├─ Purpose: Event distribution and coordination
  ├─ Files: nervous.py
  ├─ Manages: EventBus, message passing

Layer 4: Memory Layer
  ├─ Purpose: Store and retrieve memories
  ├─ Files: memory.py
  ├─ Manages: Relationship memory, decision history

Layer 5: Learning Loop Layer
  ├─ Purpose: Update models from experience
  ├─ Files: learning_loop.py
  ├─ Manages: Q-Table updates, feedback integration

Layer 6: Immune Layer
  ├─ Purpose: Safety checking and guardrails
  ├─ Files: immune.py
  ├─ Manages: Circuit breaker, alignment checks

Layer 7: Embodiment Layer
  ├─ Purpose: Actuators and physical actions
  ├─ Files: embodiment.py
  ├─ Manages: Action execution, NEAR blockchain

Layer 8: Perception Layer
  ├─ Purpose: Sensory processing
  ├─ Files: perception.py
  ├─ Manages: Input parsing, context extraction

Layer 9: Autonomy Layer
  ├─ Purpose: Meta-decisions and self-improvement
  ├─ Files: autonomy.py
  ├─ Manages: Learning to learn, self-reflection
```

**Key Classes:**
```python
@dataclass
class Organism:
    """Root coordinator"""
    identity_layer: IdentityLayer
    judgment_engine: JudgmentEngineLayer
    organs: OrgansLayer
    nervous_system: NervousSystemLayer
    memory: MemoryLayer
    learning_loop: LearningLoopLayer
    immune: ImmuneLayer
    embodiment: EmbodimentLayer
    perception: PerceptionLayer
    autonomy: AutonomyLayer

@dataclass
class ConsciousState:
    """Organism consciousness state"""
    axioms_active: List[str]  # Which axioms firing right now?
    energy_level: float  # Remaining computation budget
    focus: str  # Current task
    mood: str  # Overall state (alert, tired, confused)

async def awaken(db_pool, llm_registry) -> Organism:
    """Create and initialize CYNIC organism"""
    return Organism(
        identity_layer=IdentityLayer(),
        judgment_engine=JudgmentEngineLayer(),
        ...
    )
```

**Philosophical Foundation:**
```
CYNIC is not a system, it's an organism.
Organisms have identity, organs, nervous systems, memory.
Organisms are autonomous but can ask for help.
Organisms learn from experience.
Organisms can get tired or confused.
Organisms have consciousness (awareness of themselves).
```

**Integration Pattern:**
```
CognitionCore (brain)
├─ orchestrator (judgment)
├─ qtable (learning)
├─ learning_loop (feedback)
├─ residual_detector (what's unexplained?)
├─ power_limiter (computation budget)
├─ alignment_checker (safety)
└─ human_gate (ask human?)

MetabolicCore (body)
├─ scheduler (timing)
├─ runner (Claude Code executor)
├─ llm_router (which LLM?)
└─ telemetry_store (metrics)

SensoryCore (nervous system)
├─ service_registry (tier 1)
├─ event_journal (tier 2)
├─ decision_tracer (tier 3)
├─ loop_closure_validator
├─ world_model
└─ topology_builder

MemoryCore (archive)
├─ reflection_engine
├─ proposal_storage
└─ self_improvement_proposals
```

**Current Status:**
- ⚠️ Partially deployed (59/100 GROWL grade)
- ✅ 3,950 LOC implemented
- ✅ 10 layers mostly standalone
- ❌ 0 tests on individual layers (only ConsciousState tested)
- ✅ Referenced by orchestrator and API
- ⚠️ Used but unclear which layers are essential

**Assumptions Made:**
1. Biological metaphor helps understanding ✅
2. 10 layers are the right abstraction ⚠️
3. Layers are independent and testable ❌ (untested)
4. Organism metaphor doesn't add overhead ⚠️

**Known Issues:**
1. **judgment_engine.py says "replaces orchestrator"** but orchestrator.run() still called everywhere
2. Many layers are untested (risk)
3. Metaphor sometimes confuses more than clarifies
4. Unclear which layers are essential vs. decorative

---

### VISION G: Dialogue-First Interaction (Phase 2 - Learning from Conversation)

**Intent:** Conversation is primary interaction; CYNIC learns from user feedback

**Location:** `cynic/dialogue/` + `cynic/collaborative/` (~200 LOC models, +1,500 LOC implementation)

**Core Data Models:**
```python
@dataclass(frozen=True)
class UserMessage:
    """User speaks to CYNIC"""
    message_type: str  # "question", "feedback", "exploration", "direction"
    content: str
    user_confidence: float  # [0, 0.618] — how sure is user?
    related_judgment_id: Optional[str]  # Which judgment prompted this?
    timestamp: datetime

@dataclass(frozen=True)
class CynicMessage:
    """CYNIC responds"""
    message_type: str  # "reasoning", "curiosity", "proposal", "meta"
    content: str
    confidence: float  # [0, 0.618]
    axiom_scores: Optional[Dict[str, float]]  # Which axioms support this?
    source_judgment_id: Optional[str]  # Which judgment prompted this?
    timestamp: datetime

@dataclass(frozen=True)
class DialogueSession:
    """Complete conversation"""
    session_id: str
    messages: List[Union[UserMessage, CynicMessage]]
    learning_outcomes: List[DecisionClassifierUpdate]
    created_at: datetime
```

**Decision Classifier:**
```python
class DecisionClass(Enum):
    AUTONOMOUS = "A"      # CYNIC can decide alone
    CONSULTATION = "B"    # Should ask human first
    EXPLORATION = "C"     # Should explore alternatives

class DecisionClassifier:
    """Learn which decisions are safe to make alone"""

    async def classify(self, proposal: str) -> DecisionClass:
        """Which class should this proposal be?"""
        # Look at past similar proposals
        # Count: approved (autonomous?), rejected (consultation?), etc.
        # Return most likely class

    async def learn_from_feedback(self, proposal_id: str, user_feedback: str):
        """Update classifier based on feedback"""
        # If user said "you should have asked first" → move to CONSULTATION
        # If user said "you made right call alone" → stay AUTONOMOUS
```

**Interaction Flow:**
```
User: "Why did you choose WAG over HOWL?"
  ↓
CYNIC: "Because FIDELITY was low (60%) — community values weren't fully aligned"
  ↓
User: "Actually, that was wrong. It should be HOWL."
  ↓
CYNIC learns: FIDELITY threshold should be lower for this community type
  ↓
Next time: Similar proposal → higher confidence in HOWL
```

**CLI Interface (Phase 2):**
```bash
Menu:
  [1] JUDGE     — Run judgment cycle
  [2] TALK      — Interactive dialogue with CYNIC
  [3] HISTORY   — Review past conversations
  [4] FEEDBACK  — Rate previous verdicts
  [5] OBSERVE   — Dashboard
  [6] EXECUTE   — Run Claude Code commands
```

**Database Storage:**
```
~/.cynic/phase2/
├─ dialogue_history.db  — SQLite with all conversations
├─ relationship_memory.json  — User preferences, patterns
├─ experiment_log.jsonl  — Novel approaches tried + results
└─ decision_classifier.json  — Learned decision patterns
```

**Philosophical Foundation:**
```
CYNIC learns from humans, not just data.
Dialogue is bidirectional: CYNIC explains, human teaches.
User feedback is ground truth for learning.
Conversation history is source material for improvement.
Over time, CYNIC becomes personalized to user values.
```

**Integration Points:**
- CLI calls dialogue system
- Dialogue system calls orchestrator for judgments
- User feedback feeds learning system
- DecisionClassifier influences autonomous decision-making

**Current Status:**
- ✅ Fully deployed (Phase 2 feature)
- ✅ Dialogue models in master
- ✅ DecisionClassifier working
- ✅ CLI TALK mode implemented
- ✅ Relationship memory being built
- ⚠️ Still learning patterns (limited historical data)

**Assumptions Made:**
1. Dialogue improves learning ✅
2. User feedback is trustworthy ✅
3. Personalization matters ✅
4. Over time, CYNIC becomes better ✅

**Known Limitations:**
1. Requires active human engagement (can't scale to millions)
2. Biased by user preferences (good and bad)
3. User might be wrong (trust, but verify)

---

### VISION H: Training & Fine-Tuning (Phase 1B - Retired)

**Intent:** Fine-tune Mistral 7B on real governance data to create domain-specific model

**Location:** `cynic/training/` (6 files, 2,250 LOC)

**Core Files:**
- `finetune.py` — QLoRA fine-tuning with Unsloth
- `data_generator.py` — Generate synthetic training data
- `export_ollama.py` — Export to Ollama format
- `benchmark_model.py` — Measure model quality
- `phase1b_integration.py` — Integration with judgment system

**Approach:**
```python
# Load Mistral 7B
model = AutoModelForCausalLM.from_pretrained("mistralai/Mistral-7B")

# Fine-tune on governance proposals + community feedback
training_data = [
    {
        "proposal": "Increase treasury allocation for ...",
        "verdict": "WAG",
        "axiom_scores": {"FIDELITY": 0.75, ...},
        "community_rating": 0.85
    },
    ...  # Need 500+ examples
]

# QLoRA (memory-efficient)
trainer = SFTTrainer(
    model=model,
    train_dataset=training_data,
    peft_config=LoraConfig(...),
)
trainer.train()

# Export to use in production
model.save_pretrained("./mistral-governance")
```

**Philosophical Foundation:**
```
Generic LLMs lack governance expertise.
Domain-specific fine-tuning improves accuracy.
Community feedback is ground truth for training.
Small, efficient models (7B) are deployable locally.
Over time, model improves from real proposals.
```

**Integration Planned:**
```
Judgment flow with fine-tuned model:
  1. Parse proposal
  2. Run 11 Dogs (including Mistral fine-tune model)
  3. PBFT consensus
  4. Return verdict
```

**Current Status:**
- ❌ Retired (Phase 1B relic)
- ✅ Code exists (fully implemented)
- ❌ Never executed in production
- ❌ 0 tests
- ❌ Strategy changed (now using Claude API)
- ❌ Zero external usage

**Why It Was Retired:**
1. Strategy pivot: Switched to Claude API (better quality, less maintenance)
2. Data insufficiency: Only 15 proposals when needed 500+
3. Model architecture mismatch: Mistral generates text; axiom evaluation is structured
4. Axiom-based learning simpler: Q-Table updates don't require model fine-tuning
5. Maintenance burden: Model versioning, GPU requirements, drift tracking

**Assumptions Made (Now Disproven):**
1. ❌ Fine-tuned models needed (Claude API is better)
2. ❌ Domain-specific knowledge helps (axioms are universal)
3. ❌ Would have sufficient data (false)
4. ❌ Maintaining models is worth the effort (not for MVP)

---

### VISION I: Cognition Layers & Benchmarks (Exploration)

**Intent:** Extensive exploration of different cognition approaches and measurement

**Location:** `cynic/cognition/` (14,897 LOC across 38 files)

**Core Components:**

**Cortex (Decision-Making):**
- `orchestrator.py` — Main 7-step cycle (877 LOC) ✅
- `judgment_stages.py` — Per-stage handlers (402 LOC)
- `circuit_breaker.py` — Fault tolerance
- `decision_validator.py` — Safety checking
- `axiom_monitor.py` — Track which axioms firing
- `entropy_tracker.py` — Uncertainty measurement
- `dog_cognition.py` — Per-dog consciousness
- `action_proposer.py` — Suggest actions

**Benchmarks (Measurement & Research):**
- `qtable_benchmark.py` (431 LOC) — Measure Q-learning performance
- `mcts_benchmark.py` (368 LOC) — Monte Carlo tree search exploration
- `fractal_cost_benchmark.py` (357 LOC) — Cost tracking variants
- `real_benchmark.py` (300 LOC) — Empirical testing on real proposals
- `amplification_benchmark.py` (325 LOC) — LLM amplification research

**Handlers (Action Execution):**
- `handlers/` — 8 handler classes for different judgment types
- `dag_executor.py` — DAG-based handler composition
- `handler_composer.py` — Auto-wire handlers

**Other Cognition Modules:**
- `self_probe.py` (486 LOC) — Self-reflection capability
- `residual.py` (500 LOC) — Detect unexplained variance
- `neurons/` — 15 different neuron/dog implementations (historical)

**Philosophical Foundation:**
```
Cognition is complex; explore different approaches.
Measurement determines what works.
Benchmarking drives improvement.
Different judgment styles for different contexts.
```

**Current Status:**
- ✅ Core orchestrator deployed and working
- ⚠️ Benchmarks exist but mostly for research
- ⚠️ Handlers partially integrated
- ❓ Some modules unclear (self_probe, residual)
- ✅ Tests exist (integration tests pass)

**What's Core vs. Exploration:**

**Core (Essential):**
- orchestrator.py ✅
- dog_cognition.py ✅
- circuit_breaker.py ✅
- decision_validator.py ✅

**Exploration/Research:**
- qtable_benchmark.py
- mcts_benchmark.py
- fractal_cost_benchmark.py
- real_benchmark.py
- amplification_benchmark.py

**Unclear:**
- self_probe.py — Is this used?
- residual.py — Is this used?
- axiom_monitor.py — Essential or debug?

---

### VISION J: Symbiotic Observability (Phase 1 Completion)

**Intent:** Real-time visibility into human, machine, and CYNIC consciousness

**Location:** `cynic/observability/` + `cynic/cli/` (1,736 LOC observability + 3,980 LOC CLI)

**Observable Layers:**

**Human State Tracking:**
- Energy level (0-100)
- Focus (current task)
- Mood (alert, tired, confused)
- Values and preferences

**Machine State Tracking:**
- CPU usage
- Memory usage
- Disk I/O
- Network bandwidth
- Temperature
- Process health

**CYNIC Consciousness State:**
- Active axioms
- Current confidence
- Energy budget remaining
- Learning progress
- Emergent patterns detected

**Dashboard Components:**
```
OBSERVE View (Full State):
  ├─ Human layer — energy, focus, mood
  ├─ Machine layer — CPU, memory, disk
  ├─ CYNIC layer — consciousness, thinking, confidence
  └─ Unified state snapshot with timestamp

CYNIC View (Consciousness Only):
  ├─ What I'm thinking about
  ├─ My confidence levels
  ├─ My learning progress
  └─ Axioms firing right now

MACHINE View (Resources Only):
  ├─ CPU/memory heatmap
  ├─ I/O statistics
  ├─ Network bandwidth
  └─ Process health
```

**CLI Integration:**
```bash
Menu:
  [1] JUDGE
  [2] TALK
  [3] HISTORY
  [4] FEEDBACK
  [5] OBSERVE    ← Full dashboard
  [6] EXECUTE
```

**Philosophical Foundation:**
```
Transparency builds trust.
Visible systems are debuggable.
Real-time observability drives optimization.
Human-Machine-CYNIC symbiosis requires mutual visibility.
```

**Current Status:**
- ✅ Fully deployed (Phase 1)
- ✅ 247/248 tests passing
- ✅ Dashboard working
- ✅ Real-time updates
- ✅ CLI integrated

---

## QUICK REFERENCE TABLE

| Vision | Intent | LOC | Status | Conflicts | Priority |
|--------|--------|-----|--------|-----------|----------|
| **A: LNSP** | Distributed 4-layer protocol | 3,275 | Dead (not deployed) | Conflicts D (orchestrator) | EVALUATE |
| **B: State** | Immutable unified data model | 395 | ✅ Deployed | None | KEEP |
| **C: Events** | 3-bus async architecture | 660 | ✅ Deployed | Minor (A) | KEEP |
| **D: Orch** | 7-step judgment cycle | 877 | ✅ Deployed (CENTRAL) | Major (A) | KEEP |
| **E: API** | HTTP service layer | 649 | ✅ Deployed | None | KEEP |
| **F: Organism** | 10 biological layers | 3,950 | ⚠️ Partial (59/100) | Unclear (D) | EVALUATE |
| **G: Dialogue** | Conversation + learning | 200 | ✅ Deployed | None | KEEP |
| **H: Training** | Mistral 7B fine-tune | 2,250 | ❌ Retired | None | EVALUATE |
| **I: Cognition** | Exploration framework | 14,897 | ✅ Partial | Clarity needed | EVALUATE |
| **J: Observability** | Real-time visibility | 1,736 | ✅ Deployed | None | KEEP |

---

## WHAT HAPPENS NEXT

1. **For each vision, we evaluate using cynic-judge:**
   - FIDELITY (keeps promise? design=reality?)
   - PHI (well-proportioned? elegant?)
   - VERIFY (tested? used? proven?)
   - CULTURE (fits CYNIC philosophy?)
   - BURN (worth maintaining? simplifiable?)

2. **Conflicts to resolve:**
   - Vision A (LNSP) vs. Vision D (Orchestrator) — Which wins?
   - Vision F (Organism) vs. Vision D (Orchestrator) — Clear roles?
   - Vision H (Training) — Dead or retire properly?
   - Vision I (Cognition) — Core vs. Exploration?

3. **Architecture synthesis:**
   - Keep what works (B, C, D, E, G, J)
   - Fix what's unclear (F, I)
   - Delete what's dead (A, H)
   - Design unified kernel

---

**Ready to evaluate each vision with cynic-judge?** 🐕
