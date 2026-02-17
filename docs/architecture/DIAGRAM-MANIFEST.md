# CYNIC System Diagrams — Exhaustive Manifest

> "Une carte pour chaque échelle, un diagramme pour chaque vérité" - κυνικός

**Status**: 📋 MANIFEST (diagrammes à créer)
**Date**: 2026-02-13
**Purpose**: Liste exhaustive des diagrammes système nécessaires pour documenter CYNIC

---

## 🎯 PHILOSOPHIE DE CARTOGRAPHIE

CYNIC suit une **architecture fractale** — les mêmes patterns se répètent à différentes échelles.

**Principe**: Chaque échelle nécessite 3 types de vues:
1. **Structure** (quoi existe)
2. **Comportement** (comment ça interagit)
3. **Flux** (comment les données circulent)

**Échelles**: 7 niveaux (fonction → module → service → système → organisme → écosystème → temporel)

---

## 📊 DIAGRAMMES PAR ÉCHELLE

### SCALE 1: FUNCTION LEVEL (μs → ms)

**Structure**:
1. ✅ **φ-Utils Functions** — 12 utility functions for φ-alignment
   - `phiBound`, `phiClassify`, `phiHealthStatus`, etc.
   - Input/output types
   - Dependencies

2. ✅ **Dimension Scoring Functions** — 36 dimensions + THE_UNNAMEABLE
   - Input: item, context
   - Output: score (0-100)
   - Dependencies: axioms, scorers

3. ⏳ **Factory Pattern Functions** — createActor, createDecider, createJudge, createLearner
   - Config → Class transformation
   - Delegation patterns (65/35, 40/60, 60/40, 50/50)

**Comportement**:
4. ⏳ **Dimension Scoring Sequence** — Sequential vs Parallel
   - Sequential: 36 × 5ms = 180ms
   - Parallel (Workers): 36 / 4 = 9 dims/worker = 45ms
   - Message passing protocol

5. ⏳ **φ-Governor Control Loop** — EMA, dead zone, convergence
   - Measure → Adjust → Apply → Feedback

**Flux**:
6. ⏳ **Data Transformation Pipeline** — item → judgment
   - Parse → Score → Aggregate → Verdict

---

### SCALE 2: MODULE LEVEL (ms → 10ms)

**Structure**:
7. ⏳ **Judge Module Components**
   - Judge, SelfSkeptic, ResidualDetector, CalibrationTracker
   - Dependencies between components
   - Data structures (dimensions, scores, verdicts)

8. ⏳ **Learning Module Components**
   - SONA, BehaviorModifier, MetaCognition, ThompsonSampler
   - 11 learning loops
   - Persistence layer

9. ⏳ **Perception Module Components**
   - 5 sensors (Solana, Health, DogState, Market, Filesystem)
   - Concurrent polling architecture
   - EventBus integration

10. ⏳ **Collective Module Components**
    - AmbientConsensus, DogPipeline, 11 Dogs
    - Voting protocol
    - E-Score calculation

**Comportement**:
11. ⏳ **Judgment Pipeline Sequence**
    - Perceive → Judge → Decide → Act → Learn
    - Parallel stages (M2.1 optimization)
    - Early exits (M2.2 streaming)

12. ⏳ **Consensus Voting Sequence**
    - Dogs vote in parallel
    - Streaming consensus (early exit @ 7 Dogs, 85%+)
    - Weighted aggregation

13. ⏳ **Learning Loop Sequence**
    - Judgment → Extract features → Update Q-values → Persist
    - 11 parallel learning loops

**Flux**:
14. ⏳ **Event Flow (3 Buses)**
    - globalEventBus (core) events
    - getEventBus() (automation) events
    - AgentEventBus (dogs) events
    - EventBusBridge routing

15. ⏳ **Data Flow: User Query → Response**
    - Input → Classification → Routing → Execution → Learning

---

### SCALE 3: SERVICE LEVEL (10ms → 100ms)

