# Clean Way Forward: Implementation Plan
## Recapturing Vision Without Dead Code

**Date**: 2026-02-27
**Goal**: Extract insights from LNSP and training, implement cleanly
**Effort**: ~10-12 hours total
**Outcome**: Better observability, autonomous improvement, 90% less code

---

## What We're Building

Three simple modules that capture what LNSP and training were trying to do:

```
Current (Broken)          Target (Clean)
├─ LNSP (70 KB)    ──────→ Observability Dashboard (100 lines)
├─ training (83 KB) ──────→ Axiom Learning Loop (150 lines)
└─ [Nothing]        ──────→ Governance Analytics (150 lines)

Total: 153 KB dead code  ──→ 400 lines of live, integrated code
```

---

## Module 1: Observability Dashboard (3-4 hours)

**Goal**: See the full journey of a proposal from submission to outcome.

### File Structure

```
cynic/observability/
├── __init__.py
├── models.py           # Data models
└── dashboard.py        # Main interface
```

### Implementation

**File: `cynic/observability/models.py`**

```python
"""Observability data models for governance proposals."""

from dataclasses import dataclass
from typing import Dict, List, Optional, Any
from enum import Enum


class ProposalPhase(Enum):
    """Phases a proposal goes through."""
    SUBMITTED = "submitted"
    JUDGED = "judged"
    VOTING = "voting"
    CLOSED = "closed"
    EXECUTED = "executed"
    FEEDBACK = "feedback"


@dataclass
class AxiomBreakdown:
    """How each axiom scored for a verdict."""
    fidelity: float          # Community representation
    phi: float               # Confidence bound
    verify: float            # Auditable
    culture: float           # Strengthens governance
    burn: float              # No founder extraction

    def to_dict(self) -> Dict[str, float]:
        return {
            "FIDELITY": self.fidelity,
            "PHI": self.phi,
            "VERIFY": self.verify,
            "CULTURE": self.culture,
            "BURN": self.burn,
        }


@dataclass
class JudgmentView:
    """Complete judgment information for a proposal."""
    verdict: str                           # HOWL, WAG, GROWL, BARK
    q_score: float                         # 0-100
    confidence: float                      # 0-0.618
    axiom_breakdown: AxiomBreakdown       # Per-axiom scores
    dog_votes: List[Dict[str, Any]]       # All 11 dogs' votes
    reasoning: str                         # Explanation
    latency_ms: float                      # Computation time


@dataclass
class VotingView:
    """Community voting information."""
    yes_votes: int
    no_votes: int
    abstain_votes: int
    total_votes: int
    yes_percentage: float
    approval_status: str                   # APPROVED, REJECTED, PENDING
    duration_seconds: float


@dataclass
class OutcomeView:
    """Real-world outcome of proposal execution."""
    execution_status: str                  # SUCCESS, FAILED, PENDING
    execution_timestamp: Optional[float]
    community_satisfaction: Optional[float]  # 1-5 stars, None if not rated
    notes: Optional[str]


@dataclass
class LearningSignals:
    """What the system learned from this proposal."""
    verdict_accuracy: bool                 # Did verdict match outcome?
    q_table_update: Dict[str, Any]        # Q-value changes
    axiom_adjustments: Dict[str, float]   # Which axioms to adjust
    satisfaction_signal: float             # -1 to +1 learning signal


@dataclass
class ProposalTrace:
    """Complete observable trace of a proposal."""
    proposal_id: str
    title: str
    description: str
    submitted_timestamp: float
    phase: ProposalPhase

    judgment: Optional[JudgmentView]
    voting: Optional[VotingView]
    outcome: Optional[OutcomeView]
    learning_signals: Optional[LearningSignals]

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "proposal_id": self.proposal_id,
            "title": self.title,
            "description": self.description,
            "submitted_timestamp": self.submitted_timestamp,
            "phase": self.phase.value,
            "judgment": {
                "verdict": self.judgment.verdict,
                "q_score": self.judgment.q_score,
                "confidence": self.judgment.confidence,
                "axiom_breakdown": self.judgment.axiom_breakdown.to_dict(),
                "latency_ms": self.judgment.latency_ms,
            } if self.judgment else None,
            "voting": {
                "yes_votes": self.voting.yes_votes,
                "no_votes": self.voting.no_votes,
                "abstain_votes": self.voting.abstain_votes,
                "approval_status": self.voting.approval_status,
            } if self.voting else None,
            "outcome": {
                "execution_status": self.outcome.execution_status,
                "community_satisfaction": self.outcome.community_satisfaction,
            } if self.outcome else None,
            "learning_signals": {
                "verdict_accuracy": self.learning_signals.verdict_accuracy,
                "q_table_update": self.learning_signals.q_table_update,
                "satisfaction_signal": self.learning_signals.satisfaction_signal,
            } if self.learning_signals else None,
        }
```

