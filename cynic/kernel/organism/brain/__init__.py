"""Brain — integrated cognition system."""
# Import submodules to make them accessible as brain.llm, brain.dialogue, etc.
from . import llm
from . import dialogue
from . import cognition
from . import learning
from . import dna
from . import consensus
from . import collaborative
from . import agents

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
