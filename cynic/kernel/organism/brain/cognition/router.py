"""
CYNIC Cognitive Router  Dual-Process Theory (S1/S2).

Routes tasks between local "System 1" (Ollama) and cloud "System 2" (Gemini/Claude).
Implements metabolic gating: saves costs by using local models for simple tasks.
"""

import logging
import re
from typing import Dict, Any, Optional
from cynic.kernel.core.phi import PHI, PHI_INV

logger = logging.getLogger("cynic.kernel.cognition.router")

class CognitiveRouter:
    """
    Orchestrates System 1 (Local/Fast) and System 2 (Cloud/Deep) logic.
    """
    def __init__(self, complexity_threshold: float = 0.618): # PHI_INV
        self.threshold = complexity_threshold

    def evaluate_complexity(self, task_description: str) -> float:
        """
        Heuristic evaluation of task complexity.
        V3.5: Looks for architectural keywords and depth indicators.
        """
        score = 0.3 # Base complexity
        
        # Architectural triggers (System 2)
        heavy_keywords = [
            "fractal", "architecture", "alignment", "ethical", 
            "security audit", "deep analysis", "long-term", "complex"
        ]
        for kw in heavy_keywords:
            if kw.lower() in task_description.lower():
                score += 0.1
        
        # Formatting/summarization triggers (System 1)
        light_keywords = ["summarize", "format", "extract", "clean", "list", "convert"]
        for kw in light_keywords:
            if kw.lower() in task_description.lower():
                score -= 0.05
                
        return min(1.0, max(0.0, score))

    def should_escalate(self, complexity: float) -> bool:
        """True if task requires System 2 (Gemini/Claude)."""
        return complexity >= self.threshold

    async def route(self, task_description: str) -> Dict[str, Any]:
        """
        Decision logic for the current task.
        """
        complexity = self.evaluate_complexity(task_description)
        needs_s2 = self.should_escalate(complexity)
        
        decision = {
            "complexity": complexity,
            "mode": "SYSTEM_2 (Deep Reasoning)" if needs_s2 else "SYSTEM_1 (Local Reflex)",
            "target": "GEMINI_3" if needs_s2 else "OLLAMA_LOCAL",
            "reason": f"Complexity {complexity:.2f} relative to threshold {self.threshold:.2f}"
        }
        
        logger.info(f"Cognitive Router: {decision['mode']} selected for task.")
        return decision

# Registry instance
router = CognitiveRouter()
