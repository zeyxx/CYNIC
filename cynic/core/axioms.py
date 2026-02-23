"""
CYNIC Fractal-Dynamic-Contextual Axiom Architecture

9 Practical Axioms (5 Core + 4 Measurable Emergent) + 2 Transcendent States:

TIER 0 — CORE (A1-A5, always active, never deactivate):
    A1. FIDELITY    — Truth loyalty, 7 facets
    A2. PHI         — Harmonic proportion, 7 facets
    A3. VERIFY      — Evidence & consensus, 7 facets
    A4. CULTURE     — Memory & patterns, 7 facets
    A5. BURN        — Simplicity & action, 7 facets

TIER 2 — EMERGENT (A6-A9, activate at maturity thresholds):
    A6. AUTONOMY    — Dogs coordinate without human approval
    A7. SYMBIOSIS   — Human×Machine mutual value creation
    A8. EMERGENCE   — Patterns beyond core axioms (residual > φ⁻²)
    A9. ANTIFRAGILITY — System improves from chaos

TIER 2 TRANSCENDENT (A10-A11, states not directly implementable):
    A10. CONSCIOUSNESS — System observes its own thinking accurately
    A11. TRANSCENDENCE — All axioms active + phase transition detected

TIER 3: THE_UNNAMEABLE — Residual inexplicable variance (pointer to ∞)

Scoring:
    - Each axiom scored 0-100 across 7 fractal facets
    - Contextual weights per domain (CODE/SOLANA/MARKET/SOCIAL/HUMAN/CYNIC/COSMOS)
    - Weighted geometric mean → Q-Score ∈ [0, 100] (confidence φ-bounded to 61.8%)
    - Fractal recursion: facets → sub-facets (max 3 levels deep)
"""
from __future__ import annotations

import sys
import time
from dataclasses import dataclass, field
from enum import Enum
from collections.abc import Callable
from typing import Optional

# Python 3.9 compatibility: StrEnum added in Python 3.11
if sys.version_info >= (3, 11):
    from enum import StrEnum
else:
    class StrEnum(str, Enum):
        """Polyfill for Python <3.11."""
        pass

from cynic.core.phi import (
    AXIOMS_CORE,
    AXIOMS_FACETS,
    HOWL_MIN,
    WAG_MIN,
    GROWL_MIN,
    MAX_Q_SCORE,
    PHI,
    PHI_INV,
    PHI_INV_2,
    PHI_2,
    PHI_3,
    geometric_mean,
    phi_bound_score,
    weighted_geometric_mean,
)


# ════════════════════════════════════════════════════════════════════════════
# AXIOM NAMES & TIER DEFINITIONS
# ════════════════════════════════════════════════════════════════════════════

class Axiom(StrEnum):
    """All 11 axioms + THE_UNNAMEABLE."""

    # Tier 0: Core (always active)
    FIDELITY = "FIDELITY"
    PHI = "PHI"
    VERIFY = "VERIFY"
    CULTURE = "CULTURE"
    BURN = "BURN"

    # Tier 2: Emergent (activate at thresholds)
    AUTONOMY = "AUTONOMY"
    SYMBIOSIS = "SYMBIOSIS"
    EMERGENCE = "EMERGENCE"
    ANTIFRAGILITY = "ANTIFRAGILITY"

    # Tier 2: Transcendent (states, not directly measurable)
    CONSCIOUSNESS = "CONSCIOUSNESS"
    TRANSCENDENCE = "TRANSCENDENCE"

    # Tier 3: The Unnamed (residual pointer)
    THE_UNNAMEABLE = "THE_UNNAMEABLE"


CORE_AXIOMS: list[Axiom] = [
    Axiom.FIDELITY, Axiom.PHI, Axiom.VERIFY, Axiom.CULTURE, Axiom.BURN
]

EMERGENT_AXIOMS: list[Axiom] = [
    Axiom.AUTONOMY, Axiom.SYMBIOSIS, Axiom.EMERGENCE, Axiom.ANTIFRAGILITY,
    Axiom.CONSCIOUSNESS, Axiom.TRANSCENDENCE,
]

PRACTICAL_AXIOMS: list[Axiom] = CORE_AXIOMS + [
    Axiom.AUTONOMY, Axiom.SYMBIOSIS, Axiom.EMERGENCE, Axiom.ANTIFRAGILITY
]  # 9 axioms = the "9" the user refers to


# ════════════════════════════════════════════════════════════════════════════
# 7 FACETS PER CORE AXIOM (FRACTAL STRUCTURE)
# ════════════════════════════════════════════════════════════════════════════

