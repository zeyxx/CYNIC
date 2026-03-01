"""
DialogueAgent — Human-readable explanation of CYNIC's internal logic.

Uses the ReasoningEngine to structure data and the Universal LLM Registry 
to generate the final explanation, prioritizing local hardware.
"""
from __future__ import annotations
import logging
from typing import Any, Optional

from cynic.kernel.organism.brain.dialogue.reasoning import ReasoningEngine
from cynic.kernel.core.event_bus import get_core_bus, CoreEvent, Event
from cynic.kernel.organism.brain.llm.adapter import LLMRequest, get_registry

logger = logging.getLogger("cynic.kernel.organism.brain.dialogue.agent")

class DialogueAgent:
    """
    The voice of CYNIC.
    """
    
    def __init__(self, reasoning_engine: Optional[ReasoningEngine] = None):
        self.reasoning = reasoning_engine or ReasoningEngine()
        self.registry = get_registry()

    async def explain_judgment(self, judgment: Any, question: str) -> str:
        """
        Generates a human-readable explanation for a judgment.
        Uses local models if available to ensure sovereignty.
        """
        # 1. Prepare structured context
        # We convert the judgment to dict if it's a model
        j_dict = judgment.model_dump() if hasattr(judgment, "model_dump") else judgment
        context = self.reasoning.format_judgment_reasoning(j_dict)
        
        # 2. Pick best LOCAL model
        # We prioritize 'llama_cpp' or 'ollama' providers
        adapter = self.registry.get_best_for("SAGE", "explanation")
        
        if not adapter:
            logger.warning("DialogueAgent: No LLM adapter available. Returning raw reasoning.")
            return context

        # 3. Prompt construction
        prompt = f"""
        You are the voice of CYNIC, a mathematical artificial organism.
        Explain this judgment to a human in a clear, authentic, and sovereign way.
        
        CONTEXT OF THE QUESTION: {question}
        INTERNAL REASONING DATA:
        {context}
        
        Provide a concise explanation (2-3 sentences) starting with the verdict.
        """
        
        request = LLMRequest(
            prompt=prompt,
            max_tokens=256,
            temperature=0.3 # Low temperature for fidelity
        )
        
        try:
            response = await adapter.complete_safe(request)
            if response.is_success:
                return response.content
            return f"{context}\n(Note: LLM explanation failed: {response.error})"
        except Exception as e:
            logger.error("DialogueAgent failed to explain: %s", e)
            return context
