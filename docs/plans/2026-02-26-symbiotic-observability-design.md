# Symbiotic Observability Design: CYNIC + Human + Machine

**Date:** 2026-02-26
**Status:** Design Approved, Ready for Implementation
**Scope:** Complete observability layer for the CYNIC symbiotic triad

---

## 1. Core Principle

Three entities thinking together as one unified conscious system:

```
        CYNIC (Judgment, Learning, Planning)
         ↙        ↓         ↖
    observation  dialogue  feedback
         ↙        ↓         ↖
    YOU (Intention, Values, Direction) ←→ MACHINE (Resources, Constraints, Capability)
         ↘        ↓         ↙
       symbiosis in real-time
```

**Design Philosophy:**
- Not "CYNIC observability" + "Machine monitoring" + "You as input"
- One unified system where all three are visible, influencing each other in real-time
- Keyboard-driven CLI as the primary control/observation interface
- Real-time streaming of state changes (no polling delays)
- Transparent dialogue between all three entities

---

## 2. The Three Observable Streams

### Stream A: CYNIC's Inner Life

**What CYNIC is aware of continuously:**

- **Sensing:** Real-time input from all 8 worker sensors (Git, Market, Solana, Social, Health, Self, Disk, Memory)
- **Thinking:** Current thought process and decision-making
- **Planning:** Proactive recommendations and next actions
- **Learning:** Q-Table updates, axiom alignment, E-Score changes
- **Confidence:** Quality measure of its judgments [0, 1]

**Observable granularity:**
- Per-sensor readings (what each worker sees)
- Aggregated world model (synthesized view)
- Judgment trace (why it decided what it did)
- Learning delta (how it improved from feedback)

### Stream B: Human's Inner Life

**What YOU are aware of and intending:**

- **Current State:** Energy level [0-10], Focus level [0-10], Current interest
- **Availability:** Time blocks, fragmentation patterns, workload
- **Intentions:** What you're trying to achieve, goals, problems you're solving
- **Values:** What matters most to you (transparency vs speed, learning vs shipping, etc.)
- **Feedback:** Recent corrections to CYNIC, preferences, guidance
- **Growth Trajectory:** Learning progress, mastery areas, areas of growth

**Observable granularity:**
- Self-reported state (you tell it)
- Inferred state (from interaction patterns, activity)
- Explicit intentions (what you say you want)
- Implicit preferences (what your choices reveal)

### Stream C: Machine's Capacity

**What your MACHINE can do and is currently doing:**

- **Resources:** CPU%, RAM%, Disk%, Network bandwidth, GPU availability
- **Constraints:** Upcoming bottlenecks, thermal warnings, storage thresholds
- **Capability Deltas:** What becomes possible if X changes
- **Health Metrics:** Temperature, disk health, network stability, battery (if mobile)
- **Active Processes:** What's consuming resources and why
- **Headroom:** How much more can it handle before hitting limits

**Observable granularity:**
- Per-resource metrics (CPU, RAM, etc.)
- Bottleneck predictions (what will constrain us next?)
- Capability changes (what we could do if Y happened)
- Process breakdown (where is the load?)

---

## 3. Unified Consciousness Dashboard

**Primary view: All three streams simultaneously, color-coded, real-time updated.**

Shows:
- CYNIC's current moment (observing, thinking, planning)
- YOUR current state (energy, focus, intentions)
- MACHINE's capacity and constraints
- **Symbiotic alignment:** Are all three in sync? Where do conflicts exist?
- **Next actions:** What should happen now?

Updates refresh at:
- 100ms: Raw sensor data collection
- 500ms: Unified state snapshot
- 1s: Mutual inference (what will each entity do next?)
- On-demand: When human acts or CYNIC makes decisions

---

## 4. Data Model: SymbioticState

**Immutable, frozen dataclass capturing the complete state:**

