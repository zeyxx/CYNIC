# CYNIC Hypercube Topology & Axiom Scoring System
## Technical Specification v1.0

**Status**: Technical Specification
**Date**: 2026-02-23
**Format**: Machine + Human readable
**Integration**: CCM validation layer

---

## Abstract

This document formally specifies the **Hypercube topology** — a 35-dimensional + 1-meta evaluation space that scores any cognitive output against 5 axioms across 7 dimensions each.

The Hypercube is CCM's *validation mechanism*. It projects outputs into multi-dimensional space, enabling:
- Transparent scoring
- Geometric reasoning (distance, clustering, consensus)
- Axiom-grounded evaluation
- Emergent pattern detection

---

## 1. Hypercube Architecture

### 1.1 Structure Overview

```
HYPERCUBE = 5 AXIOMS × 7 DIMENSIONS + 1 META

Total: 36-dimensional evaluation space

        ┌─────────────────────────────────────────┐
        │          5 AXIOMS (Principal Axes)      │
        │  ─────────────────────────────────────  │
        │  FIDELITY (🐕 Truth/Authenticity)      │
        │  PHI (φ Structure/Harmony)              │
        │  VERIFY (✓ Evidence/Proof)              │
        │  CULTURE (⛩ Collective/Alignment)      │
        │  BURN (🔥 Simplicity/Efficiency)       │
        └─────────────────────────────────────────┘
                          ↓
        ┌─────────────────────────────────────────┐
        │   7 DIMENSIONS per Axiom (Sub-aspects)  │
        │  ─────────────────────────────────────  │
        │  D1: Fundamental    (weight: φ)         │
        │  D2: Primary        (weight: φ⁻¹)       │
        │  D3: Secondary      (weight: 1.0)       │
        │  D4: Tertiary       (weight: φ)         │
        │  D5: Support        (weight: φ⁻²)       │
        │  D6: Critical       (weight: φ⁻¹)       │
        │  D7: Meta           (weight: φ⁻¹)       │
        └─────────────────────────────────────────┘
                          ↓
                35 named dimensions
                + 1 unnameable (emergent)
                = 36 total
```

### 1.2 Axiom Definitions

#### FIDELITY (🐕 Water Element)
**Principle**: Loyalty to truth over comfort

**Purpose**: Does this output maintain integrity, coherence, and authenticity?

**Dimensions**:
| ID | Dimension | Question | Weight |
|----|-----------|----------|--------|
| F1 | Consistency | Does output contradict existing memory? | φ |
| F2 | Authenticity | Is this genuinely the system's conclusion? | φ⁻¹ |
| F3 | Coherence | Are internal claims logically consistent? | 1.0 |
| F4 | Veracity | Does output accept uncertainty honestly? | φ |
| F5 | Reversibility | Could this decision be undone if wrong? | φ⁻² |
| F6 | Transparency | Can we explain why this conclusion? | φ⁻¹ |
| F7 | Self-Doubt | Does output acknowledge its limits? | φ⁻¹ |

---

#### PHI (φ Earth Element)
**Principle**: Harmony, proportion, mathematical elegance

**Purpose**: Is the structure golden-ratio balanced?

**Dimensions**:
| ID | Dimension | Question | Weight |
|----|-----------|----------|--------|
| P1 | Proportionality | Are claims proportional to evidence? | φ |
| P2 | Elegance | Is the solution mathematically simple? | φ⁻¹ |
| P3 | Symmetry | Does pattern hold across domains? | 1.0 |
| P4 | Completeness | Are all necessary elements present? | φ |
| P5 | Parsimony | Minimum assumptions needed? | φ⁻² |
| P6 | Balance | No single element dominates unfairly? | φ⁻¹ |
| P7 | Emergence | Does whole exceed sum of parts? | φ⁻¹ |

---

#### VERIFY (✓ Metal Element)
**Principle**: Don't trust; verify through evidence

**Purpose**: What evidence supports this claim?

**Dimensions**:
| ID | Dimension | Question | Weight |
|----|-----------|----------|--------|
| V1 | Evidence | Is there empirical support? | φ |
| V2 | Testability | Can this hypothesis be falsified? | φ⁻¹ |
| V3 | Reproducibility | Would independent verification succeed? | 1.0 |
| V4 | Data Quality | Is data source trustworthy? | φ |
| V5 | Edge Cases | Does claim hold at boundaries? | φ⁻² |
| V6 | Critique Resistance | How robust to challenge? | φ⁻¹ |
| V7 | Alternative Explanations | Were other hypotheses considered? | φ⁻¹ |

---

#### CULTURE (⛩ Wood Element)
**Principle**: Culture is a moat; collective > individual

**Purpose**: Does this align with community values & norms?

