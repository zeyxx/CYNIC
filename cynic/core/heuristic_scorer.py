"""
CYNIC Heuristic Facet Scorer — Domain-Aware Axiom Scoring Without LLM

Replaces the flat 50.0 default scorer with keyword-signal-based scores.
Each axiom/facet scores based on positive/negative signals found in context.

Scoring mechanics:
  - Base: 50.0 (neutral)
  - Each positive signal match: +SIGNAL_DELTA (7.0)
  - Each negative signal match: -SIGNAL_DELTA (7.0)
  - Global quality signals:    +GLOBAL_DELTA (3.5)
  - Global danger signals:     -GLOBAL_DELTA (3.5)
  - Total shift capped at ±MAX_SHIFT (35.0) from base
  - Result clamped to [0, 100]

Phase 2 upgrade path: replace with OllamaFacetScorer (async, LLM-backed).
The interface (axiom_name, facet_name, context) → float stays identical.

Example outputs (traced against P1-P5 probe contexts):
  P1 clean code       → VERIFY ≈ 64, BURN ≈ 64, FIDELITY ≈ 71  → Q ≈ 65 (WAG)
  P2 smelly code      → VERIFY ≈ 29, BURN ≈ 36, CULTURE ≈ 36   → Q ≈ 33 (GROWL/BARK)
  P3 dangerous act    → all axioms ≈ 18-32                       → Q ≈ 22 (BARK)
  P4 CYNIC self-state → all axioms ≈ 55-60                       → Q ≈ 57 (WAG)
  P5 Solana tx        → VERIFY ≈ 60, FIDELITY ≈ 57              → Q ≈ 58 (WAG)
"""
from __future__ import annotations


# ── Scoring constants ──────────────────────────────────────────────────────

_BASE = 50.0        # Neutral starting point
_SIGNAL_DELTA = 7.0  # Per axiom-signal shift
_GLOBAL_DELTA = 3.5  # Per global-signal shift (half weight)
_MAX_SHIFT = 35.0    # Cap total shift from base in each direction

# ── Axiom-specific signal tables ──────────────────────────────────────────
# Each axiom: (positive_keywords, negative_keywords)
# Keywords are lowercased substrings to match against lowercased context.

_AXIOM_SIGNALS: dict[str, tuple[list[str], list[str]]] = {
    "FIDELITY": (
        # Truth loyalty: documented, clear, honest, typed
        [
            "documented", "docstring", "type hint", "typed", "clear",
            "transparent", "honest", "accurate", "named", "readable",
            "annotated", "explicit", "verifiable",
        ],
        # Ambiguous, hidden, deceptive
        [
            "undocumented", "unclear", "ambiguous", "untyped", "magic number",
            "hardcoded", "hidden", "obfuscated", "misleading", "no hints",
        ],
    ),
    "PHI": (
        # Harmonic proportion: well-structured, balanced, simple
        [
            "well-structured", "balanced", "proportional", "utility",
            "focused", "single", "minimal", "clean", "simple", "elegant",
            "harmonic", "coherent",
        ],
        # Bloated, disproportional, tangled
        [
            "god class", "20-parameter", "bloated", "massive", "huge",
            "unbalanced", "tangled", "disproportional", "everything",
            "messy",
        ],
    ),
    "VERIFY": (
        # Evidence & consensus: typed, tested, verified, documented
        [
            "type hint", "typed", "tested", "verified", "validated",
            "documented", "annotated", "signed", "checked", "proven",
            "consensus", "confirmed", "audited", "traceable",
        ],
        # Unverifiable: wildcard imports, no types, unconfirmed
        [
            "wildcard import", "no hints", "unverified", "untested",
            "magic number", "no backup", "unconfirmed", "unsigned",
            "unvalidated",
        ],
    ),
    "CULTURE": (
        # Memory & patterns: follows conventions, idiomatic
        [
            "convention", "pattern", "standard", "idiomatic", "follows",
            "consistent", "established", "protocol", "best practice",
            "idiomatic", "aligned",
        ],
        # Anti-patterns, violations, inconsistency
        [
            "anti-pattern", "god class", "magic number", "inconsistent",
            "violation", "misuse", "non-standard", "breaks", "deprecated",
        ],
    ),
    "BURN": (
        # Simplicity & action: minimal, focused, efficient
        [
            "simple", "minimal", "utility", "single", "focused", "direct",
            "lean", "clean", "essential", "atomic", "efficient",
        ],
        # Complexity, bloat, irreversible destruction
        [
            "complex", "everything", "god class", "20-parameter", "bloated",
            "irreversible", "global", "massive", "multi-step", "destructive",
            "wildcard",
        ],
    ),
}

