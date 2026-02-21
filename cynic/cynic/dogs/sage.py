"""
CYNIC Sage Dog — Chokmah (Wisdom)

SAGE is the second Dog after CYNIC in the Kabbalistic hierarchy.
Chokmah = Wisdom — the flash of primordial insight, the first emanation.

Responsibilities:
  - Evaluate code/content against CYNIC's 5 axioms (PHI, VERIFY, CULTURE, BURN, FIDELITY)
  - Detect "wisdom markers" — clarity, proportionality, axiomatic alignment
  - Phase 1: Heuristic scoring (no LLM dependency) — fast, always available
  - Phase 2: Temporal MCTS via Ollama (7 parallel perspectives) — MACRO cycle only

Why Sage?
  Chokmah = the spark before form. SAGE detects whether something was built
  with understanding or just assembled. The difference between a function
  that does exactly one thing well vs a class that grew for 3 years.

φ-integration:
  Priority: φ² = 2.618 (second after CYNIC)
  Max confidence (heuristic): PHI_INV_2 = 0.382 — honest about uncertainty
  Max confidence (temporal):  PHI_INV   = 0.618 — 7 perspectives, more certain
  consciousness_min: REFLEX (heuristic), MACRO (temporal)
  LLM path: Temporal MCTS — 7 parallel Ollama calls, asyncio.gather()

Scoring (heuristic):
  Five axiom dimensions → domain-weighted geometric mean → Q-score [0, 100]

  Weights from DEFAULT_CONTEXTUAL_WEIGHTS (φ-derived, domain-specific):
  CODE:   VERIFY=φ²(2.618), PHI=φ(1.618), BURN=φ(1.618), CULTURE=1.0, FIDELITY=φ⁻¹
  SOLANA: VERIFY=φ³(4.236), FIDELITY=φ²(2.618), CULTURE=φ, PHI=1.0, BURN=φ⁻¹
  HUMAN:  FIDELITY=φ³(4.236), CULTURE=φ²(2.618), PHI=φ, VERIFY=1.0, BURN=φ⁻¹

Scoring (temporal MCTS):
  7 perspectives → φ-weighted geometric mean → Q-score [0, 100]
  Confidence from perspective agreement (high agreement = high confidence)

VETO: impossible — SAGE advises with wisdom, never blocks.
"""
from __future__ import annotations

import logging
import re
import time
from dataclasses import dataclass, field
from typing import Any, Optional


from cynic.core.phi import (
    PHI_INV, PHI_INV_2, PHI_INV_3, MAX_Q_SCORE, MAX_CONFIDENCE,
    phi_bound_score, fibonacci,
)
from cynic.core.axioms import DEFAULT_CONTEXTUAL_WEIGHTS
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.sage")

# Fallback weights — used when domain not found in DEFAULT_CONTEXTUAL_WEIGHTS
_AXIOM_WEIGHT_FALLBACK: dict[str, float] = {
    "PHI":      1.0,
    "VERIFY":   1.0,
    "CULTURE":  1.0,
    "BURN":     1.0,
    "FIDELITY": 1.0,
}

# Wisdom markers — patterns that indicate good code
WISDOM_PATTERNS: list[str] = [
    r"\bdef\s+[a-z][a-z_]{2,30}\s*\(",       # Well-named functions (3-30 chars)
    r"\bclass\s+[A-Z][A-Za-z]{2,30}\s*[:(]", # Well-named classes
    r"\"\"\".*?\"\"\"",                        # Docstrings (at all)
    r"\b(return|yield)\b",                     # Functions return something
    r"\bif\s+\w+\s*(?:is\s+not\s+None|is\s+None|\belse\b)",  # Proper null guards
]

# Anti-patterns — code smell markers
SMELL_PATTERNS: list[str] = [
    r"\bclass\s+[A-Z][a-zA-Z]*Manager\b",    # "Manager" antipattern
    r"\bclass\s+[A-Z][a-zA-Z]*Helper\b",     # "Helper" antipattern
    r"\bclass\s+[A-Z][a-zA-Z]*Utils?\b",     # "Utils" antipattern
    r"\btodo\b|\bTODO\b|\bhack\b|\bHACK\b",  # Technical debt markers
    r"\bpass\b\s*$",                          # Empty blocks
    r"([^\n]{120,})",                         # Very long lines (>120 chars)
    r"\bglobal\s+\w+",                        # Global state mutation
]

