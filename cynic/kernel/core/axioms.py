"""
CYNIC Fractal-Dynamic-Contextual Axiom Architecture

9 Practical Axioms (5 Core + 4 Measurable Emergent) + 2 Transcendent States:

TIER 0 â€” CORE (A1-A5, always active, never deactivate):
    A1. FIDELITY    â€” Truth loyalty, dynamic facets
    A2. PHI         â€” Harmonic proportion, dynamic facets
    A3. VERIFY      â€” Evidence & consensus, dynamic facets
    A4. CULTURE     â€” Memory & patterns, dynamic facets
    A5. BURN        â€” Simplicity & action, dynamic facets

TIER 2 â€” EMERGENT (A6-A9, activate at maturity thresholds):
    A6. AUTONOMY    â€” Dogs coordinate without human approval
    A7. SYMBIOSIS   â€” HumanÃ—Machine mutual value creation
    A8. EMERGENCE   â€” Patterns beyond core axioms (residual > Ïâ»Â²)
    A9. ANTIFRAGILITY â€” System improves from chaos

TIER 2 TRANSCENDENT (A10-A11, states not directly implementable):
    A10. CONSCIOUSNESS â€” System observes its own thinking accurately
    A11. TRANSCENDENCE â€” All axioms active + phase transition detected

TIER 3: THE_UNNAMEABLE â€” Residual inexplicable variance (pointer to âˆž)

Scoring:
    - Each axiom scored 0-100 across dynamic fractal facets
    - Contextual weights per domain (CODE/SOLANA/MARKET/SOCIAL/HUMAN/CYNIC/COSMOS)
    - Weighted geometric mean â’ Q-Score âˆˆ [0, 100] (confidence Ï-bounded to 61.8%)
    - Fractal recursion: facets â’ sub-facets (max 3 levels deep)
"""

from __future__ import annotations

import time
from collections.abc import Callable
from dataclasses import dataclass, field

# Python 3.9 compatibility: StrEnum added in Python 3.11
from enum import StrEnum
from typing import Any

from cynic.kernel.core.phi import (
    GROWL_MIN,
    HOWL_MIN,
    PHI,
    PHI_2,
    PHI_3,
    PHI_INV,
    PHI_INV_2,
    WAG_MIN,
    geometric_mean,
    phi_bound_score,
    weighted_geometric_mean,
)

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# AXIOM NAMES & TIER DEFINITIONS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


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


CORE_AXIOMS: list[Axiom] = [Axiom.FIDELITY, Axiom.PHI, Axiom.VERIFY, Axiom.CULTURE, Axiom.BURN]

EMERGENT_AXIOMS: list[Axiom] = [
    Axiom.AUTONOMY,
    Axiom.SYMBIOSIS,
    Axiom.EMERGENCE,
    Axiom.ANTIFRAGILITY,
    Axiom.CONSCIOUSNESS,
    Axiom.TRANSCENDENCE,
]

PRACTICAL_AXIOMS: list[Axiom] = CORE_AXIOMS + [
    Axiom.AUTONOMY,
    Axiom.SYMBIOSIS,
    Axiom.EMERGENCE,
    Axiom.ANTIFRAGILITY,
]


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# BOOTSTRAP FACETS (STATIC FALLBACK)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