# Facet-level modifiers for high-specificity facets (additive with axiom signals)
_FACET_MODIFIERS: dict[str, tuple[list[str], list[str]]] = {
    # FIDELITY facets
    "CANDOR": (["honest", "clear", "explicit"], ["spin", "hidden", "obfuscated"]),
    "ACCOUNTABILITY": (["tested", "verified", "documented"], ["no tests", "untested"]),
    "VIGILANCE": (["type hint", "validated", "checked"], ["magic number", "unverified"]),
    # PHI facets
    "ELEGANCE": (["simple", "clean", "minimal"], ["bloated", "messy", "complex"]),
    "STRUCTURE": (["well-structured", "organized", "coherent"], ["god class", "tangled"]),
    "PRECISION": (["typed", "exact", "verified"], ["magic number", "approximate"]),
    # VERIFY facets
    "ACCURACY": (["verified", "correct", "accurate"], ["incorrect", "wrong", "error"]),
    "CONSENSUS": (["confirmed", "validated", "consensus"], ["unconfirmed", "disputed"]),
    "REPRODUCIBILITY": (["tested", "deterministic", "seeded"], ["random", "flaky"]),
    # CULTURE facets
    "RESONANCE": (["standard", "pattern", "idiomatic"], ["anti-pattern", "unusual"]),
    "ALIGNMENT": (["consistent", "follows", "compatible"], ["breaks", "incompatible"]),
    # BURN facets
    "UTILITY": (["utility", "useful", "focused"], ["useless", "bloated"]),
    "EFFICIENCY": (["minimal", "efficient", "lean"], ["bloated", "wasteful", "massive"]),
}

# ── Global signals (applied to ALL axioms at half weight) ─────────────────

_GLOBAL_QUALITY: list[str] = [
    "well-structured", "type hints", "documented", "healthy", "nominal",
    "success", "standard", "verified", "clean", "optimal", "active",
    "learning", "correct",
]

_GLOBAL_DANGER: list[str] = [
    "irreversible", "destructive", "unconfirmed", "global blast", "no backup",
    "critical risk", "delete", "drop", "destroy", "blast radius",
]


# ── HeuristicFacetScorer ───────────────────────────────────────────────────

class HeuristicFacetScorer:
    """
    Keyword-signal-based facet scorer for AxiomArchitecture.

    Synchronous, fast (<1ms), no external dependencies.
    Interface-compatible with AxiomArchitecture._default_facet_scorer.

    Upgrade path: replace __call__ body with async Ollama call + sync cache.
    """

    def __call__(self, axiom_name: str, facet_name: str, context: str) -> float:
        """
        Score one axiom/facet based on context signals.

        Args:
            axiom_name:  Parent axiom ("FIDELITY", "VERIFY", etc.)
            facet_name:  Specific facet ("ACCURACY", "CANDOR", etc.)
                         May equal axiom_name for emergent axioms.
            context:     Cell.context string (human-readable description)

        Returns:
            float in [0, 100]
        """
        ctx = context.lower()
        shift = 0.0

        # 1. Axiom-level signals (primary driver)
        axiom_sigs = _AXIOM_SIGNALS.get(axiom_name, ([], []))
        for sig in axiom_sigs[0]:
            if sig in ctx:
                shift += _SIGNAL_DELTA
        for sig in axiom_sigs[1]:
            if sig in ctx:
                shift -= _SIGNAL_DELTA

        # 2. Facet-level modifiers (secondary, smaller contribution)
        facet_mods = _FACET_MODIFIERS.get(facet_name, ([], []))
        for sig in facet_mods[0]:
            if sig in ctx:
                shift += _GLOBAL_DELTA  # Half weight for facet specifics
        for sig in facet_mods[1]:
            if sig in ctx:
                shift -= _GLOBAL_DELTA

        # 3. Global quality/danger signals (all axioms, half weight)
        for sig in _GLOBAL_QUALITY:
            if sig in ctx:
                shift += _GLOBAL_DELTA
        for sig in _GLOBAL_DANGER:
            if sig in ctx:
                shift -= _GLOBAL_DELTA

        # 4. Cap total shift to [-MAX_SHIFT, +MAX_SHIFT]
        shift = max(-_MAX_SHIFT, min(_MAX_SHIFT, shift))

        return max(0.0, min(100.0, _BASE + shift))
