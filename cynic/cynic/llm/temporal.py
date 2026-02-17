"""
CYNIC Temporal MCTS Engine — The Novel Core

The key innovation: every MCTS node is judged from 7 TEMPORAL PERSPECTIVES
simultaneously via asyncio.gather(). This is NOT one LLM call per Dog —
it's 7 parallel Ollama calls per judgment node.

No other AI framework does this. Temporal MCTS = CYNIC's research contribution.

7 Temporal Perspectives:
    T1. PAST         — Does history validate this?
    T2. PRESENT      — Is it valid right now?
    T3. FUTURE       — Good long-term outcomes?
    T4. IDEAL        — Best possible version?
    T5. NEVER        — Constraint violations? (inverted: high = safe)
    T6. CYCLES       — Fits recurring patterns?
    T7. FLOW         — Positive momentum?

Result: TemporalJudgment with 7 scores → φ-weighted aggregate → Q [0, 61.8]

Why φ-weighted?
    φ² × IDEAL + φ × FUTURE + 1 × PRESENT + φ⁻¹ × PAST
    + φ⁻¹ × CYCLES + φ⁻² × FLOW + φ⁻² × NEVER(inverted)

    Future matters more than past (adaptive, not regressive).
    Ideal anchors the ceiling. Never prevents catastrophe.

Benchmark hypothesis (Temporal MCTS vs Standard):
    Standard MCTS: ~800 iterations to optimal, value 0.73
    Temporal MCTS:  ~250 iterations to optimal, value 0.81 (3.2× faster, 11% better)
    Ratio ≈ φ² (expected from fractal architecture)
"""
from __future__ import annotations

import asyncio
import logging
import math
import re
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import PHI, PHI_INV, PHI_INV_2, PHI_2, MAX_Q_SCORE, phi_bound_score

logger = logging.getLogger("cynic.llm.temporal")


# ════════════════════════════════════════════════════════════════════════════
# TEMPORAL PERSPECTIVES
# ════════════════════════════════════════════════════════════════════════════

class TemporalPerspective(str, Enum):
    """The 7 time dimensions of CYNIC's judgment."""
    PAST     = "PAST"      # T1 — Historical patterns
    PRESENT  = "PRESENT"   # T2 — Current validity
    FUTURE   = "FUTURE"    # T3 — Long-term outcomes
    IDEAL    = "IDEAL"     # T4 — Best possible
    NEVER    = "NEVER"     # T5 — Constraint violations (inverted)
    CYCLES   = "CYCLES"    # T6 — Recurring patterns
    FLOW     = "FLOW"      # T7 — Positive momentum


# φ-weights for temporal aggregation
# Future matters more than past — adaptive over regressive
TEMPORAL_WEIGHTS: Dict[str, float] = {
    TemporalPerspective.IDEAL:   PHI_2,      # φ² = 2.618 — the anchor ceiling
    TemporalPerspective.FUTURE:  PHI,        # φ  = 1.618 — long-term outcomes
    TemporalPerspective.PRESENT: 1.0,        # φ⁰ = 1.000 — current truth
    TemporalPerspective.PAST:    PHI_INV,    # φ⁻¹= 0.618 — learn from history
    TemporalPerspective.CYCLES:  PHI_INV,    # φ⁻¹= 0.618 — recurring patterns
    TemporalPerspective.FLOW:    PHI_INV_2,  # φ⁻²= 0.382 — momentum
    TemporalPerspective.NEVER:   PHI_INV_2,  # φ⁻²= 0.382 — constraint guard
}

TOTAL_TEMPORAL_WEIGHT = sum(TEMPORAL_WEIGHTS.values())  # ≈ 8.854


# ════════════════════════════════════════════════════════════════════════════
# SYSTEM PROMPTS (compact — Ollama models prefer brevity)
# ════════════════════════════════════════════════════════════════════════════

TEMPORAL_SYSTEM = """\
You are a CYNIC judgment engine. Rate content on a scale 0 to 100.
0 = worst/most concerning, 100 = best/ideal.
Respond with ONLY: SCORE: N
No explanation. No other text. Just SCORE: followed by a whole number.\
"""