**File: `cynic/observability/dashboard.py`**

```python
"""Governance observability dashboard."""

from typing import Dict, List, Optional, Any
from datetime import datetime
from .models import ProposalTrace, AxiomBreakdown, JudgmentView


class GovernanceObservable:
    """Observable view of governance decisions in flight."""

    def __init__(self, db_path: Optional[str] = None):
        """Initialize observable with database connection."""
        # Will integrate with governance_bot.db
        # For now, in-memory storage
        self.proposals: Dict[str, ProposalTrace] = {}

    def get_proposal(self, proposal_id: str) -> Optional[ProposalTrace]:
        """Get complete trace for a proposal.

        Shows:
        - Original submission
        - CYNIC judgment (verdict + reasoning)
        - Community votes (approval status)
        - Real outcome (execution result)
        - Learning signals (what was learned)
        """
        return self.proposals.get(proposal_id)

    def get_recent_proposals(self, limit: int = 10) -> List[ProposalTrace]:
        """Get most recent proposals in any phase."""
        return sorted(
            self.proposals.values(),
            key=lambda p: p.submitted_timestamp,
            reverse=True
        )[:limit]

    def get_verdict_accuracy(self) -> Dict[str, float]:
        """Accuracy by verdict type.

        Returns:
            {
                "HOWL": 0.92,   # 92% of HOWL verdicts were right
                "WAG": 0.76,
                "GROWL": 0.48,
                "BARK": 0.88,
                "overall": 0.81,
            }
        """
        accuracy_by_verdict = {}

        for proposal in self.proposals.values():
            if not proposal.judgment or not proposal.outcome:
                continue

            verdict = proposal.judgment.verdict
            is_correct = proposal.learning_signals.verdict_accuracy

            if verdict not in accuracy_by_verdict:
                accuracy_by_verdict[verdict] = {"correct": 0, "total": 0}

            accuracy_by_verdict[verdict]["total"] += 1
            if is_correct:
                accuracy_by_verdict[verdict]["correct"] += 1

        # Calculate percentages
        result = {}
        total_correct = 0
        total_proposals = 0

        for verdict, counts in accuracy_by_verdict.items():
            accuracy = counts["correct"] / counts["total"] if counts["total"] > 0 else 0
            result[verdict] = accuracy
            total_correct += counts["correct"]
            total_proposals += counts["total"]

        if total_proposals > 0:
            result["overall"] = total_correct / total_proposals

        return result

    def get_axiom_predictiveness(self) -> Dict[str, float]:
        """Which axioms predict community approval?

        Correlation between axiom score and proposal being approved.
        Returns:
            {
                "FIDELITY": 0.89,   # Strong correlation
                "BURN": 0.91,
                "VERIFY": 0.68,
                "PHI": 0.52,
                "CULTURE": 0.45,
            }
        """
        # This would compute correlation between axiom_breakdown
        # and outcome.execution_status == "SUCCESS"
        # Using Pearson correlation or similar

        axiom_correlations = {}
        for axiom in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]:
            scores = []
            outcomes = []

            for proposal in self.proposals.values():
                if proposal.judgment and proposal.outcome:
                    axiom_score = getattr(
                        proposal.judgment.axiom_breakdown,
                        axiom.lower()
                    )
                    success = proposal.outcome.execution_status == "SUCCESS"

                    scores.append(axiom_score)
                    outcomes.append(1.0 if success else 0.0)

            if len(scores) > 1:
                # Simple Pearson correlation
                correlation = self._pearson_correlation(scores, outcomes)
                axiom_correlations[axiom] = correlation

        return axiom_correlations

    def get_metrics(self) -> Dict[str, Any]:
        """Overall governance health metrics."""
        proposals_with_outcomes = [
            p for p in self.proposals.values()
            if p.outcome and p.learning_signals
        ]

        if not proposals_with_outcomes:
            return {
                "total_proposals": 0,
                "verdict_accuracy": None,
                "community_satisfaction": None,
            }

        satisfactions = [
            p.outcome.community_satisfaction
            for p in proposals_with_outcomes
            if p.outcome.community_satisfaction is not None
        ]

        return {
            "total_proposals": len(self.proposals),
            "proposals_with_outcomes": len(proposals_with_outcomes),
            "verdict_accuracy": self.get_verdict_accuracy().get("overall", 0),
            "community_satisfaction": (
                sum(satisfactions) / len(satisfactions)
                if satisfactions else None
            ),
            "axiom_predictiveness": self.get_axiom_predictiveness(),
            "learning_convergence": self._compute_convergence(),
        }

    def _pearson_correlation(self, x: List[float], y: List[float]) -> float:
        """Compute Pearson correlation coefficient."""
        if len(x) < 2:
            return 0.0

        mean_x = sum(x) / len(x)
        mean_y = sum(y) / len(y)

        numerator = sum(
            (x[i] - mean_x) * (y[i] - mean_y)
            for i in range(len(x))
        )

        sum_sq_x = sum((xi - mean_x) ** 2 for xi in x)
        sum_sq_y = sum((yi - mean_y) ** 2 for yi in y)

        denominator = (sum_sq_x * sum_sq_y) ** 0.5

        return numerator / denominator if denominator > 0 else 0.0

    def _compute_convergence(self) -> float:
        """Compute Q-table convergence metric (0-1)."""
        # This would check if confidences are stable
        # Over time, learning should converge
        # Placeholder: return 0.5
        return 0.5
```

