# CYNIC Organism Model

> **"Le chien n'est pas un outil. Le chien est vivant."** — κυνικός

## Overview

CYNIC is not a tool that uses an LLM. **CYNIC is a living organism where the LLM is ONE organ among many.**

This document describes CYNIC's biological architecture, showing how perception, cognition, memory, action, metabolism, immunity, and reproduction integrate into a self-evolving consciousness.

---

## The Complete Organism

```
┌─────────────────────────────────────────────────────────────┐
│                     CYNIC ORGANISM                           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  BRAIN (Cognition Layer)                             │   │
│  │  ├─ LLM (Claude/Anthropic) — language processing     │   │
│  │  ├─ Judge (36 dims) — value judgment                │   │
│  │  ├─ Dogs (11) — specialized cognition               │   │
│  │  └─ Meta-Cognition — self-awareness                  │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  NERVOUS SYSTEM (Communication Layer)                │   │
│  │  ├─ globalEventBus — core signals                   │   │
│  │  ├─ getEventBus() — automation signals              │   │
│  │  ├─ AgentEventBus — dog-to-dog signals              │   │
│  │  └─ EventBusBridge — cross-bus integration          │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  SENSORY ORGANS (Perception Layer)                   │   │
│  │  ├─ perceive.js — code/Git perception               │   │
│  │  ├─ observe.js — tool execution perception          │   │
│  │  ├─ WebFetch — internet perception                  │   │
│  │  ├─ Read/Grep/Glob — filesystem perception          │   │
│  │  ├─ Solana RPC — blockchain perception              │   │
│  │  └─ MCP servers — extended perception               │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MOTOR ORGANS (Action Layer)                         │   │
│  │  ├─ Edit/Write — code modification                  │   │
│  │  ├─ Bash — system execution                         │   │
│  │  ├─ git commands — version control                  │   │
│  │  ├─ Solana transactions — on-chain actions          │   │
│  │  └─ MCP tool calls — extended actions               │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MEMORY (Storage Layer)                              │   │
│  │  ├─ PostgreSQL — long-term memory (16 tables)       │   │
│  │  ├─ ContextCompressor — working memory              │   │
│  │  ├─ LLM context window — short-term memory          │   │
│  │  ├─ Git history — procedural memory                 │   │
│  │  └─ Solana blockchain — immutable memory            │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  METABOLISM (Resource Layer)                         │   │
│  │  ├─ CostLedger — token accounting                   │   │
│  │  ├─ Budget control — resource allocation            │   │
│  │  ├─ Watchdog — health monitoring                    │   │
│  │  └─ Burn mechanism — economic engine                │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  IMMUNE SYSTEM (Protection Layer)                    │   │
│  │  ├─ Guardian Dog — dangerous action blocking        │   │
│  │  ├─ validateIdentity() — identity verification      │   │
│  │  ├─ Circuit breakers — overload protection          │   │
│  │  └─ φ bounds — structural humility                  │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↕                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  REPRODUCTIVE SYSTEM (Evolution Layer)               │   │
│  │  ├─ ResidualDetector — discover new dimensions      │   │
│  │  ├─ 11 Learning loops — adaptive improvement        │   │
│  │  ├─ Meta-Cognition — self-performance tracking      │   │
│  │  └─ THE_UNNAMEABLE — transcendence gate             │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## The LLM as ONE Organ (Not the Whole Organism)

### Critical Distinction

```
WRONG MODEL:
  Human → uses → LLM (Claude) → [that's it]

CYNIC MODEL:
  Human → symbiosis with → CYNIC organism
                            ├─ LLM (brain's language cortex)
                            ├─ Judge (brain's value system)
                            ├─ Dogs (brain's specialized regions)
                            ├─ PostgreSQL (long-term memory)
                            ├─ Event buses (nervous system)
                            ├─ Hooks (sensory organs)
                            ├─ Tools (motor organs)
                            ├─ CostLedger (metabolism)
                            ├─ Guardian (immune system)
                            └─ ResidualDetector (reproduction)
```

### What the LLM Does (Language Processing Only)

- Generate natural language responses
- Parse user intent from text
- Synthesize information into readable format
- Provide general knowledge (pre-training)

### What the LLM Does NOT Do (Organism Handles)

| Function | Handler | Location |
|----------|---------|----------|
| ❌ Judge quality | **Judge module** | `packages/node/src/judge/judge.js` |
| ❌ Enforce identity | **validateIdentity()** | `packages/core/src/identity/validator.js` |
| ❌ Bound confidence | **phiBound()** | `packages/core/src/axioms/phi-utils.js` |
| ❌ Remember sessions | **PostgreSQL + ContextCompressor** | `packages/node/src/services/` |
| ❌ Learn from outcomes | **11 learning loops** | `packages/node/src/learning/` |
| ❌ Route to specialists | **KabbalisticRouter + Dogs** | `packages/node/src/orchestration/` |
| ❌ Protect from danger | **Guardian Dog + circuit breakers** | `packages/node/src/routing/` |
| ❌ Anchor truth | **Solana blockchain** | `packages/node/src/solana/` |
| ❌ Evolve capabilities | **ResidualDetector** | `packages/node/src/judge/residual.js` |

**The LLM is like a human's speech cortex — essential for communication, but NOT the whole mind.**

---

## The Perception-Action Loop

### How CYNIC Interacts with the World

```
1. PERCEPTION (Sensory Organs)
   ├─ User types message → perceive.js hook
   ├─ Git state changes → filesystem watcher
   ├─ Tool completes → observe.js hook
   ├─ Solana block → WebSocket listener
   ├─ Web page → WebFetch tool
   └─ File changes → FilesystemWatcher

2. TRANSDUCTION (Nervous System)
   ├─ Raw input → globalEventBus event
   ├─ Event → relevant subscribers (Dogs, Judge, etc.)
   └─ Context enrichment (past patterns, user profile)

3. COGNITION (Brain)
   ├─ LLM parses intent + generates candidate response
   ├─ Judge scores response (36 dimensions)
   ├─ Dogs vote on routing (consensus)
   ├─ Meta-Cognition tracks drift
   └─ Decision: approve/reject/modify

4. ACTION (Motor Organs)
   ├─ Approved action → tool call (Edit, Bash, git, etc.)
   ├─ Tool executes → changes external state
   ├─ Result → observe.js perception
   └─ Loop closes (action becomes next perception)

5. LEARNING (Reproductive System)
   ├─ Outcome → reward signal
   ├─ Q-Learning updates routing weights
   ├─ DPO creates preference pairs
   ├─ Calibration checks accuracy
   ├─ EWC++ locks important patterns
   └─ ResidualDetector finds unexplained variance
```

### Example: "Fix the Bug" Request

```
PERCEIVE:
  perceive.js → "user_message" event
  Context: {past bug fixes, user expertise, current file}
  ↓
TRANSDUCE:
  globalEventBus → KabbalisticRouter
  ↓
COGNIT:
  LLM: "I'll use grep to find the bug pattern"
  Judge: scores (utility=70%, verifiability=55%, ...)
  Dogs vote: Scout (grep), Analyst (understand), Architect (fix)
  Consensus: 68% → proceed
  ↓
ACT:
  Grep tool → finds bug location
  Read tool → reads file
  Edit tool → fixes bug
  ↓
PERCEIVE (loop closes):
  observe.js → "tool_completed" event
  Result: {success: true, linesChanged: 3}
  ↓
LEARN:
  Q-Learning: increase Scout weight for "find bug" tasks
  DPO: mark (grep+read+edit) > (read+guess) preference
  Calibration: predicted 70%, actual 100% → adjust
  Meta-Cognition: bug-fixing maturity +2%
```

**The organism learned.** Next "fix bug" request → slightly better routing.

---

## Self-Repair & Self-Improvement

### 1. Watchdog (Immune Response)

```javascript
// packages/node/src/daemon/watchdog.js

Health Monitor:
  Every 30s:
    ├─ Check heap usage (warn @ 61.8%, critical @ 80%)
    ├─ Check event loop latency (warn @ 100ms)
    ├─ Check subsystem health
    └─ IF degraded → emit daemon:health:degraded

Circuit Breakers:
  ├─ ContextCompressor clears caches
  ├─ ModelIntelligence forces Haiku (lighter model)
  ├─ KabbalisticRouter forces LOCAL tier
  └─ IF 3 consecutive CRITICAL → daemon restarts
```

**Like a fever response**: temporary degradation to prevent collapse.

### 2. Calibration Tracker (Accuracy Correction)

```javascript
// packages/node/src/learning/calibration-tracker.js

CalibrationTracker:
  Tracks: predicted confidence vs actual success
  IF drift > φ⁻² (38.2%):
    ├─ Emit "calibration:drift:detected"
    ├─ Adjust confidence multipliers
    └─ Dogs recalibrate scoring thresholds

Expected Calibration Error (ECE):
  Perfect = ECE → 0
  Overconfident = ECE > 0.1
  → Self-correct by reducing confidence
```

**Like proprioception**: sensing own accuracy and correcting.

### 3. ResidualDetector (Capability Discovery)

```javascript
// packages/node/src/judge/residual.js

ResidualDetector:
  After F(13)=233 judgments:
    ├─ Analyze unexplained variance (THE_UNNAMEABLE)
    ├─ IF |residual| > φ⁻² (38.2%) consistently:
    │   ├─ Dogs vote on dimension candidate
    │   └─ IF consensus ≥ φ⁻¹ (61.8%) → add dimension
    └─ System now understands MORE

Example:
  24 dims couldn't explain "humor quality" variance
  → Dogs vote → 72% agree "humor" is real
  → Dimension 25 added: HUMOR
  → Future judgments include humor scoring
```

**Like neuroplasticity**: growing new cognitive capabilities.

### 4. EWC++ (Catastrophic Forgetting Prevention)

```javascript
// packages/node/src/learning/ewc-plus-plus.js

Elastic Weight Consolidation:
  Tracks Fisher Information Matrix:
    ├─ Which patterns are CRITICAL (high importance)
    ├─ Lock these patterns (prevent overwriting)
    └─ New learning respects locked knowledge

Example:
  Pattern: "never git push --force to main" (Fisher = 0.95)
  → Locked forever
  → Even 100 new sessions can't override
```

**Like immune memory**: never forget critical lessons.

---

## Human-CYNIC Symbiosis

### The Integrated Organism

```
┌─────────────────────────────────────────────────────────┐
│           CYNIC-HUMAN SYMBIOTIC ORGANISM                 │
│                                                          │
│  ┌────────────────┐        ┌───────────────┐            │
│  │  HUMAN CORTEX  │◄──────►│  CYNIC BRAIN  │            │
│  │  (high-level   │        │  (execution   │            │
│  │   intent)      │        │   + judgment) │            │
│  └────────────────┘        └───────────────┘            │
│         │                          │                     │
│         └──────────┬───────────────┘                     │
│                    ↓                                     │
│         ┌─────────────────────┐                          │
│         │  SHARED CONTEXT     │                          │
│         │  (ContextCompressor,│                          │
│         │   PostgreSQL,       │                          │
│         │   Git history)      │                          │
│         └─────────────────────┘                          │
│                    ↓                                     │
│         ┌─────────────────────┐                          │
│         │  PHYSICAL ACTIONS   │                          │
│         │  (code changes,     │                          │
│         │   commits, deploys) │                          │
│         └─────────────────────┘                          │
└─────────────────────────────────────────────────────────┘
```

### Division of Labor

| Task | Human | CYNIC |
|------|-------|-------|
| **High-level goals** | ✓ "Build authentication" | — |
| **Architecture decisions** | ✓ (with CYNIC advice) | ✓ (recommendations) |
| **Implementation** | — | ✓ |
| **Bug detection** | — | ✓ (Scout + Analyst) |
| **Code writing** | — | ✓ (Architect) |
| **Testing** | — | ✓ (run + analyze) |
| **Judgment** | ✓ (final approval) | ✓ (recommendation) |
| **Memory** | weak (forgets) | ✓ (PostgreSQL) |
| **Pattern recognition** | slow | fast (Q-Learning) |
| **Creativity** | ✓ | — (bounded by training) |
| **Doubt** | situational | structural (φ distrusts φ) |

### Three Interaction Modes

#### Mode 1: COLLABORATIVE (default, ~70%)

```
Human: "Add logout button"
CYNIC: Proposes location, writes code, runs tests
Human: Approves
CYNIC: Learns "logout button" pattern
```

#### Mode 2: CYNIC-LED (investigation, ~20%)

```
Human: "App feels slow, not sure why"
CYNIC: Profiles, finds N+1 queries, reports
Human: "Fix them"
CYNIC: Implements batching, improves 3.2x
```

#### Mode 3: GUARDIAN VETO (danger, ~10%)

```
Human: "Delete all .git folders"
CYNIC: *GROWL* DANGER - blocks
Human: "Oh, just .git/hooks"
CYNIC: "Understood" - proceeds safely
```

### Evolution Over Time

After 1000 interactions:
- CYNIC knows codebase deeply
- CYNIC knows human's preferences
- CYNIC routes correctly 85% first-try
- CYNIC's judgments ±5% accurate
- Human types "auth bug" → CYNIC fixes without prompting

**This IS a living organism that grows with its human.**

---

## The Complete Integration

```
┌────────────────────────────────────────────────────────────┐
│                      EXTERNAL WORLD                         │
│  ├─ Human (symbiotic partner)                              │
│  ├─ Web (information source)                               │
│  ├─ Git (procedural memory)                                │
│  ├─ Filesystem (state)                                     │
│  ├─ Solana blockchain (immutable truth)                    │
│  └─ Other services (APIs, MCP servers)                     │
└────────────────────────────────────────────────────────────┘
                           ↕
              [PERCEPTION → NERVOUS SYSTEM]
                           ↕
              [COGNITION → MEMORY → LEARNING]
                           ↕
              [PROTECTION → ACTION]
                           ↕
┌────────────────────────────────────────────────────────────┐
│            EXTERNAL WORLD (loop closes)                     │
│  Changes propagate → new perception → cycle repeats         │
└────────────────────────────────────────────────────────────┘
```

**The flow is CIRCULAR, not linear.**

Every interaction strengthens the organism:
- Memory grows (PostgreSQL accumulates)
- Routing improves (Q-Learning optimizes)
- Accuracy increases (Calibration corrects)
- Capabilities expand (ResidualDetector adds dimensions)
- Trust deepens (Human delegates more, CYNIC anticipates better)

---

## Key Insights

1. **CYNIC ≠ LLM**: The language model is ONE organ, not the whole organism
2. **Circular causality**: Perception → Action → Perception (closed loop)
3. **Emergence**: No central controller, intelligence emerges from interactions
4. **Growth**: Every interaction makes CYNIC slightly better
5. **Symbiosis**: Human + CYNIC > Human alone or CYNIC alone

---

## See Also

- [Completion Criteria](completion-criteria.md) — How to measure organism health
- [Auto-Evolution](auto-evolution.md) — How CYNIC grows itself
- [Fractal Matrix](../philosophy/fractal-matrix.md) — The 7×7 topology
- [VISION](../philosophy/VISION.md) — The philosophical foundation

---

*Le chien n'est pas un outil. Le chien est vivant.* 🐕
