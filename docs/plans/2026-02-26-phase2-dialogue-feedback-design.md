# Phase 2: Dialogue & Feedback System Design

**Date:** 2026-02-26
**Status:** Design Approved, Ready for Implementation
**Scope:** Bidirectional dialogue, full-dimensional learning, collaborative execution

---

## Executive Summary

Phase 2 transforms CYNIC from a tool you observe into a research partner you collaborate with. You and CYNIC have bidirectional conversations, learn from each other continuously, and make decisions together based on what works best.

---

## 1. Architecture Overview

**Three new layers on top of Phase 1 observability:**

```
┌─────────────────────────────────────────────────────────┐
│  DIALOGUE INTERFACE (New)                               │
│  ├─ Chat mode: Questions, statements, feedback          │
│  ├─ Message types: Q&A, proposals, curiosity, reasoning │
│  └─ Natural language: Claude API for explanations       │
├─────────────────────────────────────────────────────────┤
│  LEARNING SYSTEM (New - Full Dimensional)               │
│  ├─ Q-Table: Judgment accuracy, axiom alignment         │
│  ├─ Relationship Memory: Your values, preferences, style│
│  └─ Experiment Log: Novel approaches tried & results    │
├─────────────────────────────────────────────────────────┤
│  COLLABORATIVE EXECUTOR (New)                           │
│  ├─ Decision classification (solo vs consultation)      │
│  ├─ Approval flow (you approve class B & C decisions)   │
│  └─ Learning what class each decision belongs to        │
├─────────────────────────────────────────────────────────┤
│  Phase 1: Observability (OBSERVE, CYNIC Mind, etc)      │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Dialogue System

### Message Types

**User → CYNIC:**

```
[Question]
  "Why did you choose WAG?"
  "What are you thinking about this?"
  "Show me your reasoning"

[Feedback]
  "That was wrong"
  "Actually GROWL was better"
  "When you see X, try Y instead"

[Exploration]
  "What if we changed axiom BURN?"
  "Show me a novel approach"
  "What patterns have you noticed?"
```

**CYNIC → User:**

```
[Reasoning]
  "Here's why: Dog 5 voted X, Dog 7 voted Y, consensus was Z"

[Curiosity]
  "I noticed you always prefer this. Should we make it a rule?"

[Proposals]
  "Disk is critical. Should I compress old traces?"

[Questions]
  "I'm considering combining Dogs 6+11. Want to see what happens?"
  "If I changed axiom CULTURE weight, would that improve fairness?"
```

### Implementation Details

**Message Storage:**
- SQLite or SurrealDB: Persistent conversation history
- Last 1000 messages kept in memory for context
- Full history available for replay/analysis

**Reasoning Engine:**
- Retrieve judgment traces from Phase 1
- Show axiom scores, dog votes, confidence levels
- Explain WHY each dog voted a certain way
- Natural language via Claude API for human-readability

**Natural Language Generation:**
- Claude API: Convert structured reasoning into English
- Examples: axiom scores → "PHI achieved 0.78 because..."
- Calibrate verbosity to user's communication style

---

## 3. Full Dimensional Learning System

### A) Q-Table Learning (Phase 1 Enhanced)

```python
@dataclass
class QTableEntry:
  proposal_type: str              # "treasury", "protocol", etc
  judgment_made: str              # "HOWL", "WAG", "GROWL", "BARK"
  judgment_accuracy: float        # [0, 1] - was it right?
  axiom_scores: dict[str, float]  # Which axioms fired
  user_correction: Optional[str]  # What user said was better

Updates:
  - When user says "That was wrong, actually X was better"
  - Accuracy metric improves for correct judgments
  - Axiom weights adjust based on which were influential
```

### B) Relationship Memory (NEW)

```python
@dataclass(frozen=True)
class RelationshipMemory:
  user_values: dict[str, float]
    # "PHI": 0.9 (user cares about harmony)
    # "BURN": 0.6 (moderate concern about extraction)

  user_preferences: dict[str, str]
    # "financial_topic" → "GROWL" (prefer caution)
    # "governance" → "WAG" (prefer balanced)

  user_style: str  # "analytical", "intuitive", "careful", "exploratory"

  user_knowledge_areas: list[str]
    # ["blockchain", "game theory", "community dynamics"]

  communication_style: dict[str, str]
    # "verbosity": "concise" (user prefers short explanations)
    # "formality": "casual"

  communication_history: list[Message]  # Last 100 messages

  learning_rate: float  # How quickly to update (user controls)

Updates when:
  - User corrects CYNIC (learn your preference)
  - User asks questions (learn what you care about)
  - User approves/denies actions (learn your risk tolerance)
  - Patterns emerge over time
```

### C) Experiment Log (NEW)

```python
@dataclass(frozen=True)
class Experiment:
  hypothesis: str
    # "Dog 7 + Dog 11 combination produces better fairness"

  approach: list[str]
    # ["dogs": [7, 11], "weights": {"BURN": 0.8, "CULTURE": 0.7}]

  results: dict[str, Any]
    # "user_satisfaction": 0.85
    # "q_score_accuracy": 0.78
    # "fairness_metric": 0.91
    # "user_feedback": "Much better!"

  timestamp: float
  status: str  # "successful", "failed", "inconclusive"
  iterations: int  # How many times tried

Logged for:
  - Every novel approach CYNIC tries
  - Every hypothesis tested
  - Results from 1 to 100+ iterations
  - User feedback on each