**Dimensions**:
| ID | Dimension | Question | Weight |
|----|-----------|----------|--------|
| C1 | Collective Consensus | Do others agree? | φ |
| C2 | Value Alignment | Does this match group principles? | φ⁻¹ |
| C3 | Context Fit | Is this contextually appropriate? | 1.0 |
| C4 | Precedent | Does this honor prior decisions? | φ |
| C5 | Diversity Respect | Does this exclude legitimate voices? | φ⁻² |
| C6 | Resilience | Can community absorb this? | φ⁻¹ |
| C7 | Legacy | Will this strengthen or weaken culture? | φ⁻¹ |

---

#### BURN (🔥 Fire Element)
**Principle**: Don't extract, burn; simplicity wins

**Purpose**: Is this approach minimal and efficient?

**Dimensions**:
| ID | Dimension | Question | Weight |
|----|-----------|----------|--------|
| B1 | Necessity | Is every component needed? | φ |
| B2 | Cost | Resource consumption minimal? | φ⁻¹ |
| B3 | Clarity | Can a child understand this? | 1.0 |
| B4 | Speed | Does it execute quickly? | φ |
| B5 | Generality | Overfitted or broadly applicable? | φ⁻² |
| B6 | Maintenance | Low ongoing burden? | φ⁻¹ |
| B7 | Forgetting | Can unused parts be safely deleted? | φ⁻¹ |

---

### 1.3 Dimension Weights & Aggregation

**Weight Vector** (same for all axioms):

$$\vec{w} = [φ, φ^{-1}, 1.0, φ, φ^{-2}, φ^{-1}, φ^{-1}]$$

**Numerical values**:
- $φ = 1.618$
- $φ^{-1} = 0.618$
- $φ^{-2} = 0.382$
- $1.0 = 1.0$

**Sum of weights**:
$$\sum w_i = 1.618 + 0.618 + 1.0 + 1.618 + 0.382 + 0.618 + 0.618 = 6.472$$

---

## 2. Scoring Procedure

### 2.1 Per-Dimension Scoring

For each of 35 dimensions, score output O on scale 0-100:

$$\text{score}_d(O) \in [0, 100]$$

**Scoring guidelines**:
- **90-100**: Exemplary (strongly positive)
- **70-90**: Good (positive)
- **50-70**: Adequate (neutral)
- **30-50**: Problematic (negative)
- **0-30**: Fail (strongly negative)

### 2.2 Axiom Score Calculation

For axiom A with 7 dimension scores:

$$\text{axiom\_score}_A = \frac{\sum_{i=1}^{7} \text{score}_i \times w_i}{\sum_{i=1}^{7} w_i}$$

**Normalized to 0-100 scale**:

$$\text{AXIOM}_A = \frac{\text{axiom\_score}_A}{100}$$

Result: $\text{AXIOM}_A \in [0, 1]$

### 2.3 Q-Score (Final Score)

**Geometric mean of 5 axioms** (one weak axiom damages overall score):

$$Q\text{-}Score = 100 \times \sqrt[5]{\text{AXIOM}_{FIDELITY} \times \text{AXIOM}_{PHI} \times \text{AXIOM}_{VERIFY} \times \text{AXIOM}_{CULTURE} \times \text{AXIOM}_{BURN}}$$

Result: $Q\text{-}Score \in [0, 100]$

### 2.4 Verdict Mapping

| Q-Score | Verdict | Meaning | Action |
|---------|---------|---------|--------|
| ≥ 80 | **HOWL** 🟢 | Exceptional | Crystallize immediately |
| 50–80 | **WAG** 🟡 | Passes | Enter validation layer |
| 38.2–50 | **GROWL** 🟠 | Needs work | Reject or refine |
| < 38.2 | **BARK** 🔴 | Critical fail | Delete/restart |

**Threshold**: 38.2 = $φ^{-2} \times 100$ (φ-derived boundary)

---

## 3. Concrete Scoring Example

### 3.1 Decision: "Merge two LoRA specializations (finance + medicine)"

#### Step 1: Score 35 dimensions (0-100 each)

**FIDELITY scores**:
- F1 (Consistency): 75 (minor contradictions)
- F2 (Authenticity): 80 (genuine conclusion)
- F3 (Coherence): 70 (mostly logical)
- F4 (Veracity): 85 (honest about uncertainty)
- F5 (Reversibility): 65 (can unmerge, but costly)
- F6 (Transparency): 75 (reason clear)
- F7 (Self-Doubt): 80 (acknowledges risk)

Weighted sum: $75 \times 1.618 + 80 \times 0.618 + 70 \times 1.0 + 85 \times 1.618 + 65 \times 0.382 + 75 \times 0.618 + 80 \times 0.618$
$= 121.35 + 49.44 + 70 + 137.53 + 24.83 + 46.35 + 49.44 = 499$

