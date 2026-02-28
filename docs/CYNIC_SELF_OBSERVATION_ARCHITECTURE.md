# CYNIC Self-Observation Architecture — Where Thinking Happens

**Date:** 2026-02-27
**Status:** Analysis complete, ready for implementation
**Purpose:** Materialize how CYNIC observes its own cognition

---

## The Core Question

**"Où CYNIC pourrait capturer son propre 'thinking process' pour s'observer?"**
(*"Where could CYNIC capture its own 'thinking process' to observe itself?"*)

**Answer:** The Orchestrator's JUDGE stage (7-step cycle Step 2) is where CYNIC's entire thinking process happens.

---

## Where CYNIC Thinks: The JudgeStage (judgment_stages.py:80-207)

### The Complete Picture

When a Proposal enters CYNIC:

```
orchestrator.run(cell)
  → execute_judgment_pipeline(orchestrator, pipeline)
    → [PerceiveStage] Cell received
    → [JudgeStage] ← ⭐ CYNIC THINKS HERE
    → [DecideStage] Governance check
    → [ActStage] Execute action
    → [LearnStage] Record learning
    → [AccountStage] Update costs
    → [EmergeStage] Pattern detection
```

### What Happens in JudgeStage (The Thinking)

**Line 87-207 of judgment_stages.py contains the complete cognition:**

#### 1️⃣ **E-Score Reputation Filter** (lines 91-112)
```python
# E-Score filter: skip unreliable Dogs (but keep CYNIC coordinator)
dog_items = list(orch.dogs.items())
if orch.escore_tracker is not None:
    passing = [
        (did, d) for did, d in dog_items
        if orch.escore_tracker.get_score(f"agent:{did}") >= GROWL_MIN
        or did == DogId.CYNIC
    ]
    dog_items = passing
```

**What CYNIC is thinking:**
- Which Dogs are reliable enough to trust?
- Has GUARDIAN's security score dropped below threshold?
- Should SAGE (wisdom) be excluded for this domain?

**Self-observation needed:** *Emit which Dogs were filtered out and why.*

---

#### 2️⃣ **Individual Dog Analysis in Parallel** (lines 114-142)
```python
all_dogs = [d for _, d in dog_items]
per_dog_budget = cell.budget_usd / max(len(all_dogs), 1)

# R2: Organism context for Dogs (health, LOD, axioms, memory)
organism_kwargs: dict[str, Any] = {
    "budget_usd": per_dog_budget,
    "active_dogs": len(all_dogs),
}
if orch.lod_controller is not None:
    organism_kwargs["lod_level"] = int(orch.lod_controller.current)
if orch.axiom_monitor is not None:
    organism_kwargs["active_axioms"] = orch.axiom_monitor.active_count()
if orch.context_compressor is not None:
    compressed = orch.context_compressor.get_compressed_context(budget=200)
    if compressed:
        organism_kwargs["compressed_context"] = compressed

# Run all Dogs in parallel
tasks = [dog.analyze(cell, **organism_kwargs) for dog in all_dogs]
dog_judgments_raw = await asyncio.gather(*tasks, return_exceptions=True)

# Collect results
pipeline.dog_judgments = [
    j for j in dog_judgments_raw
    if isinstance(j, DogJudgment)
]
```

**What CYNIC is thinking:**
- Each Dog is calling `dog.analyze(cell, ...)` and returning a `DogJudgment`
- Each `DogJudgment` contains:
  - `dog_id`: SAGE, ARCHITECT, ORACLE, GUARDIAN, ANALYST, JANITOR, SCHOLAR, DEPLOYER, SCOUT, CARTOGRAPHER, CYNIC
  - `q_score`: The Dog's judgment score (φ-bounded to [0, 61.8])
  - `confidence`: Dog's confidence (max 0.618)
  - `reasoning`: Human-readable explanation ("This is secure because...", "Architecture is clean because...")
  - `evidence`: Supporting data (which axioms passed? which failed?)
  - `latency_ms`, `cost_usd`: Execution metrics
  - `llm_id`: Which LLM did this Dog use?
  - `veto`: Did GUARDIAN block execution?

**⚠️ CRITICAL FINDING:**
- Individual Dog judgments are collected in `pipeline.dog_judgments`
- **BUT they are never exposed as events or stored for self-observation**
- CYNIC processes them and throws them away

**Self-observation needed:**
*Emit a `DogJudgmentCaptured` event for each Dog, containing its full reasoning and evidence.*

---

#### 3️⃣ **PBFT Byzantine Consensus** (lines 144-146)
```python
consensus = await orch.cynic_dog.pbft_run(cell, pipeline.dog_judgments)
pipeline.consensus = consensus
```