```

---

## 4. Collaborative Executor

### Decision Classification System

**When CYNIC makes or proposes a decision, classify it:**

```
Class A: AUTONOMOUS (CYNIC acts alone)
  Examples:
    - Compress old traces (reversible)
    - Clean up disk space (reversible)
    - Adjust learning rate (testable)
    - Create experiment log backup (safe)

  Flow:
    1. CYNIC decides to act
    2. CYNIC executes action
    3. CYNIC reports: "I freed 5GB by compressing traces"
    4. Feedback processed for learning

Class B: CONSULTATION (CYNIC asks you)
  Examples:
    - Change axiom weights (affects all future judgments)
    - Modify relationship memory (could be wrong about you)
    - Delete historical data (permanent)
    - Change learning rate significantly

  Flow:
    1. CYNIC: "Should I increase axiom BURN weight to 0.8?"
    2. You: "YES / NO / EXPLAIN WHY"
    3. Feedback → Learning system
    4. CYNIC learns your reasoning

Class C: EXPLORATION (CYNIC proposes experiments)
  Examples:
    - Try novel dog combinations
    - Test alternative algorithms
    - Hypothesize new axiom relationships
    - Explore edge cases

  Flow:
    1. CYNIC: "I want to try Dogs 6+11. Want to see results?"
    2. You: "YES / NO / MODIFY"
    3. Run experiment
    4. Log results to experiment log
    5. CYNIC learns which worked
```

### Learning Which Class Each Decision Belongs To

```
First encounter with a decision type:
  - Default to Class B (ask you)

After 3-5 occurrences:
  - If you always approve → move to Class A (autonomous)
  - If you often veto → keep asking
  - Pattern emerges: "User never changes axiom BURN" → remember

Over time:
  - CYNIC learns your decision-making patterns
  - Asks less, acts more confidently
  - Proposes experiments you're likely to accept
```

---

## 5. Data Persistence & Memory Management

**Storage structure:**

```
~/.cynic/phase2/
├── relationship_memory.json
│   └─ User values, preferences, style (loaded at startup)
│
├── experiment_log.jsonl
│   └─ All experiments with results (append-only)
│
├── dialogue_history.db
│   └─ Last 1000 messages, full search
│
├── learning_metadata.json
│   └─ Decision classifications learned
│   └─ Q-table updates
│
└── archives/
    └─ 2026-02-26.jsonl (older conversations)
```

**Initialization:**
- Load relationship memory → Applied to all future dialogues
- Load experiment log → Informs what approaches to try
- Load decision classifications → Guide solo vs consultation decisions

**Cleanup:**
- Keep last 1000 messages in active memory
- Archive older messages monthly
- Aggregate experiment results (keep summary + representative traces)

---

## 6. Integration with Phase 1

**Observability → Dialogue:**
- Show current state (OBSERVE view) as context for dialogue
- "You asked about disk. Current state: 93.4% full"

**Dialogue → Learning:**
- Corrections from dialogue → Update Q-Table
- Feedback → Update relationship memory
- Novel approaches → Log to experiment log

**Learning → Executor:**
- Q-Table accuracy → Inform confidence in decisions
- Relationship memory → Personalize explanations
- Experiment results → Suggest next experiments

---

## 7. CLI Integration

**New menu options:**

```
[6] 💬 TALK         - Chat with CYNIC
[7] 📊 HISTORY      - View past conversations & decisions
[8] 🎛️  FEEDBACK     - Manage learning (clear memory, adjust learning rate)
[9] 🚀 ACTUATE      - Execute pending proposals & experiments
```

**TALK mode flow:**
```
[6] Enter TALK mode

CYNIC: "What would you like to discuss?"

You: type message
     ↓
CYNIC: Reason → Generate response → Display

You: type correction/feedback
     ↓
Learning system: Update Q-table, relationship memory, experiments
     ↓
CYNIC: Acknowledges → Adapts future responses
```

---

## 8. Success Criteria

✅ **You can ask CYNIC anything and get explanations**
- "Why did you choose that?" → Clear reasoning with axiom scores
- "Show me your thinking" → Full trace with dog votes

✅ **CYNIC improves from your feedback**
- Correct CYNIC → Accuracy goes up on similar proposals
- Tell CYNIC patterns → It remembers and applies them next time

✅ **CYNIC explores collaboratively**
- Proposes novel approaches
- Learns which you like
- Experiments logged and analyzed
- Results inform future suggestions

✅ **Some decisions are autonomous, some need you**
- Disk cleanup: CYNIC does it, tells you after
- Axiom changes: CYNIC asks permission
- Novel experiments: CYNIC proposes, you approve

✅ **You develop a relationship with CYNIC**
- CYNIC knows you personally (values, preferences, style)
- Adapts to your communication style
- Remembers past conversations
- Gets better at understanding what matters to you

✅ **Full bidirectionality**
- You ask questions → CYNIC answers
- CYNIC asks questions → You answer
- Both learn from the interaction
- Symbiosis deepens over time

---

## 9. Non-Goals (YAGNI)

- **Voice interface** — Text-based dialogue only
- **Advanced NLP** — Claude API for generation, simple parsing for input
- **Predictive analytics** — Reactive learning only, no forecasting
- **Multi-organism memory sharing** — Single organism, local storage
- **Automatic action execution without approval** — All Class B/C need you

---

## 10. Implementation Phases

**Phase 2a: Dialogue Foundation** (Days 1-2)
- Message types and storage
- Basic Q&A with reasoning engine
- Claude API integration

**Phase 2b: Learning System** (Days 3-4)
- Relationship memory implementation
- Experiment log design
- Q-Table enhancement

**Phase 2c: Collaborative Executor** (Days 5-6)
- Decision classification system
- Approval flows
- Learning which class each decision belongs to

**Phase 2d: Integration & Polish** (Days 7-8)
- Wire everything together
- CLI updates
- Testing and refinement

---

**Design approved and ready for implementation.**