AXIOM_FACETS: dict[str, dict[str, str]] = {
    "FIDELITY": {
        "COMMITMENT": "Long-term dedication to truth over comfort",
        "ATTUNEMENT": "Sensitivity to context, nuance, and signal",
        "CANDOR": "Honesty without spin, deception, or omission",
        "CONGRUENCE": "Alignment between stated intent and actual action",
        "ACCOUNTABILITY": "Ownership of consequences (good and bad)",
        "VIGILANCE": "Constant questioning of assumptions (self-skepticism)",
        "KENOSIS": "Emptying of ego and bias to receive truth clearly",
    },
    "PHI": {
        "COHERENCE": "Logical consistency across all levels",
        "ELEGANCE": "Simplicity and beauty in structure",
        "STRUCTURE": "Well-organized, comprehensible architecture",
        "HARMONY": "Balance between all components (none dominates)",
        "PRECISION": "Exactitude in measurement and language",
        "COMPLETENESS": "Nothing essential is missing",
        "PROPORTION": "Ï-aligned ratios in scale and scope",
    },
    "VERIFY": {
        "ACCURACY": "Correctness of facts and computations",
        "PROVENANCE": "Traceable, verified origin of claims",
        "INTEGRITY": "Unmodified since creation (hash/signature)",
        "VERIFIABILITY": "Can be independently checked by a third party",
        "TRANSPARENCY": "Process and reasoning fully visible",
        "REPRODUCIBILITY": "Results can be replicated with same inputs",
        "CONSENSUS": "Agreement among independent validators",
    },
    "CULTURE": {
        "AUTHENTICITY": "True to its stated nature and origin",
        "RESONANCE": "Aligns with and respects existing patterns",
        "NOVELTY": "Brings something genuinely new to the context",
        "ALIGNMENT": "Fits the current environment and constraints",
        "RELEVANCE": "Matters now, addresses real problems",
        "IMPACT": "Changes things in measurable ways",
        "LINEAGE": "Honors and builds on what came before",
    },
    "BURN": {
        "UTILITY": "Serves a clear, demonstrable purpose",
        "SUSTAINABILITY": "Can be maintained without collapse",
        "EFFICIENCY": "Minimal waste (tokens, compute, attention)",
        "VALUE_CREATION": "Generates measurable worth beyond cost",
        "SACRIFICE": "Willingness to give up complexity for clarity",
        "CONTRIBUTION": "Adds to the collective whole",
        "IRREVERSIBILITY": "Commitment is demonstrated (cannot be undone)",
    },
}


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# CONTEXTUAL WEIGHTS PER DOMAIN
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class Domain(StrEnum):
    """7 Reality Dimensions (what exists)."""

    CODE = "CODE"
    SOLANA = "SOLANA"
    MARKET = "MARKET"
    SOCIAL = "SOCIAL"
    HUMAN = "HUMAN"
    CYNIC = "CYNIC"
    COSMOS = "COSMOS"


DEFAULT_CONTEXTUAL_WEIGHTS: dict[str, dict[str, float]] = {
    "CODE": {
        "VERIFY": PHI_2,  # 2.618
        "PHI": PHI,  # 1.618
        "BURN": PHI,  # 1.618
        "CULTURE": 1.0,  # 1.000
        "FIDELITY": PHI_INV,  # 0.618
    },
    "SOLANA": {
        "VERIFY": PHI_3,  # 4.236
        "FIDELITY": PHI_2,  # 2.618
        "CULTURE": PHI,  # 1.618
        "PHI": 1.0,  # 1.000
        "BURN": PHI_INV,  # 0.618
    },
    "MARKET": {
        "PHI": PHI_2,  # 2.618
        "VERIFY": PHI,  # 1.618
        "FIDELITY": PHI,  # 1.618
        "CULTURE": 1.0,  # 1.000
        "BURN": PHI_INV,  # 0.618
    },
    "SOCIAL": {
        "CULTURE": PHI_2,  # 2.618
        "FIDELITY": PHI,  # 1.618
        "PHI": PHI,  # 1.618
        "VERIFY": 1.0,  # 1.000
        "BURN": PHI_INV,  # 0.618
    },
    "HUMAN": {
        "FIDELITY": PHI_3,  # 4.236
        "CULTURE": PHI_2,  # 2.618
        "PHI": PHI,  # 1.618
        "VERIFY": 1.0,  # 1.000
        "BURN": PHI_INV,  # 0.618
    },
    "CYNIC": {
        "PHI": PHI_2,  # 2.618
        "VERIFY": PHI_2,  # 2.618
        "FIDELITY": PHI,  # 1.618
        "BURN": PHI,  # 1.618
        "CULTURE": 1.0,  # 1.000
    },
    "COSMOS": {
        "CULTURE": PHI_3,  # 4.236
        "PHI": PHI_2,  # 2.618
        "FIDELITY": PHI,  # 1.618
        "VERIFY": 1.0,  # 1.000
        "BURN": PHI_INV,  # 0.618
    },
}