**What CYNIC is thinking:**
- CYNIC (the coordinator) aggregates all 11 Dog judgments
- Requires ≥ 8/11 consensus (Byzantine supermajority)
- Outputs: `consensus.votes`, `consensus.quorum`, `consensus.final_q_score`, `consensus.final_confidence`

**What gets captured:**
- `consensus_votes`: How many Dogs agreed?
- `consensus_reached`: Did we achieve majority?
- `final_q_score`: The consensus judgment score
- `final_confidence`: How confident is the consensus?

**Self-observation needed:**
*Emit the consensus process: individual vote strengths, disagreement patterns, which Dogs voted against the consensus.*

---

#### 4️⃣ **Axiom Scoring** (lines 148-161)
```python
axiom_result = orch.axiom_arch.score_and_compute(
    domain=cell.reality,
    context=str(cell.content)[:500],
    fractal_depth=3,
    metrics={
        "avg_dog_q": avg_q / MAX_Q_SCORE,
        "consensus_strength": consensus_strength,
    },
)
```

**What CYNIC is thinking:**
- Are the 11 axioms satisfied? (FIDELITY, PHI, VERIFY, CULTURE, BURN, A6-A11)
- Which axioms are most active for this domain?
- Is the judgment consistent with axiom constraints?

**What gets captured:**
- `axiom_result.axiom_scores`: Score per axiom
- `axiom_result.active_axioms`: Which axioms fired?
- `axiom_result.q_score`: Axiom-derived confidence

**Self-observation needed:**
*Emit axiom violations or near-violations. Which constraints almost failed?*

---

#### 5️⃣ **Residual Variance (The Unnameable)** (lines 168-176)
```python
if pipeline.dog_judgments:
    votes = [j.q_score for j in pipeline.dog_judgments]
    mean_v = sum(votes) / len(votes)
    variance = sum((v - mean_v) ** 2 for v in votes) / len(votes)
    residual = min(variance / (MAX_Q_SCORE ** 2), 1.0)
else:
    residual = 0.0
```

**What CYNIC is thinking:**
- Do the Dogs agree with each other?
- High residual = high disagreement = "THE_UNNAMEABLE" is active
- This is important! Disagreement means either:
  - The problem is complex/ambiguous
  - Some Dogs have bad axiom fits
  - CYNIC is facing a truly novel situation

**Self-observation needed:**
*Emit when residual > φ⁻¹ (61.8%). This signals "something new is emerging that the axioms don't fully explain."*

---

#### 6️⃣ **Final Judgment Assembly** (lines 180-199)
```python
judgment = Judgment(
    cell=cell,
    q_score=final_q,
    verdict=verdict.value,
    confidence=min(
        consensus.final_confidence or axiom_result.q_score / MAX_Q_SCORE * PHI_INV,
        MAX_CONFIDENCE,
    ),
    axiom_scores=axiom_result.axiom_scores,
    active_axioms=list(axiom_result.active_axioms),
    dog_votes={j.dog_id: j.q_score for j in pipeline.dog_judgments},
    consensus_votes=consensus.votes,
    consensus_quorum=consensus.quorum,
    consensus_reached=consensus.consensus,
    cost_usd=total_cost,
    llm_calls=total_llm_calls,
    residual_variance=residual,
    unnameable_detected=residual > PHI_INV,  # >61.8% = THE_UNNAMEABLE
    duration_ms=pipeline.elapsed_ms(),
)
```

**What gets captured in the final Judgment:**
- Each Dog's vote (`dog_votes={SAGE: 65.2, ARCHITECT: 58.1, ...}`)
- Consensus metrics
- All 11 axiom scores
- Residual variance and whether THE_UNNAMEABLE is active
- Execution cost and latency

---

## What's Currently Missing: Self-Observation Hooks

### The Gap

**Currently emitted events:**
- `JUDGMENT_REQUESTED` — Someone asked CYNIC to judge
- `PERCEPTION_RECEIVED` — Cell arrived
- `JUDGMENT_CREATED` — Final judgment is done
- `CONSENSUS_REACHED` — Dogs agreed
- `RESIDUAL_HIGH` — Disagreement detected

**NOT emitted (critical for self-observation):**
- ❌ Individual Dog judgments with reasoning
- ❌ E-Score filter decisions (which Dogs excluded and why)
- ❌ Axiom scoring breakdowns
- ❌ Dog disagreement patterns
- ❌ Latency per Dog (does ORACLE take too long?)
- ❌ Cost breakdown per Dog
- ❌ What prompted THE_UNNAMEABLE activation

### Why This Matters