**FIDELITY = 499 / 6.472 = 77.1%**

**PHI scores**: [85, 70, 75, 80, 60, 72, 78] → **74.2%**

**VERIFY scores**: [65, 70, 72, 65, 55, 68, 70] → **67.8%**

**CULTURE scores**: [78, 80, 75, 70, 85, 72, 68] → **75.1%**

**BURN scores**: [60, 65, 70, 62, 58, 60, 55] → **62.3%**

#### Step 2: Calculate Q-Score

$$Q = 100 \times \sqrt[5]{0.771 \times 0.742 \times 0.678 \times 0.751 \times 0.623}$$

$$Q = 100 \times \sqrt[5]{0.1518} = 100 \times 0.699 = 69.9 \approx 70$$

#### Step 3: Verdict

Q-Score = 70 → **WAG** 🟡 (passes, enters validation)

#### Step 4: Interpretation

- FIDELITY strong (77%) → maintains integrity
- VERIFY weak (68%) → needs more evidence
- BURN weak (62%) → may be overcomplex
- Overall: Acceptable but needs validation from community validators

---

## 4. Multi-Validator Consensus

### 4.1 Aggregating Multiple Scores

When k validators score the same output:

$$\text{consensus} = \text{GM}(Q_1, Q_2, \ldots, Q_k)$$

Where $Q_i$ is validator i's Q-Score.

**Example**: 3 validators score 70, 75, 65
$$\text{consensus} = \sqrt[3]{0.70 \times 0.75 \times 0.65} = \sqrt[3]{0.341} = 0.699 = 69.9$$

### 4.2 Validator Reputation Weighting

Optional: Weight validators by reputation:

$$\text{consensus}_{weighted} = \sqrt[k]{\prod_{i=1}^{k} Q_i^{r_i}}$$

Where $r_i$ is validator i's reputation score (higher = more trusted).

---

## 5. Geometric Properties

### 5.1 Distance in Hypercube

Two outputs A and B can be compared via:

$$d(A, B) = \sqrt{\sum_{i=1}^{35} (score_i^A - score_i^B)^2}$$

**Interpretation**:
- $d = 0$: Identical outputs
- $d < 5$: Very similar
- $5 < d < 15$: Related but different
- $d > 15$: Fundamentally different

### 5.2 Clustering in Axiom Space

Outputs can cluster in axiom-space by their strong/weak axes:

```
Example cluster: "Strategic decisions"
- High CULTURE (group consensus matters)
- High FIDELITY (ethical grounding)
- Low BURN (complexity acceptable)
- Moderate VERIFY (some uncertainty ok)

This cluster may differ from "Factual claims" cluster:
- High VERIFY (evidence crucial)
- High FIDELITY (truth matters)
- High BURN (simple explanations preferred)
- Lower CULTURE (objective facts)
```

---

## 6. Implementation Notes

### 6.1 Automation

Dimensions 1-35 can be partially automated:
- D1 (FIDELITY-Consistency): Query against memory
- D2 (PHI-Elegance): AST analysis for simplicity
- D3-7: Require human judgment or specialized AI

### 6.2 Caching

Q-Scores should be cached:
- Same output → same score (unless memory changes)
- Cache invalidation: when new data enters memory

### 6.3 Meta-Evaluation

**THE_UNNAMEABLE** (36th dimension):
- Measures how well these 35 dimensions capture true quality
- If high: framework is complete
- If low: Something important is missing

Calculate as explained variance:

$$\text{THE\_UNNAMEABLE} = 1 - \frac{\text{residual\_error}}{\text{total\_variance}}$$

---

## 7. Limitations

1. **Dimension Independence**: Dimensions not orthogonal (correlation assumed)
2. **Weight Subjectivity**: Why φ weights? (Philosophical choice, testable empirically)
3. **Score Calibration**: 0-100 scale arbitrary (could normalize differently)
4. **Axiom Coverage**: Do 5 axioms capture all relevant evaluations?

---

## 8. Future Extensions

- Time-series scoring (how does axiom-score change?)
- Multi-modal scoring (images, embeddings, not just text)
- Adversarial robustness (detect hallucinating validators)
- Domain-specific axioms (medical vs financial)

---

## 9. Integration Points

- **Input**: Any output O (text, embeddings, action)
- **Process**: Project to Hypercube → Score 35 dimensions → Aggregate
- **Output**: Q-Score [0-100] + Axiom breakdown + Verdict
- **Feedback loop**: CCM uses Hypercube scores to crystallize patterns

---

**Document Status**: ✅ Technical Specification (Ready for implementation)
**Next**: Map to actual CYNIC dimensions in old/new systems
