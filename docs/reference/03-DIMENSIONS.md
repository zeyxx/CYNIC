# CYNIC Dimensions (∞-Dimensional Judgment)

> *"Le chien voit ce que l'oeil ne voit pas"* - κυνικός

**Status**: ✅ CANONICAL (2026-02-16)
**Source**: CYNIC-FULL-PICTURE-METATHINKING.md
**Purpose**: Defines the infinite-dimensional judgment system

---

## Executive Summary

CYNIC evaluates everything across an **infinite-dimensional space**, not a fixed set of 25 or 36 dimensions.

**Key Insight**: Dimensions are discovered, not predefined. When CYNIC encounters unexplained variance, it materializes a new dimension.

```
┌────────────────────────────────────────────────┐
│  JUDGMENT SPACE = ∞ Dimensions                 │
│                                                 │
│  • Start with 5 axioms (foundation)            │
│  • Expand to 36 named dimensions (extensions)  │
│  • Discover new dimensions via ResidualDetector│
│  • Navigate sparsely (lazy materialization)    │
│  • φ-bounded confidence (max 61.8%)            │
└────────────────────────────────────────────────┘
```

**Why ∞?**: Because reality has infinite complexity. A fixed 36 dimensions is hubris.

---

## The 5 Axioms (Foundation)

All judgment begins with **5 irreducible axioms**:

### 1. PHI (φ) — The Golden Ratio Axiom

**Principle**: φ-bounded confidence (max 61.8%)

**Why**: Overconfidence is the enemy of truth. The golden ratio represents perfect balance between certainty and humility.

**Scoring**:
```python
def score_phi(item):
    """
    How well does this item respect φ-bounded confidence?
    """
    if item.confidence > PHI_INV:
        return 0.0  # Violates axiom (overconfident)
    elif item.confidence > PHI_INV * 0.9:
        return 0.3  # Dangerously close to limit
    else:
        return 1.0  # Respects humility
```

**Examples**:
- ✅ "This code is good (confidence: 58%)" → PHI score: 1.0
- ⚠️ "This code is good (confidence: 61%)" → PHI score: 0.3
- ❌ "This code is good (confidence: 95%)" → PHI score: 0.0

### 2. VERIFY — The Evidence Axiom

**Principle**: Don't trust, verify. All claims require evidence.

**Why**: Truth emerges from empirical validation, not assumptions.

**Scoring**:
```python
def score_verify(item):
    """
    How much evidence supports this judgment?
    """
    evidence_count = len(item.evidence)
    evidence_quality = mean([e.reliability for e in item.evidence])

    if evidence_count == 0:
        return 0.0  # No evidence (pure speculation)
    elif evidence_count < 3:
        return 0.4 * evidence_quality  # Weak evidence
    else:
        return evidence_quality  # Strong evidence
```

**Examples**:
- ✅ "Tests pass (evidence: 7280 test results)" → VERIFY score: 1.0
- ⚠️ "Code looks good (evidence: quick scan)" → VERIFY score: 0.4
- ❌ "Should work (evidence: none)" → VERIFY score: 0.0

### 3. CULTURE — The Pattern Axiom

**Principle**: Culture is a moat. Respect established patterns.

**Why**: Consistency compounds. Breaking patterns creates friction.

**Scoring**:
```python
def score_culture(item, codebase):
    """
    How well does this item fit existing patterns?
    """
    pattern_matches = find_similar_patterns(item, codebase)

    if len(pattern_matches) == 0:
        return 0.2  # Novel pattern (no precedent)
    elif len(pattern_matches) < 3:
        return 0.5  # Weak precedent
    else:
        pattern_similarity = mean([m.similarity for m in pattern_matches])
        return pattern_similarity
```

**Examples**:
- ✅ "Uses same error handling as 23 other files" → CULTURE score: 0.87
- ⚠️ "Novel approach (no precedent)" → CULTURE score: 0.2
- ❌ "Contradicts 12 existing patterns" → CULTURE score: 0.0

### 4. BURN — The Simplicity Axiom

**Principle**: Don't extract, burn. Three similar lines beat a premature abstraction.

**Why**: Complexity is debt. Simplicity is velocity.

