"""
Database Models for Governance Bot
"""

from datetime import datetime

from sqlalchemy import (
    JSON,
    Boolean,
    Column,
    DateTime,
    Float,
    ForeignKey,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship

Base = declarative_base()


class Community(Base):
    """Community governance configuration"""

    __tablename__ = "communities"

    community_id = Column(String, primary_key=True)
    platform = Column(String)  # "discord" or "telegram"
    community_name = Column(String)
    community_token = Column(
        String, nullable=True
    )  # Deprecated: use _community_token_encrypted
    _community_token_encrypted = Column(String, nullable=True)  # Encrypted token

    # Governance settings
    voting_period_hours = Column(Integer, default=72)
    execution_delay_hours = Column(Integer, default=24)
    quorum_percentage = Column(Float, default=25.0)
    approval_threshold_percentage = Column(Float, default=50.0)
    proposal_submission_fee_tokens = Column(Float, default=100.0)
    voting_method = Column(
        String, default="token_weighted"
    )  # "token_weighted" or "one_person_one_vote"

    # Integration
    gasdf_enabled = Column(Boolean, default=True)
    near_contract_address = Column(String, nullable=True)
    treasury_address = Column(
        String, nullable=True
    )  # Deprecated: use _treasury_address_encrypted
    _treasury_address_encrypted = Column(
        String, nullable=True
    )  # Encrypted treasury address
    fee_burn_percentage = Column(Integer, default=100)

    # CYNIC
    cynic_enabled = Column(Boolean, default=True)
    min_dogs_consensus = Column(Integer, default=6)

    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    proposals = relationship("Proposal", back_populates="community")
    e_scores = relationship("EScore", back_populates="community")


class Proposal(Base):
    """Governance proposal"""

    __tablename__ = "proposals"

    proposal_id = Column(String, primary_key=True)
    community_id = Column(String, ForeignKey("communities.community_id"))
    proposer_id = Column(String)

    title = Column(String(200))
    description = Column(Text)
    category = Column(String)  # BUDGET_ALLOCATION, GOVERNANCE_CHANGE, PARTNERSHIP, etc.
    impact_level = Column(String)  # LOW, MEDIUM, HIGH, CRITICAL

    # Timeline
    created_at = Column(DateTime, default=datetime.utcnow)
    voting_start_time = Column(DateTime)
    voting_end_time = Column(DateTime)
    execution_date = Column(DateTime, nullable=True)

    # Status
    voting_status = Column(
        String, default="PENDING"
    )  # PENDING, ACTIVE, CLOSED, CANCELLED
    execution_status = Column(
        String, default="PENDING"
    )  # PENDING, SCHEDULED, EXECUTING, COMPLETED, FAILED
    approval_status = Column(
        String, default="PENDING"
    )  # APPROVED, REJECTED, TIED, PENDING

    # CYNIC judgment
    judgment_id = Column(String, nullable=True)
    judgment_verdict = Column(String, nullable=True)  # HOWL, WAG, GROWL, BARK
    judgment_q_score = Column(Float, nullable=True)
    judgment_confidence = Column(Float, nullable=True)
    judgment_data = Column(JSON, nullable=True)  # Full judgment data

    # Votes
    yes_votes = Column(Float, default=0)
    no_votes = Column(Float, default=0)
    abstain_votes = Column(Float, default=0)

    # Execution
    near_tx_hash = Column(String, nullable=True)
    near_block_height = Column(Integer, nullable=True)

    # Learning
    outcome_determined = Column(Boolean, default=False)
    outcome = Column(String, nullable=True)
    outcome_description = Column(Text, nullable=True)
    community_satisfaction_rating = Column(Float, nullable=True)

    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    community = relationship("Community", back_populates="proposals")
    votes = relationship("Vote", back_populates="proposal")


class Vote(Base):
    """User vote on proposal"""

    __tablename__ = "votes"

    vote_id = Column(String, primary_key=True)
    proposal_id = Column(String, ForeignKey("proposals.proposal_id"))
    voter_id = Column(String)
    vote = Column(String)  # YES, NO, ABSTAIN
    vote_weight = Column(Float, default=1.0)
    reasoning = Column(Text, nullable=True)

    voted_at = Column(DateTime, default=datetime.utcnow)
    cynic_verdict_at_vote_time = Column(String, nullable=True)
    cynic_q_score_at_vote_time = Column(Float, nullable=True)

    proposal = relationship("Proposal", back_populates="votes")

    __table_args__ = (
        UniqueConstraint("proposal_id", "voter_id", name="unique_vote_per_voter"),
    )


class EScore(Base):
    """Entity reputation score (E-Score)"""

    __tablename__ = "e_scores"

    e_score_id = Column(String, primary_key=True)
    community_id = Column(String, ForeignKey("communities.community_id"))
    entity_type = Column(String)  # CYNIC, PROPOSER, VOTER
    entity_id = Column(String)

    base_score = Column(Float, default=0.618)

    # Score components (JSON)
    judgment_accuracy = Column(Float, default=0.5)
    prediction_success_rate = Column(Float, default=0.5)
    community_satisfaction = Column(Float, default=0.5)

    # Historical data
    total_judgments = Column(Integer, default=0)
    correct_predictions = Column(Integer, default=0)
    successful_proposals = Column(Integer, default=0)
    failed_proposals = Column(Integer, default=0)

    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    update_source = Column(String)  # PROPOSAL_OUTCOME, FEEDBACK, LEARNING

    community = relationship("Community", back_populates="e_scores")


class LearningOutcome(Base):
    """Outcome of proposal execution for CYNIC learning"""

    __tablename__ = "learning_outcomes"

    outcome_id = Column(String, primary_key=True)
    proposal_id = Column(String, ForeignKey("proposals.proposal_id"))

    outcome = Column(String)  # SUCCESS, PARTIAL, FAILED
    success_metrics = Column(JSON)  # Actual metrics achieved
    predicted_metrics = Column(JSON)  # CYNIC's predicted metrics
    community_satisfaction_rating = Column(Float)  # 1-5 stars

    recorded_at = Column(DateTime, default=datetime.utcnow)
    learned_at = Column(DateTime, nullable=True)


class CommunityUser(Base):
    """User metadata for community"""

    __tablename__ = "community_users"

    user_id = Column(String, primary_key=True)
    community_id = Column(String, ForeignKey("communities.community_id"))
    discord_user_id = Column(String)
    username = Column(String)

    token_balance = Column(Float, default=0)
    reputation_score = Column(Float, default=0.5)

    # Activity stats
    proposals_submitted = Column(Integer, default=0)
    votes_cast = Column(Integer, default=0)
    feedback_given = Column(Integer, default=0)

    joined_at = Column(DateTime, default=datetime.utcnow)
    last_active = Column(DateTime, default=datetime.utcnow)
