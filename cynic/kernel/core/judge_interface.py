"""
Unified Judge Interface and BaseJudge Foundation

This module defines the abstract interface for all CYNIC judges (Dogs) and provides
the base class with common functionality.

Architecture:
â"Œâ"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"
â"‚          UNIFIED JUDGE ARCHITECTURE                     â"‚
â"œâ"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"¤
â"‚                                                         â"‚
â"‚  JudgeInterface (abstract)                              â"‚
â"‚  â"œâ"€ Defines: judge(proposal_text, context) â' Judgment  â"‚
â"‚  â""â"€ Async requirement                                  â"‚
â"‚                                                         â"‚
â"‚  BaseJudge (foundation)                                 â"‚
â"‚  â"œâ"€ Dog ID (1-11)                                       â"‚
â"‚  â"œâ"€ Dog Name (e.g., "Crown Consciousness")             â"‚
â"‚  â"œâ"€ Axiom Focus (FIDELITY, PHI, VERIFY, CULTURE, BURN) â"‚
â"‚  â"œâ"€ Helpers:                                            â"‚
â"‚  â"‚  â"œâ"€ _calculate_phi_bounded_confidence()              â"‚
â"‚  â"‚  â""â"€ Performance tracking                             â"‚
â"‚  â""â"€ Abstract judge() method                             â"‚
â"‚                                                         â"‚
â"‚  Dog1-Dog11 (specialized implementations)               â"‚
â"‚  â""â"€ Each implements judge() with axiom-specific logic   â"‚
â"‚                                                         â"‚
â""â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"˜

Key principles:
1. All judges are async
2. All judges return UnifiedJudgment
3. Confidence is Ï-bounded to MAX_CONFIDENCE (0.618)
4. Q-Score is in [0, 100]
5. Verdict is in {HOWL, WAG, GROWL, BARK}
6. Each Dog specializes in one axiom
7. Judgment reasoning must be provided
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any

from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.core.unified_state import UnifiedJudgment


class JudgeInterface(ABC):
    """
    Abstract interface for all CYNIC judges.

    All judges must:
    1. Implement judge(proposal_text, context) â' UnifiedJudgment
    2. Be async methods
    3. Return a complete UnifiedJudgment with:
       - verdict: HOWL, WAG, GROWL, or BARK
       - q_score: [0, 100]
       - confidence: [0, MAX_CONFIDENCE] (0.618)
       - axiom_scores: Dict[axiom] â' [0, 100]
       - dog_votes: Dict[dog_id] â' {vote, confidence}
       - reasoning: str (explanation)
       - judgment_id: str (unique)
    """

    @abstractmethod
    async def judge(self, proposal_text: str, context: dict[str, Any]) -> UnifiedJudgment:
        """
        Judge a proposal and return structured verdict.

        Args:
            proposal_text: The proposal to judge (text)
            context: Dict with context (community values, evidence, etc)

        Returns:
            UnifiedJudgment: Immutable judgment with all verdict data

        Raises:
            ValueError: If judgment violates Ï-bounds or verdict constraints
        """
        pass


class BaseJudge(JudgeInterface):
    """
    Base class for all Dog implementations.

    Provides:
    - Dog metadata (id, name, axiom_focus)
    - Ï-bounded confidence calculation
    - Performance tracking (judgments_made, confidence_history)
    - Common validation logic
    - Async enforcement

    Subclasses must implement:
    - judge(proposal_text, context) â' UnifiedJudgment
    """

    def __init__(self, dog_id: int, dog_name: str, axiom_focus: str):
        """
        Initialize a Dog judge.

        Args:
            dog_id: Dog ID (1-11, corresponding to Sefirot)
            dog_name: Human-readable name (e.g., "Crown Consciousness")
            axiom_focus: Primary axiom focus (FIDELITY, PHI, VERIFY, CULTURE, BURN)

        Raises:
            ValueError: If dog_id not in 1-11
        """
        if not (1 <= dog_id <= 11):
            raise ValueError(f"dog_id must be in [1, 11], got {dog_id}")

        self.dog_id = dog_id
        self.dog_name = dog_name
        self.axiom_focus = axiom_focus

        # Performance tracking
        self.judgments_made = 0
        self.confidence_history: list[float] = []

    def _calculate_phi_bounded_confidence(self, base_confidence: float) -> float:
        """
        Bound confidence to Ïâ»Â¹ = 0.618 (max).

        This implements the "Law of Doubt": even the most confident judgment
        respects Ï's natural maximum (61.8%).

        Args:
            base_confidence: Raw confidence [0, 1]

        Returns:
            float: Bounded confidence [0, 0.618]
        """
        return min(base_confidence, MAX_CONFIDENCE)

    @abstractmethod
    async def judge(self, proposal_text: str, context: dict[str, Any]) -> UnifiedJudgment:
        """
        Judge a proposal (must be implemented by subclass).

        Args:
            proposal_text: The proposal to judge
            context: Context dict

        Returns:
            UnifiedJudgment: The judgment verdict
        """
        pass