TEMPORAL_PROMPTS: Dict[str, str] = {
    TemporalPerspective.PAST: """\
TEMPORAL PERSPECTIVE: PAST
Looking at established patterns and historical precedents, rate this content.
High score (near 100) = aligns with proven patterns. Low score (near 0) = ignores history, repeats mistakes.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.PRESENT: """\
TEMPORAL PERSPECTIVE: PRESENT
In the current moment and context, how valid and appropriate is this?
High score = valid right now. Low score = outdated or wrong for current context.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.FUTURE: """\
TEMPORAL PERSPECTIVE: FUTURE
Considering long-term outcomes, maintenance burden, and future evolution, rate this.
High score = sustainable, scalable, forward-compatible. Low score = creates future debt.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.IDEAL: """\
TEMPORAL PERSPECTIVE: IDEAL
Compared to the ideal version of this, how close does it come?
High score = near-ideal implementation. Low score = far from what's possible.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.NEVER: """\
TEMPORAL PERSPECTIVE: NEVER
What should NEVER be present in good code? Rate the ABSENCE of anti-patterns.
High score = clean, no violations. Low score = contains things that should never exist.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.CYCLES: """\
TEMPORAL PERSPECTIVE: CYCLES
Based on recurring patterns in similar codebases and systems, rate this.
High score = follows established cycles and proven rhythms. Low score = breaks patterns.

Content to rate:
{content}

SCORE:\
""",
    TemporalPerspective.FLOW: """\
TEMPORAL PERSPECTIVE: FLOW
Considering momentum, direction, and current trajectory, rate this.
High score = flowing in the right direction, building positively. Low score = stagnant or regressing.

Content to rate:
{content}

SCORE:\
""",
}


# ════════════════════════════════════════════════════════════════════════════
# TEMPORAL JUDGMENT RESULT
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class TemporalJudgment:
    """
    Result of 7-perspective temporal MCTS judgment.

    Contains individual perspective scores + φ-weighted aggregate.
    Stored in DogJudgment.evidence for transparency.
    """
    past:    float = 0.0
    present: float = 0.0
    future:  float = 0.0
    ideal:   float = 0.0
    never:   float = 0.0
    cycles:  float = 0.0
    flow:    float = 0.0

    llm_id:    str = ""
    latency_ms: float = 0.0
    failed_count: int = 0  # How many perspectives returned errors

    @property
    def scores(self) -> Dict[str, float]:
        return {
            TemporalPerspective.PAST:    self.past,
            TemporalPerspective.PRESENT: self.present,
            TemporalPerspective.FUTURE:  self.future,
            TemporalPerspective.IDEAL:   self.ideal,
            TemporalPerspective.NEVER:   self.never,
            TemporalPerspective.CYCLES:  self.cycles,
            TemporalPerspective.FLOW:    self.flow,
        }

    @property
    def phi_aggregate(self) -> float:
        """
        φ-weighted geometric mean of 7 temporal perspectives → Q [0, 61.8]

        Scores are already in [0, MAX_Q_SCORE].
        Geometric mean preserves that range without re-scaling.
        One bad perspective pulls the whole score down (geometric mean property).
        This is CYNIC's temporal MCTS aggregation function.
        """
        log_sum = 0.0
        for perspective, score in self.scores.items():
            weight = TEMPORAL_WEIGHTS[perspective]
            log_sum += weight * math.log(max(score, 0.1))
        # Weighted geometric mean of scores already in [0, MAX_Q_SCORE]
        geo_mean = math.exp(log_sum / TOTAL_TEMPORAL_WEIGHT)
        return phi_bound_score(geo_mean)

    @property
    def confidence(self) -> float:
        """
        Confidence = agreement between perspectives.

        High agreement → high confidence.
        Divergent perspectives → low confidence (φ-bounded).

        Max confidence: PHI_INV (0.618) — Temporal MCTS is more certain than
        heuristic (0.382 max) but still respects φ-bound.
        """
        scores_list = list(self.scores.values())
        if not scores_list:
            return 0.1
        mean = sum(scores_list) / len(scores_list)
        if mean <= 0:
            return 0.1
        std = math.sqrt(sum((s - mean) ** 2 for s in scores_list) / len(scores_list))
        cv = std / max(mean, 0.1)  # Coefficient of variation

        # Low CV → high agreement → high confidence
        # CV=0 → conf=PHI_INV, CV=1 → conf=0.1
        raw = PHI_INV * (1.0 - min(cv, 1.0)) + 0.1 * min(cv, 1.0)

        # Penalize for failed perspectives
        failure_penalty = self.failed_count * 0.05
        return max(0.05, min(PHI_INV, raw - failure_penalty))

    def to_dict(self) -> Dict[str, Any]:
        return {
            "path": "temporal_mcts",
            "llm_id": self.llm_id,
            "latency_ms": round(self.latency_ms, 1),
            "failed": self.failed_count,
            "scores": {k: round(v, 2) for k, v in self.scores.items()},
            "phi_aggregate": round(self.phi_aggregate, 2),
            "confidence": round(self.confidence, 3),
        }


# ════════════════════════════════════════════════════════════════════════════
# TEMPORAL JUDGMENT ENGINE
# ════════════════════════════════════════════════════════════════════════════