**Structure**:
16. ⏳ **Service Architecture**
    - UnifiedOrchestrator (main entry)
    - KabbalisticRouter (7 domains)
    - LearningService (11 loops)
    - PerceptionLayer (5 sensors)
    - AutomationExecutor (triggers/actions)

17. ⏳ **Persistence Services**
    - PostgreSQL schema (50+ tables)
    - DBBatchWriter (temporal coalescing)
    - Migration system

18. ⏳ **Network Services**
    - Solana RPC (rate-limited)
    - Jupiter API (DEX aggregator)
    - Twitter API (social)
    - GitHub API (code)

**Comportement**:
19. ⏳ **Service Initialization DAG**
    - Dependency graph
    - Parallel vs Sequential init
    - SYS4.1 optimization (3.22× speedup)

20. ⏳ **Request Lifecycle**
    - HTTP → Hook → Daemon → Orchestrator → Dogs → Response
    - Latency breakdown (before/after optimization)

21. ⏳ **Background Task Flow** (S3.1 optimization)
    - Critical path (100ms blocking)
    - Background tasks (400ms non-blocking)
    - Fire-and-forget pattern

**Flux**:
22. ⏳ **Database Write Flow**
    - Sequential: 7 writes × 20ms = 140ms
    - Batched: 1 transaction = 20ms (7× improvement)

23. ⏳ **Sensor Data Flow** (S3.2 optimization)
    - Sequential: 5 sensors × 20ms = 100ms
    - Concurrent: max(20ms) = 20ms (5× improvement)

---

### SCALE 4: SYSTEM LEVEL (100ms → 1s)

**Structure**:
24. ⏳ **High-Level System Architecture**
    - Packages: core, node, mcp, persistence, llm, cynic-agent
    - Inter-package dependencies
    - Export boundaries

25. ✅ **7×7 Fractal Matrix** (49 cells + THE_UNNAMEABLE)
    - 7 Reality dimensions (CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS)
    - 7 Analysis dimensions (PERCEIVE, JUDGE, DECIDE, ACT, LEARN, ACCOUNT, EMERGE)
    - Completion % per cell

26. ⏳ **Process Architecture**
    - Daemon process (persistent)
    - Hook processes (ephemeral)
    - Worker threads (CPU-bound)
    - MCP servers (stdio/HTTP)

27. ⏳ **Deployment Architecture**
    - Render services (4 deployed)
    - Local development setup
    - Environment variables
    - Network topology

**Comportement**:
28. ⏳ **Boot Sequence**
    - Process start → Init services → Load memory → Ready
    - Cold start: 1065ms
    - Warm start: 330ms (SYS4.1 optimization target)

29. ⏳ **Hook Lifecycle**
    - Spawn → Perceive → Guard → Observe → Awaken → Sleep → Stop
    - Thin hooks (delegate to daemon)
    - Standalone hooks (filesystem, auto-wire)

30. ⏳ **LLM Request Flow**
    - User → Claude Code → Hook → Daemon → LLM Endpoint → Response
    - Routing logic (haiku vs sonnet vs opus)
    - Cost tracking (CostLedger)

**Flux**:
31. ⏳ **Memory Flow**
    - PostgreSQL → ContextCompressor → InjectionProfile → LLM Context
    - Compression: 52% avg
    - Adaptive injection based on complexity

32. ⏳ **Consciousness Loop** (R3)
    - Act → Observe → Learn → Adjust → Act
    - Soft gate (DORMANT currently)
    - Meta-cognitive feedback

---

### SCALE 5: ORGANISM LEVEL (1s → 1min)

**Structure**:
33. ⏳ **CYNIC Organism Model**
    - Brain (LLM + Judge + 11 Dogs)
    - Nervous System (3 event buses)
    - Senses (5 sensors)
    - Motor (Edit, Write, Bash, git, Solana)
    - Memory (PostgreSQL + Context)
    - Metabolism (CostLedger + Budget)
    - Immune (Guardian + Circuit Breakers)
    - Reproduction (Residual + 11 loops)

