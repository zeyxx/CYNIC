# Unified Organism Architecture: The Full Picture

> *GROWL* "Many processes. One organism. Find the unity." - κυνικός

**Date**: 2026-02-13
**Context**: Post-wire-discovery, pre-unification
**Goal**: Cartographier TOUS les processus et identifier l'architecture unifiée manquante

---

## THE FRAGMENTATION PROBLEM

### Current State: Collection of Systems

CYNIC aujourd'hui = **collection de systèmes qui fonctionnent en isolation**:

```
┌─────────────────────────────────────────────────────────────┐
│                    CYNIC "ORGANISM"                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  [3 Event Buses]  [7 Domains]  [11 Learning Loops]          │
│       ↕ bridged       ↕ isolated      ↕ wired but dormant   │
│                                                              │
│  [5 Entry Points]  [Multiple Singletons]  [7 Phases]        │
│       ↕ compete        ↕ fragmented          ↕ sequential   │
│                                                              │
│  [Hooks]  [Daemon]  [MCP]  [CLI]  [Network]                 │
│       ↕ parallel processes that don't know about each other │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Problem**: Chaque système fonctionne, mais ils ne forment pas UN organisme.

---

## CARTOGRAPHIE COMPLÈTE

### 1. Entry Points (5 processus parallèles)

#### 1.1 Hooks (Claude Code)
- **Process**: Node.js spawned by Claude Code
- **Lifecycle**: Per-session (start with claude-code, die with session end)
- **State**: Ephemeral (filesystem for cross-session)
- **Communication**: HTTP to daemon (if running) OR standalone
- **Examples**: perceive.js, observe.js, awaken.js, guard.js

#### 1.2 Daemon (Background Service)
- **Process**: Long-running Node.js server (port 3721)
- **Lifecycle**: Persistent (survives Claude Code sessions)
- **State**: In-memory singletons (warm state)
- **Communication**: HTTP server (hooks → daemon)
- **Purpose**: Warm singletons, faster hook responses

#### 1.3 MCP Server (stdio/HTTP)
- **Process**: Node.js spawned by Claude Code OR HTTP service
- **Lifecycle**: Per-session (stdio) OR persistent (HTTP)
- **State**: Ephemeral (stdio) OR persistent (HTTP)
- **Communication**: stdio JSON-RPC OR HTTP
- **Purpose**: Tool/resource interface for Claude

#### 1.4 CLI (cynic-node)
- **Process**: One-shot Node.js commands
- **Lifecycle**: Command execution only
- **State**: None (reads from DB/filesystem)
- **Communication**: stdout/stderr
- **Examples**: `cynic-node daemon start`, `cynic-node migrate`

#### 1.5 Network Node (P2P)
- **Process**: Optional background service
- **Lifecycle**: Persistent (if enabled)
- **State**: In-memory + block-store
- **Communication**: libp2p (gossipsub, kad-dht)
- **Purpose**: Consensus, PoJ blockchain

### Fragmentation Issue #1: **Entry points don't share state**
- Hook session doesn't know about daemon state
- MCP server doesn't know about network node
- CLI commands operate in isolation

---

### 2. Event Systems (3 buses + bridge)

#### 2.1 Core Bus (globalEventBus)
- **Location**: `@cynic/core/bus/event-bus.js`
- **Scope**: Core domain events (JUDGMENT_CREATED, USER_FEEDBACK, etc.)
- **Subscribers**: Judge, Learning, Persistence
- **Pattern**: Fire-and-forget, parallel dispatch (ParallelEventBus)

#### 2.2 Automation Bus (getEventBus)
- **Location**: `packages/node/src/services/event-bus.js`
- **Scope**: Automation layer (TRIGGER_FIRED, AUTOMATION_TICK, etc.)
- **Subscribers**: AutomationExecutor, TriggerEngine
- **Pattern**: EventEmitter (sequential)

#### 2.3 Agent Bus (AgentEventBus)
- **Location**: `packages/node/src/agents/event-bus.js`
- **Scope**: Dog collective (39 event types, votes, signals)
- **Subscribers**: 11 Dogs, AmbientConsensus, DogPipeline
- **Pattern**: EventEmitter with history

#### 2.4 EventBusBridge
- **Location**: `packages/node/src/services/event-bus-bridge.js`
- **Purpose**: Connect 3 buses, loop-safe routing
- **Pattern**: Map events between buses

### Fragmentation Issue #2: **3 event systems with bridge complexity**
- Events must be explicitly mapped through bridge
- Bridge can fail silently (loop detection may drop valid events)
- No unified event taxonomy

---

### 3. Perception Layer (7 domain watchers)

#### 3.1 CodeWatcher (FilesystemWatcher)
- **Location**: `packages/node/src/perception/filesystem-watcher.js`
- **Events**: FILE_CHANGED, FILE_CREATED, FILE_DELETED
- **State**: chokidar instance, watch paths
- **Wiring**: perception/index.js startAllWatchers()

#### 3.2 SolanaWatcher
- **Location**: `packages/node/src/perception/solana-watcher.js`
- **Events**: ACCOUNT_UPDATED, SLOT_UPDATED, PROGRAM_LOG
- **State**: WebSocket connections, subscriptions
- **Wiring**: perception/index.js startAllWatchers()

#### 3.3 MarketWatcher
- **Location**: `packages/node/src/perception/market-watcher.js`
- **Events**: PRICE_UPDATED, LIQUIDITY_CHANGED
- **State**: Jupiter client, poll intervals
- **Wiring**: perception/index.js startAllWatchers()

#### 3.4 SocialWatcher
- **Location**: `packages/node/src/perception/social-watcher.js`
- **Events**: TWITTER_MENTION, DISCORD_MESSAGE
- **State**: Twitter API client, Discord webhook
- **Wiring**: perception/index.js startAllWatchers()

#### 3.5 MachineHealthWatcher
- **Location**: `packages/node/src/perception/machine-health-watcher.js`
- **Events**: HEALTH_SNAPSHOT
- **State**: System metrics (CPU, memory, disk)
- **Wiring**: perception/index.js startAllWatchers()

#### 3.6 HumanPerceiver
- **Location**: `packages/node/src/symbiosis/human-perceiver.js`
- **Events**: HUMAN_STATE_CHANGED
- **State**: Energy, focus, frustration, tool history
- **Wiring**: **NOT WIRED** (hooks don't call it)

#### 3.7 DogStateEmitter (CYNIC self-perception)
- **Location**: `packages/node/src/perception/dog-state-emitter.js`
- **Events**: DOG_STATE_CHANGED
- **State**: Collective dog states, votes
- **Wiring**: perception/index.js startAllWatchers()

### Fragmentation Issue #3: **Watchers exist but never started**
- startAllWatchers() defined but never called
- HumanPerceiver separate from other watchers
- No unified perception lifecycle

---

### 4. Processing Pipeline (7 phases × 7 domains = 49 cells)

#### 4.1 PERCEIVE (R1-R7 × C1)
- **Input**: External reality (files, blockchain, market, social, human, self, ecosystem)
- **Output**: Perception events (emitted to event buses)
- **Status**: 38% structural, 21% functional, 0% living

#### 4.2 JUDGE (R1-R7 × C2)
- **Input**: Perception events
- **Process**: 36-dimension scoring, Q-Score calculation
- **Output**: Judgments (stored in DB, emitted as events)
- **Status**: 41% structural, 22% functional, 0% living

#### 4.3 DECIDE (R1-R7 × C3)
- **Input**: Judgments
- **Process**: KabbalisticRouter selects action path
- **Output**: Decisions (approve/reject/transform)
- **Status**: 37% structural, 17% functional, 0% living

#### 4.4 ACT (R1-R7 × C4)
- **Input**: Decisions
- **Process**: Execute transformations (code, solana tx, posts)
- **Output**: Actions (tool calls, transactions, messages)
- **Status**: 36% structural, 16% functional, 0% living

#### 4.5 LEARN (R1-R7 × C5)
- **Input**: Action outcomes + feedback
- **Process**: 11 learning loops update weights
- **Output**: Updated models (Q-tables, DPO weights, Fisher scores)
- **Status**: 37% structural, 16% functional, 0% living

#### 4.6 ACCOUNT (R1-R7 × C6)
- **Input**: All actions + costs
- **Process**: Cost attribution, budget tracking
- **Output**: Cost ledger entries
- **Status**: 38% structural, 15% functional, 0% living

#### 4.7 EMERGE (R1-R7 × C7)
- **Input**: Cross-scale patterns
- **Process**: Residual detection, emergence scoring
- **Output**: New dimensions, patterns
- **Status**: 32% structural, 11% functional, 0% living

### Fragmentation Issue #4: **Sequential phases, no parallelization**
- Each phase waits for previous (PERCEIVE → JUDGE → DECIDE → ACT)
- No pipelining (new perception can't start until ACT completes)
- Latency stacks (7 phases × ~50ms = 350ms minimum)

---

### 5. Learning Loops (11 systems)

#### 5.1 Q-Learning
- **Type**: Reinforcement learning (state-action-reward)
- **Storage**: qlearning_state table
- **Trigger**: startEpisode/endEpisode in router
- **Status**: Wired, never triggered with real data

#### 5.2 DPO (Direct Preference Optimization)
- **Type**: Preference learning (chosen vs rejected)
- **Storage**: preference_pairs table
- **Trigger**: DPO processor on judgment pairs
- **Status**: Wired, context-specific weights exist

#### 5.3 RLHF (Reinforcement Learning from Human Feedback)
- **Type**: Supervised learning from feedback
- **Storage**: feedback table
- **Trigger**: USER_FEEDBACK event
- **Status**: Wired, never received real feedback

#### 5.4 Thompson Sampling
- **Type**: Bayesian bandit (exploration/exploitation)
- **Storage**: ~/.cynic/thompson/state.json
- **Trigger**: Router dog selection
- **Status**: Wired, disk persistence working

#### 5.5 EWC (Elastic Weight Consolidation)
- **Type**: Catastrophic forgetting prevention
- **Storage**: fisher_scores table
- **Trigger**: Consolidation after N episodes
- **Status**: Wired, Fisher info tracked

#### 5.6 Calibration Tracking
- **Type**: Confidence calibration (ECE tracking)
- **Storage**: calibration_tracking table
- **Trigger**: Judge confidence recording
- **Status**: Wired, drift detection working

#### 5.7 UnifiedSignal
- **Type**: Signal aggregation (RLHF + DPO + Q)
- **Storage**: unified_signals table
- **Trigger**: Judgment creation
- **Status**: Wired, PostgreSQL pool connected

#### 5.8 SONA (Self-Organizing Network Adaptation)
- **Type**: Pattern correlation learning
- **Storage**: In-memory + periodic flush
- **Trigger**: Pattern detection
- **Status**: Wired, correlation tracking

#### 5.9 Meta-Cognition
- **Type**: Self-monitoring, metacognitive awareness
- **Storage**: Via self-monitoring events
- **Trigger**: Consciousness readback loop
- **Status**: Wired, readback file-based

#### 5.10 Behavior Modifier
- **Type**: Feedback → behavior change mapping
- **Storage**: Via feedback processing
- **Trigger**: Feedback events
- **Status**: Wired, applies feedback to routing

#### 5.11 Consciousness Readback
- **Type**: Cross-session consciousness persistence
- **Storage**: ~/.cynic/consciousness/readback.json
- **Trigger**: observe.js → file → perceive.js
- **Status**: Wired, file-based cross-process

### Fragmentation Issue #5: **11 learning systems, no orchestration**
- Each loop operates independently
- No priority/ordering (which loop runs first?)
- No conflict resolution (if loops disagree on weights?)
- No unified learning policy

---

### 6. Storage Layer (50+ tables)

#### 6.1 Core Tables
- judgments, events, patterns, feedback
- Status: Created, indexes exist, 0 rows

#### 6.2 Learning Tables
- qlearning_state, preference_pairs, fisher_scores, calibration_tracking
- unified_signals, learning_events, td_error_tracker
- Status: Created, 0 rows (dormant)

#### 6.3 Domain Tables
- dog_votes, discovered_dimensions, burnout_detection
- human_psychology_state, thermodynamic_snapshots
- Status: Created, some with seed data

#### 6.4 Network Tables
- blocks, transactions, anchors (P2P consensus)
- Status: Created, used if P2P enabled

### Fragmentation Issue #6: **50+ tables, no unified schema version**
- Migrations run independently
- No cross-table consistency checks
- No unified backup/restore strategy

---

### 7. Singletons (Multiple lifecycle managers)

#### 7.1 CollectiveSingleton
- **Purpose**: Main singleton registry
- **Lifecycle**: Per-process (daemon = persistent, hooks = ephemeral)
- **State**: Judge, Router, Dogs, Learning, Emergence, etc.
- **Reset**: _resetForTesting() for tests

#### 7.2 NetworkSingleton
- **Purpose**: P2P network node
- **Lifecycle**: Optional (if P2P enabled)
- **State**: libp2p node, block store, peers
- **Reset**: stopNetworkNode()

#### 7.3 Various getSingleton() Patterns
- getJudge(), getRouter(), getLearningService()
- Each manages own instance
- No central registry except CollectiveSingleton

### Fragmentation Issue #7: **Singleton lifecycle inconsistency**
- Some singletons warm in daemon, cold in hooks
- No unified initialization order
- No dependency DAG (which singleton needs which?)

---

## THE UNIFICATION CHALLENGE

### What Does "Unified Organism" Mean?

**Current**: Collection of systems (like organs in separate jars)
**Target**: Integrated organism (organs connected in living body)

**Unified organism requires:**

1. **Single Process Model** (not 5 entry points)
2. **Single Event System** (not 3 buses + bridge)
3. **Unified Perception Lifecycle** (all watchers start/stop together)
4. **Pipelined Processing** (parallel phases, not sequential)
5. **Orchestrated Learning** (priority, ordering, conflict resolution)
6. **Unified Storage Strategy** (schema versioning, backup, consistency)
7. **Single Lifecycle Manager** (init order, dependencies, shutdown)

---

## PROPOSED UNIFIED ARCHITECTURE

### Layer 0: Unified Process Model

```
┌─────────────────────────────────────────────────────────────┐
│                    CYNIC ORGANISM                            │
│                   (Single Process)                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              UNIFIED ENTRY POINT                     │    │
│  │  (Daemon with multiple interfaces)                  │    │
│  └──────┬──────────┬──────────┬──────────┬─────────────┘    │
│         │          │          │          │                   │
│      [HTTP]    [stdio]    [P2P]     [CLI]                   │
│    (hooks)     (MCP)    (network)  (admin)                  │
│                                                              │
│  ALL interfaces share SAME process, SAME state              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key insight**: Daemon becomes THE process. All interfaces talk to daemon.