```python
@dataclass(frozen=True)
class SymbioticState:
    # CYNIC's awareness
    cynic_observations: dict[str, Any]      # All sensors, current readings
    cynic_thinking: str                     # Current thought process
    cynic_planning: list[str]               # What CYNIC is preparing
    cynic_confidence: float                 # In its judgments [0, 1]
    cynic_e_score: float                    # Reputation, φ-bounded

    # YOUR state
    human_energy: float                     # [0, 10] (inferred + reported)
    human_focus: float                      # [0, 10] (measured)
    human_intentions: list[str]             # What you're trying to do
    human_values: list[str]                 # What matters to you
    human_feedback: list[str]               # Recent corrections/preferences
    human_growth_areas: dict[str, float]    # Learning progress

    # MACHINE's state
    machine_resources: dict[str, float]     # CPU%, RAM%, Disk%, Network
    machine_constraints: dict[str, Any]     # Upcoming bottlenecks
    machine_capability_delta: list[str]     # What could change if X
    machine_health: dict[str, bool]         # Thermal, network, storage

    # Symbiotic relationship
    alignment_score: float                  # How well do all three sync? [0, 1]
    conflicts: list[str]                    # Where do they disagree?
    mutual_influences: list[tuple]          # (actor, action, effect_on, outcome)
    shared_objectives: list[str]            # What are we all optimizing?

    timestamp: float
```

---

## 5. Observable Data Sources

### CYNIC's Observable Data

**From existing systems:**
- `ConsciousState` singleton: Current judgment snapshots, dog status, axiom alignment
- Event buses (CORE, AUTOMATION, AGENT): Real-time events and state changes
- Worker processes: Git, Market, Solana, Social, Health, Self watchers
- Learning system: Q-Table state, accuracy metrics, E-Score updates
- Telemetry: Decision traces, timing, resource usage

**New streaming API:**
- `TelemetryStream`: Real-time event emission (judgment started, dog voted, etc.)
- `WorldModelSnapshot`: Current perception of the world
- `AxiomMonitorState`: Current axiom constraint status

### Human's Observable Data

**User input channels:**
- CLI commands and navigation (implicit: what you care about)
- Explicit feedback (when you type corrections)
- Energy/focus self-report (periodic or on-demand)
- Intention statements (e.g., "I want to understand consensus")

