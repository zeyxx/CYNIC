"""
DialogueAgent â€” Human-readable explanation of CYNIC's internal logic.

Uses the ReasoningEngine to structure data and the Universal LLM Registry
to generate the final explanation, prioritizing local hardware.
"""

from __future__ import annotations

import logging
from typing import Any, Optional

from cynic.kernel.organism.brain.llm.adapter import LLMRequest, LLMRegistry

logger = logging.getLogger("cynic.kernel.organism.brain.dialogue.agent")


class DialogueAgent:
    """
    Agent that provides human-readable explanations of CYNIC's judgments.
    """

    def __init__(self, llm_registry: Optional[LLMRegistry] = None):
        """Initialize with an isolated LLM registry."""
        self._llm_registry = llm_registry

    async def explain_judgment(self, judgment: Any, question: str = "") -> str:
        """
        Structure a judgment into a natural language explanation.
        """
        if self._llm_registry is None:
            logger.warning("DialogueAgent: No LLM registry available. Returning raw reasoning.")
            return getattr(judgment, "reasoning", "No reasoning provided.")

        context = getattr(judgment, "reasoning", "")
        
        # Select the best model for explanation
        adapter = self._llm_registry.get_best_for("DIALOGUE", "EXPLAIN")
        if not adapter:
            logger.warning("DialogueAgent: No LLM adapter available. Returning raw reasoning.")
            return context

        prompt = f"""
        Explain the following CYNIC judgment clearly:
        Verdict: {judgment.verdict}
        Q-Score: {judgment.q_score}
        Reasoning: {context}
        Context: {question}
        """

        try:
            response = await adapter.complete(LLMRequest(prompt=prompt, system="You are the Voice of CYNIC."))
            if response.is_success:
                return response.content
            return f"{context}\n(Note: LLM explanation failed: {response.error})"
        except Exception as e:
            logger.error("DialogueAgent failed to explain: %s", e)
            return context