AXIOM_FACETS: dict[str, dict[str, str]] = {
    "FIDELITY": {
        "COMMITMENT":     "Long-term dedication to truth over comfort",
        "ATTUNEMENT":     "Sensitivity to context, nuance, and signal",
        "CANDOR":         "Honesty without spin, deception, or omission",
        "CONGRUENCE":     "Alignment between stated intent and actual action",
        "ACCOUNTABILITY": "Ownership of consequences (good and bad)",
        "VIGILANCE":      "Constant questioning of assumptions (self-skepticism)",
        "KENOSIS":        "Emptying of ego and bias to receive truth clearly",
    },
    "PHI": {
        "COHERENCE":    "Logical consistency across all levels",
        "ELEGANCE":     "Simplicity and beauty in structure",
        "STRUCTURE":    "Well-organized, comprehensible architecture",
        "HARMONY":      "Balance between all components (none dominates)",
        "PRECISION":    "Exactitude in measurement and language",
        "COMPLETENESS": "Nothing essential is missing",
        "PROPORTION":   "φ-aligned ratios in scale and scope",
    },
    "VERIFY": {
        "ACCURACY":        "Correctness of facts and computations",
        "PROVENANCE":      "Traceable, verified origin of claims",
        "INTEGRITY":       "Unmodified since creation (hash/signature)",
        "VERIFIABILITY":   "Can be independently checked by a third party",
        "TRANSPARENCY":    "Process and reasoning fully visible",
        "REPRODUCIBILITY": "Results can be replicated with same inputs",
        "CONSENSUS":       "Agreement among independent validators",
    },
    "CULTURE": {
        "AUTHENTICITY": "True to its stated nature and origin",
        "RESONANCE":    "Aligns with and respects existing patterns",
        "NOVELTY":      "Brings something genuinely new to the context",
        "ALIGNMENT":    "Fits the current environment and constraints",
        "RELEVANCE":    "Matters now, addresses real problems",
        "IMPACT":       "Changes things in measurable ways",
        "LINEAGE":      "Honors and builds on what came before",
    },
    "BURN": {
        "UTILITY":         "Serves a clear, demonstrable purpose",
        "SUSTAINABILITY":  "Can be maintained without collapse",
        "EFFICIENCY":      "Minimal waste (tokens, compute, attention)",
        "VALUE_CREATION":  "Generates measurable worth beyond cost",
        "SACRIFICE":       "Willingness to give up complexity for clarity",
        "CONTRIBUTION":    "Adds to the collective whole",
        "IRREVERSIBILITY": "Commitment is demonstrated (cannot be undone)",
    },
}


# ════════════════════════════════════════════════════════════════════════════
# CONTEXTUAL WEIGHTS PER DOMAIN
# ════════════════════════════════════════════════════════════════════════════

class Domain(StrEnum):
    """7 Reality Dimensions (what exists)."""
    CODE = "CODE"
    SOLANA = "SOLANA"
    MARKET = "MARKET"
    SOCIAL = "SOCIAL"
    HUMAN = "HUMAN"
    CYNIC = "CYNIC"
    COSMOS = "COSMOS"


# Default contextual weights (φ-symmetric, learned via gradient descent over time)
DEFAULT_CONTEXTUAL_WEIGHTS: dict[str, dict[str, float]] = {
    "CODE": {
        "VERIFY":   PHI_2,     # 2.618 — Formal proof critical in code
        "PHI":      PHI,       # 1.618 — Elegant architecture
        "BURN":     PHI,       # 1.618 — Simplicity in code
        "CULTURE":  1.0,       # 1.000 — Respect existing patterns
        "FIDELITY": PHI_INV,   # 0.618 — Spec fidelity
    },
    "SOLANA": {
        "VERIFY":   PHI_3,     # 4.236 — Blockchain consensus CRITICAL
        "FIDELITY": PHI_2,     # 2.618 — Transaction exactness
        "CULTURE":  PHI,       # 1.618 — Protocol respect
        "PHI":      1.0,       # 1.000 — Gas proportion
        "BURN":     PHI_INV,   # 0.618 — Moderate burn rate
    },
    "MARKET": {
        "PHI":      PHI_2,     # 2.618 — Price proportion & timing
        "VERIFY":   PHI,       # 1.618 — Signal verification
        "FIDELITY": PHI,       # 1.618 — Accurate market reading
        "CULTURE":  1.0,       # 1.000 — Trend awareness
        "BURN":     PHI_INV,   # 0.618 — Conservative action
    },
    "SOCIAL": {
        "CULTURE":  PHI_2,     # 2.618 — Community resonance CRITICAL
        "FIDELITY": PHI,       # 1.618 — Authenticity
        "PHI":      PHI,       # 1.618 — Harmonious timing
        "VERIFY":   1.0,       # 1.000 — Fact checking
        "BURN":     PHI_INV,   # 0.618 — Moderate action pace
    },
    "HUMAN": {
        "FIDELITY": PHI_3,     # 4.236 — Human trust is paramount
        "CULTURE":  PHI_2,     # 2.618 — Human context & patterns
        "PHI":      PHI,       # 1.618 — Proportional interaction
        "VERIFY":   1.0,       # 1.000 — Accuracy in human modeling
        "BURN":     PHI_INV,   # 0.618 — Gentle action
    },
    "CYNIC": {
        "PHI":      PHI_2,     # 2.618 — Self-coherence
        "VERIFY":   PHI_2,     # 2.618 — Self-verification
        "FIDELITY": PHI,       # 1.618 — Self-honesty
        "BURN":     PHI,       # 1.618 — Self-simplification
        "CULTURE":  1.0,       # 1.000 — Pattern memory
    },
    "COSMOS": {
        "CULTURE":  PHI_3,     # 4.236 — Ecosystem patterns paramount
        "PHI":      PHI_2,     # 2.618 — Universal proportion
        "FIDELITY": PHI,       # 1.618 — Truth to collective
        "VERIFY":   1.0,       # 1.000 — Ecosystem verification
        "BURN":     PHI_INV,   # 0.618 — Conservative ecosystem action
    },
}