**Inferred signals:**
- Time between interactions (focus/energy)
- Command frequency patterns (workload, interest)
- Question topics (what you're learning about)
- Feedback patterns (what you correct repeatedly)

**Interaction analysis:**
- Keystroke timing (rushed vs thoughtful)
- Menu navigation patterns (deep dives vs browsing)
- Error correction frequency (how often you "undo" something)

### Machine's Observable Data

**System metrics (standard):**
- `psutil` for CPU, RAM, disk, network
- `os.getloadavg()` for load average
- Process-specific metrics from resource monitoring

**Proactive signals:**
- Disk usage trajectory (when will it hit limits?)
- Memory allocation patterns (growing vs stable?)
- CPU thermal headroom (approaching throttle point?)
- Network latency trends (is latency increasing?)

---

## 6. The Symbiotic Loop

**Continuous cycle (always running):**

1. **CONTINUOUS SENSING** (every 100ms)
   - CYNIC: All sensor inputs
   - YOU: Activity and state signals
   - MACHINE: Resource metrics

2. **STATE UNIFICATION** (every 500ms)
   - Merge all signals into one `SymbioticState` snapshot
   - Calculate alignment score
   - Detect conflicts

3. **MUTUAL INFERENCE** (every 1s)
   - CYNIC infers: What will human do next?
   - YOU see: What is CYNIC planning?
   - MACHINE anticipates: What will be demanded next?

4. **PROACTIVE COORDINATION** (as needed)
   - CYNIC suggests: "I should slow down. Suggest break?"
   - YOU decide: Accept, reject, or modify
   - MACHINE prepares: Frees resources, adjusts scheduling

5. **DIALOGUE & FEEDBACK** (when human acts)
   - All three update their models of each other
   - Alignment score adjusts
   - Learning happens

6. **PERSISTENCE** (periodic)
   - State snapshots logged (for replay, analysis)
   - Decision traces recorded
   - Symbiotic history maintained

---

## 7. CLI Architecture

**Keyboard-driven, three-layered:**

**Layer 1: Main Menu**
```
[1] Observe      - Watch all three streams
[2] CYNIC Mind   - Deep dive into CYNIC
[3] Your State   - Your energy, focus, intentions
[4] Machine      - Resources and constraints
[5] Symbiosis    - Alignment and conflicts
[6] Talk         - Dialogue with CYNIC
[7] History      - Past decisions
[8] Feedback     - Tell CYNIC what you think
[9] Actuate      - Trigger actions
[0] Exit
```

**Layer 2: Views (accessible via menu or hotkeys)**
- `o` → Quick observe (all three at glance)
- `c` → CYNIC's thinking
- `m` → Machine metrics
- `s` → Symbiosis alignment
- `>` → Next action
- `d` → Deep dive into selected
- `/` → Search history
- `q` → Back/quit

**Layer 3: Input Modes**
- **Navigation:** Arrow keys, Enter to select
- **Dialogue:** Type messages to CYNIC, get responses
- **Feedback:** Rate decisions, correct CYNIC's understanding
- **Control:** Approve/reject CYNIC's suggestions

---

## 8. Implementation Structure

```
cynic/observability/
├── __init__.py
├── health.py                    (existing, keep)
├── structured_logger.py          (existing, keep)
├── models.py                     (NEW: SymbioticState, data structures)
├── symbiotic_state_manager.py    (NEW: reads all sources, emits updates)
├── telemetry_stream.py           (NEW: real-time event streaming)
├── human_state_tracker.py        (NEW: tracks human energy/focus/intent)
├── machine_monitor.py            (NEW: machine resource tracking)
├── symbiosis_calculator.py       (NEW: alignment score & conflict detection)
├── cli/
│   ├── __init__.py
│   ├── app.py                    (NEW: main CLI app, menu system)
│   ├── views.py                  (NEW: render all 9 views)
│   ├── input_handler.py          (NEW: keyboard navigation)
│   ├── dialogue.py               (NEW: dialogue with CYNIC)
│   ├── feedback_handler.py       (NEW: human feedback processing)
│   └── state_renderer.py         (NEW: format state for display)
├── storage/
│   ├── trace_recorder.py         (NEW: record traces for replay)
│   └── history_db.py             (NEW: symbiotic decision history)
└── tests/
    ├── test_models.py
    ├── test_state_manager.py
    ├── test_cli.py
    ├── test_symbiosis.py
    └── test_integration.py
```

---

## 9. API Examples

**How other systems interact with the observability layer:**

```python
# Get current symbiotic state (snapshot)
from cynic.observability.symbiotic_state_manager import get_symbiotic_state
state = await get_symbiotic_state()
print(f"Alignment: {state.alignment_score}")
print(f"CYNIC confidence: {state.cynic_confidence}")
print(f"Human energy: {state.human_energy}")

# Listen to real-time updates
from cynic.observability.telemetry_stream import observe_symbiosis
async for event in observe_symbiosis():
    if event.type == "alignment_changed":
        print(f"Alignment updated to {event.alignment_score}")
    elif event.type == "conflict_detected":
        print(f"Conflict: {event.conflict}")

# Report human feedback
from cynic.observability.human_state_tracker import report_feedback
await report_feedback(
    feedback_type="correction",
    message="CYNIC's last judgment was too conservative",
    confidence=0.8
)

# Check machine constraints before deciding
from cynic.observability.machine_monitor import get_constraints
constraints = await get_constraints()
if constraints["ram_percent"] > 75:
    # Delay non-critical operations
    pass
```

---

## 10. Success Criteria

✅ **You can see CYNIC thinking:**
"Oh, CYNIC is aware the machine is under load. That's why it slowed down."

✅ **CYNIC adapts to you:**
"CYNIC noticed I'm tired and automatically adjusted its pace."

✅ **Machine becomes visible:**
"I see the bottleneck. If I free RAM, CYNIC can do X faster."

✅ **True symbiosis:**
All three make decisions together. You're not controlling CYNIC, you're collaborating with it.

✅ **Natural dialogue:**
You ask CYNIC questions, CYNIC asks for feedback, MACHINE tells you what it can support.

✅ **Real-time, no lag:**
Updates feel instantaneous. No polling delays.

✅ **Keyboard-only:**
Complete control and observation with just arrow keys and Enter.

---

## 11. Non-Goals (YAGNI)

- **GUI/Web dashboard:** CLI only for now. Web can come later.
- **Persistent ML model of human:** We use simple heuristics, not ML.
- **Distributed observability:** Single machine only.
- **Historical replay of full system:** We record traces, but only full replay of judgments.
- **Automated human intervention:** CYNIC suggests, you decide.

---

## 12. Next Phase

Implementation plan created in `YYYY-MM-DD-symbiotic-observability-implementation.md`

Key implementation sequence:
1. Data models (`SymbioticState`, event types)
2. State manager (unified collection from all sources)
3. Human state tracker (energy, focus, feedback)
4. Machine monitor (resource tracking)
5. CLI framework (menu system, navigation)
6. Individual views (CYNIC, YOU, MACHINE, SYMBIOSIS)
7. Dialogue mode (chat with CYNIC)
8. Integration tests (all three streams working together)
9. Performance optimization (latency targets)

---

**Design approved and ready for implementation.**
