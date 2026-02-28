# Phase 3 Integration Map: Architecture Cible vs Existant

**Date:** 2026-02-27
**Purpose:** Clarifier EXACTEMENT ce qui existe vs ce qu'on ajoute en Phase 3

---

## 1. État Actuel (Master Branch)

### Core Components (Déjà Existants)
| Composant | Fichier | Rôle | État |
|-----------|---------|------|------|
| **JudgeOrchestrator** | `cynic/cognition/cortex/orchestrator.py` | 7-step cycle (PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE) | ✅ EXISTE |
| **PBFT Consensus** | `cynic/consensus/pbft_engine.py` | Byzantine Fault Tolerant voting (11 Dogs) | ✅ EXISTE |
| **11 Dogs** | `cynic/cognition/neurons/` | Judgment specialists (DogId, DogJudgment) | ✅ EXISTE |
| **Axioms** | `cynic/core/axioms.py` | 11 Axioms constraint system | ✅ EXISTE |
| **Config** | `cynic/core/config.py` | Frozen config singleton | ✅ EXISTE |
| **CircuitBreaker** | `cynic/cognition/cortex/circuit_breaker.py` | Stability control (CLOSED→OPEN→HALF_OPEN) | ✅ EXISTE |
| **ConsciousnessLevel** | `cynic/core/consciousness.py` | L1-L4 parallelism control | ✅ EXISTE |
| **EventBus** | `cynic/core/event_bus.py` | Event genealogy, loop prevention | ✅ EXISTE |
| **E-Score** | `cynic/core/escore.py` | Consciousness awareness metric | ✅ EXISTE |

### ACCOUNT Step Status ⚠️
- **Mentioned in:** `orchestrator.py` line 10 (docstring)
- **Purpose documented:** "Record cost, E-Score update"
- **Implementation:** PARTIAL (not explicit method)
- **What's missing:**
  - ❌ ValueCreation capture (proposal metadata)
  - ❌ Impact measurement (4D model)
  - ❌ Registration with engine
  - ❌ Feedback to governance

---

## 2. Phase 3 New Components (À Ajouter)

### Layer 1: Sovereignty (Value Creation Tracking)

| Composant | Fichier | Rôle | Impact |
|-----------|---------|------|--------|
| **ValueCreation** | `cynic/core/value_creation.py` (NEW) | Immutable frozen dataclass | Records WHO created WHAT with 4D impact |
| **ValueCreationEngine** | `cynic/engines/value_creation_engine.py` (NEW) | Registry + impact computation | Aggregates impact across creators |

**4D Impact Model:**
- **Direct Impact:** Immediate value to creator (weight: 40%)
- **Indirect Impact:** Value to dependent creators (weight: 35%)
- **Collective Impact:** Value to broader community (weight: 25%)
- **Temporal Impact:** Long-term sustainability (weight: 10%)

### Layer 2: Emergence (Governance Weight Computation)

| Composant | Fichier | Rôle | Impact |
|-----------|---------|------|--------|
| **GovernanceWeight** | `cynic/core/governance_weight.py` (NEW) | Immutable frozen dataclass | φ-bounded [0.01, 0.50] governance weight |
| **EmergenceEngine** | `cynic/engines/emergence_engine.py` (NEW) | Weight computation | Derives governance weight from value creation |

**6-Step Algorithm:**
1. Raw weight = sum(impact) / 100
2. Domain expert boost = 1.2x (if applicable)
3. Constrain to [0.01, 0.50] (axiom bounds)
4. Apply temporal decay (older value worth less)
5. Apply reciprocal duty (power > 40% pays cost)
6. Validate all 5 axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)

### Layer 3: Coordination (Multi-Creator Collaboration)

| Composant | Fichier | Rôle | Impact |
|-----------|---------|------|--------|
| **CoordinationEngine** | `cynic/engines/coordination_engine.py` (NEW) | Collaboration tracking | Fair credit distribution |

**Credit Distribution:**
- Primary creator: 40% of impact
- Contributors: 60% proportional to hours worked

---

## 3. Integration Points: How Phase 3 Hooks Into Existing Architecture