# ════════════════════════════════════════════════════════════════════════════
# EMERGENT AXIOM ACTIVATION THRESHOLDS
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class EmergentThreshold:
    """Conditions required to activate an emergent axiom."""
    conditions: dict[str, float]  # {metric_name: min_value}
    description: str


EMERGENT_THRESHOLDS: dict[str, EmergentThreshold] = {
    "AUTONOMY": EmergentThreshold(
        conditions={
            "consensus_strength": PHI_INV,  # 61.8%
            "stable_cycles": 7.0,           # 7 consecutive stable cycles
        },
        description="Dogs coordinate without human approval for 7+ stable cycles",
    ),
    "SYMBIOSIS": EmergentThreshold(
        conditions={
            "human_trust": PHI_INV_2,       # 38.2%
            "machine_utility": PHI_INV_2,   # 38.2%
        },
        description="Mutual human×machine value creation demonstrated",
    ),
    "EMERGENCE": EmergentThreshold(
        conditions={
            "residual_variance": PHI_INV_2, # 38.2% unexplained variance
        },
        description="Patterns detected beyond core 5 axioms",
    ),
    "ANTIFRAGILITY": EmergentThreshold(
        conditions={
            "learning_velocity": 0.0,       # Positive learning rate
            "chaos_level": PHI_INV,         # 61.8% chaos threshold
        },
        description="System improves despite (because of) chaos",
    ),
    "CONSCIOUSNESS": EmergentThreshold(
        conditions={
            "meta_cognition_active": PHI_INV,  # 61.8% meta-cognition accuracy
        },
        description="System observes own thinking with >61.8% accuracy",
    ),
    "TRANSCENDENCE": EmergentThreshold(
        conditions={
            "all_axioms_active": 1.0,      # All previous axioms active
            "phase_transition": 1.0,        # Phase transition detected
        },
        description="Qualitative leap — THE_UNNAMEABLE understood",
    ),
}


# ════════════════════════════════════════════════════════════════════════════
# VERDICT
# ════════════════════════════════════════════════════════════════════════════

class Verdict(StrEnum):
    HOWL = "HOWL"    # 82-61.8: Exceptional (φ² × φ⁻¹)
    WAG = "WAG"      # 61.8-38.2: Good
    GROWL = "GROWL"  # 38.2-0: Needs work
    BARK = "BARK"    # 0-38.2: Critical


def verdict_from_q_score(q_score: float) -> Verdict:
    """Map Q-Score [0, 100] to verdict."""
    if q_score >= HOWL_MIN:       # 82.0
        return Verdict.HOWL
    elif q_score >= WAG_MIN:      # 61.8
        return Verdict.WAG
    elif q_score >= GROWL_MIN:    # 38.2
        return Verdict.GROWL
    else:
        return Verdict.BARK


# ════════════════════════════════════════════════════════════════════════════
# AXIOM ARCHITECTURE (The Heart of Judgment)
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class AxiomScore:
    """Score for a single axiom (0-100, with 7 facet breakdown)."""
    axiom: str
    score: float                        # 0-100, φ-bounded at 61.8
    facet_scores: dict[str, float]      # {facet_name: 0-100}
    depth: int = 1                      # Fractal recursion depth (1-3)
    timestamp: float = field(default_factory=time.time)