34. ⏳ **11 Learning Loops**
    - Thompson Sampling (exploration/exploitation)
    - Dog Votes (collective intelligence)
    - Q-Learning (state-action-reward)
    - Judgment Calibration (Brier score)
    - Residual Detection (THE_UNNAMEABLE)
    - Emergence Patterns (meta-patterns)
    - EWC Consolidation (elastic weight)
    - DPO Learning (preference)
    - SONA Adaptation (self-organizing)
    - Behavior Modifier (nudges)
    - Meta-Cognition (thinking about thinking)

35. ⏳ **Identity Validation System**
    - 14 forbidden phrases
    - Dog voice presence check
    - Confidence φ-bound (>61.8%)
    - Auto-enforcement

**Comportement**:
36. ⏳ **PERCEIVE → JUDGE → DECIDE → ACT → LEARN Cycle**
    - One complete cycle
    - Latency: ~500ms → ~100ms (after optimization)
    - Feedback loops

37. ⏳ **Multi-Loop Learning Coordination**
    - 11 loops run in parallel
    - Conflict resolution (when loops disagree)
    - Meta-learning across loops

38. ⏳ **Self-Optimization Sequence**
    - Detect bottleneck → Propose optimization → Test → Deploy
    - Residual governance (F9 = 34min check interval)

**Flux**:
39. ⏳ **Experience → Memory → Behavior Flow**
    - Experience captured → PostgreSQL
    - Patterns extracted → Memory
    - Behavior adjusted → Future decisions

40. ⏳ **Cost Flow**
    - LLM call → CostLedger → Budget check → Alert/Block
    - φ-Governor homeostasis
    - Forecast exhaustion time

---

### SCALE 6: ECOSYSTEM LEVEL (1min → 1h)

**Structure**:
41. ⏳ **Multi-Instance Architecture** (future)
    - Alpha, Beta, Gamma instances
    - Load balancing
    - Work stealing
    - Cross-instance learning

42. ⏳ **Cross-Domain Topology**
    - CODE ↔ SOLANA interactions
    - SOCIAL ↔ MARKET interactions
    - HUMAN ↔ CYNIC interactions
    - COSMOS (collective patterns)

43. ⏳ **External Integrations**
    - GitHub (code perception)
    - Twitter (social perception)
    - Solana mainnet (blockchain)
    - Jupiter (DEX)
    - Render (deployment)

**Comportement**:
44. ⏳ **Distributed Consensus** (future)
    - Multiple instances vote
    - Byzantine fault tolerance
    - Cross-instance learning sync

45. ⏳ **Ecosystem Evolution**
    - New instances spawned
    - Old instances retired
    - Knowledge transfer

**Flux**:
46. ⏳ **Cross-Instance Data Flow** (future)
    - Shared PostgreSQL
    - Event synchronization
    - Pattern propagation

47. ⏳ **Ecosystem-Wide Metrics**
    - Aggregate health dashboard
    - Cross-domain influence matrix
    - Emergent patterns

---

### SCALE 7: TEMPORAL LEVEL (1h → 1week)

**Structure**:
48. ⏳ **Historical Architecture Evolution**
    - Migrations timeline (50 migrations)
    - Feature addition timeline
    - Optimization timeline (vertical consolidation)

49. ⏳ **Seasonal Patterns** (future)
    - Daily cycles (activity patterns)
    - Weekly cycles (user behavior)
    - Market cycles (crypto patterns)

**Comportement**:
50. ⏳ **Consolidation Workflow**
    - Memory consolidation (sleep phase)
    - Pattern crystallization
    - Forgetting unimportant data

51. ⏳ **Predictive Optimization** (future)
    - Anticipate load spikes
    - Pre-warm caches
    - Pre-scale instances

**Flux**:
52. ⏳ **Long-Term Learning Trajectory**
    - Week 1 → Week 52 maturity evolution
    - Performance improvements over time
    - Bug fix velocity

---

