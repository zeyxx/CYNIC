"""
Unified State Models for CYNIC Consciousness

Consolidates immutable state contracts for judgments, learning, and value creation.
Enforces φ-bounds and allows extra metadata for topological flexibility.
"""
from __future__ import annotations

import time
from collections import deque
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field, ConfigDict

from cynic.kernel.core.phi import MAX_CONFIDENCE, PHI_INV, fibonacci

# ── Base Model with extra="allow" for flexibility ──
class UnifiedModel(BaseModel):
    model_config = ConfigDict(extra="allow", frozen=False)

class UnifiedJudgment(UnifiedModel):
    """Immutable record of a single judgment verdict."""
    judgment_id: str
    verdict: str # HOWL | WAG | GROWL | BARK
    q_score: float
    confidence: float
    dog_votes: Dict[Any, Any] = Field(default_factory=dict)
    axiom_scores: Dict[str, float] = Field(default_factory=dict)
    reality: str = "CODE"
    state_key: str = ""
    analysis: str = "JUDGE"
    time_dim: str = "PRESENT"
    lod: int = 1
    level_used: str = "REFLEX"
    reasoning: str = ""
    consensus_reason: str = ""
    timestamp: float = Field(default_factory=time.time)
    consensus_reached: bool = True
    consensus_votes: int = 0
    consensus_quorum: int = 7

class UnifiedLearningOutcome(UnifiedModel):
    """Immutable record of predicted vs actual outcome."""
    judgment_id: str
    predicted_verdict: str
    actual_verdict: str
    satisfaction_rating: float # [0, 1]
    timestamp: float = Field(default_factory=time.time)

class ValueCreation(UnifiedModel):
    """Phase 3: Immutable record of a value-creating event."""
    creation_id: str
    creator_id: str
    creation_type: str 
    description: str
    timestamp: float = Field(default_factory=time.time)
    direct_impact: float = 0.0
    indirect_impact: float = 0.0
    collective_impact: float = 0.0
    temporal_impact: float = 0.0

class ImpactMeasurement(UnifiedModel):
    """Phase 3: Computed impact metrics."""
    human_id: str
    total_impact: float
    dimension_scores: Dict[str, float] = Field(default_factory=dict)
    governance_weight: float = 0.01
    timestamp: float = Field(default_factory=time.time)

class GovernanceCommunity(UnifiedModel):
    """Immutable record of a community."""
    community_id: str
    name: str
    platform: str
    token_symbol: str = "CYNIC"
    voting_period_h: int = 72
    quorum_pct: float = 25.0
    threshold_pct: float = 50.0
    gasdf_enabled: bool = True
    near_address: Optional[str] = None
    created_at: float = Field(default_factory=time.time)

class GovernanceProposal(UnifiedModel):
    """Immutable record of a governance proposal."""
    proposal_id: str
    community_id: str
    proposer_id: str
    title: str
    description: str
    category: str
    status: str = "PENDING"
    created_at: float = Field(default_factory=time.time)
    voting_end: float = 0.0
    yes_votes: float = 0.0
    no_votes: float = 0.0
    judgment_id: Optional[str] = None
    verdict: Optional[str] = None

class GovernanceVote(UnifiedModel):
    """Immutable record of a user vote."""
    vote_id: str
    proposal_id: str
    voter_id: str
    choice: str
    weight: float = 1.0
    timestamp: float = Field(default_factory=time.time)

# ── Buffers ──

class JudgmentBuffer(BaseModel):
    buffer: deque = Field(default_factory=lambda: deque(maxlen=fibonacci(11)))
    def add(self, item: UnifiedJudgment): self.buffer.append(item)
    def get_recent(self, n: int): return list(self.buffer)[-n:]

class OutcomeBuffer(BaseModel):
    buffer: deque = Field(default_factory=lambda: deque(maxlen=fibonacci(10)))
    def add(self, item: UnifiedLearningOutcome): self.buffer.append(item)
    def get_recent(self, n: int): return list(self.buffer)[-n:]

class ValueBuffer(BaseModel):
    buffer: deque = Field(default_factory=lambda: deque(maxlen=fibonacci(12)))
    def add(self, item: ValueCreation): self.buffer.append(item)

class ImpactBuffer(BaseModel):
    buffer: deque = Field(default_factory=lambda: deque(maxlen=fibonacci(10)))
    def add(self, item: ImpactMeasurement): self.buffer.append(item)

class CommunityBuffer(BaseModel):
    buffer: Dict[str, GovernanceCommunity] = Field(default_factory=dict)
    def add(self, item: GovernanceCommunity): self.buffer[item.community_id] = item

class ProposalBuffer(BaseModel):
    buffer: deque = Field(default_factory=lambda: deque(maxlen=fibonacci(11)))
    def add(self, item: GovernanceProposal): self.buffer.append(item)

class UnifiedConsciousState(BaseModel):
    """Consolidated state of CYNIC's consciousness."""
    recent_judgments: JudgmentBuffer = Field(default_factory=JudgmentBuffer)
    learning_outcomes: OutcomeBuffer = Field(default_factory=OutcomeBuffer)
    value_creations: ValueBuffer = Field(default_factory=ValueBuffer)
    impact_measurements: ImpactBuffer = Field(default_factory=ImpactBuffer)
    communities: CommunityBuffer = Field(default_factory=CommunityBuffer)
    proposals: ProposalBuffer = Field(default_factory=ProposalBuffer)
    total_judgments: int = 0
    dog_agreement_scores: Dict[int, float] = Field(default_factory=dict)
    consciousness_level: str = "REFLEX"

    def add_judgment(self, j: UnifiedJudgment):
        self.recent_judgments.add(j)
        self.total_judgments += 1

    def add_outcome(self, o: UnifiedLearningOutcome):
        self.learning_outcomes.add(o)

    def add_value_creation(self, v: ValueCreation):
        self.value_creations.add(v)

    def add_impact_measurement(self, i: ImpactMeasurement):
        self.impact_measurements.add(i)
        
    def add_community(self, c: GovernanceCommunity):
        self.communities.add(c)
        
    def add_proposal(self, p: GovernanceProposal):
        self.proposals.add(p)

    async def reach_consensus_judgment(self, judgments: list[UnifiedJudgment]) -> UnifiedJudgment:
        """Reach consensus using PBFT engine (convenience wrapper)."""
        from cynic.brain.consensus.pbft_engine import PBFTEngine
        engine = PBFTEngine(num_dogs=len(judgments) or 11)
        return await engine.reach_consensus(judgments)