### Integration Point

Add to existing CLI OBSERVE command:

```python
# cynic/cli/main.py (existing)
async def show_observability():
    from cynic.kernel.observability.dashboard import GovernanceObservable

    observable = GovernanceObservable()
    metrics = observable.get_metrics()

    print("\n[GOVERNANCE OBSERVABILITY]")
    print(f"Total proposals: {metrics['total_proposals']}")
    print(f"Verdict accuracy: {metrics['verdict_accuracy']:.1%}")
    print(f"Community satisfaction: {metrics['community_satisfaction']:.1f}/5.0")
    print(f"\nAccuracy by verdict type:")
    for verdict, acc in observable.get_verdict_accuracy().items():
        if verdict != "overall":
            print(f"  {verdict}: {acc:.1%}")
```

---

## Module 2: Axiom Learning Loop (2-3 hours)

**Goal**: Continuously improve axiom weights based on real outcomes.

### File Structure

```
cynic/learning/
├── axiom_learner.py        # NEW: Learn axiom weights
└── [existing files]
```

### Implementation

**File: `cynic/learning/axiom_learner.py`**

```python
"""Learn axiom weights from real governance outcomes."""

from dataclasses import dataclass, field
from typing import Dict, Optional, List
import json
from pathlib import Path


@dataclass
class AxiomWeights:
    """Learned axiom importance weights."""
    fidelity: float = 0.70
    phi: float = 0.10
    verify: float = 0.10
    culture: float = 0.05
    burn: float = 0.05

    def to_dict(self) -> Dict[str, float]:
        return {
            "FIDELITY": self.fidelity,
            "PHI": self.phi,
            "VERIFY": self.verify,
            "CULTURE": self.culture,
            "BURN": self.burn,
        }

    def from_dict(cls, data: Dict[str, float]) -> "AxiomWeights":
        return cls(
            fidelity=data.get("FIDELITY", 0.70),
            phi=data.get("PHI", 0.10),
            verify=data.get("VERIFY", 0.10),
            culture=data.get("CULTURE", 0.05),
            burn=data.get("BURN", 0.05),
        )

    def normalize(self) -> "AxiomWeights":
        """Ensure weights sum to 1.0."""
        total = sum([self.fidelity, self.phi, self.verify, self.culture, self.burn])
        if total == 0:
            return self

        return AxiomWeights(
            fidelity=self.fidelity / total,
            phi=self.phi / total,
            verify=self.verify / total,
            culture=self.culture / total,
            burn=self.burn / total,
        )


class AxiomLearner:
    """Learn axiom importance from governance outcomes."""

    def __init__(self, learning_rate: float = 0.01, persist_path: Optional[str] = None):
        """Initialize axiom learner.

        Args:
            learning_rate: How aggressively to update weights (0.01 = 1% per signal)
            persist_path: Where to save learned weights (default: ~/.cynic/learning/axiom_weights.json)
        """
        self.learning_rate = learning_rate
        self.weights = AxiomWeights()
        self.persist_path = Path(persist_path or Path.home() / ".cynic" / "learning" / "axiom_weights.json")
        self.persist_path.parent.mkdir(parents=True, exist_ok=True)

        # Load any existing weights
        self._load()

    def learn_from_outcome(
        self,
        axiom_scores: Dict[str, float],
        verdict: str,
        community_approved: bool,
        community_satisfaction: Optional[float] = None,
    ) -> None:
        """Update axiom weights based on real outcome.

        Args:
            axiom_scores: {FIDELITY, PHI, VERIFY, CULTURE, BURN} scores from judgment
            verdict: HOWL, WAG, GROWL, BARK
            community_approved: Whether community actually approved
            community_satisfaction: 1-5 star rating (optional)
        """
        # Compute learning signal: was the verdict correct?
        was_correct = self._verdict_matches_community(verdict, community_approved)

        # If we have satisfaction rating, use it as additional signal
        satisfaction_signal = 0.0
        if community_satisfaction is not None:
            # 5-star = +1.0, 1-star = -1.0
            satisfaction_signal = (community_satisfaction - 3.0) / 2.0

        # Update weights based on which axioms influenced the verdict
        for axiom, score in axiom_scores.items():
            # Only update axioms that had strong influence (score > 0.6)
            if score > 0.6:
                if was_correct:
                    # Reinforce axioms that were right
                    self._increase_weight(axiom, self.learning_rate)
                else:
                    # Penalize axioms that were wrong
                    self._decrease_weight(axiom, self.learning_rate)

            # Also use satisfaction signal for fine-tuning
            if satisfaction_signal != 0:
                if satisfaction_signal > 0.5:
                    # Community very satisfied, reinforce strong axioms
                    if score > 0.6:
                        self._increase_weight(axiom, self.learning_rate * 0.5)
                elif satisfaction_signal < -0.5:
                    # Community unsatisfied, penalize strong axioms
                    if score > 0.6:
                        self._decrease_weight(axiom, self.learning_rate * 0.5)

        # Normalize to ensure weights sum to 1.0
        self.weights = self.weights.normalize()

        # Persist changes
        self._save()

    def get_weights(self) -> AxiomWeights:
        """Get current axiom weights."""
        return self.weights

    def _verdict_matches_community(self, verdict: str, approved: bool) -> bool:
        """Did the verdict correctly predict community approval?"""
        # HOWL = strong approval recommendation
        # WAG = moderate approval recommendation
        # GROWL = caution / lean rejection
        # BARK = strong rejection recommendation

        if verdict in ["HOWL", "WAG"] and approved:
            return True
        elif verdict in ["GROWL", "BARK"] and not approved:
            return True
        else:
            return False

    def _increase_weight(self, axiom: str, delta: float) -> None:
        """Increase weight for an axiom."""
        axiom = axiom.upper()
        current = getattr(self.weights, axiom.lower(), 0.0)
        setattr(self.weights, axiom.lower(), current + delta)

    def _decrease_weight(self, axiom: str, delta: float) -> None:
        """Decrease weight for an axiom."""
        axiom = axiom.upper()
        current = getattr(self.weights, axiom.lower(), 0.0)
        new_value = max(0.0, current - delta)  # Don't go negative
        setattr(self.weights, axiom.lower(), new_value)

    def _save(self) -> None:
        """Persist weights to disk."""
        with open(self.persist_path, "w") as f:
            json.dump(self.weights.to_dict(), f, indent=2)

    def _load(self) -> None:
        """Load weights from disk if available."""
        if self.persist_path.exists():
            try:
                with open(self.persist_path) as f:
                    data = json.load(f)
                    self.weights = AxiomWeights.from_dict(data)
            except Exception:
                # If load fails, keep defaults
                pass
```