### Integration Point #1: ACCOUNT Step Enhancement

**Current State:**
```
JudgeOrchestrator._cycle_macro()
  → step_learn() [exists]
  → step_account() [STUB, needs enhancement]
  → step_emerge() [exists]
```

**Phase 3 Change:**
```
JudgeOrchestrator._cycle_macro()
  → step_learn() [unchanged]
  → step_account() [ENHANCED]
      ├── Create ValueCreation record (NEW)
      ├── Register with ValueCreationEngine (NEW)
      └── Return impact for EMERGE (NEW)
  → step_emerge() [unchanged, but now has impact data]
```

### Integration Point #2: EMERGE Step Enhancement

**Current:** E-Score and residual detection
**Phase 3 adds:** EmergenceEngine integration
```python
# In step_emerge():
governance_weights = {}
for creator_id in active_creators:
    gw = emergence_engine.compute_governance_weight(
        human_id=creator_id,
        decision_type=decision_type,
        time_period=30  # Last 30 days
    )
    governance_weights[creator_id] = gw.final_weight
```

### Integration Point #3: PBFT Consensus Update

**Current:**
```python
# PBFT takes equal weight per Dog (1/11)
consensus = pbft_consensus(dog_votes)
```

**Phase 3 vision (Phase 4+):**
```python
# PBFT could weight human votes by governance_weight
# e.g., creator with high impact gets higher PBFT weight
# (NOT in Phase 3, but architectural path)
```

---

## 4. Dataflow: End-to-End Proposal → Judgment → Impact → Governance

```
┌─────────────────────────────────────────────────────────────┐
│ Proposal Submission                                          │
│ (proposal_id, created_by, action, impact_estimate)         │
└──────────────────┬──────────────────────────────────────────┘
                   │
    ┌──────────────▼────────────────┐
    │ PERCEIVE: Normalize input     │
    └──────────────┬────────────────┘
                   │
    ┌──────────────▼────────────────┐
    │ JUDGE: 11 Dogs + PBFT         │
    │ (verdict: HOWL/WAG/GROWL/BARK)│
    └──────────────┬────────────────┘
                   │
    ┌──────────────▼────────────────┐
    │ DECIDE: Governance check      │
    │ (current: human veto)         │
    │ (Phase 4: weighted by impact) │
    └──────────────┬────────────────┘
                   │
    ┌──────────────▼────────────────┐
    │ ACT: Execute approved action  │
    │ record execution_success      │
    └──────────────┬────────────────┘
                   │
    ┌──────────────▼────────────────┐
    │ LEARN: Q-Table update         │
    │ (satisfaction rating)         │
    └──────────────┬────────────────┘
                   │
    ┌──────────────▼───────────────────────────────┐
    │ ACCOUNT: [PHASE 3 ENHANCEMENT]               │
    │ ├─ Measure impact (4D model)                 │
    │ ├─ Create ValueCreation record               │
    │ ├─ Register with ValueCreationEngine         │
    │ └─ Return impact metrics                     │
    └──────────────┬───────────────────────────────┘
                   │
    ┌──────────────▼───────────────────────────────┐
    │ EMERGE: [PHASE 3 ENHANCEMENT]                │
    │ ├─ Detect patterns (existing)                │
    │ ├─ Calculate governance weights (NEW)        │
    │ │  ├─ Call EmergenceEngine                   │
    │ │  └─ Compute weight per creator             │
    │ ├─ Distribute credit (NEW)                   │
    │ │  └─ Call CoordinationEngine                │
    │ └─ Update awareness                          │
    └──────────────┬───────────────────────────────┘
                   │
                   ▼
         ┌─────────────────────┐
         │ System Ready for     │
         │ Next Judgment       │
         │ (with improved      │
         │  governance weights)│
         └─────────────────────┘
```

---

## 5. Timeline: When Things Happen

### Task 1-2: Foundation (Day 1-2)
```
ValueCreation ──┐
                ├─→ (ready for engines)
GovernanceWeight─┘
```

### Task 3-3.5: ValueCreationEngine (Day 3-4)
```
ValueCreationEngine ──┐
                      ├─→ Profile: measure latency
                      └─→ ACCOUNT step can call it
```