# Baseline Q for each axiom when evidence is ambiguous
AXIOM_BASELINE: float = 0.50  # Neutral — CYNIC doubts itself

# Cold confidence when only heuristics (no LLM)
HEURISTIC_CONFIDENCE: float = 0.28  # Below PHI_INV_2 — honest about uncertainty


class SageDog(LLMDog):
    """
    Sage (Chokmah) — Wisdom evaluator.

    Phase 1: Heuristic scoring of the 5 CYNIC axioms via static analysis.
    Phase 2: Temporal MCTS — 7 parallel Ollama calls via asyncio.gather().
             7 temporal perspectives (PAST/PRESENT/FUTURE/IDEAL/NEVER/CYCLES/FLOW)
             → φ-weighted geometric mean → Q-score [0, 61.8]

    SAGE provides philosophical grounding to the judgment pipeline —
    "This code was written by someone who understood the problem" vs
    "This code grew without intention."
    """

    DOG_ID = DogId.SAGE

    def __init__(self) -> None:
        super().__init__(DogId.SAGE, task_type="wisdom")
        self._heuristic_count: int = 0
        self._llm_count: int = 0
        self._compressor: Any | None = None  # ContextCompressor — injected via set_compressor()

    def set_compressor(self, compressor: Any) -> None:
        """
        Inject ContextCompressor for bidirectional attention feedback.

        After each judgment, SAGE signals which content was relevant via
        compressor.boost(), so the Compressor prioritizes high-attention
        chunks in future compressions.
        """
        self._compressor = compressor
        logger.info("SageDog: ContextCompressor injected — bidirectional attention loop active")

    def _signal_attention(self, text: str, q_score: float) -> None:
        """
        Signal to ContextCompressor which content was relevant for this judgment.

        Called after every judgment (heuristic and temporal paths).
        Only signals for judgments with meaningful Q-score (≥ GROWL_MIN).
        """
        if self._compressor is None or not text:
            return
        weight = q_score / MAX_Q_SCORE  # Normalize to [0, 1]
        if weight < PHI_INV_2:          # Only signal for GROWL+ judgments (≥ 38.2%)
            return
        try:
            self._compressor.boost(text, weight)
        except CynicError:
            logger.debug("Compressor boost failed (non-critical)", exc_info=True)

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.SAGE,
            sefirot="Chokmah — Wisdom",
            consciousness_min=ConsciousnessLevel.MACRO,
            uses_llm=True,   # Phase 2: Temporal MCTS via Ollama
            supported_realities={
                "CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS",
            },
            supported_analyses={
                "PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE",
            },
            technology="5-axiom heuristic (Phase 1); Temporal MCTS 7×Ollama (Phase 2)",
            max_concurrent=4,
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Evaluate a Cell through the lens of CYNIC's 5 axioms.

        Phase 1 (heuristic): Fast, always available. Confidence ≤ 0.382.
        Phase 2 (temporal):  7 parallel Ollama calls via asyncio.gather().
                             Confidence up to 0.618 (φ-bounded).

        Temporal MCTS auto-selects when LLMRegistry has been injected
        and an LLM adapter is available.
        """
        start = time.perf_counter()
        text = self._extract_text(cell)

        # Phase 2: Temporal MCTS (if LLM available)
        adapter = await self.get_llm()
        if adapter is not None:
            lod_level: int = kwargs.get("lod_level", 0)
            compressed_context: str = kwargs.get("compressed_context", "")
            return await self._temporal_path(
                cell, text, adapter, start,
                lod_level=lod_level, compressed_context=compressed_context,
            )

        # Phase 1: Heuristic fallback
        return self._heuristic_path(cell, text, start)

    async def _temporal_path(
        self,
        cell: Cell,
        text: str,
        adapter: Any,
        start: float,
        lod_level: int = 0,
        compressed_context: str = "",
    ) -> DogJudgment:
        """
        Phase 2: Temporal perspectives via Ollama/Haiku.

        LOD=FULL (0): full temporal_judgment() — 7 perspectives, rich signal.
        LOD=REDUCED (1): fast_temporal_judgment() — 3 perspectives, 57% cheaper.
        Higher LOD values won't reach MACRO (blocked by _apply_lod_cap in orchestrator).

        compressed_context: CYNIC memory injected into each LLM call's system prompt.
        Enables memory-aware judgment even with stateless Haiku/Ollama backends.
        """
        from cynic.llm.temporal import temporal_judgment, fast_temporal_judgment

        content = text or cell.context or ""
        if lod_level == 1:
            # R2: REDUCED LOD — organism under resource pressure → 3-perspective fast path
            tj = await fast_temporal_judgment(adapter, content, context=compressed_context)
            perspective_count = 3
        else:
            tj = await temporal_judgment(adapter, content, context=compressed_context)
            perspective_count = 7

        q_score = tj.phi_aggregate
        confidence = tj.confidence
        latency = (time.perf_counter() - start) * 1000

        # Best/worst perspectives for reasoning
        scores = tj.scores
        best = max(scores.items(), key=lambda kv: kv[1])
        worst = min(scores.items(), key=lambda kv: kv[1])
        reasoning = (
            f"*sniff* Chokmah (temporal/{perspective_count}p): Q={q_score:.1f}. "
            f"Strongest: {best[0]} ({best[1]:.1f}). "
            f"Weakest: {worst[0]} ({worst[1]:.1f}). "
            f"LLM: {tj.llm_id}."
        )

        self._llm_count += 1
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(q_score),
            confidence=confidence,
            reasoning=reasoning,
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
            veto=False,
        )
        self.record_judgment(judgment)
        self._signal_attention(text, judgment.q_score)  # ← SAGE→Compressor feedback
        return judgment

    def _heuristic_path(
        self,
        cell: Cell,
        text: str,
        start: float,
    ) -> DogJudgment:
        """Phase 1: 5-axiom heuristic (no LLM). Always available."""
        axiom_scores = self._score_axioms(text, cell.reality, cell)
        q_score, confidence, evidence = self._aggregate(axiom_scores, text, cell.reality)

        worst_axiom = min(axiom_scores.items(), key=lambda kv: kv[1])
        best_axiom = max(axiom_scores.items(), key=lambda kv: kv[1])
        reasoning = (
            f"*sniff* Wisdom: Q={q_score:.1f} via 5-axiom heuristic. "
            f"Strength: {best_axiom[0]} ({best_axiom[1]:.0%}). "
            f"Concern: {worst_axiom[0]} ({worst_axiom[1]:.0%})."
        )

        self._heuristic_count += 1
        latency = (time.perf_counter() - start) * 1000

        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(q_score),
            confidence=confidence,
            reasoning=reasoning,
            evidence=evidence,
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        self._signal_attention(text, judgment.q_score)  # ← SAGE→Compressor feedback
        return judgment

    # ── Axiom Scoring ─────────────────────────────────────────────────────

    def _score_axioms(
        self,
        text: str,
        reality: str,
        cell: Cell,
    ) -> dict[str, float]:
        """
        Score each of the 5 axioms on [0, 1].

        0.0 = axiom violated  (e.g., infinite scope)
        0.5 = axiom neutral   (no evidence either way)
        1.0 = axiom exemplary (clear scope, verifiable, etc.)
        """
        return {
            "PHI":      self._score_phi(text, cell),
            "VERIFY":   self._score_verify(text, reality),
            "CULTURE":  self._score_culture(text),
            "BURN":     self._score_burn(text, cell),
            "FIDELITY": self._score_fidelity(text, cell),
        }

    def _score_phi(self, text: str, cell: Cell) -> float:
        """
        PHI axiom: Is scope bounded?

        Evidence:
        - Short content → well-scoped (+)
        - Very long content → potentially unbounded (-)
        - Low complexity from cell → bounded (+)
        - Fibonacci-harmonic length preferred
        """
        length = len(text) if text else 0

        # Length heuristic: prefer ≤F(12)=144 lines worth (~7200 chars)
        if length == 0:
            return AXIOM_BASELINE
        if length < fibonacci(10) * 50:   # < ~2750 chars — well-scoped
            score = 0.75
        elif length < fibonacci(12) * 50: # < ~7200 chars — acceptable
            score = 0.55
        elif length < fibonacci(13) * 50: # < ~11700 chars — getting large
            score = 0.38
        else:
            score = 0.22  # Too large

        # Complexity modifier: lower cell complexity → more bounded
        complexity_mod = 1.0 - cell.complexity * 0.3  # At complexity=1: -0.3
        return max(0.1, min(1.0, score * complexity_mod))

    def _score_verify(self, text: str, reality: str) -> float:
        """
        VERIFY axiom: Are claims verifiable?

        Evidence:
        - Type hints → verifiable contracts (+)
        - Tests mentioned → verifiable behavior (+)
        - Hard-coded magic numbers → unverifiable (-)
        - Comments explaining "why" → self-documenting (+)
        """
        if not text:
            return AXIOM_BASELINE

        score = AXIOM_BASELINE
        text_lower = text.lower()

        # Type hints
        if re.search(r":\s*(?:str|int|float|bool|list|dict|Optional|Union|Any)\b", text):
            score += 0.15
        if "-> " in text:  # Return type annotations
            score += 0.10

        # Docstrings / comments
        if '"""' in text or "'''" in text:
            score += 0.10
        if re.search(r"#\s+\w", text):  # Meaningful comments (# word...)
            score += 0.05

        # Magic numbers (unverifiable)
        magic = len(re.findall(r"\b(?<!\.)\d{3,}\b(?!\.\d)", text))
        score -= min(0.20, magic * 0.04)

        # Assertions / tests
        if re.search(r"\bassert\b|\bpytest\b|\bunittest\b", text_lower):
            score += 0.10

        return max(0.0, min(1.0, score))

    def _score_culture(self, text: str) -> float:
        """
        CULTURE axiom: Does the code carry culture?

        Evidence:
        - Matches wisdom patterns (+)
        - Contains smell patterns (-)
        - Snake_case naming in Python (+)
        - Consistent indentation (+)
        """
        if not text:
            return AXIOM_BASELINE

        score = AXIOM_BASELINE

        # Wisdom pattern matches
        wisdom_hits = sum(
            1 for pat in WISDOM_PATTERNS
            if re.search(pat, text, re.DOTALL)
        )
        score += wisdom_hits * 0.06  # up to +0.30 for 5 hits

        # Smell pattern matches
        smell_hits = sum(
            1 for pat in SMELL_PATTERNS
            if re.search(pat, text, re.MULTILINE)
        )
        score -= smell_hits * 0.08  # -0.08 per smell

        # Python snake_case convention
        func_names = re.findall(r"\bdef\s+([a-zA-Z_]\w*)", text)
        if func_names:
            snake_ratio = sum(1 for n in func_names if n == n.lower()) / len(func_names)
            score += (snake_ratio - 0.5) * 0.20  # ±0.10 modifier

        return max(0.0, min(1.0, score))

    def _score_burn(self, text: str, cell: Cell) -> float:
        """
        BURN axiom: Simplicity over complexity.

        "Don't extract, burn." — three similar lines beat a premature abstraction.

        Evidence:
        - Short, focused functions (+)
        - Nested classes / multiple levels of inheritance (-)
        - Too many parameters (-)
        - Minimal import count for task at hand (+)
        """
        if not text:
            return AXIOM_BASELINE

        score = 0.55  # Slight positive default — benefit of the doubt

        # Count function definitions
        functions = re.findall(r"\bdef\s+\w+\s*\(([^)]*)\)", text)
        if functions:
            # Penalize functions with many parameters (>5 is complex)
            avg_params = sum(
                len([p for p in f.split(",") if p.strip()])
                for f in functions
            ) / len(functions)
            if avg_params <= 3:
                score += 0.15
            elif avg_params <= 5:
                score += 0.05
            elif avg_params <= 8:
                score -= 0.10
            else:
                score -= 0.20

        # Deep nesting indicator (many leading spaces)
        lines = text.split("\n")
        deep_lines = sum(1 for l in lines if l.startswith(" " * 16))  # 4+ levels
        if deep_lines > len(lines) * 0.2:
            score -= 0.15

        # Import count — many imports → potentially over-engineered
        imports = len(re.findall(r"^(?:import|from)\s", text, re.MULTILINE))
        if imports > 10:
            score -= 0.10
        elif imports > 20:
            score -= 0.20

        # Abstract base class (extra layer)
        if re.search(r"\(ABC\)|\(Abstract", text):
            score -= 0.05  # mild penalty — abstractions need justification

        return max(0.0, min(1.0, score))

    def _score_fidelity(self, text: str, cell: Cell) -> float:
        """
        FIDELITY axiom: Loyal to truth, honest purpose.

        Evidence:
        - Function/class names match their content (+)
        - No misleading names (e.g., 'update' that actually deletes) (-)
        - Risk score from cell: high risk + low coverage → low fidelity
        - Context matches content (+)
        """
        if not text:
            return AXIOM_BASELINE

        score = AXIOM_BASELINE

        # Risk modifier: high-risk cells must be extra faithful
        risk_penalty = cell.risk * 0.20  # Max -0.20 for risk=1.0
        score -= risk_penalty

        # Context vs content coherence (basic check)
        context = cell.context or ""
        if context and len(context) > 10:
            # Context mentions code concepts — check if they appear in text
            context_words = set(re.findall(r"\b[a-z_]{3,}\b", context.lower()))
            text_words = set(re.findall(r"\b[a-z_]{3,}\b", text.lower()))
            if context_words:
                overlap = len(context_words & text_words) / len(context_words)
                score += overlap * 0.20  # Up to +0.20 if context words appear

        # Deceptive naming patterns (e.g., "delete" function named "update")
        deceptive = re.search(
            r"\bdef\s+(?:update|save|create)\w*\s*\([^)]*\)[^:]*:.*?(?:delete|remove|drop)\b",
            text, re.DOTALL | re.IGNORECASE
        )
        if deceptive:
            score -= 0.20

        # Honest error handling (try/except present)
        if re.search(r"\btry\s*:\s", text):
            score += 0.10

        return max(0.0, min(1.0, score))

    # ── Aggregation ────────────────────────────────────────────────────────

    def _aggregate(
        self,
        axiom_scores: dict[str, float],
        text: str,
        reality: str = "CODE",
    ) -> tuple[float, float, dict[str, Any]]:
        """
        Domain-weighted geometric mean of axiom scores → Q-score.

        Uses DEFAULT_CONTEXTUAL_WEIGHTS for the domain (α3 fix).
        E.g., CODE: VERIFY=φ²=2.618, SOLANA: VERIFY=φ³=4.236.

        geo_mean = exp(Σ w_i·log(s_i) / Σ w_i)  ∈ [0, 1]
        q_score  = geo_mean × MAX_Q_SCORE         ∈ [0, 100]
        """
        import math

        domain_weights = DEFAULT_CONTEXTUAL_WEIGHTS.get(reality, _AXIOM_WEIGHT_FALLBACK)

        log_sum = 0.0
        total_weight = 0.0
        for ax, score in axiom_scores.items():
            w = domain_weights.get(ax, 1.0)
            log_sum += w * math.log(max(score, 0.01))
            total_weight += w

        geo_mean = math.exp(log_sum / total_weight) if total_weight > 0 else 0.0

        # Scale to [0, MAX_Q_SCORE]
        q_score = geo_mean * MAX_Q_SCORE

        # Confidence: heuristic path → always HEURISTIC_CONFIDENCE
        # Grows slightly if text is substantial (more evidence = more confidence)
        text_factor = min(1.0, len(text) / 1000) if text else 0.0
        confidence = min(
            HEURISTIC_CONFIDENCE + text_factor * 0.05,
            PHI_INV_2,  # Heuristic capped at PHI_INV_2 (0.382) — knows its limits
        )

        evidence = {
            "axioms": {ax: round(sc, 3) for ax, sc in axiom_scores.items()},
            "geometric_mean": round(geo_mean, 3),
            "text_length": len(text),
            "path": "heuristic",
        }

        return q_score, confidence, evidence

    # ── Text Extraction ────────────────────────────────────────────────────

    def _extract_text(self, cell: Cell) -> str:
        """Extract analyzable text from a Cell."""
        parts = []
        if cell.content:
            if isinstance(cell.content, dict):
                # Code cells often have {"code": "..."} shape
                raw = cell.content.get("code") or cell.content.get("text") or str(cell.content)
            else:
                raw = str(cell.content)
            parts.append(raw[:5000])  # Cap for speed

        if cell.context:
            parts.append(str(cell.context)[:500])

        return "\n".join(parts) if parts else ""

    # ── Health ────────────────────────────────────────────────────────────

    async def health_check(self) -> DogHealth:
        total = self._judgment_count
        adapter = await self.get_llm()
        path = "temporal_mcts" if adapter is not None else "heuristic"
        status = (
            HealthStatus.HEALTHY if total > 0 or self._active
            else HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Heuristic: {self._heuristic_count}, "
                f"Temporal: {self._llm_count}, "
                f"path={path}"
            ),
        )