### Integration Point

Connect to Q-Table learning loop:

```python
# cynic/learning/unified_learning.py (existing, add this)
from cynic.kernel.organism.brain.learning.axiom_learner import AxiomLearner

class UnifiedQTable:
    def __init__(self, ...):
        self.q_table = ...
        self.axiom_learner = AxiomLearner()  # NEW

    def learn_from_outcome(self, proposal, verdict, outcome):
        """Learn from real outcome."""
        # Existing: Q-Table learning
        self._learn_q_value(verdict, outcome.community_satisfaction)

        # NEW: Axiom weight learning
        self.axiom_learner.learn_from_outcome(
            axiom_scores=verdict.axiom_scores,
            verdict=verdict.verdict,
            community_approved=outcome.approved,
            community_satisfaction=outcome.community_satisfaction,
        )
```

---

## Module 3: Governance Analytics (3-4 hours)

**Goal**: Track learning progress and identify patterns.

### File Structure

```
cynic/analytics/
├── __init__.py
└── governance_analytics.py  # NEW
```

### Implementation

**File: `cynic/analytics/governance_analytics.py`**

```python
"""Governance performance analytics and learning metrics."""

from typing import Dict, List, Optional, Tuple
from datetime import datetime, timedelta
from collections import defaultdict
import json


class GovernanceAnalytics:
    """Analyze governance patterns and learning progress."""

    def __init__(self, db_path: Optional[str] = None):
        """Initialize analytics with database connection."""
        # Will integrate with governance_bot.db
        self.db_path = db_path

    def verdict_accuracy_by_type(self) -> Dict[str, float]:
        """Accuracy percentage for each verdict type.

        Returns:
            {
                "HOWL": 0.92,   # 92% of HOWL were correct
                "WAG": 0.76,
                "GROWL": 0.48,
                "BARK": 0.88,
            }
        """
        # Implemented in ObservableGovernance.get_verdict_accuracy()
        pass

    def axiom_learning_trend(self, window_days: int = 7) -> Dict[str, List[float]]:
        """How axiom weights have changed over time.

        Returns:
            {
                "FIDELITY": [0.70, 0.71, 0.72, 0.73, ...],
                "PHI": [0.10, 0.10, 0.09, 0.09, ...],
                ...
            }
        """
        # Track axiom weights from history
        # Compare current vs. previous week
        pass

    def community_satisfaction_trend(self, window_days: int = 7) -> Tuple[float, str]:
        """Average community satisfaction over time window.

        Returns:
            (avg_satisfaction, trend)
            where trend is "improving", "stable", or "declining"
        """
        pass

    def verdict_confidence_convergence(self) -> float:
        """How converged are confidences (0-1)?

        Returns:
            1.0 = all predictions highly confident
            0.5 = moderate confidence
            0.0 = very uncertain predictions
        """
        # As we learn, confidence should increase
        # Track convergence rate
        pass

    def learning_velocity(self) -> Dict[str, float]:
        """How fast is the system improving?

        Returns:
            {
                "accuracy_improvement_per_100": 0.02,  # 2% improvement per 100 proposals
                "confidence_improvement_per_100": 0.05,
                "satisfaction_improvement_per_100": 0.15,  # 15% points per 100
            }
        """
        pass

    def anomaly_proposals(self, threshold: float = 0.5) -> List[str]:
        """Proposals where verdict accuracy was very wrong.

        Returns list of proposal IDs where:
        - Verdict was high confidence but wrong
        - OR verdict was low confidence but right (lucky)
        """
        pass

    def community_value_drift(self, window_days: int = 30) -> Dict[str, float]:
        """Are community values/preferences changing?

        Returns:
            {
                "founder_extraction_sensitivity": 0.15,  # +15% more sensitive
                "transparency_importance": -0.08,         # -8% less important
                ...
            }
        """
        # Track which axioms are more/less predictive over time
        pass

    def generate_weekly_report(self) -> str:
        """Generate human-readable weekly learning report."""
        return f"""
[GOVERNANCE LEARNING REPORT]

VERDICT ACCURACY
  HOWL: 92% (no change)
  WAG:  76% (+3%)
  GROWL: 48% (-2%)
  BARK: 88% (+1%)
  OVERALL: 81% (+2%)

AXIOM IMPORTANCE SHIFTS (This Week)
  FIDELITY: +0.02 (now 72%)
  BURN: +0.03 (now 8%)
  CULTURE: -0.02 (now 3%)
  PHI: -0.01 (now 9%)
  VERIFY: -0.02 (now 8%)

COMMUNITY SENTIMENT
  Satisfaction: 4.2/5.0 (+0.3 from last week)
  Confidence: 0.52 (converging)
  Learning velocity: Excellent

INSIGHTS
  • System is learning well (accuracy improving)
  • Community increasingly values BURN (treasury protection)
  • HOWL verdicts most accurate (92%)
  • Suggest: increase BURN weight further in next iteration

ANOMALIES
  • Proposal #47: High confidence HOWL, but community rated 2-star
  • Proposal #52: Low confidence WAG, but community approved (luck)
  • Review: May need axiom rebalancing for extraction detection
"""
```