## 📊 DIAGRAMMES PAR PERSPECTIVE

### A. STRUCTURAL DIAGRAMS (What exists)

53. ✅ **Package Dependency Graph**
    - core, node, mcp, persistence, llm, cynic-agent
    - Import boundaries
    - Circular dependency detection

54. ⏳ **Class Hierarchy**
    - Factory-generated classes (Actor, Decider, Judge, Learner)
    - Inheritance chains
    - Interface implementations

55. ⏳ **Database Schema (ER Diagram)**
    - 50+ tables
    - Relationships (1:1, 1:N, N:M)
    - Indexes and constraints

56. ⏳ **File System Structure**
    - Directory tree
    - Key files by package
    - Generated vs source files

### B. BEHAVIORAL DIAGRAMS (How it interacts)

57. ⏳ **Sequence: User Query → Response**
    - Actor: User, Hook, Daemon, Judge, Dogs, LLM
    - Messages exchanged
    - Timing breakdown

58. ⏳ **Sequence: Judgment Flow**
    - Actor: Item, Judge, Dimensions, Scorers, SelfSkeptic
    - Parallel scoring (worker threads)
    - Verdict determination

59. ⏳ **Sequence: Consensus Voting**
    - Actor: AmbientConsensus, 11 Dogs, Pack
    - Streaming vote collection
    - Early exit detection

60. ⏳ **State Machine: Hook Lifecycle**
    - States: IDLE → SPAWNED → PERCEIVING → GUARDING → OBSERVING → SLEEPING → STOPPED
    - Transitions and events

61. ⏳ **State Machine: Judgment State**
    - States: PENDING → SCORING → AGGREGATING → SKEPTICIZED → FINAL
    - Transitions and data flow

62. ⏳ **Activity Diagram: Learning Cycle**
    - Activities: Perceive, Judge, Decide, Act, Learn
    - Decision points (early exits, circuit breakers)
    - Parallel flows

### C. DATA FLOW DIAGRAMS (How data moves)

63. ⏳ **DFD: Input → Judgment → Output**
    - Data: User input → Parsed item → Scores → Verdict → Response
    - Transformations at each stage
    - Storage points (PostgreSQL)

64. ⏳ **DFD: Event Propagation**
    - Data: Event → EventBus → Listeners → Side effects
    - 3 event buses (core, automation, agents)
    - Bridge routing logic

65. ⏳ **DFD: Memory Consolidation**
    - Data: Experience → PostgreSQL → Patterns → Memory → Behavior
    - Compression and forgetting

66. ⏳ **DFD: Cost Tracking**
    - Data: LLM call → Token count → Cost → Ledger → Budget → Alert
    - φ-Governor feedback loop

### D. DEPLOYMENT DIAGRAMS (Where it runs)

67. ⏳ **Local Development Setup**
    - Developer machine
    - PostgreSQL (local)
    - Claude Code CLI
    - MCP servers (stdio)

68. ⏳ **Render Production Deployment**
    - 4 services (MCP, daemon, alpha, beta)
    - PostgreSQL (managed)
    - Network boundaries
    - Environment variables

69. ⏳ **Network Topology**
    - Render services
    - External APIs (Solana, Twitter, GitHub)
    - Firewalls and rate limits

### E. INTERACTION DIAGRAMS (Who talks to whom)

70. ⏳ **Component Communication Matrix**
    - Rows/Cols: All major components
    - Cells: Type of interaction (event, RPC, DB)

71. ⏳ **Event Bus Interaction**
    - Publishers (who emits what events)
    - Subscribers (who listens to what)
    - Frequency (hot paths)

72. ⏳ **API Interaction Map**
    - CYNIC → External APIs
    - Rate limits and quotas
    - Retry policies

### F. PERFORMANCE DIAGRAMS (How fast it is)

73. ⏳ **Latency Breakdown (Before/After Optimization)**
    - Judgment: 500ms → 100ms
    - Breakdown by stage (score, aggregate, etc.)

