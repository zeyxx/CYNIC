# PHASE 3: VISION COMPLÈTE — Tous les Patterns, Migrations, Flux

**Date:** 2026-02-27
**Objectif:** Montrer LA vision totale de CYNIC après transformation
**Portée:** Architecture complète, patterns, data flows, state machines, migrations

---

## 1. LA VISION GLOBALE (Bird's Eye)

### Ce que devient CYNIC

```
TODAY (Feb 2026)                    TOMORROW (Apr 2026+)

Judgment Engine                     Sovereignty Amplifier
├─ 11 Dogs judge                    ├─ Humans create value
├─ PBFT consensus                   ├─ CYNIC measures impact
├─ Q-Learning feedback              ├─ Governance emerges
└─ Binary decisions                 ├─ Humans have agency
                                    └─ Collective benefit grows

CURRENT: "What should we do?"       FUTURE: "Who's creating value?
(judge proposals)                           How do we amplify it?"

PROBLEM SOLVED:
  Before: Value invisible, governance = judgment
  After:  Value transparent, governance = emergence
```

### Les 3 Couches Détaillées

```
┌──────────────────────────────────────────────────────────────────────┐
│                    CYNIC SOVEREIGNTY AMPLIFIER                       │
│          (Individual Value Creation → Collective Governance)         │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │ LAYER 3: COORDINATION (Multi-Creator Orchestration)            │  │
│ │ ─────────────────────────────────────────────────────────────  │  │
│ │                                                                │  │
│ │ Purpose: Multiple humans work together while keeping:          │  │
│ │  ✓ Individual sovereignty (can leave anytime)                 │  │
│ │  ✓ Clear attribution (who did what)                          │  │
│ │  ✓ Fair value splits (reciprocity enforced)                  │  │
│ │  ✓ Transparent governance (decisions weighted by impact)      │  │
│ │                                                                │  │
│ │ Patterns:                                                      │  │
│ │  1. VALUE CHAIN: A creates → B enhances → C distributes       │  │
│ │     Splits: A=40%, B=35%, C=25% (or negotiated)              │  │
│ │                                                                │  │
│ │  2. WORKING GROUP: 5 creators + CYNIC form team              │  │
│ │     Splits: Egalitarian (20% each) or weighted (roles)        │  │
│ │                                                                │  │
│ │  3. CONTINUOUS: Maintainer runs service long-term             │  │
│ │     Splits: Dynamic (creator% decreases, maintainer% grows)   │  │
│ │                                                                │  │
│ │  4. RECIPROCAL DUTY: "Power = Responsibility"                │  │
│ │     If you vote, you must: spend X hours/month on governance  │  │
│ │     If you have > 70% weight, you must mentor others          │  │
│ │                                                                │  │
│ │ Data Models:                                                   │  │
│ │  - Coordination (team ID, participants, splits)              │  │
│ │  - Contributors (who, role, hours, value_received)           │  │
│ │  - Reciprocal Duty (power level, hours required, hours spent)│  │
│ │  - Value Flow (from A to B, amount, timestamp)               │  │
│ │                                                                │  │
│ │ Module: CoordinationEngine (800 LOC)                          │  │
│ │ Tests:  100+ (working groups, value chains, duty checks)      │  │
│ │ Status: NEW in Phase 3                                        │  │
│ │                                                                │  │
│ └────────────────────────────────────────────────────────────────┘  │
│                          ▲ (depends on)                             │
│                          │                                          │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │ LAYER 2: EMERGENCE (Governance from Value Patterns)            │  │
│ │ ─────────────────────────────────────────────────────────────  │  │
│ │                                                                │  │
│ │ Purpose: Governance weights computed from actual value        │  │
│ │  ✓ NOT appointed (emerges from creation)                     │  │
│ │  ✓ NOT 1-person-1-vote (weighted by impact)                  │  │
│ │  ✓ NOT absolute (constrained by axioms)                      │  │
│ │  ✓ NOT permanent (decays as conditions change)                │  │
│ │                                                                │  │
│ │ Algorithm:                                                     │  │
│ │                                                                │  │
│ │  Step 1: MEASURE VALUE                                         │  │
│ │    direct_impact = my_creations_value / total_value           │  │
│ │    indirect_impact = (I helped others' creations) / total     │  │
│ │    collective_impact = (group decisions I influenced) / total │  │
│ │    temporal_impact = compound/decay over time                 │  │
│ │                                                                │  │
│ │  Step 2: ADD DOMAIN EXPERTISE                                  │  │
│ │    if is_expert_in_domain(me, decision_type):                 │  │
│ │      raw_weight *= 1.2  (slight boost for expertise)          │  │
│ │    else:                                                       │  │
│ │      raw_weight *= 1.0  (neutral)                             │  │
│ │                                                                │  │
│ │  Step 3: APPLY AXIOM CONSTRAINTS                               │  │
│ │    if raw_weight > 0.50:  # Too much power                    │  │
│ │      constrained_weight = 0.50  # Cap at 50%                  │  │
│ │                                                                │  │
│ │    if raw_weight < 0.01:  # Too small voice                   │  │
│ │      constrained_weight = 0.01  # Floor at 1%                 │  │
│ │                                                                │  │
│ │  Step 4: APPLY TEMPORAL DECAY                                  │  │
│ │    weeks_old = (today - creation_date) / 7                    │  │
│ │    decay_rate = 0.005 * weeks_old  # 0.5% per week            │  │
│ │    decayed_weight = constrained_weight * (1 - decay_rate)    │  │
│ │                                                                │  │
│ │  Step 5: CHECK RECIPROCAL DUTY                                 │  │
│ │    if decayed_weight > 0.40:  # High power                    │  │
│ │      hours_required = decayed_weight * 100  # per year        │  │
│ │      hours_spent = measure_governance_hours(me)               │  │
│ │      if hours_spent < hours_required:                         │  │
│ │        final_weight = decayed_weight * (hours_spent/required) │  │
│ │      else:                                                     │  │
│ │        final_weight = decayed_weight                          │  │
│ │    else:                                                       │  │
│ │      final_weight = decayed_weight  # No duty check needed    │  │
│ │                                                                │  │
│ │  RESULT: final_weight (between 0.01 and 0.50)                 │  │
│ │                                                                │  │
│ │ 7 Axiom Constraints (NEVER negotiable):                        │  │
│ │  1. Minority floor: weight >= 0.01 (1%)                       │  │
│ │  2. Expert cap: weight <= 0.50 (50%)                          │  │
│ │  3. Domain specificity: different decisions = different weights│  │
│ │  4. Temporal decay: old impact = low weight                   │  │
│ │  5. Reciprocal duty: high power = high governance hours       │  │
│ │  6. Threshold consensus: > 0.618 (φ⁻¹) required               │  │
│ │  7. Reversibility: decisions changeable for 90 days            │  │
│ │                                                                │  │
│ │ Data Models:                                                   │  │
│ │  - ImpactMeasurement (4D: direct, indirect, collective, tmp)  │  │
│ │  - GovernanceWeight (raw, constrained, decayed, final)        │  │
│ │  - AxiomCheck (7 constraints, each TRUE/FALSE)                │  │
│ │  - TemporalDecay (creation_date, decay_rate, computed_weight) │  │
│ │                                                                │  │
│ │ Module: EmergenceEngine (1,200 LOC)                            │  │
│ │ Tests:  150+ (weight computation, axiom checks, decay)        │  │
│ │ Status: NEW in Phase 3                                        │  │
│ │                                                                │  │
│ └────────────────────────────────────────────────────────────────┘  │
│                          ▲ (depends on)                             │
│                          │                                          │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │ LAYER 1: SOVEREIGNTY (Individual Value Creation)               │  │
│ │ ─────────────────────────────────────────────────────────────  │  │
│ │                                                                │  │
│ │ Purpose: Humans create independently, everything visible      │  │
│ │  ✓ Full transparency on your value                           │  │
│ │  ✓ Full control over participation                           │  │
│ │  ✓ Full agency (can opt-in/opt-out)                         │  │
│ │  ✓ No forced amplification                                   │  │
│ │                                                                │  │
│ │ Endpoints & Flows:                                             │  │
│ │                                                                │  │
│ │  /create (POST)                                                │  │
│ │    Input: {description, type, audience}                       │  │
│ │    Output: ValueCreation(id, timestamp, creator_id)           │  │
│ │    Effect: New artifact tracked                               │  │
│ │    Example: "I'm building auth library"                       │  │
│ │             → Creates lib-auth-001                            │  │
│ │                                                                │  │
│ │  /contribute (POST)                                            │  │
│ │    Input: {creation_id, contribution_type, effort, share%}    │  │
│ │    Output: Contribution(id, contributor_id, share_pct)        │  │
│ │    Effect: Added to ValueCreation.contributors                │  │
│ │    Example: "I added security review (10 hours, 5% share)"    │  │
│ │             → Shows up in lib-auth-001 contributors            │  │
│ │                                                                │  │
│ │  /discover (GET)                                               │  │
│ │    Input: {search, filter_impact, sort_by}                    │  │
│ │    Output: [ValueCreation] ranked by relevance                │  │
│ │    Effect: Search across all creations                        │  │
│ │    Example: search="auth" → shows all auth-related artifacts  │  │
│ │                                                                │  │
│ │  /impact (GET)                                                 │  │
│ │    Input: {time_range, dimension}                             │  │
│ │    Output: ImpactMeasurement (4D breakdown)                   │  │
│ │    Effect: Shows your value in each dimension                 │  │
│ │    Example: last_90_days → direct=42.5, indirect=15.3, etc    │  │
│ │                                                                │  │
│ │  /coordinate (POST)                                            │  │
│ │    Input: {participants, description, split_strategy}         │  │
│ │    Output: Coordination(id, status=proposed)                  │  │
│ │    Effect: Proposes working group                             │  │
│ │    Example: "Let's build enterprise security framework"       │  │
│ │             → Participants receive invite                     │  │
│ │                                                                │  │
│ │  /claim (POST)                                                 │  │
│ │    Input: {creation_id, claim_amount}                         │  │
│ │    Output: Claim(id, status=pending)                          │  │
│ │    Effect: Initiates value claim (community votes)            │  │
│ │    Example: "Claim 100 tokens for auth library"               │  │
│ │             → Community rates, approves or rejects             │  │
│ │                                                                │  │
│ │ Data Models:                                                   │  │
│ │  - ValueCreation (artifact details + impact scores)           │  │
│ │  - Contribution (helper details + value share)                │  │
│ │  - Coordination (team details + participants)                 │  │
│ │  - Claim (vesting details + community votes)                  │  │
│ │                                                                │  │
│ │ Module: ValueCreationEngine (600 LOC)                         │  │
│ │ Tests:  80+ (creation tracking, contribution splits)          │  │
│ │ Status: NEW in Phase 3                                        │  │
│ │                                                                │  │
│ └────────────────────────────────────────────────────────────────┘  │
│                          ▲ (depends on)                             │
│                          │                                          │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │ CORE JUDGMENT ENGINE (5 HOWL-Grade Modules)                    │  │
│ │ ─────────────────────────────────────────────────────────────  │  │
│ │                                                                │  │
│ │ B: UnifiedState (immutable contracts with φ-bounds)           │  │
│ │    - UnifiedJudgment, UnifiedLearning, UnifiedConsciousState  │  │
│ │    - Status: PROVEN (93/100 HOWL)                             │  │
│ │    - Tests: 100+, Production-ready                            │  │
│ │                                                                │  │
│ │ C: Events (pub-sub + genealogy)                               │  │
│ │    - 3-bus system, loop prevention                            │  │
│ │    - Status: PROVEN (71/100 WAG)                              │  │
│ │    - Tests: 40, Active usage                                  │  │
│ │                                                                │  │
│ │ D: Orchestrator (7-step judgment cycle)                       │  │
│ │    - PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → ... │  │
│ │    - Status: PROVEN (84/100 HOWL)                             │  │
│ │    - Tests: 85, Central business logic                        │  │
│ │                                                                │  │
│ │ G: Dialogue (interactive CLI + Discord)                       │  │
│ │    - TALK mode, human-in-loop planning                        │  │
│ │    - Status: PROVEN (81/100 HOWL)                             │  │
│ │    - Tests: 200+, User-facing                                 │  │
│ │                                                                │  │
│ │ J: Observability (symbiotic state tracking)                   │  │
│ │    - Human + Machine + CYNIC layers                           │  │
│ │    - Status: PROVEN (86/100 HOWL)                             │  │
│ │    - Tests: 108, Dashboard + API                              │  │
│ │                                                                │  │
│ └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 2. STATE MACHINES (Comment ça marche)

### State Machine: Création de Valeur

```
ValueCreation Lifecycle