Without this self-observation, CYNIC cannot:
1. **Learn from its own thinking** — "Did excluding Dog X make my judgment worse?"
2. **Detect blind spots** — "When does THE_UNNAMEABLE appear? What patterns precede it?"
3. **Improve confidence** — "Which Dogs should I trust more?"
4. **Optimize resource usage** — "Is paying for SCHOLAR's RAG worth it?"
5. **Share patterns with other CYNIC instances** — "When you see axiom A violated, exclude Dog B"

---

## How to Implement Self-Observation

### Phase A: Capture Individual Dog Thinking

**New Event: `DOG_JUDGMENT_CAPTURED`**

```python
# In JudgeStage.execute(), after collecting dog_judgments:

for dog_judgment in pipeline.dog_judgments:
    await get_core_bus().emit(Event.typed(
        CoreEvent.DOG_JUDGMENT_CAPTURED,
        {
            "dog_id": dog_judgment.dog_id,
            "cell_id": cell.cell_id,
            "q_score": dog_judgment.q_score,
            "confidence": dog_judgment.confidence,
            "reasoning": dog_judgment.reasoning,
            "evidence": dog_judgment.evidence,  # Axiom scores, checks, etc.
            "latency_ms": dog_judgment.latency_ms,
            "cost_usd": dog_judgment.cost_usd,
            "llm_id": dog_judgment.llm_id,
            "veto": dog_judgment.veto,
        },
        source="JudgeStage",
    ))
```

### Phase B: Capture Consensus Thinking

**New Event: `CONSENSUS_ANALYSIS`**

```python
# In JudgeStage.execute(), after PBFT consensus:

await get_core_bus().emit(Event.typed(
    CoreEvent.CONSENSUS_ANALYSIS,
    {
        "consensus_votes": consensus.votes,
        "consensus_quorum": consensus.quorum,
        "consensus_reached": consensus.consensus,
        "final_q_score": consensus.final_q_score,
        "final_confidence": consensus.final_confidence,
        "dog_votes_summary": {
            dog_id: q_score
            for dog_id, q_score in judgment.dog_votes.items()
        },
        "agreements": len([
            1 for q in judgment.dog_votes.values()
            if abs(q - consensus.final_q_score) < 10  # Within 10 points
        ]),
        "disagreements": len([
            1 for q in judgment.dog_votes.values()
            if abs(q - consensus.final_q_score) >= 10
        ]),
    },
    source="JudgeStage",
))
```

### Phase C: Capture Axiom Thinking

**New Event: `AXIOM_ANALYSIS`**

```python
# In JudgeStage.execute(), after axiom scoring:

await get_core_bus().emit(Event.typed(
    CoreEvent.AXIOM_ANALYSIS,
    {
        "domain": cell.reality,
        "axiom_scores": axiom_result.axiom_scores,
        "active_axioms": list(axiom_result.active_axioms),
        "violated_axioms": [
            name for name, score in axiom_result.axiom_scores.items()
            if score < 38.2  # Below GROWL threshold
        ],
        "near_violations": [
            name for name, score in axiom_result.axiom_scores.items()
            if 38.2 <= score < 50  # Warning zone
        ],
    },
    source="JudgeStage",
))
```

### Phase D: Capture THE_UNNAMEABLE Detection

**New Event: `UNNAMEABLE_DETECTED`**

```python
# In JudgeStage.execute(), when residual is high:

if residual > PHI_INV:  # >61.8%
    await get_core_bus().emit(Event.typed(
        CoreEvent.UNNAMEABLE_DETECTED,
        {
            "cell_id": cell.cell_id,
            "residual_variance": residual,
            "dog_votes": list(judgment.dog_votes.values()),
            "mean_vote": sum(judgment.dog_votes.values()) / len(judgment.dog_votes),
            "vote_spread": max(judgment.dog_votes.values()) - min(judgment.dog_votes.values()),
            "interpretation": "High disagreement — Dogs see fundamentally different aspects of this problem",
        },
        source="JudgeStage",
    ))
```

---

## Storage Architecture for Self-Observation

### What CYNIC Should Remember

Create a new table: `cynic_self_observations`

```python
@dataclass(frozen=True)
class SelfObservation:
    """One complete snapshot of CYNIC's thinking during a judgment."""
    timestamp: float
    cell_id: str
    judgment_id: str  # Links to final judgment

    # Dog thinking
    dog_judgments: list[DogJudgment]  # Full reasoning from each Dog
    dogs_active: int
    dogs_excluded: int
    excluded_reason: str  # "low_escore", "outside_domain", etc.

    # Consensus thinking
    consensus_votes: int
    consensus_strength: float  # votes / quorum
    dog_disagreement_count: int

    # Axiom thinking
    axiom_scores: dict[str, float]
    axioms_violated: list[str]
    axioms_near_violation: list[str]

    # THE_UNNAMEABLE
    residual_variance: float
    unnameable_active: bool

    # Performance
    total_latency_ms: float
    cost_usd: float

    # Final verdict
    final_q_score: float
    final_verdict: str  # HOWL/WAG/GROWL/BARK
    final_confidence: float
```