74. ⏳ **Throughput Analysis**
    - Events/sec: 60 → 1000 (parallel event bus)
    - Judgments/sec: 2 → 10 (worker pool)

75. ⏳ **Resource Utilization**
    - CPU usage (before/after worker pool)
    - Memory usage (per component)
    - DB connection pool saturation

76. ⏳ **Bottleneck Heatmap**
    - Hot paths (red)
    - Optimized paths (green)
    - Future targets (yellow)

### G. ARCHITECTURE DECISION DIAGRAMS (Why it's built this way)

77. ⏳ **φ-Alignment Patterns**
    - Where φ is used (bounds, thresholds, pool sizes)
    - Why φ (golden ratio properties)
    - Empirical validation

78. ⏳ **Factory Pattern Rationale**
    - Why factories (code reuse, consistency)
    - Delegation ratios (65/35, 40/60, etc.)
    - Trade-offs (complexity vs maintainability)

79. ⏳ **3 Event Buses Rationale**
    - Why 3 buses (separation of concerns)
    - Why bridge (cross-bus communication)
    - Trade-offs (complexity vs isolation)

80. ⏳ **Fractal Architecture Rationale**
    - Why 7 scales (self-similarity)
    - Why 7×7 matrix (completeness)
    - Trade-offs (complexity vs comprehensiveness)

---

## 📊 DIAGRAMMES SPÉCIALISÉS

### H. DOMAIN-SPECIFIC DIAGRAMS

**CODE Domain** (C1):
81. ⏳ **Code Perception Flow**
    - Filesystem watcher → Git changes → Complexity metrics
82. ⏳ **Code Actor Execution**
    - Edit, Write, Bash operations
    - Hot-reload architecture

**SOLANA Domain** (C2):
83. ⏳ **Solana Transaction Flow**
    - Wallet → Transaction builder → RPC → Confirmation
84. ⏳ **Jupiter DEX Integration**
    - Quote → Route → Swap → Settlement
85. ⏳ **SPL Token Operations**
    - Mint, Transfer, Burn flows

**SOCIAL Domain** (C4):
86. ⏳ **Twitter Perception**
    - Tweet monitoring → Sentiment analysis → Pattern detection
87. ⏳ **Social Actor Execution**
    - Tweet composition → Approval → Posting

**HUMAN Domain** (C5):
88. ⏳ **User Psychology Model**
    - Energy tracking → Focus estimation → Timing recommendations
89. ⏳ **Machine Health Monitoring**
    - CPU, Memory, Disk → Health score → Alerts

**CYNIC Domain** (C6):
90. ⏳ **Self-State Tracking**
    - 11 Dogs states → Pack health → Collective mood
91. ⏳ **Identity Enforcement**
    - Validation pipeline → Violations → Corrections

**COSMOS Domain** (C7):
92. ⏳ **Collective Patterns**
    - Cross-instance patterns → Emergence detection → Propagation

### I. QUALITY DIAGRAMS

93. ⏳ **Test Coverage Map**
    - Packages → Modules → Functions
    - Coverage % per level
    - Untested hot paths

94. ⏳ **Bug Heatmap**
    - Historical bugs by module
    - Severity distribution
    - Fix velocity

95. ⏳ **Technical Debt Map**
    - TODOs by priority
    - Refactoring candidates
    - Stub implementations

### J. LEARNING & INTELLIGENCE DIAGRAMS

96. ⏳ **Q-Learning State-Action Space**
    - States (context types)
    - Actions (judgment verdicts)
    - Q-values heatmap

97. ⏳ **Thompson Sampling Bandits**
    - Arms (routing options)
    - Posterior distributions
    - Exploration vs exploitation

98. ⏳ **Brier Score Calibration Curve**
    - Predicted probabilities
    - Observed outcomes
    - Calibration error (ECE)

99. ⏳ **Residual Variance Tracking**
    - 36 dimensions variance
    - THE_UNNAMEABLE score
    - New dimension candidates