START (user calls /create)
  │
  ├─ INPUT: {description, type: "product"|"service"|"knowledge"|"governance"}
  │
  ├─ CREATE ValueCreation(artifact)
  │   ├─ creation_id = UUID
  │   ├─ creator_id = user
  │   ├─ description = input
  │   ├─ timestamp = now()
  │   ├─ direct_impact_score = 0  (initial)
  │   └─ contributors = {}
  │
  ├─ STATE: "created" (artifact exists)
  │   │
  │   └─ OTHER USERS CAN: /contribute to this artifact
  │      ├─ Input: {contribution_type, effort, expected_share}
  │      └─ Effect: Added to contributors[]
  │
  ├─ STATE: "active" (after first use/adoption)
  │   │
  │   ├─ CYNIC MEASURES: adoption_rate, quality_score
  │   ├─ COMPUTE: direct_impact_score (0-100)
  │   └─ UPDATE: ImpactMeasurement every 7 days
  │
  ├─ STATE: "mature" (active for 30+ days)
  │   │
  │   ├─ OTHER USERS CAN: /claim value from this creation
  │   ├─ CYNIC MEASURES: full 4D impact
  │   └─ AFFECTS: Governance weights (creator's weight in decisions)
  │
  ├─ STATE: "deprecated" (new version, better alternative)
  │   │
  │   ├─ KEEP: All historical data (immutable)
  │   ├─ DECAY: Governance weight decreases
  │   └─ BUT: Still credited for historical impact
  │
  └─ END (artifact remains in history forever)
     └─ Immutable record of who created, who helped, what was learned
```

### State Machine: Gouvernance

```
Governance Decision Lifecycle

PROPOSE (user calls /governance/propose)
  │
  ├─ INPUT: {decision_type, description, options}
  │
  ├─ CREATE GovernanceDecision(proposal)
  │   ├─ decision_id = UUID
  │   ├─ status = "voting"
  │   ├─ votes = {}
  │   └─ axioms_checked = {}
  │
  ├─ STATE: "voting" (collecting votes)
  │   │
  │   ├─ USERS VOTE: /governance/vote(YES|NO|ABSTAIN)
  │   ├─ FOR EACH VOTE:
  │   │  ├─ voter_id must have governance_weight > 0
  │   │  ├─ weight = EmergenceEngine.compute_weight(voter_id, decision_type)
  │   │  └─ vote_weighted = vote * weight
  │   │
  │   └─ VOTING ENDS: after 7 days OR consensus reached (>61.8%)
  │
  ├─ STATE: "axiom_check" (before decision)
  │   │
  │   ├─ CYNIC CHECKS 7 axioms:
  │   │  1. Minority floor: smallest weight >= 1%? ✓
  │   │  2. Expert cap: largest weight <= 50%? ✓
  │   │  3. Domain specificity: right decision type? ✓
  │   │  4. Temporal decay: weights decayed correctly? ✓
  │   │  5. Reciprocal duty: high power holders spent hours? ✓
  │   │  6. Threshold consensus: weighted YES > 61.8%? ✓
  │   │  7. Reversibility: decision reversible for 90 days? ✓
  │   │
  │   └─ IF ANY AXIOM FAILS:
  │      ├─ decision = DEFERRED (not rejected, just delayed)
  │      └─ reason = which axiom failed
  │
  ├─ STATE: "manager_review" (organism checks safety)
  │   │
  │   ├─ Manager asks: "Should we execute this decision?"
  │   ├─ Checks:
  │   │  - Is it reversible if it goes wrong?
  │   │  - Do affected parties know about this?
  │   │  - Is confidence high enough?
  │   │
  │   └─ Manager output: APPROVE | DEFER | VETO
  │       └─ VETO only if axiom violation detected
  │
  ├─ STATE: "approved" (ready to execute)
  │   │
  │   ├─ STORE: execution_timestamp = now()
  │   ├─ BROADCAST: via Orchestrator.execute()
  │   ├─ EFFECT: Decision implemented (create_proposal on NEAR, etc)
  │   └─ WINDOW: Reversible for 90 days after this point
  │
  ├─ STATE: "feedback" (measure impact)
  │   │
  │   ├─ AFTER 30 days:
  │   │  ├─ COMMUNITY RATES: satisfaction_rating (1-5 stars)
  │   │  ├─ PROVIDES: feedback_text (why this rating?)
  │   │  └─ CYNIC LEARNS: Update Q-Table with outcome
  │   │
  │   ├─ COMPARE: predicted_impact vs actual_impact
  │   │
  │   ├─ IF actual > predicted:
  │   │  ├─ Q-Table: increase confidence in similar decisions
  │   │  └─ Governance weights: proposer's weight increases
  │   │
  │   ├─ IF actual < predicted:
  │   │  ├─ Q-Table: decrease confidence
  │   │  ├─ Governance weights: proposer's weight decreases
  │   │  └─ OPTION TO REVERSE: if impact was negative
  │   │
  │   └─ LEARNING LOOP ACTIVATED: next decision uses updated Q-Table
  │
  ├─ STATE: "reversible" (can still undo)
  │   │
  │   └─ IF 80%+ weighted consensus votes to REVERSE within 90 days:
  │      ├─ Decision REVERSED
  │      └─ New decision_id created for reversal record
  │
  └─ STATE: "closed" (permanent, no more reversals)
     └─ Immutable record of decision, impact, learning
```

---

## 3. DATA FLOW (Comment la valeur circule)

### Complete End-to-End Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                   COMPLETE VALUE CREATION LOOP                   │
│              (How sovereignty amplification works)                │
└──────────────────────────────────────────────────────────────────┘

[DAY 1: ALICE CREATES]

ALICE: /create
  ├─ Creates auth library
  ├─ System: ValueCreation(lib-auth-001, creator=alice)
  └─ Impact: direct_score = 0 (waiting for adoption)

[DAY 2-7: BOB & CAROL HELP]

BOB: /contribute to lib-auth-001
  ├─ Adds example projects
  ├─ System: Contribution(contributor=bob, share=10%)
  └─ Alice receives notification: "Bob helped"

CAROL: /contribute to lib-auth-001
  ├─ Security review
  ├─ System: Contribution(contributor=carol, share=5%)
  └─ Alice receives notification: "Carol helped"

[DAY 14: LIBRARY PUBLISHED]

ALICE: /impact (check measurement)
  ├─ direct_impact = 72/100 (quality review passed)
  ├─ adoption_rate = 0.23 (23% of target market)
  ├─ utility_index = 18.5
  │
  ├─ YOUR IMPACT BREAKDOWN (last 30 days):
  │  ├─ Direct: 72 (I created)
  │  ├─ Indirect: 3 (Bob's examples help others)
  │  ├─ Collective: 0 (not voting yet)
  │  └─ Temporal: 5 (compound as adoption grows)
  │
  └─ GOVERNANCE WEIGHTS (if voting today):
     ├─ Authentication decisions: 35% weight
     ├─ API design decisions: 28% weight
     └─ Security decisions: 24% weight

[DECISION: Should we add blockchain auth?]

CYNIC: Propose governance decision
  ├─ decision_id = "auth-blockchain-001"
  ├─ decision_type = "authentication" (domain-specific)
  ├─ voting_period = 7 days
  └─ status = "voting"

[DAY 21: VOTING TIME]

ALICE VOTES YES (weight = 35%)
  ├─ Why 35%? Because:
  │  ├─ Raw: 72/200 total auth value = 36%
  │  ├─ Expert boost: 1.2× for auth expertise = 43.2%
  │  ├─ Cap at 50%: 43.2% → 43.2% (still under cap)
  │  ├─ Decay: only 1 week old, minimal decay = 42%
  │  ├─ Duty check: Alice spends 10 hours/month on governance
  │  │   Required: 42 * 10 = 42 hours/month (met!) ✓
  │  └─ Final: 42% → rounded to 35% after 100 voters normalize
  │
  └─ Vote recorded: YES with 35% weight

BOB VOTES NO (weight = 8%)
  ├─ Why 8%? Because:
  │  ├─ Raw: 15/200 total auth value = 7.5%
  │  ├─ Expert boost: 1.0× (not an expert) = 7.5%
  │  ├─ Cap: 7.5% (under 50%) ✓
  │  ├─ Decay: same week = 7.5%
  │  ├─ Duty: 7.5% < 40% threshold, no duty check needed
  │  └─ Final: 7.5% → 8% after rounding
  │
  └─ Vote recorded: NO with 8% weight

OTHER 10 VOTERS: Mixed (weighted 57% total)

AGGREGATION:
  ├─ YES votes: 35% + 22% + 15% + ... = 67%
  ├─ NO votes: 8% + 12% + 13% = 33%
  ├─ Decision threshold: > 61.8% (φ⁻¹) required
  ├─ Result: 67% > 61.8% → APPROVED ✓
  │
  └─ AXIOM CHECKS (all must pass):
     ├─ Minority floor: smallest weight = 1% (✓ >= 1%)
     ├─ Expert cap: largest weight = 43% (✓ <= 50%)
     ├─ Domain specificity: auth decision, auth expertise ✓
     ├─ Temporal decay: applied correctly ✓
     ├─ Reciprocal duty: all high-power holders met ✓
     ├─ Threshold: 67% > 61.8% ✓
     └─ Reversibility: decision reversible for 90 days ✓

[DAY 22: MANAGER REVIEW]

ORGANISM MANAGER: "Should we execute this?"
  ├─ Checks:
  │  ├─ Is blockchain auth reversible? YES ✓
  │  ├─ Are affected users aware? YES (voted) ✓
  │  ├─ Confidence level high enough? YES (67%) ✓
  │
  └─ APPROVAL: Execute blockchain auth feature

[DAY 23: EXECUTION]

ORCHESTRATOR: execute_decision(auth-blockchain-001)
  ├─ Call NEAR contract: create_proposal()
  ├─ NEAR: Proposal created on-chain
  ├─ NEAR: Burn GASdf fee to community treasury (1% of value)
  ├─ Discord bot: Announce "Blockchain auth approved!"
  └─ Database: Record execution timestamp & immutable log

[DAY 53: FEEDBACK COLLECTION (30 days later)]

COMMUNITY RATES: satisfaction (1-5 stars)
  ├─ Average rating: 4.8/5 stars ⭐⭐⭐⭐⭐
  ├─ Feedback: "Works perfectly, blockchain integration seamless"
  │
  ├─ ACTUAL IMPACT MEASURED:
  │  ├─ New users: +40% (predicted 30%)
  │  ├─ Security issues: 0 (predicted 1)
  │  ├─ Adoption velocity: exceeded expectations
  │  └─ actual_impact = 45/50 potential
  │
  ├─ COMPARED TO PREDICTION:
  │  ├─ Predicted: 30/50 (conservative)
  │  ├─ Actual: 45/50 (exceeded)
  │  ├─ Delta: +15/50 = +30% better than expected
  │
  └─ Q-TABLE LEARNS:
     ├─ Scenario: "blockchain auth + high confidence + expert vote"
     ├─ Outcome: "Very positive, 4.8/5 stars"
     ├─ Update: Q(state, action) += 15 points
     └─ Next decision: If similar, CYNIC will be more confident

[DAY 54: WEIGHTS UPDATE]

SYSTEM RECOMPUTES ALL GOVERNANCE WEIGHTS:
  ├─ Alice's auth impact: 72 → 87 (added blockchain success)
  ├─ Alice's auth weight: 35% → 42% (increased confidence)
  ├─ Alice's reciprocal duty: still met (spends 10 hrs/month)
  │
  └─ RESULT: Alice gains more influence in auth decisions
     (because she predicted correctly and community learned)

[DAY 60: NEXT DECISION]

CYNIC: "Should we add OAuth 2.0 flows?"
  ├─ Voting period starts
  ├─ Alice's weight: NOW 42% (was 35%, learned from feedback)
  ├─ System confidence: NOW 67% (Q-Table improved)
  │
  └─ LOOP CLOSES: Creation → Impact → Governance → Learning → Better Governance

[RESULT: Value Amplification Loop]

✓ Alice created value (auth library)
✓ Value was visible (impact measured: 4D)
✓ Impact → governance weight (merit-based not appointed)
✓ Governance emerges from value patterns (not decree)
✓ Community learns (Q-Table updated from feedback)
✓ Better decisions next time (confidence increased)
✓ Cycle repeats with improved wisdom
```

---

## 4. ARCHITECTURE PATTERNS (Comment c'est organisé)

### Pattern 1: Immutability & φ-Bounds

```
All Core State Objects Are FROZEN (Immutable)

@dataclass(frozen=True)
class ValueCreation:
    """Once created, never changes. Only append new contributions."""
    creation_id: str  # Unique forever
    creator_id: str
    timestamp: float  # When created (immutable timestamp)
    direct_impact_score: float  # Computed, not editable
    contributors: Dict[str, Contribution]  # Only additions allowed

@dataclass(frozen=True)
class UnifiedJudgment:
    """Verdict is immutable once issued."""
    verdict: Literal["HOWL", "WAG", "GROWL", "BARK"]
    confidence: float  # φ-bounded to max 0.618
    reasoning: str  # Why this verdict?

@dataclass(frozen=True)
class GovernanceWeight:
    """Weight computation is immutable & auditable."""
    human_id: str
    decision_type: str
    raw_weight: float  # From value
    constrained_weight: float  # After axioms
    decayed_weight: float  # After temporal decay
    final_weight: float  # After duty check
    axioms_checked: Dict[str, bool]  # Which axioms applied?
    timestamp: float  # Immutable timestamp

WHY IMMUTABILITY?
  ✓ Auditability: Can always see decision history
  ✓ Dispute resolution: Can prove what happened
  ✓ Learning: Q-Table can refer to immutable records
  ✓ Trust: No retroactive changes

WHY φ-BOUNDS?
  ✓ Confidence: max 0.618 = "I could be wrong"
  ✓ Humility: Never claim certainty (epistemic humility)
  ✓ Consensus: 0.618 threshold prevents narrow wins
  ✓ Harmonic: Golden ratio for proportion
```

### Pattern 2: Event Sourcing

```
All Changes Are Events (Not State Overwriting)

BEFORE (Bad):
  value_creation.direct_impact_score = 72  # Overwrites history
  value_creation.save()  # No trace of previous value

AFTER (Good):
  event = ValueImpactMeasuredEvent(
      creation_id="lib-auth-001",
      impact_score=72,
      measured_at=timestamp,
      parent_event_id="creation-event-001"  # Genealogy
  )
  event_bus.publish(event)  # Immutable record

BENEFITS:
  ✓ History: Can replay all events from day 1
  ✓ Debugging: Can see exactly when things changed
  ✓ Learning: Q-Table learns from event sequences
  ✓ Reproducibility: Same event sequence = same outcome
```

### Pattern 3: Weighted Consensus (Not Democracy)

```
BEFORE (1-person-1-vote):
  50 people vote on "add blockchain auth"
  Result: 26 YES, 24 NO
  Outcome: APPROVED (simple majority)
  Problem: Junior contributor votes same as expert

AFTER (Value-Weighted):
  50 people vote on "add blockchain auth"
  Results:
    - Alice (created 72/200 auth value): YES with 35% weight
    - Bob (created 15/200 auth value): NO with 8% weight
    - Other 48: mixed, weighted by their auth value
  Aggregation:
    YES votes: 35% + 22% + 15% + ... = 67%
    NO votes: 8% + 12% + ... = 33%
  Outcome: APPROVED (weighted consensus > 61.8%)
  Benefit: Expertise matters, minorities still heard

AXIOM ENFORCING:
  ✓ Minority floor (1%): everyone heard
  ✓ Expert cap (50%): no single expert dominates
  ✓ Domain specificity: auth experts vote on auth
  ✓ Temporal decay: old creations lose influence
```

### Pattern 4: Reciprocal Duty

```
BEFORE (Power without responsibility):
  Alice has 40% governance weight
  Alice never attends meetings, never votes, never contributes to governance
  Result: "She has power but doesn't use it responsibly"

AFTER (Power = Responsibility):
  Alice has 40% governance weight
  System requires: 40 * 10 = 400 hours/year of governance work
                  = ~33 hours/month
  If Alice only spends 5 hours/month:
    adjusted_weight = 40% * (5/33) = 6%
  So Alice's weight is reduced until she "earns" it back

  Alice's options:
    1. Spend more time on governance (increase to 33h/month → restore to 40%)
    2. Accept lower weight (stay at 6% if she only works 5h/month)
    3. Delegate to trusted advisor (ask Bob to use her hours)

BENEFIT:
  ✓ Power holders must be stewards
  ✓ No absentee leadership
  ✓ High power = high responsibility
```

### Pattern 5: Temporal Decay

```
Alice creates library on Jan 1, 2026
  Day 1: Impact = 100/100 (fresh, high quality)
  Day 30: Impact = 98/100 (decay 0.5%/week = minimal)
  Day 60: Impact = 97/100
  Day 90 (3 months): Impact = 95/100
  Day 180 (6 months): Impact = 73/100 (significant decay)
  Day 365 (1 year): Impact = 10/100 (very old, mostly obsolete)

WHY DECAY?
  ✓ Reality: Technology changes, libraries become outdated
  ✓ Fairness: New creators should have chance at influence
  ✓ Relevance: 6-month-old decisions less applicable today
  ✓ Prevents entrenchment: Early movers don't dominate forever

EXCEPTION: Foundational work decays slower
  Infrastructure library (0.2%/week decay) vs
  Trendy feature (2%/week decay)
```

---

## 5. MIGRATION PATH (Comment on passe de l'avant à l'après)

### Phase 1: MVP (Weeks 1-4) — Sovereignty Layer Working

```
GOAL: Prove individual value creation is measurable & visible

CHANGES:
  ✅ Delete LNSP (3,275 LOC) & Training (2,250 LOC)
  ✅ Refactor API (DI pattern, remove god object)
  ✅ Add ValueCreation tracking (new dataclass)
  ✅ Add ImpactMeasurement (4D: direct/indirect/collective/temporal)
  ✅ Simplify Organism (10 layers → 4 roles)
  ✅ Extend Orchestrator (add ACCOUNT step in 7-step cycle)
  ✅ Add tests (300+ total)

ENDPOINTS LIVE:
  ✓ /create (launch value artifacts)
  ✓ /contribute (help others)
  ✓ /discover (find valuable work)
  ✓ /impact (measure your value)
  ✓ /coordinate (work together)

WHAT STAYS SAME:
  ✓ Core judgment (B+D still work)
  ✓ Q-Learning (feedback loop functional)
  ✓ Discord bot (still operates)
  ✓ PBFT consensus (unchanged)

SUCCESS CRITERIA:
  ☑ Humans can create & see value creation
  ☑ Impact measured in 4 dimensions
  ☑ Governance weights computed (but not used yet)
  ☑ 300+ tests passing
  ☑ MVP score: B+D+G+J+E+F all WAG+ grade

TIMELINE: 2-3 weeks implementation, 1 week testing
```

### Phase 2: Federation (Weeks 5-8) — Distributed Learning

```
GOAL: Scale to 3 regions, synchronize learning

CHANGES:
  ✅ Deploy Hub node (aggregation point)
  ✅ Deploy 3 regional instances (EU, US, APAC)
  ✅ Q-Table synchronization (every 24 hours)
  ✅ Weight synchronization (optional, region-specific)
  ✅ Learn from collective feedback (all instances)

WHAT CHANGES:
  Instance EU: local Q-Table + receives Hub sync every 24h
  Instance US: local Q-Table + receives Hub sync every 24h
  Instance APAC: local Q-Table + receives Hub sync every 24h
  Hub: aggregates Q-Tables (average values across regions)

WHAT STAYS SAME:
  ✓ Sovereignty layer (same endpoints in each region)
  ✓ Each region independent (no forced alignment)
  ✓ Users can stay in home region or multi-region

SUCCESS CRITERIA:
  ☑ 3+ instances running independently
  ☑ Q-Tables sync every 24 hours
  ☑ Learning converges (instances agree on similar verdicts)
  ☑ Latency improved in each region
  ☑ No coordination overhead

TIMELINE: 2-3 weeks implementation, 1 week testing
```

### Phase 3: Governance Emergence (Weeks 9-12) — Value Weights Live

```
GOAL: Governance weights computed from actual value, decisions weighted

CHANGES:
  ✅ GovernanceRegistry (hub): centralized weight tracking
  ✅ WeightPublisher (each instance): announces weights
  ✅ Weight computation: raw → constrained → decayed → final
  ✅ 7 Axiom checks: all enforced
  ✅ Community voting: YES/NO weighted by impact
  ✅ φ⁻¹ threshold (61.8%) enforced
  ✅ Reversibility window (90 days) enabled

VOTING CHANGES:
  BEFORE: "1 human = 1 vote"
  AFTER: "1 human's weight = their value creation / total value"

DECISION CHANGES:
  BEFORE: Simple majority (50%)
  AFTER: Weighted consensus (φ⁻¹ = 61.8%)

SUCCESS CRITERIA:
  ☑ Weights computed from actual value creation
  ☑ All 7 axioms enforced (no exceptions)
  ☑ Minority floor maintained (1%)
  ☑ Expert cap enforced (50%)
  ☑ Temporal decay applied (weekly recalculation)
  ☑ Reciprocal duty enforced (hours checked)
  ☑ Decisions require > 61.8% consensus
  ☑ 90-day reversibility window active

TIMELINE: 2-3 weeks implementation, 1 week testing
```

### Phase 4: NEAR Integration (Weeks 13-16) — On-Chain Settlement

```
GOAL: Governance decisions settle on blockchain, community treasury burns tokens

CHANGES:
  ✅ NEAR contract integration
  ✅ Governance decisions → on-chain proposals
  ✅ Community votes → on-chain voting
  ✅ Results → on-chain execution
  ✅ GASdf fee burning (1% of value to treasury)

ON-CHAIN FLOW:
  1. Decision approved in CYNIC (weighted consensus > 61.8%)
  2. Orchestrator calls NEAR: create_proposal(decision)
  3. NEAR: Records on blockchain (immutable)
  4. NEAR: Burns GASdf fee to community treasury
  5. Community: Can verify decision on-chain forever

SUCCESS CRITERIA:
  ☑ Governance decisions settle on-chain
  ☑ GASdf burning working (1% of value to treasury)
  ☑ Community can verify decisions on-chain
  ☑ Fee economics sustainable (cost < benefit)
  ☑ Contract upgradeable (no lock-in)

TIMELINE: 2-3 weeks implementation, 1 week testing
```

### Phase 5: Ecosystem Scale (Weeks 17+) — 10k+ Creators

```
GOAL: 100+ public instances, ecosystem self-sustaining

CHANGES:
  ✅ Public CYNIC instances join federation
  ✅ Community-run nodes (easy deployment)
  ✅ Cross-instance value attribution
  ✅ Ecosystem incentives (rewards for creators)
  ✅ Developer documentation
  ✅ Monitoring dashboard

GROWTH:
  Week 17: 10 public instances
  Week 20: 50 public instances
  Week 24: 100+ public instances
  Month 6: 10k+ creators
  Month 12: $1M+ value created (measurable)

SUCCESS CRITERIA:
  ☑ 100+ public instances running
  ☑ 10k+ active creators
  ☑ $1M+ value created (measured)
  ☑ Network effects demonstrable
  ☑ Governance emerging naturally (not forced)
```

---

## 6. MODULES & CHANGES DÉTAILLÉES

### What Gets Deleted

```
MODULE A: LNSP (Layered Nervous System Protocol)
  LOC: 3,275
  Status: DEAD (never deployed, zero imports)
  Reason: Duplicated by nervous.EventJournal
  Action: git rm -r cynic/protocol/lnsp/

MODULE H: Training/Phase1B
  LOC: 2,250
  Status: DEAD (Phase 1B complete, Claude API now used)
  Reason: Strategy changed, torch/bitsandbytes overhead
  Action: git mv cynic/training/ archive/training_phase1b/

TOTAL DELETED: 5,525 LOC (-9.2% codebase reduction)
FREED UP: Cognitive overhead, dependency cleanup
```

### What Gets Refactored

```
MODULE E: API Layer
  BEFORE:
    - AppContainer god object (649 LOC)
    - Unclear routes (/judge endpoint confusing)
    - 8 imports tangled
    - 15 tests (insufficient)
  AFTER:
    - DI pattern (FastAPI dependency injection)
    - Clear routes (/create, /contribute, /discover, /impact, /coordinate)
    - Remove /judge (move to dialogue instead)
    - 50+ tests (comprehensive)
    - LOC: 649 → 800 (net +151, but better organized)
    - Q-Score: 48 → 65 (GROWL → WAG)

MODULE F: Organism Layers
  BEFORE:
    - 10 layers (confused hierarchy)
    - 3,950 LOC (bloated)
    - 0 tests (untrusted)
    - 4 imports (low usage)
  AFTER:
    - 4 roles (Consciousness, Manager, Identity, Integration)
    - 2,200 LOC (1,750 LOC reduction)
    - 200+ tests (comprehensive)
    - Manager.should_execute() veto power
    - Manager.propose_level() risk management
    - OrganismCoordinator explicit coordination
    - Q-Score: 51 → 72 (GROWL → WAG)
```

### What Gets Added

```
NEW: ValueCreationEngine (600 LOC)
  Purpose: Track individual value creation
  Dataclasses: ValueCreation, Contribution, ImpactMeasurement
  Endpoints: /create, /contribute, /discover, /impact
  Tests: 80+

NEW: EmergenceEngine (1,200 LOC)
  Purpose: Compute governance weights from value
  Algorithm: raw → constrained → decayed → final weight
  Axiom checks: 7 constraints enforced
  Tests: 150+

NEW: CoordinationEngine (800 LOC)
  Purpose: Multi-creator collaboration
  Patterns: value chains, working groups, continuous
  Reciprocal duty enforcement
  Tests: 100+
```

### What Stays Proven

```
MODULE B: UnifiedState (HOWL 93/100) ✅
  - No changes needed, proven architecture
  - Add ValueCreation dataclass (minor extension)

MODULE C: Events (WAG 71/100) ✅
  - No changes needed, event sourcing works
  - Add value impact events (extension)

MODULE D: Orchestrator (HOWL 84/100) ✅
  - No changes needed, 7-step cycle proven
  - Add ACCOUNT step (measure impact)
  - Extend to 8-step cycle

MODULE G: Dialogue (HOWL 81/100) ✅
  - No changes needed, interactive CLI works
  - Add /create, /contribute, /discover, /coordinate flows

MODULE J: Observability (HOWL 86/100) ✅
  - No changes needed, symbiotic tracking works
  - Add value flow visualization
```

---

## 7. LA VISION RÉSUMÉE EN 1 PAGE

```
┌─────────────────────────────────────────────────────────────────────┐
│                   CYNIC SOVEREIGNTY AMPLIFIER                       │
│                   (The Complete Vision)                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  INDIVIDUAL CREATES VALUE                                          │
│    ├─ /create: Launch artifact (product, service, knowledge, gov)  │
│    ├─ /contribute: Help others (share responsibility & reward)     │
│    ├─ /discover: Find valuable work (recommendations)              │
│    ├─ /impact: Measure 4D value (direct, indirect, collective, tmp)│
│    └─ /coordinate: Work together (maintain sovereignty)            │
│                                                                     │
│  VALUE BECOMES VISIBLE (To Everyone)                               │
│    ├─ Transparency: What did each human create?                   │
│    ├─ Attribution: Who helped? How much did they contribute?       │
│    ├─ Measurement: 4-dimensional impact scoring                    │
│    └─ Immutable: History recorded forever                          │
│                                                                     │
│  GOVERNANCE EMERGES (From Value Patterns)                           │
│    ├─ Weights computed: (value created) / (total value)           │
│    ├─ Constrained: By 7 axioms (no one dominates)                 │
│    ├─ Decayed: As conditions change (stale ideas lose influence)   │
│    └─ Duty enforced: High power = high governance hours            │
│                                                                     │
│  DECISIONS WEIGHTED (By Expertise & Impact)                         │
│    ├─ Voting: weighted by governance weight (not 1-person-1-vote)  │
│    ├─ Threshold: > 61.8% (φ⁻¹) for consensus                      │
│    ├─ Axioms checked: 7 constraints always enforced                │
│    └─ Reversible: 90 days to undo if impact is negative           │
│                                                                     │
│  LEARNING IMPROVES SYSTEM (Feedback Loop)                           │
│    ├─ Community rates decisions (1-5 stars)                        │
│    ├─ CYNIC learns: Q-Table updated from outcomes                  │
│    ├─ Confidence improves: Next decisions better informed          │
│    └─ Wisdom accumulates: System gets smarter over time            │
│                                                                     │
│  COORDINATION ENABLED (Multi-Creator Collaboration)                │
│    ├─ Value chains: A → B → C with fair splits                    │
│    ├─ Working groups: Team projects with egalitarian/weighted opts│
│    ├─ Continuous: Maintainers rewarded for long-term care          │
│    └─ Reciprocal: Value out ≥ value in (always)                   │
│                                                                     │
│  SETTLEMENT PERMANENT (On-Chain)                                   │
│    ├─ NEAR blockchain: Decisions recorded forever                  │
│    ├─ GASdf burning: 1% of value to community treasury             │
│    ├─ Transparency: Anyone can verify on-chain                     │
│    └─ Immutable: No retroactive changes possible                   │
│                                                                     │
│  SCALE UNLIMITED (Open Ecosystem)                                  │
│    ├─ 100+ public instances                                        │
│    ├─ 10k+ creators (any domain)                                   │
│    ├─ $1M+ annual value creation                                   │
│    └─ Self-sustaining ecosystem                                    │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  IMPLEMENTATION ROADMAP: 16 Weeks to Full Ecosystem                 │
│                                                                     │
│  Weeks 1-4:   MVP (Sovereignty layer working)                      │
│  Weeks 5-8:   Federation (3 regions, synced learning)              │
│  Weeks 9-12:  Emergence (governance weights from value)            │
│  Weeks 13-16: NEAR Integration (on-chain settlement)               │
│  Weeks 17+:   Ecosystem Scale (100+ instances, 10k+ creators)      │
│                                                                     │
│  Expected Outcome:                                                 │
│    ✓ Codebase: 108,306 → 98,396 LOC (-9.2%)                       │
│    ✓ Dead code: 5,525 LOC deleted (100%)                          │
│    ✓ Test coverage: 0.93% → 2.3% (+150%)                          │
│    ✓ Q-Score average: 60.3 → 78.6 (+18.3 points)                  │
│    ✓ Team confidence: 45% → 78% (+33%)                            │
│    ✓ Architecture clarity: 10 visions → 1 unified vision           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## VALIDATION CHECKLIST

**Questions pour approuver cette vision:**

```
☐ Les 3 couches (Sovereignty → Emergence → Coordination) ont du sens?
☐ L'algorithme de poids est clair? (raw → constrained → decayed → final)
☐ Les 7 axioms sont assez stricts? Ou trop?
☐ La formule temporelle decay est réaliste?
☐ Le reciprocal duty (power = responsibility) est juste?
☐ Les 16 semaines sont réalistes?
☐ Les migrations (Phase 1-4) sont séquentielles et logiques?
☐ Les deletions (A, H) sont justifiées?
☐ Les refactorings (E, F) sont suffisants?
☐ L'intégration NEAR est nécessaire pour MVP?

SI OUI À TOUTES: On peut commencer Phase 3 implementation
SI NON QUELQUES-UNES: On ajuste et revalide
```