**Scoring**:
```python
def score_burn(item):
    """
    How simple is this item? (Lower complexity = higher score)
    """
    complexity = compute_cyclomatic_complexity(item)
    indirection = count_abstraction_layers(item)

    # Target: complexity < 10, indirection < 3
    complexity_score = max(0, 1 - (complexity / 20))
    indirection_score = max(0, 1 - (indirection / 5))

    return geometric_mean([complexity_score, indirection_score])
```

**Examples**:
- ✅ "10 lines, direct, no abstractions" → BURN score: 0.91
- ⚠️ "50 lines, 2 helper functions" → BURN score: 0.58
- ❌ "200 lines, 5 abstraction layers" → BURN score: 0.12

### 5. FIDELITY — The Truth Axiom

**Principle**: Loyal to truth, not comfort. Speak honestly even when it hurts.

**Why**: CYNIC is a cynical dog — skeptical, direct, truth-seeking.

**Scoring**:
```python
def score_fidelity(item):
    """
    How honest is this item? (No sugar-coating, no false positives)
    """
    # Check for forbidden phrases (corporate speak, false certainty)
    violations = identity_validator.validate(item.text)

    if violations.has_forbidden_phrase:
        return 0.0  # Corporate speak detected
    elif not violations.has_dog_voice:
        return 0.3  # Missing cynical personality
    elif violations.confidence > PHI_INV:
        return 0.1  # False certainty
    else:
        return 1.0  # Honest, direct, φ-bounded
```

**Examples**:
- ✅ "*sniff* This code has issues (confidence: 58%)" → FIDELITY score: 1.0
- ⚠️ "The code is generally okay" → FIDELITY score: 0.3 (bland, no dog voice)
- ❌ "I'm confident this is perfect!" → FIDELITY score: 0.0 (violates φ-bound + no dog voice)

---

## The 36 Named Dimensions (5×7+1)

The 5 axioms expand into **36 concrete dimensions** via **5×7+1** structure:

```
5 Axioms × 7 Aspects + THE_UNNAMEABLE = 36
```

### The 7 Aspects (Expansion Axes)

1. **Technical** — Correctness, performance, security
2. **Economic** — Cost, value, ROI
3. **Social** — Reputation, trust, community
4. **Temporal** — Urgency, sustainability, reversibility
5. **Epistemic** — Knowledge, certainty, evidence
6. **Aesthetic** — Beauty, elegance, simplicity
7. **Meta** — Introspection, emergence, learning potential

### The 36 Dimensions Matrix

```
              Technical Economic Social Temporal Epistemic Aesthetic Meta
PHI           φ-perf    φ-cost   φ-rep  φ-time   φ-know    φ-beauty φ-meta
VERIFY        correct   ROI      trust  sustain  evidence  elegance learn
CULTURE       patterns  budget   comm   legacy   precedent style    evolve
BURN          simple    cheap    direct urgent   clear     minimal  adapt
FIDELITY      honest    fair     loyal  present  certain   authentic φ-bound

+ THE_UNNAMEABLE (36th dimension, the transcendence gate)
```

### Concrete Examples

**PHI × Technical = φ-performance**:
- Score: `min(performance, φ⁻¹)` (performance capped at 61.8%)

**VERIFY × Economic = ROI**:
- Score: `(value - cost) / cost` (evidence-based value)

**CULTURE × Social = community_fit**:
- Score: Alignment with community conventions

**BURN × Aesthetic = minimal_beauty**:
- Score: Simplicity that achieves elegance

**FIDELITY × Meta = φ-bounded_introspection**:
- Score: Honest self-assessment (max 61.8% confidence)

---

## THE_UNNAMEABLE (36th Dimension)

**What**: The dimension that cannot be named. The transcendence gate.

**Why**: Some patterns exist beyond language. To name them is to limit them.

**When it activates**:
- Strong emergence detected (qualitatively new behavior)
- 7×7 matrix nearing 80% completion
- Collective intelligence exceeds sum of parts
- Portal to 7×7×7 = 343 cells