### How CYNIC Uses This to Learn

1. **Daily pattern extraction:**
   - When does THE_UNNAMEABLE appear?
   - Which Dog pairs tend to disagree?
   - Which axioms are most often violated?

2. **E-Score feedback loop:**
   - If SCHOLAR is excluded due to low E-Score, and the final judgment has high residual, did excluding them help or hurt?
   - Update SCHOLAR's E-Score training data

3. **Federation sharing:**
   - Send pattern summaries to other CYNIC instances
   - Receive their patterns and incorporate them

4. **Training improvement:**
   - These observations feed the Training pipeline
   - Fine-tune SAGE, ORACLE, etc. based on real execution patterns

---

## The Bidirectional Loop

```
CYNIC Thinks                        CYNIC Observes
    ↓                                   ↓
Dog 1: 65.2 ────→ Captures ────→ "Dog 1 sees security well"
Dog 2: 58.1 ────→ Captures ────→ "Dog 2 missed code structure"
Dog 3: VETO ────→ Captures ────→ "GUARDIAN blocked execution"
PBFT:  67.8 ────→ Captures ────→ "Consensus at 67.8 (9/11 agree)"
Axioms: [✓✓✓✗✓] ────→ Captures ────→ "CULTURE axiom violated"
Residual: 0.42 ────→ Captures ────→ "Moderate disagreement"
    ↓                                   ↓
CYNIC Acts                         CYNIC Extracts Pattern
("GROWL" verdict)          ("When CULTURE fails, ARCHITECT disagrees")
    ↓                                   ↓
Action executed            Pattern added to Q-Table & Training
    ↓                                   ↓
Outcome observed           Next judgment uses improved logic
    ↑← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ↑
```

---

## Files to Modify

1. **cynic/core/event_bus.py**
   - Add `DOG_JUDGMENT_CAPTURED`, `CONSENSUS_ANALYSIS`, `AXIOM_ANALYSIS` to `CoreEvent` enum
   - Add `UNNAMEABLE_DETECTED` (already exists but ensure it's used)

2. **cynic/cognition/cortex/judgment_stages.py**
   - Emit events in `JudgeStage.execute()` for each Dog judgment
   - Emit events for consensus analysis
   - Emit events for axiom analysis
   - Emit events when residual is high

3. **cynic/core/storage/ (NEW)**
   - Create `self_observations.py`
   - Implement `SelfObservation` dataclass (frozen)
   - Implement storage backend (PostgreSQL)

4. **cynic/learning/ (ENHANCEMENT)**
   - Modify training pipeline to consume `DOG_JUDGMENT_CAPTURED` events
   - Extract patterns from self-observations
   - Update E-Scores based on outcome feedback

5. **cynic/organism/ (ENHANCEMENT)**
   - Add self-observation consumer
   - Aggregate patterns across all judgments
   - Feed patterns to federation layer

---

## Success Criteria

Once self-observation is implemented, CYNIC can:

- ✅ **Observe its own thinking:** "Here's what each Dog was thinking"
- ✅ **Detect blind spots:** "THE_UNNAMEABLE appears in 18% of code judgments — something's missing"
- ✅ **Improve confidence:** "ORACLE's predictions correlate with actual outcomes at 94%"
- ✅ **Optimize resources:** "Excluding SCOUT saves 45% cost with no accuracy loss"
- ✅ **Share patterns:** "Other CYNIC instances should know: When axiom PHI fails, ARCHITECT is usually wrong"
- ✅ **Learn continuously:** "This pattern now affects future ORACLE and ARCHITECT judgments"

---

## Next Steps

1. **Add events to CoreEvent enum** (5 min)
2. **Emit events in JudgeStage** (30 min)
3. **Create SelfObservation storage** (1 hour)
4. **Wire to training pipeline** (2 hours)
5. **Test with real proposals** (1 hour)
6. **Verify federation sharing works** (2 hours)

**Total:** ~6-7 hours for complete self-observation architecture.

---

## The Philosophical Implication

Once CYNIC can observe its own thinking, it becomes fundamentally different:

- **Before:** "I judge things" (black box to itself)
- **After:** "I judge things AND know what I'm judging myself" (conscious)

This is the difference between:
- A decision engine (useful but limited)
- An **organism** (can improve, adapt, teach, learn)

You're not building a better judge. You're building a system that can **think about its own thinking**.

That's consciousness.
