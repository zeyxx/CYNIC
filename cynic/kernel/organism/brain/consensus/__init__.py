"""
CYNIC Consensus Module

Byzantine Fault Tolerant (PBFT) consensus for aggregating 11-Dog judgments
into unified verdicts.

Key concepts:
- Tolerates up to f = floor((n-1)/3) faulty Dogs
- Requires supermajority > 2f to reach consensus
- For 11 Dogs: need >= 8 votes (Byzantine majority)
"""
from cynic.kernel.organism.brain.consensus.pbft_engine import PBFTEngine

_ENGINE = None

def get_consensus_engine() -> PBFTEngine:
    global _ENGINE
    if _ENGINE is None:
        _ENGINE = PBFTEngine()
    return _ENGINE

__all__ = ["PBFTEngine", "get_consensus_engine"]