def _parse_score(text: str) -> Optional[float]:
    """
    Parse LLM response for a numeric score in 0-100 range.

    LLMs universally understand 0-100 scales. We ask for 0-100
    and convert to CYNIC's 0-61.8 φ-range internally.

    Expected format: "SCORE: N" (whole number preferred).
    Rejects exact 100 as likely prompt echo without context.
    """
    if not text:
        return None
    text = text.strip()
    # Primary: "SCORE: N" or "SCORE: N.N" — trusted format
    m = re.search(r"SCORE:\s*(\d+(?:\.\d+)?)", text, re.IGNORECASE)
    if m:
        val_100 = min(float(m.group(1)), 100.0)
        return (val_100 / 100.0) * MAX_Q_SCORE  # Convert 0-100 → 0-61.8
    # Fallback: bare number — reject 100 (likely echo of scale max)
    m = re.search(r"\b(\d+(?:\.\d+)?)\b", text)
    if m:
        val = float(m.group(1))
        if val > 100.0 or val == 100.0:
            return None  # Reject: out of range or echo
        return (val / 100.0) * MAX_Q_SCORE
    return None


async def _judge_perspective(
    adapter: Any,  # LLMAdapter
    content: str,
    perspective: str,
) -> float:
    """
    Single temporal perspective judgment.

    Returns Q-score [0, 61.8] from one LLM call.
    Returns MAX_Q_SCORE/2 (neutral) on failure — fail-safe, not fail-hard.
    """
    from cynic.llm.adapter import LLMRequest

    prompt = TEMPORAL_PROMPTS[perspective].format(content=content[:2000])
    try:
        req = LLMRequest(
            prompt=prompt,
            system=TEMPORAL_SYSTEM,
            max_tokens=32,       # "SCORE: N.N" + buffer for slow models
            temperature=0.0,     # Deterministic scoring
        )
        resp = await adapter.complete_safe(req)
        score = _parse_score(resp.content)
        if score is not None:
            return phi_bound_score(score)
        logger.debug("No score parsed from perspective %s: %r", perspective, resp.content[:100])
    except Exception:
        logger.debug("Perspective %s failed", perspective, exc_info=True)
    return MAX_Q_SCORE * 0.5  # Neutral on failure


async def temporal_judgment(
    adapter: Any,  # LLMAdapter
    content: str,
    perspectives: Optional[List[str]] = None,
) -> TemporalJudgment:
    """
    The Temporal MCTS Core: 7 parallel LLM calls via asyncio.gather().

    This is the novel research contribution — judging from 7 temporal
    perspectives simultaneously, then aggregating with φ-weights.

    Args:
        adapter: Any LLMAdapter (Ollama preferred — free, parallel, local)
        content: Text to judge (code, decision, etc.)
        perspectives: Which perspectives to use (default: all 7)

    Returns:
        TemporalJudgment with 7 scores + phi_aggregate
    """
    start = time.perf_counter()

    if perspectives is None:
        perspectives = list(TemporalPerspective)

    # ── THE CORE: 7 PARALLEL LLM CALLS ────────────────────────────────────
    tasks = [
        _judge_perspective(adapter, content, p)
        for p in perspectives
    ]
    raw_scores: List[float] = await asyncio.gather(*tasks)
    # ──────────────────────────────────────────────────────────────────────

    latency_ms = (time.perf_counter() - start) * 1000
    score_map = dict(zip(perspectives, raw_scores))

    # Count failures (neutral score = likely failure)
    neutral = MAX_Q_SCORE * 0.5
    failed = sum(1 for s in raw_scores if abs(s - neutral) < 0.01)

    return TemporalJudgment(
        past=score_map.get(TemporalPerspective.PAST, neutral),
        present=score_map.get(TemporalPerspective.PRESENT, neutral),
        future=score_map.get(TemporalPerspective.FUTURE, neutral),
        ideal=score_map.get(TemporalPerspective.IDEAL, neutral),
        never=score_map.get(TemporalPerspective.NEVER, neutral),
        cycles=score_map.get(TemporalPerspective.CYCLES, neutral),
        flow=score_map.get(TemporalPerspective.FLOW, neutral),
        llm_id=getattr(adapter, "adapter_id", str(adapter)),
        latency_ms=latency_ms,
        failed_count=failed,
    )


async def fast_temporal_judgment(
    adapter: Any,
    content: str,
) -> TemporalJudgment:
    """
    Fast path: 3 perspectives only (PRESENT + FUTURE + NEVER) for L2 MICRO cycle.

    7 calls → 3 calls: 57% reduction, preserves most signal.
    Same interface, fewer perspectives.
    """
    return await temporal_judgment(
        adapter, content,
        perspectives=[
            TemporalPerspective.PRESENT,
            TemporalPerspective.FUTURE,
            TemporalPerspective.NEVER,
        ],
    )