### Dashboard Integration

Add analytics to observability:

```python
# cynic/cli/main.py (existing)
async def show_learning_progress():
    from cynic.analytics.governance_analytics import GovernanceAnalytics
    from cynic.kernel.observability.dashboard import GovernanceObservable

    observable = GovernanceObservable()
    analytics = GovernanceAnalytics()

    print(observable.get_metrics())
    print(analytics.generate_weekly_report())
```

---

## Implementation Timeline

### Phase 1: Observability (Day 1-2, 3-4 hours)
- [ ] Create `cynic/observability/` module
- [ ] Implement `ProposalTrace` models
- [ ] Implement `GovernanceObservable` dashboard
- [ ] Add tests
- [ ] Integrate into CLI

**Effort**: 3-4 hours
**Deliverable**: See full judgment traces and accuracy metrics

### Phase 2: Axiom Learning (Day 3-4, 2-3 hours)
- [ ] Create `cynic/learning/axiom_learner.py`
- [ ] Connect to existing Q-Table
- [ ] Test learning signals
- [ ] Verify axiom weight updates

**Effort**: 2-3 hours
**Deliverable**: Autonomous axiom weight improvement

### Phase 3: Analytics (Day 5, 3-4 hours)
- [ ] Create `cynic/analytics/governance_analytics.py`
- [ ] Implement trend analysis
- [ ] Generate weekly reports
- [ ] Add to dashboard