@dataclass
class AxiomArchitectureState:
    """Runtime state of the axiom system."""
    # Core axioms always active
    active_axioms: list[str] = field(default_factory=lambda: [
        a.value for a in CORE_AXIOMS
    ])
    # Emergent axiom states
    emergent_states: dict[str, bool] = field(default_factory=lambda: {
        a.value: False for a in EMERGENT_AXIOMS
    })
    # Learned contextual weights per domain (starts from defaults)
    learned_weights: dict[str, dict[str, float]] = field(
        default_factory=lambda: {k: dict(v) for k, v in DEFAULT_CONTEXTUAL_WEIGHTS.items()}
    )
    # Activation timestamps
    activation_log: list[dict] = field(default_factory=list)


class AxiomArchitecture:
    """
    Fractal-Dynamic-Contextual Axiom System.

    - FRACTAL: Each axiom expands to 7 facets (recursive, φ-bounded to depth 3)
    - DYNAMIC: Axioms activate when maturity thresholds crossed
    - CONTEXTUAL: Domain-specific weights learned via gradient descent

    Usage:
        arch = AxiomArchitecture()
        judgment = arch.score(domain="CODE", raw_scores={"FIDELITY": 72.0, ...})
    """

    def __init__(
        self,
        state: Optional[AxiomArchitectureState] = None,
        facet_scorer: Callable[[str, str, str], float] | None = None,
    ) -> None:
        self.state = state or AxiomArchitectureState()
        # External facet scorer (injected for LLM-backed scoring)
        # Signature: (axiom_name, facet_name, context) → score [0-100]
        self._facet_scorer = facet_scorer or self._default_facet_scorer

    # ─── Active Axioms ────────────────────────────────────────────────────

    @property
    def active_axioms(self) -> list[str]:
        """All currently active axioms (core + activated emergent)."""
        active = list(self.state.active_axioms)
        for axiom, is_active in self.state.emergent_states.items():
            if is_active and axiom not in active:
                active.append(axiom)
        return active

    def check_emergent_activation(self, metrics: dict[str, float]) -> list[str]:
        """
        Check if any emergent axioms should activate based on current metrics.

        Returns list of newly activated axiom names.
        """
        newly_activated: list[str] = []

        for axiom_name, threshold in EMERGENT_THRESHOLDS.items():
            if self.state.emergent_states.get(axiom_name, False):
                continue  # Already active

            # Check all conditions
            all_met = all(
                metrics.get(condition_key, 0.0) >= min_value
                for condition_key, min_value in threshold.conditions.items()
            )

            if all_met:
                self.state.emergent_states[axiom_name] = True
                if axiom_name not in self.state.active_axioms:
                    self.state.active_axioms.append(axiom_name)
                self.state.activation_log.append({
                    "axiom": axiom_name,
                    "timestamp": time.time(),
                    "metrics": dict(metrics),
                })
                newly_activated.append(axiom_name)

        return newly_activated

    # ─── Fractal Scoring ──────────────────────────────────────────────────

    def score_axiom_fractal(
        self,
        axiom_name: str,
        context: str,
        depth: int = 1,
    ) -> float:
        """
        Score an axiom with fractal recursion.

        depth=1: Score 7 facets → geometric mean (DEFAULT)
        depth=2: Score each facet's sub-facets (7×7) → φ-bounded
        depth=3: Maximum depth (φ³ diminishing returns)

        Returns score [0, 100].
        """
        if depth > 3:  # φ-bounded recursion depth
            return 50.0  # Neutral at max depth

        if axiom_name not in AXIOM_FACETS:
            # Emergent axiom: score directly via external scorer
            return self._facet_scorer(axiom_name, axiom_name, context)

        facets = AXIOM_FACETS[axiom_name]
        facet_scores: list[float] = []

        for facet_name in facets:
            if depth < 3 and facet_name in AXIOM_FACETS:
                # Recursive: score sub-facets
                sub_score = self.score_axiom_fractal(facet_name, context, depth + 1)
            else:
                # Leaf: direct scoring
                sub_score = self._facet_scorer(axiom_name, facet_name, context)
            facet_scores.append(max(0.0, min(100.0, sub_score)))

        return geometric_mean(facet_scores) if facet_scores else 0.0

    def _default_facet_scorer(
        self, axiom_name: str, facet_name: str, context: str
    ) -> float:
        """
        Default facet scorer — neutral 50.0 (no LLM).
        Override by injecting a real scorer via facet_scorer parameter.
        """
        return 50.0

    # ─── Q-Score Computation ──────────────────────────────────────────────

    def compute_q_score(
        self,
        domain: str,
        axiom_scores: dict[str, float],
        metrics: Optional[dict[str, float]] = None,
    ) -> float:
        """
        Compute φ-bounded Q-Score from axiom scores.

        Process:
        1. Check emergent axiom activation (if metrics provided)
        2. Get contextual weights for domain
        3. Weighted geometric mean of active axiom scores
        4. φ-bound to [0, 61.8]

        Returns Q-Score ∈ [0, 100].
        """
        # 1. Emergent activation check
        if metrics:
            self.check_emergent_activation(metrics)

        # 2. Get active axioms and their scores
        active = self.active_axioms
        values: list[float] = []
        weights: list[float] = []

        domain_weights = self.state.learned_weights.get(
            domain, DEFAULT_CONTEXTUAL_WEIGHTS.get(domain, {})
        )

        for axiom in active:
            score = axiom_scores.get(axiom, 50.0)  # Default neutral if not provided
            score = max(0.0, min(100.0, score))
            weight = domain_weights.get(axiom, 1.0)  # Default weight = 1.0
            values.append(score)
            weights.append(weight)

        if not values:
            return 0.0

        # 3. Weighted geometric mean → Q-Score ∈ [0, 100]
        q_raw = weighted_geometric_mean(values, weights)

        # 4. φ-bound to [0, 100] — NO rescaling (D1 decision: Q-Score [0,100])
        return phi_bound_score(q_raw)

    def score_and_compute(
        self,
        domain: str,
        context: str,
        fractal_depth: int = 1,
        metrics: Optional[dict[str, float]] = None,
    ) -> FullAxiomResult:
        """
        Full pipeline: score all active axioms + compute Q-Score.

        Returns FullAxiomResult with per-axiom breakdown and Q-Score.
        """
        # Score each active axiom
        axiom_scores: dict[str, float] = {}
        axiom_details: dict[str, AxiomScore] = {}

        for axiom_name in self.active_axioms:
            score = self.score_axiom_fractal(axiom_name, context, fractal_depth)
            axiom_scores[axiom_name] = score

            # Detailed facet breakdown (depth=1 only for performance)
            facet_scores: dict[str, float] = {}
            if axiom_name in AXIOM_FACETS:
                for facet_name in AXIOM_FACETS[axiom_name]:
                    facet_scores[facet_name] = self._facet_scorer(
                        axiom_name, facet_name, context
                    )
            axiom_details[axiom_name] = AxiomScore(
                axiom=axiom_name,
                score=score,
                facet_scores=facet_scores,
                depth=fractal_depth,
            )

        # Compute Q-Score
        q_score = self.compute_q_score(domain, axiom_scores, metrics)
        verdict = verdict_from_q_score(q_score)

        return FullAxiomResult(
            domain=domain,
            q_score=q_score,
            verdict=verdict.value,
            axiom_scores=axiom_scores,
            axiom_details=axiom_details,
            active_axioms=list(self.active_axioms),
            timestamp=time.time(),
        )

    # ─── Weight Learning ──────────────────────────────────────────────────

    def update_contextual_weights(
        self,
        domain: str,
        gradient: dict[str, float],
        learning_rate: float = 0.038,  # φ⁻² / 10
    ) -> None:
        """
        Update contextual weights for domain via gradient ascent.

        Called by learning system (LOOP 11: Kabbalistic Router).
        """
        if domain not in self.state.learned_weights:
            self.state.learned_weights[domain] = dict(
                DEFAULT_CONTEXTUAL_WEIGHTS.get(domain, {})
            )

        weights = self.state.learned_weights[domain]
        for axiom, grad in gradient.items():
            if axiom in weights:
                weights[axiom] = max(0.0, weights[axiom] + learning_rate * grad)


@dataclass
class FullAxiomResult:
    """Complete result of axiom scoring."""
    domain: str
    q_score: float                          # [0, 100] — HOWL ≥82, WAG ≥61.8
    verdict: str                            # HOWL/WAG/GROWL/BARK
    axiom_scores: dict[str, float]          # {axiom_name: 0-100}
    axiom_details: dict[str, AxiomScore]    # Detailed breakdown
    active_axioms: list[str]                # Which axioms were active
    timestamp: float

    def to_dict(self) -> dict:
        return {
            "domain": self.domain,
            "q_score": round(self.q_score, 3),
            "verdict": self.verdict,
            "axiom_scores": {k: round(v, 2) for k, v in self.axiom_scores.items()},
            "active_axioms": self.active_axioms,
            "active_count": len(self.active_axioms),
            "timestamp": self.timestamp,
        }