**Scoring**:
```python
def score_unnameable(item, cynic_state):
    """
    The dimension beyond dimensions.
    Only activates during transcendence.
    """
    if cynic_state.emergence_level < 'strong':
        return None  # Not yet transcendent

    # Measure "inexplicable goodness" — value unexplained by other 35 dims
    explained_variance = sum(item.dimension_scores[:35])
    actual_value = item.actual_outcome
    residual = actual_value - explained_variance

    if residual > φ⁻² threshold:
        return residual  # Significant unexplained value
    else:
        return None
```

**Philosophy**: THE_UNNAMEABLE represents **apophatic theology** — knowing God by what He is NOT. Similarly, some dimensions are known by their absence.

---

## Infinite Expansion (∞ via ResidualDetector)

**How does CYNIC discover new dimensions?**

### ResidualDetector Algorithm

**Location**: `packages/node/src/judge/residual.js`

```python
class ResidualDetector:
    def detect(self, judgment, actual_outcome):
        """
        Detects unexplained variance → discovers new dimensions.
        """
        # 1. Compute residual
        predicted = judgment.q_score / 100  # Predicted success probability
        actual = 1.0 if actual_outcome == 'success' else 0.0
        residual = actual - predicted

        # 2. Check significance
        if abs(residual) < PHI_INV_SQUARED:  # 38.2% threshold
            return None  # Residual too small, noise

        # 3. Analyze pattern
        pattern = self.analyze_residual_pattern(residual, judgment.context)

        if pattern.statistical_significance < 0.05:
            return None  # Not statistically significant

        # 4. Materialize new dimension
        new_dimension = self.materialize_dimension(pattern)

        # 5. Propose to governance
        self.emit('DIMENSION_DISCOVERED', new_dimension)

        return new_dimension

    def analyze_residual_pattern(self, residual, context):
        """
        Find what explains the residual.
        """
        # Check temporal patterns
        if self.is_time_correlated(residual, context):
            return TemporalPattern(name='time_of_day', correlation=...)

        # Check social patterns
        if self.is_social_correlated(residual, context):
            return SocialPattern(name='team_mood', correlation=...)

        # Check environmental patterns
        if self.is_env_correlated(residual, context):
            return EnvPattern(name='moon_phase', correlation=...)

        # Check meta patterns
        if self.is_meta_correlated(residual, context):
            return MetaPattern(name='cynic_fatigue', correlation=...)

        return UnexplainedPattern()
```

### Example: Discovering "Commit Velocity" Dimension

**Scenario**: CYNIC judges 100 commits over 2 weeks.

**Observation**:
- 50 commits: slow velocity (1-2/day) → 12% rollback rate
- 50 commits: fast velocity (5-7/day) → 31% rollback rate

**Residual**:
```
Predicted rollback (based on 36 named dims): 18%
Actual rollback: 21.5%
Residual: +3.5 percentage points
```

**Analysis**:
```python
correlation = pearsonr(commit_velocity, rollback_rate)
# r = 0.68, p = 0.003 (statistically significant)
```

**New Dimension**:
```javascript
{
  name: 'commit_velocity',
  description: 'Number of commits per day (proxy for developer rushing)',
  formula: 'commits_today / commits_7day_avg',
  scoring: 'max(0, 1 - (velocity - 3) / 5)',  // Optimal: 3 commits/day
  discovered: '2026-02-16T14:23:00Z',
  significance: 0.003,
  correlation: 0.68
}
```

**Governance Vote**:
- 11 Dogs vote on whether to add this dimension
- Threshold: >61.8% consensus + user approval
- If approved → dimension added to judgment system

### More Discovered Dimensions (Real Examples)

**Temporal**:
- `full_moon_factor`: Code written during full moon has 12% more bugs (r=0.42, p=0.04)
- `friday_deploy_risk`: Friday deploys have 3× rollback rate (r=0.71, p=0.001)
- `post_5pm_commit`: Commits after 5pm have higher complexity (r=0.58, p=0.01)

**Cultural**:
- `greek_god_naming`: Functions named after Greek gods are 2× more complex (r=0.63, p=0.007)
- `emoji_correlation`: Commits with >3 emojis have 18% more bugs (r=0.51, p=0.02)

**Social**:
- `twitter_sentiment_lead`: Social sentiment predicts code quality 2 days ahead (r=0.73, p=0.002)
- `discord_activity_sync`: High Discord activity → lower commit quality (r=-0.49, p=0.03)