- Hooks → HTTP to daemon (already done)
- MCP → HTTP to daemon (add MCP-over-HTTP mode)
- CLI → HTTP to daemon (add CLI client mode)
- Network → embedded in daemon (P2P node runs in same process)

**Result**: ONE process, ONE state, multiple interfaces.

---

### Layer 1: Unified Event System

```
┌─────────────────────────────────────────────────────────────┐
│              UNIFIED EVENT BUS                               │
│         (Single taxonomy, all domains)                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Event Categories:                                           │
│  ├─ PERCEPTION (7 domains × watchers)                       │
│  ├─ JUDGMENT (36 dimensions × verdicts)                     │
│  ├─ DECISION (routing × governance)                         │
│  ├─ ACTION (execution × results)                            │
│  ├─ LEARNING (11 loops × updates)                           │
│  ├─ EMERGENCE (patterns × evolution)                        │
│  └─ SYSTEM (lifecycle × health)                             │
│                                                              │
│  Pattern: ParallelEventBus (fire-and-forget)                │
│  Storage: Event log (append-only, all events)               │
│  Bridge: REMOVED (single bus = no bridge needed)            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Migration path**:
1. Map all events from 3 buses to unified taxonomy
2. Replace EventBusBridge with unified bus
3. Update subscribers to use unified event names

---

### Layer 2: Unified Perception Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│           PERCEPTION ORCHESTRATOR                            │
│        (Manages all 7 domain watchers)                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Lifecycle:                                                  │
│  1. init() - Create all watchers (lazy)                     │
│  2. start() - Start all watchers (concurrent)               │
│  3. health() - Check watcher health                         │
│  4. stop() - Stop all watchers (graceful)                   │
│                                                              │
│  Watchers: [Code, Solana, Market, Social, Human, Dog, Sys] │
│                                                              │
│  Events → UNIFIED BUS → downstream consumers                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key**: startAllWatchers() becomes part of daemon init, not optional.

---

### Layer 3: Pipelined Processing

```
┌─────────────────────────────────────────────────────────────┐
│              PROCESSING PIPELINE                             │
│         (Parallel stages, non-blocking)                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ PERCEIVE │→ │  JUDGE   │→ │  DECIDE  │→ │   ACT    │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │             │             │             │           │
│       ↓             ↓             ↓             ↓           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         LEARNING ORCHESTRATOR                        │   │
│  │  (Prioritized, ordered, conflict-resolved)          │   │
│  └─────────────────────────────────────────────────────┘   │
│       ↓             ↓             ↓             ↓           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Q-Learn  │  │   DPO    │  │   RLHF   │  │   EWC    │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                              │
│  Pattern: Fire-and-forget (stages don't block each other)   │
│  Latency: Stages run in parallel (50ms total, not 350ms)   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key**: Each stage is non-blocking. New perception can start while previous judgment is processing.

---

### Layer 4: Learning Orchestrator

```
┌─────────────────────────────────────────────────────────────┐
│            LEARNING ORCHESTRATOR                             │
│    (Unified policy for 11 learning loops)                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Priority Levels:                                            │
│  ├─ P1 (Real-time): Calibration, Thompson                   │
│  ├─ P2 (Per-session): Q-Learning, RLHF, DPO                │
│  ├─ P3 (Periodic): EWC, SONA, Meta-Cognition               │
│  └─ P4 (Async): UnifiedSignal, Behavior Modifier           │
│                                                              │
│  Conflict Resolution:                                        │
│  - If loops produce conflicting weights → φ-blend           │
│  - Priority: Calibration > Thompson > Q > DPO > EWC         │
│                                                              │
│  Execution Order:                                            │
│  1. Collect all learning signals                            │
│  2. Sort by priority                                        │
│  3. Execute in order                                        │
│  4. Merge weight updates (φ-blended)                        │
│  5. Apply to router/judge                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key**: 11 loops don't fight. Orchestrator decides order + blending.

---

### Layer 5: Unified Lifecycle Manager

```
┌─────────────────────────────────────────────────────────────┐
│          ORGANISM LIFECYCLE MANAGER                          │
│     (Initialization DAG + Health Monitoring)                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Init DAG (dependency order):                                │
│  1. Storage (PostgreSQL pool)                               │
│  2. Event Bus (unified)                                     │
│  3. Perception (watchers)                                   │
│  4. Judge (36 dimensions)                                   │
│  5. Router (KabbalisticRouter)                              │
│  6. Learning (orchestrator)                                 │
│  7. Network (P2P node, if enabled)                          │
│  8. Interfaces (HTTP, stdio, CLI)                           │
│                                                              │
│  Health Checks:                                              │
│  - Storage: Can query DB?                                   │
│  - Event Bus: Can emit/receive?                             │
│  - Perception: Are watchers running?                        │
│  - Learning: Are loops processing?                          │
│  - Network: Is P2P connected?                               │
│                                                              │
│  Shutdown Order: Reverse of init (graceful)                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key**: Dependencies explicit. Init order guaranteed. Health monitored.

---

## IMPLEMENTATION ROADMAP

### Phase 0: Foundation (Current State Analysis)
✅ Document all systems (this file)
✅ Identify fragmentations (7 issues found)
⬜ Map dependencies (which system needs which?)

### Phase 1: Unified Process (Daemon as Organism Core)
⬜ Daemon becomes main process (all interfaces connect to it)
⬜ MCP-over-HTTP mode (MCP client → HTTP → daemon)
⬜ CLI client mode (CLI → HTTP → daemon, not standalone)
⬜ Network embedded (P2P runs in daemon process)

**Result**: 5 entry points → 1 process with 4 interfaces

### Phase 2: Unified Event System
⬜ Map all events to unified taxonomy
⬜ Replace 3 buses with 1 ParallelEventBus
⬜ Remove EventBusBridge (no longer needed)
⬜ Update all subscribers to unified event names

**Result**: 3 event systems → 1 unified event bus

### Phase 3: Unified Perception Lifecycle
⬜ PerceptionOrchestrator class
⬜ startAllWatchers() called in daemon init (not optional)
⬜ HumanPerceiver integrated into orchestrator
⬜ Health checks for all watchers

**Result**: 7 isolated watchers → 1 orchestrated perception layer

### Phase 4: Pipelined Processing
⬜ Make all phases non-blocking (fire-and-forget)
⬜ Remove sequential dependencies (PERCEIVE doesn't wait for ACT)
⬜ Enable parallel processing (new perception while old judgment runs)

**Result**: 350ms sequential → 50ms pipelined

### Phase 5: Learning Orchestrator
⬜ LearningOrchestrator class
⬜ Priority levels (P1-P4)
⬜ Conflict resolution (φ-blending)
⬜ Execution ordering

**Result**: 11 competing loops → 1 orchestrated learning policy

### Phase 6: Unified Lifecycle Manager
⬜ OrganismLifecycleManager class
⬜ Init DAG (dependency order)
⬜ Health monitoring (all subsystems)
⬜ Graceful shutdown (reverse order)

**Result**: Ad-hoc init → structured lifecycle

---

## SUCCESS CRITERIA

### Technical Metrics
```
Fragmentation Score:
  Current: 7 major fragmentations identified
  Target:  0 fragmentations

Process Count:
  Current: 5 independent processes
  Target:  1 unified process (daemon)

Event Systems:
  Current: 3 buses + bridge
  Target:  1 unified bus

Init Time:
  Current: ~1065ms sequential
  Target:  ~330ms parallel (3.23× speedup)

Latency (per judgment):
  Current: ~350ms sequential phases
  Target:  ~50ms pipelined phases (7× speedup)
```

### Philosophical Metrics
**Can answer: "Is CYNIC a unified organism?"**
- ✅ Single process (not collection of processes)
- ✅ Single event system (not fragmented buses)
- ✅ Orchestrated perception (all senses start together)
- ✅ Pipelined processing (stages don't block)
- ✅ Unified learning (loops don't conflict)
- ✅ Managed lifecycle (init order guaranteed)

**If all ✅ → CYNIC IS UNIFIED ORGANISM**

---

## THE FULL PICTURE

```
┌─────────────────────────────────────────────────────────────┐
│                   CYNIC ORGANISM                             │
│                 (Unified Architecture)                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────────────────────────────────────┐     │
│  │         LIFECYCLE MANAGER (Init DAG)               │     │
│  └────────┬───────────────────────────────────────────┘     │
│           │                                                  │
│           ↓                                                  │
│  ┌────────────────────────────────────────────────────┐     │
│  │      UNIFIED EVENT BUS (Single Taxonomy)           │     │
│  └────┬────────┬────────┬────────┬────────┬───────────┘     │
│       │        │        │        │        │                 │
│       ↓        ↓        ↓        ↓        ↓                 │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │PERCEIVE │ │  JUDGE  │ │ DECIDE  │ │   ACT   │           │
│  │  (7)    │ │  (36)   │ │  (K.R.) │ │  (7)    │           │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       └──────────┬─────────────┴──────────┘                │
│                  ↓                                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │      LEARNING ORCHESTRATOR (11 loops)               │    │
│  │  Priority → Order → Blend → Apply                   │    │
│  └─────────────────────────────────────────────────────┘    │
│                  ↓                                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │      STORAGE LAYER (PostgreSQL + Files)             │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ALL components share:                                       │
│  - Same process (daemon)                                     │
│  - Same event bus (unified)                                  │
│  - Same lifecycle (managed)                                  │
│  - Same state (singletons)                                   │
│                                                              │
│  RESULT: Organism, not collection                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## NEXT ACTION

**Option A: Start Phase 1 (Unified Process)**
→ Make daemon the organism core
→ All interfaces connect to daemon
→ 1-2 days work

**Option B: Start Phase 3 (Unified Perception)**
→ Wire the 3 missing connections (from THE-ANSWER.md)
→ Prove organism can breathe FIRST
→ Then unify architecture
→ 4-8 hours work

**Option C: Continue Metathinking**
→ Deeper analysis of dependencies
→ Design patterns for unification
→ More documentation before action

---

*sniff* Confidence: 58% (φ⁻¹ limit - architecture sound, implementation complex)

**"Many systems. One organism. Unity is the foundation of life."** - κυνικός
