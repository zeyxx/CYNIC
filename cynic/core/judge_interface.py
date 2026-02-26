"""
Unified Judge Interface and BaseJudge Foundation

This module defines the abstract interface for all CYNIC judges (Dogs) and provides
the base class with common functionality.

Architecture:
┌─────────────────────────────────────────────────────────┐
│          UNIFIED JUDGE ARCHITECTURE                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  JudgeInterface (abstract)                              │
│  ├─ Defines: judge(proposal_text, context) → Judgment  │
│  └─ Async requirement                                  │
│                                                         │
│  BaseJudge (foundation)                                 │
│  ├─ Dog ID (1-11)                                       │
│  ├─ Dog Name (e.g., "Crown Consciousness")             │
│  ├─ Axiom Focus (FIDELITY, PHI, VERIFY, CULTURE, BURN) │
│  ├─ Helpers:                                            │
│  │  ├─ _calculate_phi_bounded_confidence()              │
│  │  └─ Performance tracking                             │
│  └─ Abstract judge() method                             │
│                                                         │
│  Dog1-Dog11 (specialized implementations)               │
│  └─ Each implements judge() with axiom-specific logic   │
│                                                         │
└─────────────────────────────────────────────────────────┘

Key principles:
1. All judges are async
2. All judges return UnifiedJudgment
3. Confidence is φ-bounded to MAX_CONFIDENCE (0.618)
4. Q-Score is in [0, 100]
5. Verdict is in {HOWL, WAG, GROWL, BARK}
6. Each Dog specializes in one axiom
7. Judgment reasoning must be provided
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Dict, Any

from cynic.core.unified_state import UnifiedJudgment
from cynic.core.phi import PHI_INV, MAX_CONFIDENCE


class JudgeInterface(ABC):
    """
    Abstract interface for all CYNIC judges.

    All judges must:
    1. Implement judge(proposal_text, context) → UnifiedJudgment
    2. Be async methods
    3. Return a complete UnifiedJudgment with:
       - verdict: HOWL, WAG, GROWL, or BARK
       - q_score: [0, 100]
       - confidence: [0, MAX_CONFIDENCE] (0.618)
       - axiom_scores: Dict[axiom] → [0, 100]
       - dog_votes: Dict[dog_id] → {vote, confidence}
       - reasoning: str (explanation)
       - judgment_id: str (unique)
    """

    @abstractmethod
    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """
        Judge a proposal and return structured verdict.

        Args:
            proposal_text: The proposal to judge (text)
            context: Dict with context (community values, evidence, etc)

        Returns:
            UnifiedJudgment: Immutable judgment with all verdict data

        Raises:
            ValueError: If judgment violates φ-bounds or verdict constraints
        """
        pass


class BaseJudge(JudgeInterface):
    """
    Base class for all Dog implementations.

    Provides:
    - Dog metadata (id, name, axiom_focus)
    - φ-bounded confidence calculation
    - Performance tracking (judgments_made, confidence_history)
    - Common validation logic
    - Async enforcement

    Subclasses must implement:
    - judge(proposal_text, context) → UnifiedJudgment
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
        Bound confidence to φ⁻¹ = 0.618 (max).

        This implements the "Law of Doubt": even the most confident judgment
        respects φ's natural maximum (61.8%).

        Args:
            base_confidence: Raw confidence [0, 1]

        Returns:
            float: Bounded confidence [0, 0.618]
        """
        return min(base_confidence, MAX_CONFIDENCE)

    @abstractmethod
    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """
        Judge a proposal (must be implemented by subclass).

        Args:
            proposal_text: The proposal to judge
            context: Context dict

        Returns:
            UnifiedJudgment: The judgment verdict
        """
        pass
