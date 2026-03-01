"""Interactive dialogue mode for CYNIC conversations."""

from __future__ import annotations

from typing import Optional, Any
from cynic.kernel.organism.brain.dialogue.storage import get_dialogue_store
from cynic.kernel.organism.brain.dialogue.models import UserMessage, CynicMessage
from cynic.kernel.organism.brain.dialogue.llm_bridge import LLMBridge
from cynic.kernel.organism.brain.dialogue.reasoning import ReasoningEngine
from cynic.kernel.organism.brain.learning.memory_store import get_memory_store
from cynic.kernel.observability.symbiotic_state_manager import get_current_state


class DialogueMode:
    """Interactive dialogue mode for conversing with CYNIC."""

    def __init__(self):
        self.storage = None
        self.memory_store = None
        self.llm_bridge = LLMBridge()
        self.reasoning_engine = ReasoningEngine()
        self._initialized = False

    async def initialize(self) -> None:
        """Initialize dialogue mode async components."""
        if not self._initialized:
            self.storage = await get_dialogue_store()
            self.memory_store = await get_memory_store()
            self._initialized = True

    async def process_message(self, user_input: str) -> str:
        """Process user message and generate response.

        Args:
            user_input: The user's message or question.

        Returns:
            CYNIC's response as a string.
        """
        await self.initialize()

        # Determine message type
        message_type = self._classify_message_type(user_input)

        # Save user message
        user_msg = UserMessage(
            message_type=message_type,
            content=user_input,
            user_confidence=0.5,
            related_judgment_id=None
        )
        await self.storage.save_message(user_msg)

        # Get relationship memory for personalization
        memory = await self.memory_store.load_memory()

        # Get current judgment context
        try:
            state = await get_current_state()
            # cynic_thinking is a string, so we create a minimal judgment dict
            judgment = {
                "verdict": "UNKNOWN",
                "q_score": 0,
                "confidence": 0,
                "axiom_scores": {}
            }
        except Exception:
            # Fallback if state is not available
            judgment = {
                "verdict": "UNKNOWN",
                "q_score": 0,
                "confidence": 0,
                "axiom_scores": {}
            }

        # Prepare context for LLM
        context = self._prepare_context_for_llm(user_input, judgment, memory)

        # Generate response
        response_text = await self.llm_bridge.generate_response(context)

        # Save CYNIC response
        cynic_msg = CynicMessage(
            message_type="response",
            content=response_text,
            confidence=0.3,
            axiom_scores={},
            source_judgment_id=None
        )
        await self.storage.save_message(cynic_msg)

        return response_text

    def _classify_message_type(self, message: str) -> str:
        """Classify user message type.

        Args:
            message: The user's message to classify.

        Returns:
            Classification: "question", "feedback", "exploration", or "statement".
        """
        lower = message.lower()

        if any(word in lower for word in ["why", "how", "what", "explain"]):
            return "question"
        elif any(word in lower for word in ["wrong", "actually", "should be"]):
            return "feedback"
        elif any(word in lower for word in ["if", "what if", "try", "explore"]):
            return "exploration"
        else:
            return "statement"

    def _prepare_context_for_llm(self, user_input: str,
                                 judgment: dict[str, Any],
                                 memory: Optional[Any] = None) -> dict[str, Any]:
        """Prepare structured context for Claude API.

        Args:
            user_input: The user's question or statement.
            judgment: CYNIC's current judgment data.
            memory: User's relationship memory (optional).

        Returns:
            Dictionary with context for LLM prompt generation.
        """
        # Ensure judgment is a dict
        if not isinstance(judgment, dict):
            judgment = {
                "verdict": "UNKNOWN",
                "q_score": 0,
                "confidence": 0,
                "axiom_scores": {}
            }

        context = {
            "question": user_input,
            "verdict": judgment.get("verdict", "UNKNOWN"),
            "q_score": judgment.get("q_score", 0),
            "confidence": judgment.get("confidence", 0),
            "axiom_scores": judgment.get("axiom_scores", {})
        }

        if memory:
            context["communication_style"] = memory.user_style
            context["verbosity"] = memory.communication_style.get(
                "verbosity", "balanced"
            )

        # Add reasoning summary
        reasoning_summary = self.reasoning_engine.format_judgment_reasoning(judgment)
        context["reasoning_summary"] = reasoning_summary

        return context

    async def get_greeting(self) -> str:
        """Get initial greeting message.

        Returns:
            Greeting text introducing CYNIC's dialogue mode.
        """
        return """I'm CYNIC, your AI research partner. Let's discuss and explore together!

You can:
- Ask me WHY I made a judgment (I'll explain my reasoning)
- Give me FEEDBACK if I was wrong (I'll learn from you)
- Propose EXPERIMENTS to try novel approaches
- Ask QUESTIONS about my thinking

What's on your mind?"""

    async def close(self) -> None:
        """Close dialogue mode and cleanup resources."""
        if self.llm_bridge:
            await self.llm_bridge.close()