class FacetRegistry:
    """
    Dynamic registry for Axiom facets.
    Checks SurrealDB first, falls back to static AXIOM_FACETS.
    """

    def __init__(self, storage: Any | None = None) -> None:
        self.storage = storage
        self._cache: dict[str, dict[str, str]] = {}

    async def get_facets(self, axiom: str, reality: str) -> dict[str, str]:
        cache_key = f"{axiom}:{reality}"
        if cache_key in self._cache:
            return self._cache[cache_key]

        facets = {}
        if self.storage and hasattr(self.storage, "axiom_facets"):
            try:
                db_facets = await self.storage.axiom_facets.get_all(axiom, reality)
                if db_facets:
                    facets = {f["facet"]: f.get("description", "") for f in db_facets}
            except Exception:
                pass

        if not facets and axiom in AXIOM_FACETS:
            facets = AXIOM_FACETS[axiom]

        if facets:
            self._cache[cache_key] = facets
        return facets

    async def register_facet(
        self, axiom: str, reality: str, facet: str, description: str = ""
    ) -> None:
        if self.storage and hasattr(self.storage, "axiom_facets"):
            await self.storage.axiom_facets.save(
                {
                    "axiom": axiom,
                    "reality": reality,
                    "facet": facet,
                    "description": description,
                }
            )
        cache_key = f"{axiom}:{reality}"
        if cache_key not in self._cache:
            self._cache[cache_key] = {}
        self._cache[cache_key][facet] = description


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# ACTIVATION THRESHOLDS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@dataclass
class EmergentThreshold:
    conditions: dict[str, float]
    description: str


EMERGENT_THRESHOLDS: dict[str, EmergentThreshold] = {
    "AUTONOMY": EmergentThreshold(
        conditions={"consensus_strength": PHI_INV, "stable_cycles": 7.0},
        description="Dogs coordinate without human approval for 7+ stable cycles",
    ),
    "SYMBIOSIS": EmergentThreshold(
        conditions={"human_trust": PHI_INV_2, "machine_utility": PHI_INV_2},
        description="Mutual humanÃ—machine value creation demonstrated",
    ),
    "EMERGENCE": EmergentThreshold(
        conditions={"residual_variance": PHI_INV_2},
        description="Patterns detected beyond core 5 axioms",
    ),
    "ANTIFRAGILITY": EmergentThreshold(
        conditions={"learning_velocity": 0.0, "chaos_level": PHI_INV},
        description="System improves despite (because of) chaos",
    ),
    "CONSCIOUSNESS": EmergentThreshold(
        conditions={"meta_cognition_active": PHI_INV},
        description="System observes own thinking with >61.8% accuracy",
    ),
    "TRANSCENDENCE": EmergentThreshold(
        conditions={"all_axioms_active": 1.0, "phase_transition": 1.0},
        description="Qualitative leap â€” THE_UNNAMEABLE understood",
    ),
}


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# VERDICT & RESULTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class Verdict(StrEnum):
    HOWL = "HOWL"
    WAG = "WAG"
    GROWL = "GROWL"
    BARK = "BARK"


def verdict_from_q_score(q_score: float) -> Verdict:
    if q_score >= HOWL_MIN:
        return Verdict.HOWL
    if q_score >= WAG_MIN:
        return Verdict.WAG
    if q_score >= GROWL_MIN:
        return Verdict.GROWL
    return Verdict.BARK


@dataclass
class AxiomScore:
    axiom: str
    score: float
    facet_scores: dict[str, float] = field(default_factory=dict)
    depth: int = 1
    timestamp: float = field(default_factory=time.time)


@dataclass
class FullAxiomResult:
    domain: str
    q_score: float
    verdict: str
    axiom_scores: dict[str, float]
    axiom_details: dict[str, AxiomScore]
    active_axioms: list[str]
    timestamp: float

    def to_dict(self) -> dict:
        return {
            "domain": self.domain,
            "q_score": round(self.q_score, 3),
            "verdict": self.verdict,
            "axiom_scores": {k: round(v, 2) for k, v in self.axiom_scores.items()},
            "active_axioms": self.active_axioms,
            "timestamp": self.timestamp,
        }