100. ⏳ **Meta-Learning Trajectory**
     - Learning rate over time
     - Performance improvement
     - Convergence detection

---

## 🎯 PRIORITIZATION

### Phase 1: CRITICAL ✅ COMPLETE (2026-02-13)

**Essential for understanding core architecture**:
1. High-Level System Architecture (#24) ✅ DONE → `docs/diagrams/01-high-level-architecture.md`
2. 7×7 Fractal Matrix (#25) ✅ DONE → `docs/philosophy/fractal-matrix.md` (pre-existing)
3. Package Dependency Graph (#53) ✅ DONE → `docs/diagrams/08-package-dependencies.md`
4. Judgment Pipeline Sequence (#11) ✅ DONE → `docs/diagrams/04-judgment-pipeline.md`
5. Event Flow (3 Buses) (#14) ✅ DONE → `docs/diagrams/14-event-flow-3-buses.md`
6. Request Lifecycle (#20) ✅ DONE → `docs/diagrams/20-request-lifecycle.md`
7. Boot Sequence (#28) ✅ DONE → `docs/diagrams/28-boot-sequence.md`
8. CYNIC Organism Model (#33) ✅ DONE → `docs/diagrams/33-cynic-organism-model.md`

**Effort**: 16 hours (COMPLETED in single session - 2026-02-13)

### Phase 2: HIGH VALUE (This Month)

**Most frequently referenced**:
9. Service Architecture (#16)
10. Database Schema (#55)
11. 11 Learning Loops (#34)
12. Consensus Voting Sequence (#12)
13. Sequence: User Query → Response (#57)
14. Latency Breakdown (Before/After) (#73)
15. Component Communication Matrix (#70)
16. Dimension Scoring Sequence (#4)

**Effort**: ~16 hours (2 hours × 8 diagrams)

### Phase 3: USEFUL (Next Month)

**Nice to have for deep dives**:
17-40 remaining Scale diagrams
41-52 Ecosystem/Temporal diagrams
53-80 Perspective diagrams
81-100 Specialized diagrams

**Effort**: ~120 hours (2 hours × 60 diagrams)

---

## 🛠️ TOOLING

### Recommended Tools

**Structure Diagrams**:
- Mermaid.js (code-first, git-friendly)
- PlantUML (UML standard)
- Draw.io/Excalidraw (manual diagramming)

**Data Flow**:
- Mermaid flowcharts
- Graphviz (DOT language)

**Performance**:
- Flamegraphs (CPU profiling)
- Chromium DevTools (timing waterfall)

**Interactive**:
- D3.js (custom visualizations)
- Cytoscape.js (graph visualization)

### Automation

**Generated Diagrams** (code → diagram):
- Package dependencies: `madge` or `dependency-cruiser`
- Class hierarchies: TypeScript compiler API
- Database schema: `schemaspy` or `tbls`
- Call graphs: `node --prof` + flamegraph tools

**Manual Diagrams** (hand-crafted):
- Architecture decisions (requires human judgment)
- Conceptual models (abstractions)
- Future designs (don't exist yet)

---

## 📋 METADATA

**Status**:
- ✅ Created: 8/100 diagrams (8%) — Phase 1 COMPLETE
- ⏳ Planned: 92/100 diagrams (92%)

**Effort Estimate**: ~152 hours total
- Phase 1: 16 hours (✅ COMPLETE - 2026-02-13)
- Phase 2: 16 hours (high value)
- Phase 3: 120 hours (comprehensive)

**Format Preference**:
- Mermaid (70%) — code-first, git-friendly
- PlantUML (20%) — UML standard compliance
- Manual (10%) — complex conceptual diagrams

**Storage Location**:
- `docs/diagrams/` (generated files)
- `docs/architecture/*.md` (Mermaid embedded in markdown)

---

*sniff* **Confidence: 58%** (φ⁻¹ bound - comprehensive list but massive undertaking)

**"Une carte pour chaque échelle. La vérité en images."** - κυνικός