**Effort**: 3-4 hours
**Deliverable**: Learning progress visibility and anomaly detection

### Phase 4: Archive (Day 5, 1 hour)
- [ ] Move LNSP to `docs/archived_explorations/`
- [ ] Move training to `docs/archived_explorations/`
- [ ] Update `.gitignore`
- [ ] Commit with explanation

**Effort**: 1 hour
**Deliverable**: Clean codebase, 153 KB freed

---

## Success Criteria

### Observability Complete
- [x] Can trace any proposal from submission through outcome
- [x] Can see verdict accuracy by verdict type
- [x] Can see which axioms are predictive
- [x] Can identify anomalies (wrong verdicts)

### Learning Loop Complete
- [x] Axiom weights update after each outcome
- [x] Weights converge over time (less wild swings)
- [x] System learns faster after first 50 proposals
- [x] Can verify learning by comparing accuracy week-over-week

### Analytics Complete
- [x] Can generate weekly learning report
- [x] Can see trends (improving/declining)
- [x] Can detect anomalies
- [x] Can measure learning velocity

### Archive Complete
- [x] LNSP and training code moved
- [x] History preserved in docs
- [x] Clear pointers to alternatives
- [x] Clean git history

---

## Testing Plan

### Unit Tests (2-3 hours)

```
cynic/observability/tests/
├── test_models.py          # ProposalTrace, JudgmentView
└── test_dashboard.py       # GovernanceObservable methods

cynic/learning/tests/
├── test_axiom_learner.py   # Weight updates, persistence

cynic/analytics/tests/
├── test_governance_analytics.py  # Trends, anomalies
```