@dataclass
class AxiomArchitectureState:
    active_axioms: list[str] = field(default_factory=lambda: [a.value for a in CORE_AXIOMS])
    emergent_states: dict[str, bool] = field(
        default_factory=lambda: {a.value: False for a in EMERGENT_AXIOMS}
    )
    learned_weights: dict[str, dict[str, float]] = field(
        default_factory=lambda: {k: dict(v) for k, v in DEFAULT_CONTEXTUAL_WEIGHTS.items()}
    )
    activation_log: list[dict] = field(default_factory=list)


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# AXIOM ARCHITECTURE
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class AxiomArchitecture:
    """Fractal-Dynamic-Contextual Axiom System."""

    def __init__(
        self,
        state: AxiomArchitectureState | None = None,
        facet_scorer: Callable[[str, str, str], float] | None = None,
        registry: FacetRegistry | None = None,
        on_facets_missing: Callable[[str, str], Any] | None = None,
    ) -> None:
        self.state = state or AxiomArchitectureState()
        self._facet_scorer = facet_scorer or self._default_facet_scorer
        self.registry = registry or FacetRegistry()
        self.on_facets_missing = on_facets_missing

    @property
    def active_axioms(self) -> list[str]:
        active = list(self.state.active_axioms)
        for axiom, is_active in self.state.emergent_states.items():
            if is_active and axiom not in active:
                active.append(axiom)
        return active

    def check_emergent_activation(self, metrics: dict[str, float]) -> list[str]:
        newly_activated = []
        for name, threshold in EMERGENT_THRESHOLDS.items():
            if self.state.emergent_states.get(name, False):
                continue
            if all(metrics.get(k, 0.0) >= v for k, v in threshold.conditions.items()):
                self.state.emergent_states[name] = True
                if name not in self.state.active_axioms:
                    self.state.active_axioms.append(name)
                self.state.activation_log.append(
                    {"axiom": name, "timestamp": time.time(), "metrics": dict(metrics)}
                )
                newly_activated.append(name)
        return newly_activated

    async def score_axiom_fractal(
        self,
        axiom_name: str,
        context: str,
        reality: str = "CYNIC",
        depth: int = 1,
        max_depth: int = 3,
    ) -> float:
        if depth > max_depth:
            return 50.0
        facets = await self.registry.get_facets(axiom_name, reality)

        if not facets:
            # Try to trigger dreaming if callback exists
            if self.on_facets_missing:
                await self.on_facets_missing(axiom_name, reality)
                # Retry once after dreaming
                facets = await self.registry.get_facets(axiom_name, reality)

            if not facets:
                return self._facet_scorer(axiom_name, axiom_name, context)

        scores = []
        for facet_name in facets:
            sub_facets = await self.registry.get_facets(facet_name, reality)
            if depth < max_depth and sub_facets:
                s = await self.score_axiom_fractal(
                    facet_name, context, reality, depth + 1, max_depth
                )
            else:
                s = self._facet_scorer(axiom_name, facet_name, context)
            scores.append(max(0.0, min(100.0, s)))
        return geometric_mean(scores) if scores else 50.0

    def _default_facet_scorer(self, a, f, c) -> float:
        return 50.0

    def compute_q_score(
        self, domain: str, axiom_scores: dict[str, float], metrics: dict[str, float] | None = None
    ) -> float:
        if metrics:
            self.check_emergent_activation(metrics)
        active = self.active_axioms
        vals, weights = [], []
        dw = self.state.learned_weights.get(domain, DEFAULT_CONTEXTUAL_WEIGHTS.get(domain, {}))
        for axiom in active:
            vals.append(max(0.0, min(100.0, axiom_scores.get(axiom, 50.0))))
            weights.append(dw.get(axiom, 1.0))
        return phi_bound_score(weighted_geometric_mean(vals, weights)) if vals else 0.0

    async def score_and_compute(
        self,
        domain: str,
        context: str,
        fractal_depth: int = 1,
        metrics: dict[str, float] | None = None,
        dog_inputs: dict[str, float] | None = None,
    ) -> FullAxiomResult:
        axiom_scores, axiom_details = {}, {}
        inputs = dog_inputs or {}
        for name in self.active_axioms:
            score = (
                inputs[name]
                if name in inputs
                else await self.score_axiom_fractal(name, context, domain, fractal_depth)
            )
            axiom_scores[name] = score
            f_scores = {}
            facets = await self.registry.get_facets(name, domain)
            if facets:
                for f_name in facets:
                    f_scores[f_name] = inputs.get(f_name, self._facet_scorer(name, f_name, context))
            axiom_details[name] = AxiomScore(
                axiom=name, score=score, facet_scores=f_scores, depth=fractal_depth
            )

        q = self.compute_q_score(domain, axiom_scores, metrics)
        return FullAxiomResult(
            domain=domain,
            q_score=q,
            verdict=verdict_from_q_score(q).value,
            axiom_scores=axiom_scores,
            axiom_details=axiom_details,
            active_axioms=list(self.active_axioms),
            timestamp=time.time(),
        )

    def update_contextual_weights(
        self, domain: str, gradient: dict[str, float], lr: float = 0.038
    ) -> None:
        if domain not in self.state.learned_weights:
            self.state.learned_weights[domain] = dict(DEFAULT_CONTEXTUAL_WEIGHTS.get(domain, {}))
        w = self.state.learned_weights[domain]
        for axiom, grad in gradient.items():
            if axiom in w:
                w[axiom] = max(0.0, w[axiom] + lr * grad)
