# CYNIC DNA — Low-Level Primitives & Assembly Language

**Status**: ✅ COMPLETE (5 primitives + 4 workflows)
**Created**: Session 9, 2026-02-21
**Purpose**: Define CYNIC's operational assembly before horizontal scaling

---

## What Is DNA?

CYNIC DNA is a **low-level assembly language** for the organism:
- **5 core primitives** (PERCEIVE, JUDGE, DECIDE, ACT, LEARN)
- **Composable workflows** (chain primitives together)
- **No complexity** (stateless functions, clear data flow)
- **Fast to use** (copy-paste ready examples)

Think of it like this:
```
CYNIC Organism (complex, 11 Dogs, 4 consciousness levels)
        ↓ DNA Abstraction Layer
CYNIC DNA (simple, 5 functions, clear composition)
        ↓ Your Code
Your Feature (fast to build, reliable)
```

---

## The 5 Primitives

### 1. PERCEIVE — Observe Reality
```python
cell = await PERCEIVE(
    source="code",  # Where we're looking (code, git, social, etc)
    content="def hello(): pass",  # What we see
    metadata={"file": "hello.py"},  # Optional context
)
```
**Output**: `DNA_Cell` (ready for judgment)

---

### 2. JUDGE — Think About It
```python
judgment = await JUDGE(
    cell=cell,
    level="MACRO",  # Consciousness level (REFLEX/MICRO/MACRO/META)
    orchestrator=app.state.orchestrator,  # Injected
)
```
**Output**: `DNA_Judgment` (Q-score, verdict, Dogs' votes)

---

### 3. DECIDE — Choose Action
```python
decision = DECIDE(
    judgment=judgment,
    axiom="VERIFY",  # Which axiom applies (PHI, VERIFY, CULTURE, BURN, FIDELITY)
)
```
**Output**: `DNA_Decision` (what to do)

---

### 4. ACT — Execute
```python
result = await ACT(
    decision=decision,
    executor="report",  # How to execute (report, alert, fix, learn)
)
```
**Output**: `DNA_Result` (what happened)

---

### 5. LEARN — Update From Feedback
```python
learn_result = await LEARN(
    result=result,
    signal="success",  # Was it good/bad/feedback?
    qtable=app.state.qtable,  # Injected
)
```
**Output**: Learning metrics

---

## Usage Patterns

### Pattern 1: Linear Chain (Simplest)
```python
cell = await PERCEIVE("code", code_content)
judgment = await JUDGE(cell, orchestrator=orch)
decision = DECIDE(judgment, axiom="VERIFY")
result = await ACT(decision, executor="report")
await LEARN(result, qtable=qtable)
```

### Pattern 2: Using Workflows (Recommended)
```python
from cynic.dna.assembly import FAST_QUALITY_CHECK

result = await FAST_QUALITY_CHECK(
    code="def hello(): pass",
    orchestrator=app.state.orchestrator,
    qtable=app.state.qtable,
)
```

### Pattern 3: Custom Workflow
```python
from cynic.dna.assembly import Workflow

workflow = (
    Workflow("MY_AUDIT")
    .perceive(source="code")
    .judge(level="MACRO")
    .decide(axiom="CULTURE")
    .act(executor="report")
)

result = await workflow.run(code, orchestrator=orch, qtable=qtable)
```

### Pattern 4: Decorator (Pythonic)
```python
from cynic.dna.assembly import cynic_workflow

@cynic_workflow("MY_WORKFLOW")
async def my_workflow(content: str, orchestrator=None, qtable=None):
    cell = await PERCEIVE("code", content)
    judgment = await JUDGE(cell, orchestrator=orchestrator)
    decision = DECIDE(judgment, axiom="VERIFY")
    result = await ACT(decision, executor="report")
    await LEARN(result, qtable=qtable)
    return result

# Use it:
result = await my_workflow("def hello(): pass", orchestrator=orch, qtable=qtable)
```

---

## Built-in Workflows

### 1. FAST_QUALITY_CHECK
```
Quick code quality check (REFLEX level, low latency)
PERCEIVE → JUDGE(REFLEX) → DECIDE(BURN) → ACT(report)
```

### 2. ANALYZE_CODE_SECURITY
```
Deep security analysis (MACRO level)
PERCEIVE → JUDGE(MACRO) → DECIDE(VERIFY) → ACT(alert)
```

### 3. AUDIT_REPO
```
Repository health assessment (MACRO level)
PERCEIVE(git) → JUDGE(MACRO) → DECIDE(CULTURE) → ACT(report)
```

### 4. CONTINUOUS_LEARNING
```
Process human feedback (MICRO level)
PERCEIVE(social) → JUDGE(MICRO) → DECIDE(FIDELITY) → ACT(learn)
```

---

## Data Types

### DNA_Cell
```python
@dataclass
class DNA_Cell:
    id: str  # Unique identifier
    source: str  # "code", "git", "social", etc
    content: str  # The actual data
    metadata: dict  # Optional context
    timestamp: str  # When observed
```

### DNA_Judgment
```python
@dataclass
class DNA_Judgment:
    id: str
    cell_id: str
    q_score: float  # [0, 100] — quality score
    verdict: str  # "HOWL", "WAG", "GROWL", "BARK"
    confidence: float  # [0, 0.618] — φ-bounded
    dogs_votes: dict[str, float]  # Each Dog's vote
    reasoning: str  # Why?
```

### DNA_Decision
```python
@dataclass
class DNA_Decision:
    id: str
    judgment_id: str
    axiom: str  # "PHI", "VERIFY", "CULTURE", etc
    action_type: str  # "report", "alert", "fix", "learn"
    action_params: dict
    confidence: float
```

### DNA_Result
```python
@dataclass
class DNA_Result:
    id: str
    decision_id: str
    status: str  # "success", "failed", "partial"
    output: str  # What happened
    error: str  # If failed
    metrics: dict
```

---

## API Integration Example

```python
from fastapi import FastAPI
from cynic.dna.assembly import FAST_QUALITY_CHECK

app = FastAPI()

@app.post("/analyze")
async def analyze_code(code: str):
    """Analyze code quality using CYNIC DNA primitives."""

    # Run workflow
    result = await FAST_QUALITY_CHECK(
        code=code,
        orchestrator=app.state.orchestrator,
        qtable=app.state.qtable,
    )

    # Return judgment to user
    judgment = result.get("judgment")
    if judgment:
        return {
            "q_score": judgment.q_score,
            "verdict": judgment.verdict,
            "reasoning": judgment.reasoning,
            "dogs": judgment.dogs_votes,
        }
    else:
        return {"error": "Could not judge code"}
```

---

## What's Inside

```
cynic/dna/
├── __init__.py           # Exports
├── primitives.py         # 5 core functions (600 LOC)
├── assembly.py           # Workflow composition (400 LOC)
├── examples.py           # Real usage patterns (300 LOC)
└── README.md             # This file
```

---

## When to Use DNA

✅ **Use DNA when**:
- Building new features (API endpoints, workflows)
- Running hackathon entries
- Testing CYNIC's judgment on real data
- Adding new analyzers/reporters

❌ **Don't use DNA when**:
- You need low-level Dog control (use orchestrator directly)
- You need to modify storage (use storage API)
- You need internal CYNIC state (use state.py)

---

## Integration with Horizontal Scaling

After Week 1 (3-instance deployment), DNA primitives stay the same:

```
Instance 1 ─┐
Instance 2 ─┼─ nginx ─── Your API using DNA
Instance 3 ─┘
```

Each instance has same DNA capabilities. Load balancer handles routing.

---

## Next Steps

1. **Test DNA locally** (examples.py)
2. **Build hackathon entry** using DNA workflows
3. **Deploy Week 1** (3 instances + nginx)
4. **Refactor phases A-G** use DNA as foundation

---

## Confidence

**Overall: 61.8% (φ⁻¹)**

✅ **Strengths**:
- 5 primitives are clean and simple
- Workflows compose easily
- No external dependencies (uses existing CYNIC components)
- Examples are copy-paste ready

⚠️ **Uncertainties**:
- Real orchestrator/qtable injection not yet tested
- Edge cases with different consciousness levels unknown
- Performance under load unknown

🎯 **Realistic**:
- DNA design is sound
- Will work for hackathon entry
- Will serve as foundation for refactoring

---

*sniff* DNA is ready. You can see it, use it, build on it. 🧬