### Task 4-4.5: EmergenceEngine (Day 5-6)
```
EmergenceEngine ───┐
                   ├─→ Profile: measure latency
                   └─→ EMERGE step can call it
```

### Task 5-5.5: Orchestrator Integration (Day 7-8)
```
ACCOUNT step enhancement
  + register with ValueCreationEngine
  + profile latency
```

### Task 6: CoordinationEngine (Day 9-10)
```
CoordinationEngine ──┐
                     └─→ EMERGE step calls for credit distribution
```

### Task 7: Integration Tests (Day 11-12)
```
Full pipeline: Proposal → CYNIC → ACCOUNT → EMERGE
             ↓
   Governance weights emerge from value
```

### Task 8: System Profiling (Day 13-15)
```
PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE
  Full cycle latency measurement at scale
  Bottleneck analysis
  Production readiness verification
```

---

## 6. What Phase 3 DOES NOT Change

- ✅ JudgeOrchestrator exists and works
- ✅ PBFT consensus stays 1-dog-1-vote (for Phase 3)
- ✅ 11 Dogs and their logic unchanged
- ✅ Axioms constraints unchanged
- ✅ Q-Learning loop unchanged
- ✅ CircuitBreaker logic unchanged
- ✅ LNSP nervous system (separate, not touched)
- ✅ Training fine-tuning (separate, not touched)

---

## 7. Phase 4+ Vision (NOT Phase 3)

Once Phase 3 works:

### Phase 4a: Weighted PBFT
```python
# Dogs vote, but PBFT weights by creator's governance weight
# High-impact creators' dog choices carry more weight
```

### Phase 4b: Federation
```python
# 3 regions sync ValueCreationEngine
# Governance weights computed locally, replicated globally
```

### Phase 4c: NEAR Integration
```python
# Governance weights → NEAR contract calls
# Verdicts execute on-chain
# GASdf burned to treasury
```

---

## 8. Architecture Validation Questions

**Before Phase 3 implementation, answer:**

1. **ValueCreation 4D Model**: Does 40/35/25/10 weighting match your vision?
   - [ ] Yes, this is right
   - [ ] Adjust weights (how?)
   - [ ] Different model altogether

2. **GovernanceWeight Bounds**: φ-bounded [0.01, 0.50] means no one person > 50% voting power?
   - [ ] Yes, this is right
   - [ ] Different bounds (what?)

3. **Temporal Decay**: Older value creation gradually loses weight (exponential decay, 5% per day)?
   - [ ] Yes, this is right
   - [ ] Different decay function (what?)

4. **Integration Point**: ACCOUNT & EMERGE enhancement as shown above?
   - [ ] Yes, this is the path
   - [ ] Different integration (how?)

5. **Credit Distribution**: Primary 40%, contributors 60% proportional to hours?
   - [ ] Yes, this is fair
   - [ ] Different distribution (how?)

---

## Summary Table: Phase 3 Integration

| Step | Current | Phase 3 | New Complexity |
|------|---------|---------|-----------------|
| PERCEIVE | ✅ Works | No change | - |
| JUDGE | ✅ Works | No change | - |
| DECIDE | ✅ Works | No change | - |
| ACT | ✅ Works | No change | - |
| LEARN | ✅ Works | No change | - |
| ACCOUNT | ⚠️ Stub | ENHANCED | Records impact via ValueCreationEngine |
| EMERGE | ✅ Works | ENHANCED | Computes governance weights via EmergenceEngine |
| **New** | - | ValueCreationEngine | Tracks + aggregates impact |
| **New** | - | EmergenceEngine | Computes weights from impact |
| **New** | - | CoordinationEngine | Fair credit distribution |

---

## Next Step: User Validation

**Before proceeding with Task 1, please confirm:**
1. ValueCreation 4D model weights (40/35/25/10)?
2. GovernanceWeight bounds ([0.01, 0.50])?
3. Integration points (ACCOUNT & EMERGE enhancements)?
4. Timeline and task order correct?

**Once validated, Phase 3 implementation can begin.**
