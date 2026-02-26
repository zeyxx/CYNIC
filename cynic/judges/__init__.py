"""
CYNIC Judges Package

Unified Judge interface and 11 Dog implementations.

Public API:
- JudgeInterface: Abstract base for all judges
- BaseJudge: Foundation class with common utilities
- Dog1-Dog11: Specialized judge implementations
- get_all_dogs(): Factory function to create all 11 Dogs
"""

from cynic.core.judge_interface import JudgeInterface, BaseJudge
from cynic.judges.dog_implementations import (
    Dog1_CrownConsciousness,
    Dog2_WisdomAnalyzer,
    Dog3_UnderstandingSynthesizer,
    Dog4_MercyAdvocate,
    Dog5_SeverityCritic,
    Dog6_HarmonyMediator,
    Dog7_VictoryAffirmer,
    Dog8_SplendorClarifier,
    Dog9_FoundationKeeper,
    Dog10_KingdomExecutor,
    Dog11_EarthGuardian,
    get_all_dogs,
)

__all__ = [
    "JudgeInterface",
    "BaseJudge",
    "Dog1_CrownConsciousness",
    "Dog2_WisdomAnalyzer",
    "Dog3_UnderstandingSynthesizer",
    "Dog4_MercyAdvocate",
    "Dog5_SeverityCritic",
    "Dog6_HarmonyMediator",
    "Dog7_VictoryAffirmer",
    "Dog8_SplendorClarifier",
    "Dog9_FoundationKeeper",
    "Dog10_KingdomExecutor",
    "Dog11_EarthGuardian",
    "get_all_dogs",
]