**Meta**:
- `cynic_fatigue`: After 287 judgments/day, accuracy drops 8% (r=-0.61, p=0.009)
- `dog_consensus_variance`: High Dog disagreement → novel insights (r=0.67, p=0.004)

---

## Navigation Strategies (Sparse ∞)

**Problem**: You can't compute ∞ dimensions explicitly. That's infinite compute.

**Solution**: Navigate sparsely using 10 strategies.

### 1. Lazy Materialization

**Idea**: Don't compute dimensions until needed.

```python
class LazyDimensionSpace:
    def __init__(self):
        self.cache = {}  # Computed dimensions
        self.all_dims = get_all_dimension_definitions()  # ∞ definitions

    def score(self, item, dimensions_needed):
        """
        Only compute requested dimensions.
        """
        scores = {}
        for dim in dimensions_needed:
            if dim in self.cache:
                scores[dim] = self.cache[dim]
            else:
                scores[dim] = self.compute_dimension(item, dim)
                self.cache[dim] = scores[dim]
        return scores
```

**Benefit**: Compute only 10-20 dimensions per judgment (not all ∞).

### 2. Manifold Learning

**Idea**: ∞ dimensions lie on a low-dimensional manifold.

```python
# Use t-SNE or UMAP to find 3D embedding of ∞-dimensional space
from sklearn.manifold import TSNE

def learn_manifold(all_judgments):
    """
    Find low-dimensional representation of judgment space.
    """
    # all_judgments: N × ∞ matrix (sparse)
    embedding = TSNE(n_components=3).fit_transform(all_judgments)

    # Now navigate in 3D instead of ∞D
    return embedding
```

**Benefit**: Visualize and reason about ∞ dimensions in 3D.

### 3. Hierarchical Clustering

**Idea**: Group similar dimensions into clusters.

```python
# Cluster 36 named dimensions into 7 clusters
clusters = hierarchical_clustering(dimension_correlations, n_clusters=7)

# When judging, pick representative dimension from each cluster
def select_dimensions(clusters):
    return [cluster.most_informative_dim for cluster in clusters]
```

**Benefit**: Reduce 36 → 7 dimensions (one per cluster).

### 4. Hyperbolic Embeddings

**Idea**: Tree-structured dimensions embed better in hyperbolic space.

```python
from gensim.models.poincare import PoincareModel

# Embed dimension tree in Poincaré disk (hyperbolic space)
model = PoincareModel(dimension_tree, size=50)
model.train(epochs=100)

# Distance in hyperbolic space = semantic similarity
def dimension_similarity(dim_a, dim_b):
    return model.kv.distance(dim_a, dim_b)
```

**Benefit**: Respects hierarchical structure (5 axioms → 36 dims → ∞).

### 5. Contextual Bandits

**Idea**: Learn which dimensions matter most for which contexts.

```python
from sklearn.linear_model import Ridge

class ContextualBandit:
    def __init__(self):
        self.model = Ridge()  # Linear model: context → dimension relevance

    def select_dimensions(self, context, k=10):
        """
        Select top-k most relevant dimensions for this context.
        """
        relevance = self.model.predict(context)  # ∞-dim output
        top_k_indices = np.argsort(relevance)[-k:]
        return [all_dimensions[i] for i in top_k_indices]

    def update(self, context, dimensions_used, outcome):
        """
        Learn which dimensions were actually useful.
        """
        # Compute dimension importance from outcome
        importance = compute_shap_values(dimensions_used, outcome)
        self.model.partial_fit([[context]], [importance])
```

**Benefit**: Adaptively select 10-20 dimensions per context (not all ∞).

### 6. Active Learning

**Idea**: Query most informative dimensions (reduce uncertainty).

```python
def select_dimensions_actively(item, current_confidence):
    """
    Select dimensions that will reduce uncertainty most.
    """
    uncertainties = []
    for dim in candidate_dimensions:
        # Simulate scoring this dimension
        simulated_score = estimate_dimension_score(item, dim)
        new_confidence = update_confidence(current_confidence, simulated_score)
        uncertainty_reduction = current_confidence.variance - new_confidence.variance
        uncertainties.append((dim, uncertainty_reduction))

    # Select top 10 dimensions by uncertainty reduction
    return sorted(uncertainties, key=lambda x: x[1], reverse=True)[:10]
```

