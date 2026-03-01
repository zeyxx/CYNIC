"""Reasoning engine for explaining CYNIC's judgments."""

from __future__ import annotations

from typing import Any


class ReasoningEngine:
    """Extract and format reasoning from CYNIC's judgment process."""

    # Map axioms to explanation templates
    AXIOM_EXPLANATIONS = {
        "PHI": "Mathematical/structural harmony and elegance",
        "BURN": "Energy expenditure, sustainability, and non-extraction",
        "VERIFY": "Verifiability, accuracy, and truth",
        "CULTURE": "Community resonance, authenticity, and alignment",
        "FIDELITY": "Faithful execution and authentic intention",
    }

    def format_judgment_reasoning(self, judgment: dict[str, Any]) -> str:
        """Format a judgment into human-readable reasoning."""
        verdict = judgment.get("verdict", "UNKNOWN")
        q_score = judgment.get("q_score", 0)
        confidence = judgment.get("confidence", 0)

        lines = [
            f"I chose {verdict} with confidence {confidence:.1%}",
            f"Quality score: {q_score}/100",
        ]

        # Add axiom influences
        axiom_scores = judgment.get("axiom_scores", {})
        if axiom_scores:
            high_axioms = sorted(axiom_scores.items(), key=lambda x: x[1], reverse=True)[:3]

            lines.append("Key axiom influences:")
            for axiom, score in high_axioms:
                lines.append(f"  • {axiom}: {score:.1%}")

        # Add dog vote summary
        dog_votes = judgment.get("dog_votes", {})
        if dog_votes:
            consensus = list(dog_votes.values())[0]
            agreement_count = sum(1 for v in dog_votes.values() if v == consensus)
            lines.append(f"Consensus: {agreement_count}/{len(dog_votes)} dogs agreed")

        return "\n".join(lines)

    def extract_axiom_explanations(self, axiom_scores: dict[str, float]) -> dict[str, str]:
        """Extract which axioms mattered and why."""
        explanations = {}

        for axiom, score in sorted(axiom_scores.items(), key=lambda x: x[1], reverse=True):
            if axiom in self.AXIOM_EXPLANATIONS:
                explanation = self.AXIOM_EXPLANATIONS[axiom]
                explanations[axiom] = f"{explanation} ({score:.1%})"

        return explanations

    def create_context_for_claude(
        self, question: str, judgment: dict[str, Any], user_communication_style: str = "balanced"
    ) -> dict[str, Any]:
        """Create structured context for Claude API call."""
        return {
            "question": question,
            "verdict": judgment.get("verdict"),
            "q_score": judgment.get("q_score"),
            "confidence": judgment.get("confidence"),
            "axiom_scores": judgment.get("axiom_scores", {}),
            "dog_votes": judgment.get("dog_votes", {}),
            "reasoning_summary": self.format_judgment_reasoning(judgment),
            "axiom_explanations": self.extract_axiom_explanations(judgment.get("axiom_scores", {})),
            "communication_style": user_communication_style,
            "verbosity": self._determine_verbosity(user_communication_style),
        }

    def _determine_verbosity(self, style: str) -> str:
        """Determine how detailed response should be."""
        verbosity_map = {
            "concise": "1-2 sentences max",
            "balanced": "2-3 sentences with detail",
            "detailed": "comprehensive explanation with examples",
        }
        return verbosity_map.get(style, verbosity_map["balanced"])
