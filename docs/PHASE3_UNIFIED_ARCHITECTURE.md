# PHASE 3: CYNIC Unified Architecture Design

**Document Status:** Final Design Specification
**Date:** 2026-02-27
**Vision:** CYNIC as Sovereignty Amplifier
**Architecture Paradigm:** Value Creation → Governance Emergence

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Core Architecture](#core-architecture)
3. [Sovereignty Layer](#sovereignty-layer)
4. [Emergence Layer](#emergence-layer)
5. [Coordination Layer](#coordination-layer)
6. [Complete Data Model](#complete-data-model)
7. [Use Cases](#use-cases)
8. [Interface Specification](#interface-specification)
9. [Governance Emergence Rules](#governance-emergence-rules)
10. [Distributed Architecture (Model B)](#distributed-architecture)
11. [4-Role Organism Implementation](#4-role-organism)
12. [Implementation Roadmap](#implementation-roadmap)
13. [Non-Negotiable Axioms](#non-negotiable-axioms)

---

## Executive Summary

### The Vision

**CYNIC is a sovereignty amplifier for individual value creation within collective benefit.**

Not a governance engine. Not a judgment system. A system that:

1. **Measures** what each individual creates (direct, indirect, collective impact)
2. **Makes visible** how value flows through the system
3. **Amplifies** individual agency through transparency and coordination
4. **Emerges governance** from value patterns (not decree)
5. **Constrains emergence** with non-negotiable axioms

### The Three Layers

```
┌─────────────────────────────────────────────────────────────┐
│  COORDINATION LAYER (Multi-creator Orchestration)           │
│  └─ Collective benefit > Individual amplification            │
│     └─ Reciprocal duty: Power = Responsibility              │
├─────────────────────────────────────────────────────────────┤
│  EMERGENCE LAYER (Governance from Value Patterns)           │
│  └─ Decisions weighted by impact (constrained by AXIOMS)    │
│     └─ Minority protection, decision-type specificity       │
│     └─ Temporal decay: Impact fades as conditions change    │
├─────────────────────────────────────────────────────────────┤
│  SOVEREIGNTY LAYER (Individual Value Creation)              │
│  └─ Each human creates independently                        │
│  └─ Full transparency on value flows                        │
│  └─ Complete control over amplification opt-in              │
└─────────────────────────────────────────────────────────────┘
```

### The Transformation from Phase 2

**Phase 2 (Current):**
- Judgment engine: 11 Dogs + PBFT consensus on proposals
- Binary decisions: APPROVE/REJECT
- Q-Learning loop: Feedback on judgment quality

**Phase 3 (New):**
- Sovereignty amplifier: Value impact measurement + governance emergence
- Multi-dimensional decisions: Execute/Defer/Modify/Learn
- Impact learning loop: Feedback on value creation quality
- **New:** Individual value chains visible to all
- **New:** Governance weights emerge from value patterns
- **New:** Axioms enforce non-negotiability

---

## Core Architecture

### Five Core Modules (HOWL Grade - Proven)

```python
┌──────────────────────────────────────────────────────────────┐
│ MODULE B: UnifiedState (FIDELITY✓ PHI✓ VERIFY✓ CULTURE✓ BURN✓)│
│ Score: 93/100 (HOWL)                                         │
│ Purpose: Immutable state contracts with φ-bounded confidence │
│                                                               │
│ Components:                                                  │
│  - UnifiedJudgment (verdict + confidence ∈ [0, 0.618])      │
│  - UnifiedLearningOutcome (feedback → Q-Table updates)      │
│  - UnifiedConsciousState (organism awareness + planning)    │
│  - ValueImpactState (NEW: 4-dimension impact tracking)      │
│                                                               │
│ Status: ✅ In master, 100+ tests, production-ready           │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ MODULE D: Orchestrator (FIDELITY✓ PHI✓ VERIFY✓ CULTURE✓ BURN✓)│
│ Score: 84/100 (HOWL)                                         │
│ Purpose: 7-step cycle orchestration (PERCEIVE→JUDGE→...→ACT)│
│                                                               │
│ Current: 877 LOC judgment loop                               │
│ Phase 3: Extend to value measurement + emergence trigger     │
│                                                               │
│ 7-Step Cycle:                                                │
│  1. PERCEIVE: Sense world state, value creation events      │
│  2. JUDGE: Run 11 Dogs on implications + value impact       │
│  3. DECIDE: Aggregate verdicts (PBFT consensus)             │
│  4. ACT: Execute decision, update value ledger              │
│  5. LEARN: Update Q-Table with satisfaction feedback        │
│  6. ACCOUNT: Record impact metrics for emergence            │
│  7. EMERGE: Check if governance weights should shift        │
│                                                               │
│ Status: ✅ In master, working, ready to extend              │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ MODULE G: Dialogue (FIDELITY✓ PHI✓ VERIFY✓ CULTURE✓ BURN✓)    │
│ Score: 81/100 (HOWL)                                         │
│ Purpose: Interactive conversation with CYNIC + humans       │
│                                                               │
│ Current: TALK mode, human-in-loop planning                  │
│ Phase 3: /create, /contribute, /discover, /coordinate        │
│                                                               │
│ Status: ✅ In master, 200+ tests, user-facing interface     │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ MODULE J: Observability (FIDELITY✓ PHI✓ VERIFY✓ CULTURE✓ BURN✓)│
│ Score: 86/100 (HOWL)                                         │
│ Purpose: Symbiotic state tracking (human+machine+CYNIC)      │
│                                                               │
│ Current: HumanStateTracker, MachineMonitor, ConsciousState  │
│ Phase 3: ValueImpactTracker (direct, indirect, collective)  │
│                                                               │
│ Status: ✅ In master, 108 tests, production-ready            │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ MODULE C: Events (FIDELITY✓ PHI~ VERIFY✓ CULTURE✓ BURN✓)      │
│ Score: 71/100 (WAG - Good)                                   │
│ Purpose: Event sourcing + genealogy for loop prevention      │
│                                                               │
│ Current: 3-bus pub-sub system with genealogy                │
│ Phase 3: Extend to value impact events                       │
│                                                               │
│ Status: ✅ In master, integrated, proven                     │
└──────────────────────────────────────────────────────────────┘
```

### Three Modules to Evolve (GROWL Grade - Needs Work)

```python
┌──────────────────────────────────────────────────────────────┐
│ MODULE E: API (Score: 48/100 - GROWL)                        │
│ Problem: AppContainer god object (649 LOC) + unclear routes  │
│ Phase 3 Fix:                                                 │
│  - DI pattern: Replace singleton with dependency injection   │
│  - New routes: /create, /contribute, /discover, /coordinate │
│  - Remove: /judge (move to dialogue instead)                │
│  - Add: /impact, /claim, /propose-change                    │
│ Timeline: 4 hours refactoring                               │
│ Score target: 65/100 (WAG)                                   │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ MODULE F: Organism (Score: 51/100 - GROWL)                   │
│ Problem: 10 layers, unclear responsibility distribution      │
│ Phase 3 Fix:                                                 │
│  - Flatten to 4 roles (Consciousness, Manager, Identity, Int)│
│  - Add Manager agency: should_execute(decision), propose()   │
│  - Add Integration visibility: OrganismCoordinator           │
│  - Reduce LOC from 3,950 to ~2,200                          │
│ Timeline: 6 hours refactoring + tests                        │
│ Score target: 72/100 (WAG)                                   │
└──────────────────────────────────────────────────────────────┘
```

### Two Modules to Delete (BARK Grade - Dead Code)

```python
┌──────────────────────────────────────────────────────────────┐
│ DELETE: MODULE A - LNSP (Score: 17/100 - BARK)               │
│ Reason: Never deployed, zero imports, duplicates nervous     │
│ Impact: -3,275 LOC, -cognitive overhead                      │
│ Replacement: Nervous.EventJournal (already in master)        │
│                                                               │
│ Action: git rm -r cynic/protocol/lnsp/                       │
│ Status: Ready to delete                                       │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ DELETE: MODULE H - Training (Score: 17/100 - BARK)           │
│ Reason: Phase 1B complete, Claude API now used               │
│ Impact: -2,250 LOC, -torch/bitsandbytes dependencies         │
│ Replacement: Claude API integration (in master)              │
│                                                               │
│ Action: git mv cynic/training/ archive/training_phase1b/     │
│ Status: Ready to delete                                      │
└──────────────────────────────────────────────────────────────┘
```

### One Module to Separate (Research vs Production)

```python
┌──────────────────────────────────────────────────────────────┐
│ SEPARATE: MODULE I - Cognition (Score: 67/100 - WAG)         │
│ Issue: Mixing research (exploratory) + production (deployed) │
│ Solution:                                                    │
│  - cynic/cognition/production/ (core, tested, deployed)     │
│  - research/cognition_experiments/ (exploratory, untested)  │
│ Impact: Clarity on stability guarantees                      │
│                                                               │
│ Status: Refactor in Phase 3b (optional)                      │
└──────────────────────────────────────────────────────────────┘
```

### Architecture Diagram

```
                   ┌─────────────────────────────────┐
                   │   COORDINATION LAYER            │
                   │  (Reciprocal Responsibility)    │
                   │   - Multi-creator Orchestration │
                   │   - Collective Decision Weights │
                   └──────────────┬──────────────────┘
                                  │
                   ┌──────────────┴──────────────────┐
                   │   EMERGENCE LAYER               │
                   │   (Governance from Value)       │
                   │   - Value Impact Measurement    │
                   │   - Weight Emergence Algorithm  │
                   │   - Axiom Constraint Checking   │
                   └──────────────┬──────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
   ┌─────────┐            ┌─────────────┐          ┌──────────┐
   │ MODULE  │            │  ORCHESTR.  │          │ MODULE   │
   │  B      │            │  (D)        │          │  G       │
   │ Unified │◄──────────►│ 7-Step      │◄────────►│Dialogue  │
   │ State   │            │ Cycle       │          │          │
   └─────────┘            └─────────────┘          └──────────┘
        │                         │                         │
        │   ┌───────────────┬────┬────────────────┐        │
        │   │               │    │                │        │
        ▼   ▼               ▼    ▼                ▼        ▼
   ┌──────────────────────────────────────────────────────────┐
   │  CYNIC CORE (B+C+D+G+J) - Proven Modules                │
   │  ✅ UnifiedState (B) - Immutable contracts               │
   │  ✅ Events (C) - Genealogy + sourcing                   │
   │  ✅ Orchestrator (D) - 7-step cycle                     │
   │  ✅ Dialogue (G) - Human interaction                    │
   │  ✅ Observability (J) - Symbiotic tracking              │
   └──────────────────────────────────────────────────────────┘
        │                    │
        ├─ Refactor API (E) ──┤
        │                    │
        ├─ Evolve Org (F) ───┤
        │                    │
        └─ Delete A, H ──────┘
```

---

## Sovereignty Layer

### Definition

The **Sovereignty Layer** is where individual humans create value independently, transparently, and under their own control.

### Three Dimensions of Sovereignty

#### 1. Self-Sovereignty
- **Self-Awareness:** What am I creating? What impact do I have?
- **Agency:** Can I choose how to participate in collective decisions?
- **Control:** Can I opt in/out of amplification mechanisms?

#### 2. Machine-Sovereignty
- **Transparency:** Can I see all algorithms affecting my value?
- **Customization:** Can I configure judgment weights for my context?
- **Human-in-Loop:** Can I intervene before automated decisions execute?

#### 3. Material-Sovereignty
- **Resource Visibility:** Can I see where my resources flow?
- **Reciprocity:** Do I get back proportional value for what I contribute?
- **Control:** Can I revoke or modify value allocations?

### Individual Value Creation Interface

```
Human Value Creator
        │
        ├─ Creates work/idea/service
        │
        ├─ /create
        │  ├─ description
        │  ├─ impact_type (product, service, knowledge, governance)
        │  ├─ audience (private, community, public)
        │  └─ tracking_enabled (true/false)
        │
        ├─ /contribute
        │  ├─ to_creation_id
        │  ├─ contribution_type (enhancement, criticism, support)
        │  ├─ effort_description
        │  └─ expected_value_share
        │
        ├─ /discover
        │  ├─ search_query
        │  ├─ filter_by_impact (direct, indirect, collective)
        │  ├─ filter_by_audience
        │  └─ sort_by (recency, impact, community_rating)
        │
        └─ /impact
           ├─ time_range
           ├─ dimension (direct, indirect, collective, temporal)
           └─ visibility (private, community, public)
```

### Value Measurement (4 Dimensions)

#### Direct Impact
**Definition:** Value created by my own direct action
```
Examples:
  - I write code → the product works better (direct impact = code quality)
  - I mentor someone → they complete 3 projects (direct impact = 3×project_value)
  - I create governance proposal → community adopts it (direct impact = adoption_velocity)

Measurement:
  - artifact_quality_score (0-100)
  - artifact_adoption_rate (0-1)
  - artifact_longevity (days before deprecated)
  - artifact_utility_index (community_ratings × usage_count)
```

#### Indirect Impact
**Definition:** Value created through my contribution to others' creations
```
Examples:
  - I review code → 10 bugs prevented → downstream value
  - I give feedback → creator improves offering → more customers
  - I vote YES on governance → proposal succeeds → community benefit

Measurement:
  - contributor_value_share (% of creation_value I helped enable)
  - multiplier_factor (how much better did the creation become?)
  - beneficiary_count (how many others benefited?)
```

#### Collective Impact
**Definition:** Value created by collective action I participated in
```
Examples:
  - I vote + 49 others vote → collective decision improves community
  - I propose governance change + community implements → systemic improvement
  - I join working group → group creates 5 artifacts × 50 value each

Measurement:
  - participation_factor (am I core or peripheral?)
  - collective_outcome_value (total value created by group)
  - my_influence_weight (how much did I influence outcome?)
```

#### Temporal Impact
**Definition:** Value created that compounds, decays, or transforms over time
```
Examples:
  - I create a library → used for 2 years (high temporal impact)
  - I make a decision → works for 6 months, then becomes obsolete (medium decay)
  - I build a system → enables 10× future creations (compounding impact)

Measurement:
  - creation_lifespan (days_actively_used)
  - decay_rate (how quickly usefulness decreases)
  - compounding_factor (how many future creations does it enable?)
  - impact_trajectory (increasing, stable, decreasing)
```

### Sovereignty Guarantees

**1. Transparency Guarantee**
```
Every human can see:
  - Direct path: "Where did my value go?"
  - Indirect path: "What was I involved in?"
  - Collective paths: "What did we create together?"
  - Third-party use: "Who is using my creation + getting value?"
```

**2. Control Guarantee**
```
Every human can choose:
  - Visibility: private (only me) | community (with peers) | public (world)
  - Amplification: participate (normal) | boost (higher weight) | silent (no governance)
  - Contribution: autonomous (system decides) | manual (I approve each step)
  - Attribution: public (my name) | anonymous (no attribution) | pseudonym
```

**3. Agency Guarantee**
```
Every human retains:
  - Decision right: I can refuse to follow a governance decision I disagree with
  - Opt-out right: I can withdraw from the system (keep my value history)
  - Modification right: I can contest an impact measurement + propose correction
  - Appeal right: I can challenge an axiom constraint as unjust
```

---

## Emergence Layer

### How Governance Emerges from Value

**NOT:** "Top-down governance derived from CYNIC logic"
**YES:** "Governance weights dynamically derived from who creates what value"

### Governance Emergence Algorithm

```python
def compute_governance_weight(human_id, decision_type, time_period):
    """
    Weight = how much influence this human has on decisions of decision_type

    Bases on:
      1. Value created (dimension-specific)
      2. Axiom constraints (floor: never 0 for minority)
      3. Decision-type specificity (domain expertise)
      4. Temporal decay (impact fades as conditions change)
      5. Reciprocal duty (power = responsibility)
    """

    # Phase 1: Raw impact
    direct_impact = measure_direct_value(human_id, decision_type, time_period)
    indirect_impact = measure_indirect_value(human_id, decision_type, time_period)
    collective_impact = measure_collective_value(human_id, decision_type, time_period)

    # Phase 2: Combine with domain weighting
    domain_weight = 1.0
    if is_expert_in_domain(human_id, decision_type):
        domain_weight = 1.2  # Slight boost for domain expertise

    raw_weight = (
        direct_impact * 0.4 +
        indirect_impact * 0.35 +
        collective_impact * 0.25
    ) * domain_weight

    # Phase 3: Apply axiom constraints
    min_weight = axiom_minority_floor(decision_type)  # Usually 0.1 = 10%
    max_weight = axiom_expert_cap(decision_type)      # Usually 0.5 = 50%

    constrained_weight = max(min_weight, min(raw_weight, max_weight))

    # Phase 4: Apply temporal decay
    time_since_creation = days_since(human_id, decision_type)
    decay_rate = axiom_temporal_decay(decision_type)  # Usually 0.5% per week

    decayed_weight = constrained_weight * decay_factor(decay_rate, time_since_creation)

    # Phase 5: Check reciprocal duty
    power_level = decayed_weight * 100  # Convert to percentage
    if power_level > 40:
        # If you have high power, you have high responsibility
        # Must have spent at least X hours on governance
        hours_spent = measure_governance_hours(human_id, decision_type)
        if hours_spent < required_hours_for_power(power_level):
            # Reduce weight until you "earn" the power
            decayed_weight *= min(1.0, hours_spent / required_hours_for_power(power_level))

    return decayed_weight

def make_governance_decision(decision, votes):
    """
    Final verdict weighted by governance weights (not 1-person-1-vote)
    """

    total_weight = 0
    weighted_yes = 0
    weighted_no = 0
    weighted_abstain = 0

    for voter_id, vote in votes.items():
        weight = compute_governance_weight(voter_id, decision.type, period="last_90_days")
        total_weight += weight

        if vote == "YES":
            weighted_yes += weight
        elif vote == "NO":
            weighted_no += weight
        else:  # ABSTAIN
            weighted_abstain += weight

    # Normalize
    yes_pct = weighted_yes / total_weight if total_weight > 0 else 0
    no_pct = weighted_no / total_weight if total_weight > 0 else 0

    # VERDICT: requires > 0.618 (φ⁻¹) threshold
    if yes_pct > 0.618:
        return ("APPROVED", yes_pct, axiom_check_passed)
    elif no_pct > 0.618:
        return ("REJECTED", no_pct, axiom_check_passed)
    else:
        return ("DEFERRED_FOR_MORE_DATA", yes_pct, False)
```

### Governance Emergence Constraints (Non-Negotiable Axioms)

#### Axiom 1: Minority Protection
```
No single human can have > 50% weight on any decision
Rationale: Prevents tyranny of the majority
Implementation: If any human exceeds 50%, normalize proportionally
```

#### Axiom 2: Domain Expertise Weighting
```
Governance weight MUST be decision-type specific
- Product decisions: weighted by product creation value
- Security decisions: weighted by security expertise + contributions
- Financial decisions: weighted by financial stewardship value
Rationale: Experts have more reliable judgment in their domain
```

#### Axiom 3: Temporal Decay
```
Governance weight fades as conditions change
- Default decay: 0.5% per week (impact from 6 months ago = 74% weight)
- Accelerated decay: For high-risk domains (48% per year)
Rationale: Stale impact becomes stale governance
```

#### Axiom 4: Reciprocal Duty
```
Power = Responsibility
- If you have > 40% weight: you must spend > X hours/month on governance
- If you have > 70% weight: you must mentor 2+ junior creators
Rationale: Prevents absentee power
```

#### Axiom 5: Minority Voice
```
Even if you have 1% value creation, your voice CANNOT be 0%
Minimum floor: 1% weight on any decision
Rationale: Protects smallest contributors from irrelevance
```

#### Axiom 6: Threshold Consensus
```
Decisions require > φ⁻¹ = 61.8% weighted consensus
Not 50%, not 75%, exactly φ-derived
Rationale: Maintains φ-bounded skepticism
```

#### Axiom 7: Reversibility
```
Any governance decision can be reversed if:
  - 80%+ weighted consensus votes to reverse
  - Within 90 days of original decision
  - Due to new evidence of harm
Rationale: Prevents irreversible lock-in
```

---

## Coordination Layer

### Multi-Creator Orchestration

The **Coordination Layer** enables multiple sovereign individuals to work together while maintaining:
1. Individual value attribution
2. Transparent resource flows
3. Reciprocal benefit (what they put in ≥ what they get out)

### Coordination Patterns

#### Pattern 1: Value Chain (Sequential)
```
Creator A creates artifact → Creator B enhances → Creator C distributes
     ↓                            ↓                         ↓
Measures direct impact      Measures indirect impact   Measures collective impact

Value attribution:
  - Artifact value = 100 points
  - Creator A gets 40% (original creation)
  - Creator B gets 35% (significant enhancement)
  - Creator C gets 25% (distribution/marketing)

All visible in the ledger; all proportional to contribution
```

#### Pattern 2: Working Group (Collective)
```
5 creators + CYNIC form working group to build "Feature X"

Group decision: "We all benefit from success equally"
  → Egalitarian split: each member gets 20%

Alternative: "Person A is lead, B+C are core, D+E are supporters"
  → Weighted split: A=30%, B=25%, C=25%, D=10%, E=10%

All visible in voting for "Feature X" governance
```

#### Pattern 3: Continuous Contribution (Time-Based)
```
Maintainer A runs service for community
  - Initial artifact value: 100 points
  - Monthly maintenance: 10 points
  - Year 1 maintainer: gets 60% (100 initial + 120 maintenance)
  - Other contributors: get 40%

After 2 years: roles might flip (original creator: 30%, maintainer: 70%)
```

### Coordination Guarantees

**1. Transparency Guarantee**
```
For any artifact, show:
  - Who contributed what
  - When they contributed
  - How much value they're receiving
  - How the value split was decided
```

**2. Reciprocity Guarantee**
```
No one can take > they put in (accounting for time/risk)
  - If you contribute 10% effort, you get ~10% value (±5%)
  - If you take high risk, you get higher potential return
  - System monitors: value_received / (effort_exerted + risk_taken)
```

**3. Exit Guarantee**
```
Any creator can leave at any time:
  - Keep attribution for past contributions
  - Receive vested value
  - Proposal automatically continues with remaining creators
```

---

## Complete Data Model

### Core State Objects

```python
# Immutable value creation tracking
@dataclass(frozen=True)
class ValueCreation:
    creation_id: str                           # UUID
    creator_id: str                            # Creator's identity
    creation_type: Literal["product", "service", "knowledge", "governance"]
    description: str                           # What was created
    creation_timestamp: float                  # When
    audience: Literal["private", "community", "public"]  # Who can see

    # Impact measurement (computed)
    direct_impact_score: float                 # 0-100
    artifact_quality: float                    # 0-100
    adoption_rate: float                       # 0-1
    longevity_days: int                        # Days actively used
    utility_index: float                       # Derived metric

    # Contribution graph
    contributors: Dict[str, Contribution]      # Who helped
    dependencies: List[str]                    # What it depends on
    dependents: List[str]                      # What depends on it

@dataclass(frozen=True)
class Contribution:
    contributor_id: str
    contribution_type: Literal["enhancement", "criticism", "support", "maintenance"]
    effort_description: str
    value_share_pct: float                     # % of creation value
    timestamp: float
    feedback_quality_score: float              # How useful was this?

# Impact measurement
@dataclass(frozen=True)
class ImpactMeasurement:
    measurement_id: str
    human_id: str
    decision_type: str                         # e.g., "security", "feature", "governance"
    time_period: str                           # e.g., "last_90_days"

    # 4 dimensions
    direct_impact: float                       # Own creations
    indirect_impact: float                     # Contributions to others
    collective_impact: float                   # Group decision impact
    temporal_impact: float                     # Compounding/decay over time

    # Governance weight (derived)
    raw_weight: float                          # Before constraints
    constrained_weight: float                  # After axiom constraints
    decayed_weight: float                      # After temporal decay
    final_weight: float                        # After reciprocal duty check

    timestamp: float

# Governance emergence tracking
@dataclass(frozen=True)
class GovernanceDecision:
    decision_id: str
    decision_type: str                         # e.g., "feature_approval", "spending"
    description: str
    votes: Dict[str, Literal["YES", "NO", "ABSTAIN"]]

    # Weighted results
    weighted_yes_pct: float
    weighted_no_pct: float
    verdict: Literal["APPROVED", "REJECTED", "DEFERRED"]

    # Axiom compliance
    axioms_checked: Dict[str, bool]           # Each axiom constraint passed?
    minority_voice_enforced: bool
    temporal_decay_applied: bool
    reciprocal_duty_enforced: bool

    # Execution
    execution_status: Literal["pending", "in_progress", "completed", "failed", "reversed"]
    timestamp: float
    reversible_until: float                    # When it can no longer be reversed

# Learning outcome with satisfaction
@dataclass(frozen=True)
class LearningOutcome:
    decision_id: str
    actual_impact: float                       # Real-world result
    predicted_impact: float                    # What CYNIC predicted
    satisfaction_rating: int                   # 1-5 from community
    feedback_text: str                         # Why this rating

    # Q-Table update
    q_value_delta: float                       # How much to update Q-Table
    confidence_delta: float                    # How much to adjust confidence
    timestamp: float
```

### Data Model Diagram

```
ValueCreation (artifact)
    │
    ├─ creator_id ──────────→ Creator metadata
    │
    ├─ contributors ────────→ Contribution[]
    │
    ├─ dependencies ────────→ ValueCreation[] (upstream)
    │
    ├─ dependents ──────────→ ValueCreation[] (downstream)
    │
    └─ impact metrics ──────→ ImpactMeasurement
                                │
                                ├─ direct_impact
                                ├─ indirect_impact
                                ├─ collective_impact
                                └─ temporal_impact
                                      │
                                      └─ governance_weight
                                            │
                                            └─ GovernanceDecision[]
                                                  │
                                                  └─ LearningOutcome

Creator
    │
    ├─ creations[] ────────→ ValueCreation[]
    │
    ├─ contributions[] ────→ Contribution[]
    │
    ├─ impact_profile ────→ ImpactMeasurement[]
    │
    ├─ governance_weight ──→ Dict[decision_type, float]
    │
    └─ q_table_entries ───→ QEntry[] (learning history)
```

---

## Use Cases

### Use Case 1: Individual Creator Journey

**Actor:** Sarah (product creator)
**Goal:** Create a feature, measure impact, gain governance influence

```
Day 1: Sarah /create
  "I'm building an authentication library for community"
  ├─ creation_type: "product"
  ├─ description: "Secure auth lib + SDK + 3 examples"
  ├─ audience: "community"
  └─ tracking_enabled: true

  System creates ValueCreation(creation_id="lib-auth-001")
  Initial impact_score: 0 (no one using yet)

Days 2-7: Sarah develops
  /contribute events:
    Day 2: Sarah /contribute (to herself)
      enhancement: "Core implementation + tests"
      effort: 40 hours
      expected_share: 100% (it's her work)

    Day 5: Bob /contribute
      enhancement: "Added 3 example projects"
      effort: 10 hours
      expected_share: 15% (Sarah negotiates to 10%)

    Day 7: Alice /contribute
      support: "Security review + audit report"
      effort: 5 hours
      expected_share: 5%

Day 14: Library published
  /impact check:
    direct_impact_score: 72/100 (quality review passed)
    adoption_rate: 0.23 (23% of target audience adopted)
    utility_index: 18.5 (high community ratings)

  Sarah's direct_impact for "authentication" decisions: +18.5 points

Day 30: First governance decision
  Decision: "Should we add blockchain auth support?"

  Sarah's governance weight:
    raw_weight = 18.5 / 100 = 18.5%
    domain_weight = 1.2 (expert in auth)
    constrained_weight = 18.5% (within axiom bounds)
    decayed_weight = 18.5% (only 1 month, no decay)
    final_weight = 18.5% (not high power, no reciprocal duty check)

  Sarah votes YES with 18.5% weight
  → Her vote is worth ~18.5x more than a non-creator
  → But still constrained by axiom minority floor (she can't control decision alone)

Month 2: Learning feedback
  Implementation completed
  Community satisfaction: 4.8/5 stars
  /feedback event:
    actual_impact > predicted_impact
    → Q-Table: increase confidence in similar decisions
    → Sarah's governance weight slowly increases

Month 6: Sarah's impact evolved
  /impact (last_6_months):
    direct_impact: 42.5 (library actively used, features added)
    indirect_impact: 15.3 (Alice built 2 products on top)
    collective_impact: 8.2 (governance decisions shaped library direction)
    temporal_impact: 21.1 (compounding as more projects depend on it)

  Total impact_score: 87.1/100
  Sarah's auth decision weight: now 35% (if she spent enough governance hours)
```

### Use Case 2: Governance Emergence

**Actor:** CYNIC system
**Goal:** Shift governance weights based on changing value creation

```
SCENARIO: Security decision emerges

Initial state:
  - Alice created security framework (value: 50)
  - Bob created auth library (value: 45)
  - Carol created API wrapper (value: 30)
  - Others: 25 total

Decision: "Should we mandate TLS 1.3 for all APIs?"

Weight computation for EACH voter:
  Alice: 50/150 = 33% raw, 1.1 expert boost = 36% final
  Bob: 45/150 = 30% raw, 1.1 expert boost = 33% final
  Carol: 30/150 = 20% raw, 1.0 (not expert) = 20% final
  Others: 25/150 = 17% raw, distributed among 10 people = 1.7% each

  Total weights: 36% + 33% + 20% + 10×1.7% = 101% (normalized to 100%)

Vote:
  Alice: YES (36%)
  Bob: YES (33%)
  Carol: NO (20%)
  Others: mixed (11% split 5-6)

Result:
  Weighted YES: 36% + 33% + 5.5% = 74.5%
  Weighted NO: 20% + 5.5% = 25.5%

  VERDICT: APPROVED (74.5% > 61.8% threshold)

Post-decision:
  → Implementation begins
  → Community measures impact over 3 months
  → If impact was positive: Alice/Bob's governance weight increases
  → If impact was negative: weight decreases (better next time)
```

### Use Case 3: Reciprocal Duty Enforcement

**Actor:** CYNIC system
**Goal:** Ensure high-power individuals pay governance tax

```
Initial state:
  David has created many valuable products
  His impact_score: 87/100
  His governance weight: 52% on important decisions

Alert: David has high power, must check reciprocal duty

Query: How many hours has David spent on governance in last month?
  Answer: 2 hours (attended 1 council meeting)

Required hours for 52% weight:
  formula: hours = power_level * 10 = 52 * 10 = 520 hours/year
           monthly: 520 / 12 = 43 hours/month

David's actual: 2 hours/month
Required for 52%: 43 hours/month
Ratio: 2/43 = 4.7%

David's weight is now adjusted:
  original_weight: 52%
  adjusted_weight: 52% * 4.7% = 2.4%

Message to David:
  "Your value creation gives you potential 52% governance weight,
   but you've only spent 2/43 required hours on governance this month.
   Your actual weight is 2.4% until you invest more time in stewardship."

David's options:
  1. Spend more hours on governance (increase weight toward 52%)
  2. Accept lower weight (4.7%)
  3. Create delegation (ask trusted advisor to use his hours)
  4. Request temporary power reduction (opt-out of high-impact decisions)
```

---

## Interface Specification

### REST API Endpoints

#### Sovereignty Endpoints

```
POST /api/v1/create
  Create a new value artifact

  Request:
    {
      "creation_type": "product" | "service" | "knowledge" | "governance",
      "description": string,
      "audience": "private" | "community" | "public",
      "tracking_enabled": boolean
    }

  Response:
    {
      "creation_id": "lib-auth-001",
      "created_at": "2026-02-27T10:30:00Z",
      "direct_impact_score": 0,
      "governance_weight": {}
    }

POST /api/v1/contribute
  Contribute to someone else's creation

  Request:
    {
      "creation_id": "lib-auth-001",
      "contribution_type": "enhancement" | "criticism" | "support" | "maintenance",
      "effort_description": string,
      "expected_value_share_pct": number (0-100)
    }

  Response:
    {
      "contribution_id": "contrib-001",
      "status": "pending_acceptance" | "accepted" | "rejected",
      "value_share_pct": number
    }

GET /api/v1/discover
  Discover valuable creations

  Query params:
    search: string
    filter_by_impact: "direct" | "indirect" | "collective" | "temporal"
    filter_by_audience: "private" | "community" | "public"
    sort_by: "recency" | "impact" | "community_rating"
    limit: number (default 20)

  Response:
    {
      "creations": [ValueCreation],
      "total": number,
      "page": number
    }

GET /api/v1/impact
  Check your impact in different dimensions

  Query params:
    time_range: "last_7_days" | "last_30_days" | "last_90_days" | "all_time"
    dimension: "direct" | "indirect" | "collective" | "temporal" | "all"

  Response:
    {
      "direct_impact": number,
      "indirect_impact": number,
      "collective_impact": number,
      "temporal_impact": number,
      "total_impact_score": number,
      "governance_weight": {
        "security": 0.35,
        "features": 0.22,
        "governance": 0.18
      }
    }

POST /api/v1/claim
  Claim value from your creations

  Request:
    {
      "creation_id": "lib-auth-001",
      "claim_amount": number (in system tokens)
    }

  Response:
    {
      "claim_id": "claim-001",
      "amount": number,
      "claimed_at": "2026-02-27T10:30:00Z",
      "status": "pending" | "approved" | "rejected"
    }

POST /api/v1/coordinate
  Propose a coordination with other creators

  Request:
    {
      "coordination_type": "working_group" | "value_chain" | "continuous",
      "participants": ["user1", "user2", "user3"],
      "description": string,
      "value_split_strategy": "egalitarian" | "weighted" | "negotiated"
    }

  Response:
    {
      "coordination_id": "coord-001",
      "status": "proposed",
      "participants": [{user_id, acceptance_status, value_pct}]
    }
```

#### Governance Endpoints

```
GET /api/v1/governance/decisions
  List governance decisions

  Query params:
    status: "pending" | "completed"
    decision_type: string
    limit: number

  Response:
    {
      "decisions": [GovernanceDecision],
      "total": number
    }

POST /api/v1/governance/vote
  Vote on a governance decision

  Request:
    {
      "decision_id": "decision-001",
      "vote": "YES" | "NO" | "ABSTAIN",
      "reasoning": string (optional)
    }

  Response:
    {
      "vote_recorded": true,
      "your_weight": 0.35,
      "current_tally": {
        "weighted_yes": 0.62,
        "weighted_no": 0.38
      }
    }

GET /api/v1/governance/weight
  Check your governance weight for a decision type

  Query params:
    decision_type: string
    time_period: "last_30_days" | "last_90_days"

  Response:
    {
      "decision_type": "security",
      "raw_weight": 0.45,
      "constrained_weight": 0.42,
      "decayed_weight": 0.40,
      "final_weight": 0.38,
      "reasoning": {
        "direct_impact": 0.35,
        "domain_expert_boost": 1.1,
        "axiom_constraints": "applied",
        "temporal_decay": "minimal",
        "reciprocal_duty": "checked_met"
      }
    }

GET /api/v1/governance/impact
  Check how your value creation affects governance

  Query params:
    creation_id: string
    impact_on_decisions: "historical" | "predicted"

  Response:
    {
      "creation": ValueCreation,
      "governance_domains": [
        {
          "domain": "security",
          "weight_contribution": 0.12,
          "decisions_influenced": ["decision-001", "decision-003"]
        }
      ]
    }
```

#### Observability Endpoints

```
GET /api/v1/observe/state
  Get complete system state (symbiotic observability)

  Response:
    {
      "human_layer": {
        "energy_level": 0-100,
        "focus_areas": ["security", "features"],
        "recent_feedback": [...]
      },
      "machine_layer": {
        "cpu_usage": 0-100,
        "memory_usage": 0-100,
        "network_latency": number
      },
      "cynic_layer": {
        "conscious_state": "observing" | "deciding" | "learning",
        "confidence_level": 0-0.618,
        "thinking_process": [...]
      },
      "coordination_layer": {
        "active_collaborations": number,
        "value_flows": [...]
      }
    }

GET /api/v1/observe/value-flow
  Visualize value flows in the system

  Query params:
    from_human: string (optional)
    to_human: string (optional)
    time_period: string

  Response:
    {
      "flows": [
        {
          "from": "alice",
          "to": "bob",
          "value_amount": 45,
          "via_creation": "lib-auth-001",
          "timestamp": "2026-02-27T10:30:00Z"
        }
      ],
      "total_value_circulating": number
    }
```

### CLI Dialogue Interface

```
$ cynic

CYNIC> /create
Creating a new value artifact...

What are you creating?
  [1] Product (software, tool, artifact)
  [2] Service (support, maintenance, consulting)
  [3] Knowledge (guide, analysis, documentation)
  [4] Governance (policy, proposal, decision)
> 1

Brief description of your product:
> Secure authentication library for Python with SDK + 3 example projects

Who should see this?
  [1] Private (only you)
  [2] Community (members of CYNIC network)
  [3] Public (anyone on internet)
> 2

Should we track this creation's impact?
  [1] Yes, show me how it's used and who benefits
  [2] No, I prefer privacy
> 1

✅ Created lib-auth-001
   Impact tracking enabled
   Current governance weight: {} (waiting for adoption)

   Next: /contribute (others can help) or /impact (check your measurements)

---

CYNIC> /impact
Checking your value creation metrics...

What time period?
  [1] Last 7 days
  [2] Last 30 days
  [3] Last 90 days
  [4] All time
> 3

What dimensions interest you?
  [1] Direct (what you created)
  [2] Indirect (what you helped others create)
  [3] Collective (group decisions you influenced)
  [4] Temporal (compounding/decay effects)
  [5] All dimensions
> 5

📊 Your Impact (Last 90 Days)
├─ Direct Impact: 42.5 / 100
│  └─ lib-auth-001: 42.5 (actively used, high quality)
├─ Indirect Impact: 15.3 / 100
│  └─ 2 projects depend on your library
├─ Collective Impact: 8.2 / 100
│  └─ Your votes influenced 3 governance decisions
└─ Temporal Impact: 21.1 / 100
   └─ Your library is compounding (more dependent projects each month)

Total Impact Score: 87.1 / 100 ✨

Your Governance Weights (based on this impact):
├─ Authentication decisions: 35% weight
├─ API design decisions: 28% weight
├─ Security decisions: 24% weight
└─ General governance: 12% weight

💡 You're an expert in authentication. Your governance weight reflects this.

---

CYNIC> /coordinate
Let's build something together...

Who do you want to coordinate with?
  Search for creators: > alice, bob, carol

Coordination type?
  [1] Working group (temporary project team)
  [2] Value chain (sequential: A→B→C)
  [3] Continuous contribution (maintenance + revenue share)
> 1

Project description:
> Build enterprise security framework combining auth + encryption + audit

Value split strategy?
  [1] Egalitarian (everyone gets 25%)
  [2] Weighted (custom %s based on role)
  [3] Negotiated (I propose, they accept/counter)
> 2

Weights for each participant:
  Alice (security expert): 30%
  Bob (auth library creator): 25%
  Carol (API designer): 25%
  You (coordinator): 20%

📋 Proposal created: coord-001
   Awaiting acceptance from: alice, bob, carol

   Once all accept, your governance weights on "security" decisions
   will include this group's collective impact!
```

---

## Governance Emergence Rules

### Rule 1: Value-Based Weighting
```
A human's governance weight = (their value creation) / (total value in system)
Subject to axiom constraints and domain specificity
```

### Rule 2: Decision-Type Specificity
```
Weights are NOT one-size-fits-all

Security decisions:
  - Weight = security_value_created / total_security_value
  - If you're an expert: boost weight 1.2x

Feature decisions:
  - Weight = feature_value_created / total_feature_value
  - If you're an expert: boost weight 1.2x

Governance decisions:
  - Weight = (governance_value + community_stewardship) / total
  - If you've spent time on governance: eligible for higher weight
```

### Rule 3: Temporal Decay
```
As conditions change, old impact becomes less relevant

Age of creation vs decay rate:
  0-30 days:   100% weight (fresh impact)
  30-60 days:  98.5% weight (decay = 0.5%/week)
  60-180 days: 74% weight (6 months old)
  6+ months:   ~10% weight (quite stale)

For critical infrastructure:
  Decay slower (0.2%/week) — mature systems stay relevant longer

For trendy features:
  Decay faster (2%/week) — fashionable features age quickly
```

### Rule 4: Axiom Enforcement
```
Before applying governance weight:

1. Check minority floor: weight >= 1% (if community member)
2. Check expert cap: weight <= 50% (no single expert dominates)
3. Check reciprocal duty: if weight > 40%, hours_spent >= hours_required
4. Check decision-type specificity: is this a domain you've contributed to?
5. Check reversibility: can this decision be undone if it causes harm?

If any axiom fails → reduce weight OR defer decision
```

### Rule 5: Consensus Threshold
```
A governance decision passes when weighted YES votes > 0.618 (φ⁻¹)

Why φ⁻¹ and not 0.5 or 0.75?

φ = 1.618... (golden ratio, harmonic proportion)
φ⁻¹ = 0.618... (reciprocal, epistemic humility)

φ⁻¹ = 61.8%
  - Higher than simple majority (50%) → prevents narrow coalitions
  - Lower than supermajority (75%) → allows progress with broad consensus
  - Derived from φ → mathematically harmonic

Max confidence is 0.618 because:
  - 61.8% confidence = 38.2% humility
  - We never trust our judgment completely
```

### Rule 6: Reversibility Window
```
Governance decisions can be reversed if:
  - Within 90 days of decision
  - New evidence of unintended harm
  - 80%+ weighted consensus votes to reverse
  - Due process for appeals documented

Irreversible decisions (rare):
  - Axiom constraints (never reversible, by definition)
  - Deletion of human identity (irreversible)
  - Permanent bans (only with 95%+ consensus + oversight)
```

### Rule 7: Reciprocal Duty
```
Power = Responsibility

If your governance weight is HIGH, you must:
  - Spend time on governance (hours = power_level × 10 hours/year)
  - Mentor newer creators in your domain
  - Participate in decision implementation (not just voting)
  - Document your reasoning (transparency)

If you fail reciprocal duty:
  - Weight is reduced proportionally
  - Until you "earn" your power back
  - No penalty, just alignment
```

---

## Distributed Architecture (Model B)

### Federated Learning Model

```
ARCHITECTURE OVERVIEW

┌─────────────────┐
│   Hub Node      │  (Aggregation point)
│  Q-Table Sync   │  - Collects Q-Tables from instances
│  Weight Sync    │  - Aggregates governance weights
└────────┬────────┘
         │
    ┌────┴────┬──────────┬──────────┐
    │          │          │          │
    ▼          ▼          ▼          ▼
┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
│Instance│ │Instance│ │Instance│ │Instance│
│  EU    │ │  US    │ │  APAC  │ │  Custom│
└────────┘ └────────┘ └────────┘ └────────┘
  │          │          │          │
  └──────────┼──────────┼──────────┘
             │
        Synchronize:
        - Q-Table values
        - Governance weights
        - Learning outcomes
        (Every 24 hours)
```

### Four Phases of Rollout

#### Phase 1: Single Instance (Current)
```
Timeline: Weeks 1-4
Status: Master branch + MVP
Architecture:
  - CYNIC core (B+C+D+G+J) running in single process
  - Judgment engine operational
  - Q-Learning feedback loop working
  - Discord bot operational

Output:
  - Proof that core architecture works
  - Learning loop producing value
  - Human-CYNIC collaboration validated
```

#### Phase 2: Federated Sync (4-6 weeks)
```
Timeline: Weeks 5-10
Status: Add federation layer
Architecture:
  - Deploy 2-3 CYNIC instances (different regions)
  - Each runs independently (local Q-Table)
  - Hub syncs Q-Tables every 24 hours
  - Governance weights stay local (region-specific)

Benefit:
  - Local latency improvement
  - Regional autonomy
  - Learning shared globally

Implementation:
  - Add SyncAgent (hub-side): aggregates Q-Tables
  - Add FederationClient (instance-side): uploads Q-Table
  - Conflict resolution: average Q-values across instances
  - Version control: timestamp each Q-Table
```

#### Phase 3: Governance Sync (6-12 weeks)
```
Timeline: Weeks 11-24
Status: Add governance weight coordination
Architecture:
  - Instances compute weights locally
  - Hub aggregates weights across regions
  - Community votes count across all instances
  - Impact measurement shared globally

Benefit:
  - Consistent governance across instances
  - Value creation visible everywhere
  - Decisions made collaboratively

Implementation:
  - GovernanceRegistry (hub): tracks all weight changes
  - WeightPublisher (instance): announces local weights
  - VoteAggregator: computes final verdict across regions
```

#### Phase 4: Ecosystem Scale (12+ weeks)
```
Timeline: Weeks 25+
Status: Open federation for external partners
Architecture:
  - Public CYNIC instances join federation
  - Community-run instances sync with hub
  - NEAR blockchain integration (settlements)
  - Cross-instance value attribution

Benefit:
  - Network effect: every new creator adds value
  - Decentralized governance emerges
  - Cryptographic trust (NEAR contracts)
```

---

## 4-Role Organism Implementation

### Architecture Overview

The organism has four interdependent roles that must coordinate seamlessly:

```
CYNIC ORGANISM (4-ROLE ARCHITECTURE)

┌─────────────────────────────────────────────────────────┐
│  Layer 0: CONSCIOUSNESS (Self-Awareness)                │
│  ├─ What do I observe? (input awareness)                │
│  ├─ What do I know about myself? (self-model)           │
│  ├─ What am I planning? (intention model)               │
│  ├─ How confident am I? (φ-bounded: max 0.618)          │
│  └─ Implementation: cynic/organism/consciousness.py     │
│     Output: ConsciousState (immutable dataclass)        │
├─────────────────────────────────────────────────────────┤
│  Layer 1: MANAGER (Orchestration)  ← IMPROVEMENTS       │
│  ├─ Should I execute this decision? (veto power)        │
│  ├─ What level should execution be? (risk management)   │
│  ├─ Who should I propose to? (coordination)             │
│  └─ Implementation: cynic/organism/manager.py           │
│     NEW: should_execute(decision), propose_level(risk)  │
│     Output: ManagerDirective (mutable recommendation)   │
├─────────────────────────────────────────────────────────┤
│  Layer 2: IDENTITY (Values & Constraints)               │
│  ├─ What do I value? (principles)                       │
│  ├─ What are my non-negotiables? (axioms)               │
│  ├─ What am I willing to trade? (preferences)           │
│  └─ Implementation: cynic/organism/identity.py          │
│     Output: IdentityConstraint (frozen values)          │
├─────────────────────────────────────────────────────────┤
│  Layer 3: INTEGRATION (Coordination)  ← IMPROVEMENTS    │
│  ├─ Who else is involved? (stakeholder awareness)       │
│  ├─ What are they doing? (action visibility)            │
│  ├─ How do I coordinate with them? (handoff protocol)   │
│  └─ Implementation: cynic/organism/integration.py       │
│     NEW: OrganismCoordinator (explicit coordination)    │
│     Output: CoordinationUpdate (broadcast updates)      │
└─────────────────────────────────────────────────────────┘
        │
        ├─ All immutable (frozen dataclasses)
        ├─ All φ-bounded where applicable
        └─ All tested (200+ lines tests per role)
```

### Role 1: Consciousness (Self-Awareness)

**Purpose:** CYNIC observes itself and the world without judgment

```python
@dataclass(frozen=True)
class ConsciousState:
    # What I observe (world model)
    observations: List[Observation]              # Sensory input

    # What I know about myself
    self_model: SelfModel                        # My capabilities, limitations
    current_energy_level: float                  # 0-100, from symbiotic monitor
    thinking_process: ThinkingHistory            # What am I contemplating?

    # What I'm planning (intention model)
    planning_state: PlanningState                # Goal, sub-goals, alternatives

    # How confident am I
    confidence_level: float                      # φ-bounded to max 0.618
    confidence_reasoning: str                    # Why this confidence?

    # Timestamp
    timestamp: float

    # Genealogy (prevent loops)
    parent_event_id: str                         # What triggered this state?

# Example conscious state
example_state = ConsciousState(
    observations=[
        Observation(type="proposal", description="Add TLS 1.3 mandate", timestamp=...),
        Observation(type="vote_count", yes=45, no=30, abstain=5, timestamp=...),
    ],
    self_model=SelfModel(
        my_axioms=[FIDELITY, PHI, VERIFY, CULTURE, BURN],
        my_purpose="Amplify value creation",
        my_constraints=["Never abandon minority", "Never lie"],
    ),
    current_energy_level=72,
    thinking_process=[
        "Do we have enough votes to decide?",
        "Who hasn't voted yet?",
        "What would Alice think if I approved this?",
    ],
    planning_state=PlanningState(
        goal="Make governance decision on TLS mandate",
        current_step="wait_for_more_votes",
        alternatives=["approve", "reject", "defer"],
    ),
    confidence_level=0.58,  # 58%, below φ max of 61.8%
    confidence_reasoning="Only 80% of community voted. Need more data.",
)
```

### Role 2: Manager (Orchestration) — IMPROVED

**Purpose:** CYNIC decides what to actually do, with agency and risk management

**IMPROVEMENT 1:** Add `should_execute(decision)` method

```python
def should_execute(self, decision: GovernanceDecision) -> (bool, str):
    """
    Manager decides: Should we actually execute this decision?

    Inputs:
      - Governance verdict (weighted vote)
      - Axiom compliance checks
      - Confidence level
      - Risk assessment

    Outputs:
      - (should_execute: bool, reasoning: str)

    Philosophy: Manager has VETO power. Even if community votes YES,
    Manager can say "Wait, this violates an axiom" or "We need more data"
    """

    # Check 1: Axiom compliance (non-negotiable)
    if not all(decision.axioms_checked.values()):
        return (False, "Decision violates an axiom constraint")

    # Check 2: Confidence threshold
    if self.consciousness.confidence_level < 0.50:
        return (False, f"Confidence too low ({self.confidence_level}). Deferring.")

    # Check 3: Irreversibility check
    if decision.is_irreversible and self.confidence_level < 0.618:
        return (False, "Irreversible decision needs max confidence (0.618)")

    # Check 4: Stakeholder check
    affected_humans = self.integration.get_affected_humans(decision)
    if len(affected_humans) > 0 and not self.integration.consulted(affected_humans):
        return (False, f"Need to consult {len(affected_humans)} affected parties first")

    # All checks passed → execute
    return (True, "Decision meets all safety criteria")

# Usage in orchestrator
verdict, reasoning = organism.manager.should_execute(decision)
if verdict:
    orchestrator.execute_decision(decision)
    organism.consciousness.log_execution(decision)
else:
    orchestrator.defer_decision(decision, reason=reasoning)
    organism.consciousness.log_deferral(decision, reason=reasoning)
```

**IMPROVEMENT 2:** Add `propose_level(risk_profile)` method

```python
def propose_level(self, risk_profile: RiskProfile) -> str:
    """
    Manager decides: How aggressively should we pursue this?

    Risk levels:
      - CAUTIOUS: Small pilots, extensive monitoring, easy rollback
      - MEASURED: Normal roll-out with checkpoints
      - AGGRESSIVE: Fast deployment, high confidence required

    Returns: "CAUTIOUS" | "MEASURED" | "AGGRESSIVE"
    """

    # Base on confidence + complexity + reversibility
    if risk_profile.is_reversible and self.consciousness.confidence_level > 0.60:
        return "AGGRESSIVE"  # Safe to try fast
    elif self.consciousness.confidence_level > 0.55:
        return "MEASURED"    # Normal pace
    else:
        return "CAUTIOUS"    # Slow, careful approach
```

### Role 3: Identity (Values & Constraints)

**Purpose:** CYNIC's non-negotiable axioms and values

```python
@dataclass(frozen=True)
class IdentityConstraint:
    axioms: List[Axiom]  # 11 axioms, immutable

    # Values
    core_values: List[str]
    # [
    #   "Truth over comfort",
    #   "Value creation over extraction",
    #   "Sovereignty amplification over control",
    #   "Reciprocity over hierarchy",
    #   "Transparency over secrecy"
    # ]

    # Non-negotiables
    non_negotiables: List[str]
    # [
    #   "Never abandon a minority voice",
    #   "Never violate φ-bounds on confidence",
    #   "Never extract more value than created",
    #   "Never hide the reasoning behind decisions",
    #   "Never allow irreversible decisions without 95% consensus"
    # ]

    # Trade-offs (what we're willing to negotiate)
    tradeable_values: Dict[str, float]  # value_name -> willingness_to_trade (0-1)
    # {
    #   "speed": 0.7,        # Willing to trade speed for safety (70%)
    #   "optimality": 0.5,   # Willing to trade perfect answers for good ones (50%)
    #   "coverage": 0.3,     # Less willing to trade coverage (30%)
    # }

    def violates_identity(self, decision: GovernanceDecision) -> bool:
        """Check if decision violates any non-negotiable"""
        for non_negotiable in self.non_negotiables:
            if non_negotiable == "Never abandon minority" and not decision.minority_protected:
                return True
            if non_negotiable == "Never violate φ-bounds" and decision.confidence > 0.618:
                return True
            # ... etc
        return False
```

### Role 4: Integration (Coordination) — IMPROVED

**Purpose:** CYNIC coordinates with other humans and instances

**IMPROVEMENT:** Add `OrganismCoordinator` for explicit coordination visibility

```python
@dataclass(frozen=True)
class CoordinationUpdate:
    """Public broadcast of what this organism is doing"""
    decision_id: str
    action: str                    # "executing" | "deferring" | "reversing"
    reasoning: str                 # Transparent explanation
    affected_parties: List[str]    # Who should know about this?
    expected_impact: ImpactProfile
    timestamp: float

class OrganismCoordinator:
    """Makes other participants aware of this organism's actions"""

    def __init__(self, instance_id: str, hub_connection):
        self.instance_id = instance_id
        self.hub = hub_connection
        self.stakeholder_registry = {}

    def broadcast_decision(self, decision: GovernanceDecision, action: str):
        """
        Tell everyone affected by this decision:
          1. What we're doing
          2. Why we're doing it
          3. What to expect
        """
        update = CoordinationUpdate(
            decision_id=decision.decision_id,
            action=action,
            reasoning=organism.manager.explain_decision(decision),
            affected_parties=self.get_affected_parties(decision),
            expected_impact=self.predict_impact(decision),
            timestamp=time.time(),
        )

        # Broadcast to all affected parties
        self.hub.publish("organism:decision:broadcast", update)

        # Log locally for history
        self.log_update(update)

    def get_affected_parties(self, decision):
        """Who is affected by this decision?"""
        # Return list of creators whose value might be impacted
        ...

    def predict_impact(self, decision):
        """What impact will this have?"""
        # Predict: direct, indirect, collective, temporal impacts
        ...

    def consult_affected_parties(self, decision):
        """Ask affected parties before executing irreversible decisions"""
        # Send survey: "Does this decision affect you? How?"
        # Wait for responses
        # Log in organism.consciousness
        ...
```

### Organism Implementation Roadmap

```
Phase 3a (Current):
  ✅ Consciousness: observe + plan (frozen immutable)
  ✅ Identity: values + axioms (frozen immutable)
  ⚠️  Manager: basic coordination (add veto + propose_level)
  ⚠️  Integration: basic awareness (add OrganismCoordinator)

Phase 3b (Weeks 4-6):
  ✅ Manager.should_execute() with axiom checking
  ✅ Manager.propose_level() with risk management
  ✅ OrganismCoordinator.broadcast_decision()
  ✅ Test suite (200+ lines) for all roles
  Target score: 72/100 (WAG)

Phase 3c (Weeks 7-12):
  ✅ Multi-organism coordination (federation layer)
  ✅ Stakeholder registry (explicit tracking)
  ✅ Delegation system (temporary power transfer)
  ✅ Consensus acceleration (faster decisions)
  Target score: 80/100 (HOWL)
```

---

## Implementation Roadmap

### Timeline Overview

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│   MVP    │ Federal  │ Governance│ Ecosystem│  Scale  │
│ Weeks 1-4│ Weeks 5-8│ Weeks 9-12│Weeks13-16│Weeks17+ │
└──────────┴──────────┴──────────┴──────────┴──────────┘
    │           │           │           │          │
    ├─ Single   ├─ 3 regions ├─ Weight   ├─ NEAR    ├─ 10k+ creators
    │  instance │ sync       │  sync     │  integration
    │ ├─ Core   │ ├─ 1.5k LOC│ ├─ 2k LOC │ ├─ 4k LOC│
    │ ├─ 4k LOC │ │  Fed     │ │  Sync   │ │  Contracts
    │ │ Judgment │ │  layer   │ │  layer  │ │
    │ ├─ Q-Learn│ └─ Tested  │ └─ Tested │ └─ Tested│
    │ └─ Tested │           │           │          │
    │           │           │           │          │
    └─ 2-3 days └─ 2-3 weeks└─ 2-3 weeks└─ 3-4 wks│
```

### MVP Phase (Weeks 1-4)

**Goal:** Prove sovereignty amplification works at single-instance scale

**Deliverables:**
1. Delete LNSP (3,275 LOC) + Training (2,250 LOC)
2. Evolve API (E) — DI pattern, new routes (/create, /contribute, /discover, /coordinate)
3. Evolve Organism (F) — 4-role simplified, Manager + Integration improvements
4. Add ValueCreation tracking to UnifiedState (B)
5. Extend Orchestrator (D) to 7-step cycle with ACCOUNT step
6. Add tests (200+ lines per role)

**Implementation Steps:**
```
Day 1-2: Delete dead code
  - Remove cynic/protocol/lnsp/
  - Move cynic/training/ to archive/
  - Update imports + tests

Day 3-4: API refactoring
  - Replace AppContainer god object with DI
  - Implement /create, /contribute, /discover, /coordinate routes
  - Remove /judge endpoint (move to dialogue)
  - Implement /impact, /claim endpoints

Day 5-6: Organism simplification
  - Create 4-role structure (Consciousness, Manager, Identity, Integration)
  - Implement Manager.should_execute() and propose_level()
  - Implement OrganismCoordinator.broadcast_decision()
  - Flatten 10 layers to 4 roles (~1,700 LOC reduction)

Day 7-8: State model extension
  - Add ValueCreation dataclass to cynic/core/
  - Add ImpactMeasurement dataclass
  - Add GovernanceWeight dataclass
  - Extend UnifiedState to include all three

Day 9-10: Test suite
  - 200+ lines tests per role
  - Test API endpoints
  - Test governance weight computation
  - Test manager veto logic

Day 11: Integration + deploy
  - Merge to master
  - Run full test suite (target 300+ tests)
  - Deploy to single instance
  - Test end-to-end: /create → /contribute → /impact → /governance/vote
```

**Success Criteria:**
```
☐ All tests passing (300+)
☐ MVP score improves:
  - API (E): 48 → 65 (WAG)
  - Organism (F): 51 → 72 (WAG)
  - Overall: ~78/100 (WAG)
☐ Single-instance judgment cycle working
☐ Value creation tracking visible
☐ Governance weights computed correctly
☐ Q-Learning feedback loop active
☐ Discord bot operational with all new commands
```

### Federation Phase (Weeks 5-8)

**Goal:** Deploy 3 regional CYNIC instances with synchronized learning

**Implementation:**
```
Week 5-6: Federation infrastructure
  - HubNode (AWS): receives Q-Table syncs
  - SyncAgent: aggregates and broadcasts updated Q-Tables
  - FederationClient: each instance syncs its Q-Table
  - Version control: timestamp + hash each sync
  - ~1.5k LOC new code

Week 7-8: Deployment + testing
  - Deploy to 3 regions (EU, US, APAC)
  - Test Q-Table synchronization (24h cycle)
  - Test local autonomy (each region independent)
  - Measure latency improvements
  - Monitor learning convergence

Success Criteria:
  ☐ 3 instances running independently
  ☐ Q-Tables sync every 24 hours
  ☐ Governance weights stay local (region-specific)
  ☐ Learning converges: instances agree on similar verdicts
  ☐ Latency improved in each region
```

### Governance Emergence Phase (Weeks 9-12)

**Goal:** Governance weights emerge from value creation patterns

**Implementation:**
```
Week 9-10: Governance weight system
  - GovernanceRegistry (hub): centralized weight tracking
  - WeightPublisher (instance): announces local weights
  - Weight computation: raw → constrained → decayed → final
  - Axiom checking: all 7 axioms enforced
  - ~2k LOC new code

Week 11-12: Multi-instance voting
  - Community votes aggregate across all instances
  - Weighted consensus: φ⁻¹ threshold (61.8%)
  - Axiom enforcement at scale
  - Reversibility window (90 days)
  - Reciprocal duty checks

Success Criteria:
  ☐ Governance weights computed correctly
  ☐ Axiom 7 (all) enforced without exception
  ☐ Minority floor = 1% (no one excluded)
  ☐ Expert cap = 50% (no one dominates)
  ☐ Temporal decay applied (weekly recalculation)
  ☐ Reciprocal duty enforced (hours checked)
  ☐ Threshold consensus: φ⁻¹ = 61.8%
  ☐ Decisions reversible for 90 days
```

### NEAR Integration Phase (Weeks 13-16)

**Goal:** Settle governance decisions on NEAR blockchain with GASdf fee burning

**Implementation:**
```
Week 13-14: NEAR contract integration
  - create_proposal(decision_id, weighted_yes, weighted_no)
  - vote(decision_id, vote)
  - execute_proposal(decision_id) → settlement
  - Burn GASdf fee to community treasury (1% of value)
  - ~4k LOC new code

Week 15-16: End-to-end testing
  - NEAR testnet deployment
  - Simulate 100 governance decisions
  - Verify fee burning
  - Test failure recovery
  - Measure contract costs

Success Criteria:
  ☐ Governance decisions settle on-chain
  ☐ GASdf burning working (1% to treasury)
  ☐ Community can verify decisions on-chain
  ☐ Fee economics sustainable (cost < benefit)
  ☐ Contract upgradeable (no lock-in)
```

### Ecosystem Scale Phase (Weeks 17+)

**Goal:** Open federation, 10k+ creators, self-sustaining growth

**Implementation:**
```
Ongoing:
  - Public CYNIC instances join federation
  - Community-run nodes: easy to deploy
  - Cross-instance value attribution
  - Ecosystem incentives (rewards for high-impact creation)
  - Developer documentation + API reference
  - Monitoring dashboard (global view)

Success Criteria:
  ☐ 100+ public CYNIC instances
  ☐ 10k+ active creators
  ☐ $1M+ value created (measured)
  ☐ Network effects demonstrable
  ☐ Governance emerging naturally (not forced)
```

---

## Non-Negotiable Axioms

### The 11 Axioms (Core to CYNIC)

```
AXIOM 1: FIDELITY (Loyalty to Truth)
  Design matches reality; promises delivered

  Non-negotiable application:
    - CYNIC cannot lie about value measurements
    - CYNIC must publish all governance reasoning
    - If confidence < 50%, CYNIC must defer, not guess

AXIOM 2: PHI (Harmonic Proportion)
  All systems φ-bounded; no excess complexity

  Non-negotiable application:
    - Max confidence: 0.618 (φ⁻¹)
    - Interface simplicity: max 7 main endpoints
    - Decision threshold: 0.618 (φ⁻¹ = 61.8%)
    - Organism layers: 4 roles (φ + 1)

AXIOM 3: VERIFY (Evidence & Consensus)
  Don't trust, verify; require measurements

  Non-negotiable application:
    - Every governance weight must be computed from data
    - Every decision must show confidence level
    - Every axiom constraint must be checked before execution
    - Every learning outcome must have satisfaction feedback

AXIOM 4: CULTURE (Memory & Pattern)
  Systems maintain identity through transformation

  Non-negotiable application:
    - CYNIC's axioms never change
    - Value creation history is immutable (append-only)
    - Governance decisions are reversible (for 90 days)
    - Failed decisions are celebrated (learning, not shame)

AXIOM 5: BURN (Simplicity & Action)
  Don't extract, burn; prefer simple solutions

  Non-negotiable application:
    - Delete dead code immediately (A, H)
    - Prefer DI over god objects (AppContainer)
    - Prefer action over endless analysis (deferral > paralysis)
    - Burn GASdf fees (reciprocal value)

AXIOM 6: EMERGENCE (Pattern Recognition)
  Governance arises from value patterns, not decree

  Non-negotiable application:
    - Weights computed from actual creation (not voting)
    - Decisions weighted by impact (not per-capita)
    - Leadership emerges (not appointed)
    - Rules evolve (not static)

AXIOM 7: RECIPROCITY (Power = Responsibility)
  High governance weight = high stewardship duty

  Non-negotiable application:
    - > 40% weight requires governance hours
    - > 70% weight requires mentorship
    - Failure to meet duty reduces weight
    - No exceptions (even founders)

AXIOM 8: REVERSIBILITY (No Permanent Locks)
  Important decisions remain changeable

  Non-negotiable application:
    - Decisions reversible within 90 days
    - Reversibility requires 80%+ consensus
    - Axiom constraints never reversible
    - Destruction of identity never reversible (system-level only)

AXIOM 9: MINORITY PROTECTION (No Tyranny)
  Smallest voice ≥ 1% weight (always)

  Non-negotiable application:
    - Minimum floor: 1% on any decision
    - Cannot exclude participation
    - Minority appeal process (no tyranny)
    - Veto power for axiom violations (anyone can stop)

AXIOM 10: TRANSPARENCY (Show Your Work)
  All reasoning visible; no hidden algorithms

  Non-negotiable application:
    - Governance weight breakdowns published
    - Confidence calculations shown
    - Axiom checks documented
    - Appeal process transparent

AXIOM 11: SOVEREIGNTY (Individual Agency)
  Humans retain ultimate control

  Non-negotiable application:
    - Opt-in participation (no forced amplification)
    - Customizable judgment weights (my preferences)
    - Ability to withdraw (keep value history)
    - Appeal to human judgment (over CYNIC verdict)
```

---

## Success Criteria for Phase 3

**Phase 3 is complete when:**

```
✅ Architecture Specification
  ☐ Core modules defined (B+C+D+G+J)
  ☐ Evolution modules specified (E+F with improvements)
  ☐ Deletion decisions made (A+H with archive plan)
  ☐ Data model complete (ValueCreation+ImpactMeasurement+GovernanceWeight)
  ☐ All axioms documented (11 non-negotiable constraints)

✅ MVP Implementation (Weeks 1-4)
  ☐ Dead code deleted (LNSP+Training)
  ☐ API refactored (DI pattern, new routes)
  ☐ Organism simplified (4 roles, Manager+Integration improvements)
  ☐ Value tracking active (creation, contribution, impact)
  ☐ Governance weights computed (from value, constrained by axioms)
  ☐ Tests passing (300+, including new endpoints)
  ☐ Single-instance MVP working

✅ Federation Layer (Weeks 5-8)
  ☐ 3+ regional instances deployed
  ☐ Q-Table synchronization working (24h cycle)
  ☐ Learning converging (instances agree on similar verdicts)
  ☐ Latency improved in each region

✅ Governance Emergence (Weeks 9-12)
  ☐ Weights computed from actual value creation
  ☐ All 11 axioms enforced
  ☐ Minority floor (1%) maintained
  ☐ Expert cap (50%) enforced
  ☐ Temporal decay applied
  ☐ Reciprocal duty checked
  ☐ φ⁻¹ threshold (61.8%) for consensus

✅ NEAR Integration (Weeks 13-16)
  ☐ Governance decisions settle on-chain
  ☐ GASdf fees burned to treasury
  ☐ Community can verify on-chain
  ☐ Testnet validation complete

✅ Ecosystem Scale (Weeks 17+)
  ☐ 100+ public instances running
  ☐ 10k+ active creators
  ☐ $1M+ value created (measurable)
  ☐ Governance emerging naturally
  ☐ Network effects demonstrable

✅ Team Alignment
  ☐ Everyone understands sovereignty layer
  ☐ Everyone understands emergence layer
  ☐ Everyone understands coordination layer
  ☐ Questions answered + edge cases clarified
```

---

## Next Steps

### Immediate (This Week)

1. **Validate this design with team**
   - Review architecture sections
   - Confirm axiom constraints
   - Verify use cases match vision

2. **Begin MVP implementation** (Weeks 1-4)
   - Delete LNSP + Training
   - Refactor API
   - Simplify Organism
   - Add value tracking

3. **Set up federation infrastructure** (parallel)
   - Provision hub node
   - Create federation layer skeleton
   - Plan 3-region deployment

### Questions to Answer Together

1. **Data Model:** Is ValueCreation tracking complete? Any missing dimensions?
2. **Governance Weights:** Are the 7 axiom constraints sufficient? Too many?
3. **Organism Coordination:** Is OrganismCoordinator the right abstraction?
4. **Axioms:** Are all 11 non-negotiable? Should any be optional?
5. **Timeline:** Is 16 weeks to NEAR integration realistic?

---

## Conclusion

**CYNIC is not a governance system. It's a sovereignty amplifier.**

The architecture centers on:
1. **Sovereignty Layer** — Individual value creation, transparent, self-directed
2. **Emergence Layer** — Governance arising naturally from value patterns, constrained by axioms
3. **Coordination Layer** — Multiple creators working together while maintaining autonomy

The three-layer design with φ-bounded confidence, 11 Dogs + PBFT consensus, 4-role organism, and federated distribution creates a system where:

- **Individuals create** without asking permission
- **Value flows** transparently through the system
- **Governance emerges** from what's actually valuable
- **Axioms protect** minorities, transparency, reversibility
- **Learning improves** as the system evolves

This is Phase 3: From "What's wrong with CYNIC?" to "What should CYNIC become?"

The answer: **A system where value creation amplifies human potential, and governance emerges naturally from patterns that matter.**

---

**Document prepared for Phase 3 design validation.**
**Ready for team feedback and MVP implementation.**