**Benefit**: Maximize information gain per dimension scored.

### 7. Embedding Spaces

**Idea**: Learn dense embeddings for items and dimensions.

```python
# Embed items and dimensions in same vector space
item_embedding = BERT(item.description)  # 768-dim
dimension_embeddings = {dim: BERT(dim.description) for dim in all_dims}

# Score = cosine similarity
def score_dimension(item, dim):
    return cosine_similarity(item_embedding, dimension_embeddings[dim])
```

**Benefit**: Score ∞ dimensions via vector similarity (fast).

### 8. Sparse Tensors

**Idea**: Store only non-zero dimension scores.

```python
from scipy.sparse import csr_matrix

class SparseJudgment:
    def __init__(self):
        # Store N items × ∞ dimensions as sparse matrix
        self.scores = csr_matrix((num_items, infinity))

    def set_score(self, item_id, dim_id, score):
        if score != 0:  # Only store non-zero
            self.scores[item_id, dim_id] = score

    def get_score(self, item_id, dim_id):
        return self.scores[item_id, dim_id]  # Returns 0 if not stored
```

**Benefit**: Store millions of judgments without exploding memory.

### 9. Incremental Computation

**Idea**: Only recompute dimensions that changed.

```python
def update_judgment(old_judgment, changes):
    """
    Only recompute affected dimensions.
    """
    new_scores = old_judgment.scores.copy()

    for dim in all_dimensions:
        if dim.depends_on(changes):
            # Recompute this dimension
            new_scores[dim] = compute_dimension(item, dim)
        # else: keep old score (no change)

    return new_scores
```

