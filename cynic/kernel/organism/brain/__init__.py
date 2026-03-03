"""Brain â€" integrated cognition system."""

# Import submodules to make them accessible as brain.llm, brain.dialogue, etc.
from . import agents, cognition, collaborative, consensus, dialogue, dna, learning, llm

__all__ = [
    "llm",
    "dialogue",
    "cognition",
    "learning",
    "dna",
    "consensus",
    "collaborative",
    "agents",
]