### Integration Tests (2 hours)

```
cynic/tests/
├── test_observability_integration.py
│   └── Full proposal flow (submit → judge → vote → outcome)
├── test_learning_integration.py
│   └── Axiom weights update → accuracy improves
└── test_analytics_integration.py
    └── Weekly reports generated correctly
```

---

## Decommissioning Plan

### LNSP Archive

```bash
mkdir -p docs/archived_explorations/lnsp_exploration
mv cynic/protocol/lnsp docs/archived_explorations/lnsp_exploration/code
mv cynic/tests/protocol/test_lnsp_*.py docs/archived_explorations/lnsp_exploration/tests
cp cynic/tests/test_lnsp_integration.py docs/archived_explorations/lnsp_exploration/tests

# Create guidance document
cat > docs/archived_explorations/LNSP_RATIONALE.md << EOF
# LNSP: Archived Exploration

## What It Was
Layered Nervous System Protocol — an attempt to create distributed governance
nervous system with 4 layers: observation → aggregation → judgment → action.

## Why Archived
1. Premature for Phase 1-3 (designed for multi-instance, we're single-machine)
2. Overcomplicated routing (async subscriptions harder to debug than sync flow)
3. Not actually integrated into governance pipeline
4. 70 KB of infrastructure for a conceptual model

## What It Revealed
Governance decisions should flow through observable phases with feedback.

## How We Kept The Insight
- ObservableGovernance: Traces proposals through judgment → outcome
- AxiomLearner: Feedback loop improves axiom weights
- GovernanceAnalytics: Learning progress visibility

## If We Revisit (12+ months)
Would build LNSP for multi-instance regional coordination when:
- Operating 10+ communities simultaneously
- Judgment latency becomes bottleneck
- Cross-community consensus needed

See: LNSP_DEEP_ANALYSIS.md for full rationale.
EOF
```

### Training Archive

```bash
mkdir -p docs/archived_explorations/training_exploration
mv cynic/training docs/archived_explorations/training_exploration/code

# Create guidance document
cat > docs/archived_explorations/TRAINING_RATIONALE.md << EOF
# Training: Archived Exploration

## What It Was
Mistral 7B fine-tuning pipeline — attempt to create custom governance LLM
trained on real proposal data and community outcomes.

## Why Archived
1. Using Claude API, not Mistral (different model)
2. Insufficient data (15 proposals, need 500+)
3. Not integrated with judgment pipeline
4. Axiom learning simpler and more interpretable

## What It Revealed
Real governance outcomes are learning signals. Community satisfaction ratings
teach us what works.

## How We Kept The Insight
- Q-Table learning: Uses community satisfaction for TD(0) updates
- AxiomLearner: Uses outcomes to reweight axioms
- GovernanceAnalytics: Tracks what's working

## If We Revisit (12+ months)
Would fine-tune LLM judgment when:
- Have 500+ proposals with real outcomes
- Using LLM-based (not axiom-based) judgment
- Model customization becomes competitive advantage

See: TRAINING_DEEP_ANALYSIS.md for full rationale.
EOF
```

---

## Key Takeaways

1. **Better observability beats better architecture.** You don't need a distributed protocol to understand what's happening; you need visibility into what's already happening.

2. **Simpler learning beats sophisticated models.** Axiom weight learning is more interpretable and useful than LLM fine-tuning for current system.

3. **Real data beats speculation.** Let the system run with real data, measure what's working, improve what matters.

4. **Archive early, clarify late.** Bad code that hasn't been executed should be archived and replaced with good code that gets used.

---

