"""
CYNIC Temporal Judgment  7-Perspective Fractal Scoring.

Evaluates a Cell across 7 temporal dimensions to build a robust Q-Score.
This is the core reasoning engine for high-level Dogs like SAGE.

Perspectives:
1. PAST: Historical alignment and patterns.
2. PRESENT: Immediate technical/ethical state.
3. FUTURE: Long-term impact and scalability.
4. CYCLE: Periodic behavior and resonance.
5. TREND: Directional momentum.
6. EMERGENCE: Novelty and system-level properties.
7. TRANSCENDENCE: Final alignment with the Golden Ratio (PHI).
"""

from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.kernel.core.phi import PHI, PHI_INV, PHI_INV_2, weighted_geometric_mean
from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.kernel.brain.llm.temporal")

@dataclass
class TemporalJudgment:
    """The aggregate result of a 7-perspective temporal analysis."""
    phi_aggregate: float
    confidence: float
    reasoning: str
    perspective_scores: Dict[str, float]
    llm_id: str
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "phi_aggregate": self.phi_aggregate,
            "confidence": self.confidence,
            "reasoning": self.reasoning,
            "perspective_scores": self.perspective_scores,
            "llm_id": self.llm_id,
            "metadata": self.metadata
        }

async def temporal_judgment(
    adapter: LLMAdapter, 
    content: str, 
    system_prompt: str = "You are a fractal reasoning agent.",
    multimodal_data: Optional[List[Any]] = None
) -> TemporalJudgment:
    """
    Performs a full temporal audit of the provided content.
    For simplicity in this version, we use a single LLM call that evaluates all 7 perspectives.
    """
    t0 = time.perf_counter()
    
    # 1. Construct the 7-perspective prompt
    full_prompt = f"""
    AUDIT TASK: Analyze the following content across 7 temporal perspectives.
    
    CONTENT:
    {content}
    
    PERSPECTIVES TO EVALUATE:
    1. PAST (History/Legacy)
    2. PRESENT (Current State)
    3. FUTURE (Impact/Scale)
    4. CYCLE (Resonance)
    5. TREND (Direction)
    6. EMERGENCE (Novelty)
    7. TRANSCENDENCE (PHI Alignment)
    
    OUTPUT FORMAT:
    Output exactly 7 scores (0-100) followed by a brief aggregate reasoning.
    Format:
    PAST: X
    PRESENT: X
    FUTURE: X
    CYCLE: X
    TREND: X
    EMERGENCE: X
    TRANSCENDENCE: X
    REASONING: [Your justification]
    """
    
    # 2. Execute LLM Call
    request = LLMRequest(
        prompt=full_prompt,
        system=system_prompt,
        multimodal_data=multimodal_data or [],
        temperature=PHI_INV # 0.618 for creative but bounded reasoning
    )
    
    response = await adapter.complete_safe(request)
    
    if not response.is_success:
        logger.error(f"Temporal judgment failed: {response.error}")
        return TemporalJudgment(0.0, 0.0, f"Error: {response.error}", {}, adapter.adapter_id)

    # 3. Parse Scores
    import re
    scores = {}
    perspectives = ["PAST", "PRESENT", "FUTURE", "CYCLE", "TREND", "EMERGENCE", "TRANSCENDENCE"]
    
    for p in perspectives:
        match = re.search(fr"{p}:\s*(\d+)", response.content)
        scores[p] = float(match.group(1)) if match else 50.0
        
    reasoning_match = re.search(r"REASONING:\s*(.*)", response.content, re.DOTALL)
    reasoning = reasoning_match.group(1).strip() if reasoning_match else "No reasoning provided."
    
    # 4. PHI-Weighted Aggregation
    # We weight TRANSCENDENCE and FUTURE higher
    weights = [1.0, 1.0, PHI, 1.0, 1.0, PHI, PHI**2]
    score_list = [scores[p] for p in perspectives]
    
    phi_aggregate = weighted_geometric_mean(score_list, weights)
    
    return TemporalJudgment(
        phi_aggregate=phi_aggregate,
        confidence=0.618, # Base confidence for LLM reasoning
        reasoning=reasoning,
        perspective_scores=scores,
        llm_id=adapter.llm_id,
        metadata={"latency_ms": (time.perf_counter() - t0) * 1000}
    )