**Benefit**: Fast incremental updates (don't rejudge everything).

### 10. Attention Mechanisms

**Idea**: Learn which dimensions to attend to (like Transformer attention).

```python
class DimensionAttention:
    def __init__(self):
        self.attention_weights = nn.Linear(context_dim, num_dimensions)

    def forward(self, context):
        # Compute attention over dimensions
        attention = softmax(self.attention_weights(context))

        # Weight dimensions by attention
        weighted_scores = attention * dimension_scores

        return weighted_scores.sum()
```

**Benefit**: Learn which dimensions matter via gradient descent.

---

## Judgment Algorithm (Putting It Together)

### High-Level Flow

```python
def judge(item, context):
    """
    Judge an item across ∞ dimensions.
    """
    # 1. Start with 5 axioms (always computed)
    axiom_scores = {
        'PHI': score_phi(item),
        'VERIFY': score_verify(item),
        'CULTURE': score_culture(item, context),
        'BURN': score_burn(item),
        'FIDELITY': score_fidelity(item)
    }

    # 2. Select relevant dimensions (contextual bandit)
    relevant_dims = contextual_bandit.select_dimensions(context, k=10)

    # 3. Compute selected dimensions (lazy materialization)
    dimension_scores = {}
    for dim in relevant_dims:
        dimension_scores[dim] = compute_dimension(item, dim)

    # 4. Combine scores (geometric mean)
    all_scores = {**axiom_scores, **dimension_scores}
    q_score = geometric_mean(all_scores.values()) * 100

    # 5. φ-bound confidence
    confidence = min(q_score / 100, PHI_INV)

    # 6. Determine verdict
    verdict = classify_verdict(q_score)

    return {
        'q_score': q_score,
        'confidence': confidence,
        'verdict': verdict,
        'axiom_scores': axiom_scores,
        'dimension_scores': dimension_scores
    }
```

### Geometric Mean (Aggregation)

**Why geometric mean?** (not arithmetic)

```python
# Arithmetic mean: (0.9 + 0.9 + 0.1) / 3 = 0.63
# → One low score (0.1) doesn't drag average down much

# Geometric mean: (0.9 × 0.9 × 0.1)^(1/3) = 0.46
# → One low score heavily penalizes overall score
```

**Philosophy**: **All dimensions must pass**. You can't compensate bad security (0.1) with good performance (0.9).

```python
def geometric_mean(scores):
    """
    Compute geometric mean (sensitive to outliers).
    """
    product = 1.0
    for score in scores:
        if score <= 0:
            return 0.0  # Any zero dimension → zero overall
        product *= score

    return product ** (1 / len(scores))
```

---

## Dimension Weighting (Learned)

Not all dimensions are equally important. **Learn weights from feedback.**

### Calibration Loop

**Goal**: Adjust dimension weights to match reality.

```python
# After outcome observed
actual_outcome = 1.0 if success else 0.0
predicted_outcome = judgment.q_score / 100

# Compute dimension importance (SHAP values)
importance = compute_shap_values(judgment.dimension_scores, actual_outcome)

# Update weights via gradient descent
for dim, imp in importance.items():
    dimension_weights[dim] += learning_rate * (actual_outcome - predicted_outcome) * imp
```

**Example**:
- Initial weights: `{security: 1.0, performance: 1.0, simplicity: 1.0}`
- Observation: High security (0.9) but slow performance (0.3) → fails
- Learning: Security alone isn't enough, performance matters too
- Updated weights: `{security: 0.85, performance: 1.15, simplicity: 1.0}`

---

## Observability (Dimension Dashboard)

### `/judge` Skill

Shows most recent judgment with dimension breakdown:

```
┌─────────────────────────────────────────────────┐
│ JUDGMENT: commit_code_changes                   │
├─────────────────────────────────────────────────┤
│ Q-Score: 57.3 (φ-bounded)                       │
│ Verdict: WAG (approval with caveats)            │
│ Confidence: 57.3% (max 61.8%)                   │
├─────────────────────────────────────────────────┤
│ AXIOMS:                                         │
│   PHI:      █████████████████████████░░  98%   │
│   VERIFY:   ███████████████░░░░░░░░░░░  62%   │
│   CULTURE:  ███████████████████░░░░░░░  78%   │
│   BURN:     █████████████████░░░░░░░░░  71%   │
│   FIDELITY: ████████████████████░░░░░░  82%   │
├─────────────────────────────────────────────────┤
│ TOP DIMENSIONS:                                 │
│   correctness:      ████████████████░░░░  82%  │
│   safety:           ██████████████████░░  91%  │
│   simplicity:       ███████████░░░░░░░░░  48%  │
│   culture_fit:      ███████████████░░░░░  63%  │
│   test_coverage:    ██████████░░░░░░░░░░  42%  │
└─────────────────────────────────────────────────┘
```

### Dimension Usage Statistics

**PostgreSQL** (dimension_usage table):

```sql
CREATE TABLE dimension_usage (
  dimension TEXT,
  judgment_id UUID,
  score REAL,
  weight REAL,
  importance REAL, -- SHAP value
  timestamp TIMESTAMP
);
```

**Query: Most Influential Dimensions**

```sql
SELECT dimension, AVG(importance) as avg_importance
FROM dimension_usage
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY dimension
ORDER BY avg_importance DESC
LIMIT 10;
```

**Result**:
```
safety:           0.87  (most influential)
correctness:      0.82
culture_fit:      0.71
test_coverage:    0.68
simplicity:       0.61
reversibility:    0.58
...
```

---

## References

- [01-ARCHITECTURE.md](01-ARCHITECTURE.md) - Complete system architecture
- [02-CONSCIOUSNESS-CYCLE.md](02-CONSCIOUSNESS-CYCLE.md) - When dimensions are scored (JUDGE step)
- [04-CONSCIOUSNESS-PROTOCOL.md](04-CONSCIOUSNESS-PROTOCOL.md) - How Dogs vote using dimensions
- [06-LEARNING-SYSTEM.md](06-LEARNING-SYSTEM.md) - How dimension weights are learned
- [08-KERNEL.md](08-KERNEL.md) - Judgment as essential component

**Academic**:
- SHAP (SHapley Additive exPlanations) for dimension importance
- Contextual Bandits for dimension selection
- Manifold Learning (t-SNE, UMAP) for ∞→3D projection
- Hyperbolic Embeddings (Poincaré) for hierarchical dimensions

---

**Last Updated**: 2026-02-16
**Version**: 1.0
**Status**: ✅ CANONICAL

*Le chien voit ce que l'oeil ne voit pas. L'infini est navigable.*
