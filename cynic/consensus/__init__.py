"""
CYNIC Consensus Module

Byzantine Fault Tolerant (PBFT) consensus for aggregating 11-Dog judgments
into unified verdicts.

Key concepts:
- Tolerates up to f = floor((n-1)/3) faulty Dogs
- Requires supermajority > 2f to reach consensus
- For 11 Dogs: need >= 8 votes (Byzantine majority)
"""
from cynic.consensus.pbft_engine import PBFTEngine

__all__ = ["PBFTEngine"]
