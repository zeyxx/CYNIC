"""
Database operations for Governance Bot
"""

import logging
from contextlib import asynccontextmanager
from datetime import datetime, timedelta
from sqlalchemy import create_engine, select
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession, async_sessionmaker

from models import Base, Community, Proposal, Vote, EScore, CommunityUser, LearningOutcome
from config import DATABASE_URL

logger = logging.getLogger(__name__)

# Async database setup
engine = None
async_session = None


async def init_db(db_url: str = DATABASE_URL):
    """Initialize database"""
    global engine, async_session

    # Convert sqlite to aiosqlite for async support
    if db_url.startswith("sqlite"):
        db_url = db_url.replace("sqlite", "sqlite+aiosqlite", 1)

    engine = create_async_engine(db_url, echo=False)
    async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    logger.info(f"Database initialized: {db_url}")


async def get_session() -> AsyncSession:
    """Get async database session"""
    return async_session()


@asynccontextmanager
async def session_context():
    """Context manager for automatic session cleanup"""
    session = await get_session()
    try:
        yield session
    finally:
        await session.close()


# Community operations
async def get_community(session: AsyncSession, community_id: str) -> Community:
    """Get community by ID"""
    result = await session.execute(select(Community).where(Community.community_id == community_id))
    return result.scalar_one_or_none()


async def create_community(session: AsyncSession, community_data: dict) -> Community:
    """Create new community"""
    community = Community(**community_data)
    session.add(community)
    await session.commit()
    logger.info(f"Community created: {community.community_id}")
    return community


# Proposal operations
async def get_proposal(session: AsyncSession, proposal_id: str) -> Proposal:
    """Get proposal by ID"""
    result = await session.execute(select(Proposal).where(Proposal.proposal_id == proposal_id))
    return result.scalar_one_or_none()


async def create_proposal(session: AsyncSession, proposal_data: dict) -> Proposal:
    """Create new proposal"""
    proposal = Proposal(**proposal_data)
    session.add(proposal)
    await session.commit()
    logger.info(f"Proposal created: {proposal.proposal_id}")
    return proposal


async def list_proposals(session: AsyncSession, community_id: str, status: str = None) -> list[Proposal]:
    """List proposals for community"""
    query = select(Proposal).where(Proposal.community_id == community_id)

    if status:
        query = query.where(Proposal.voting_status == status)

    query = query.order_by(Proposal.created_at.desc())
    result = await session.execute(query)
    return result.scalars().all()


async def update_proposal_status(session: AsyncSession, proposal_id: str, status: str):
    """Update proposal voting status"""
    proposal = await get_proposal(session, proposal_id)
    if proposal:
        proposal.voting_status = status
        proposal.updated_at = datetime.utcnow()
        await session.commit()
        logger.info(f"Proposal {proposal_id} status updated to {status}")


async def update_proposal_judgment(
    session: AsyncSession, proposal_id: str, judgment_data: dict
):
    """Update proposal with CYNIC judgment"""
    proposal = await get_proposal(session, proposal_id)
    if proposal:
        proposal.judgment_id = judgment_data.get("judgment_id")
        proposal.judgment_verdict = judgment_data.get("verdict")
        proposal.judgment_q_score = judgment_data.get("q_score")
        proposal.judgment_confidence = judgment_data.get("confidence")
        proposal.judgment_data = judgment_data
        proposal.updated_at = datetime.utcnow()
        await session.commit()
        logger.info(f"Judgment added to proposal {proposal_id}")


# Vote operations
async def create_vote(session: AsyncSession, vote_data: dict) -> Vote:
    """Create new vote"""
    # Remove old vote if exists
    old_vote = await get_user_vote(session, vote_data["proposal_id"], vote_data["voter_id"])
    if old_vote:
        await session.delete(old_vote)

    vote = Vote(**vote_data)
    session.add(vote)
    await session.commit()
    logger.info(f"Vote recorded: {vote.vote_id}")
    return vote


async def get_user_vote(session: AsyncSession, proposal_id: str, voter_id: str) -> Vote:
    """Get user's vote on proposal"""
    query = select(Vote).where(
        (Vote.proposal_id == proposal_id) & (Vote.voter_id == voter_id)
    )
    result = await session.execute(query)
    return result.scalar_one_or_none()


async def count_votes(session: AsyncSession, proposal_id: str) -> dict:
    """Count votes on proposal"""
    votes = await session.execute(select(Vote).where(Vote.proposal_id == proposal_id))
    all_votes = votes.scalars().all()

    yes_count = sum(v.vote_weight for v in all_votes if v.vote == "YES")
    no_count = sum(v.vote_weight for v in all_votes if v.vote == "NO")
    abstain_count = sum(v.vote_weight for v in all_votes if v.vote == "ABSTAIN")

    return {
        "yes": yes_count,
        "no": no_count,
        "abstain": abstain_count,
        "total": yes_count + no_count + abstain_count,
        "vote_count": len(all_votes)
    }


async def update_vote_counts(session: AsyncSession, proposal_id: str):
    """Update vote counts on proposal"""
    proposal = await get_proposal(session, proposal_id)
    if proposal:
        counts = await count_votes(session, proposal_id)
        proposal.yes_votes = counts["yes"]
        proposal.no_votes = counts["no"]
        proposal.abstain_votes = counts["abstain"]
        await session.commit()


# E-Score operations
async def get_or_create_e_score(
    session: AsyncSession, community_id: str, entity_type: str, entity_id: str
) -> EScore:
    """Get or create E-Score"""
    query = select(EScore).where(
        (EScore.community_id == community_id)
        & (EScore.entity_type == entity_type)
        & (EScore.entity_id == entity_id)
    )
    result = await session.execute(query)
    e_score = result.scalar_one_or_none()

    if not e_score:
        e_score = EScore(
            e_score_id=f"escore_{community_id}_{entity_type}_{entity_id}",
            community_id=community_id,
            entity_type=entity_type,
            entity_id=entity_id,
        )
        session.add(e_score)
        await session.commit()

    return e_score


async def update_e_score(
    session: AsyncSession, community_id: str, entity_type: str, entity_id: str, update_data: dict
):
    """Update E-Score"""
    e_score = await get_or_create_e_score(session, community_id, entity_type, entity_id)

    for key, value in update_data.items():
        if hasattr(e_score, key):
            setattr(e_score, key, value)

    e_score.updated_at = datetime.utcnow()
    await session.commit()
    logger.info(f"E-Score updated for {entity_type} {entity_id}")


# Proposal status checks
async def is_voting_active(session: AsyncSession, proposal_id: str) -> bool:
    """Check if proposal voting is active"""
    proposal = await get_proposal(session, proposal_id)
    if not proposal:
        return False

    now = datetime.utcnow()
    return (
        proposal.voting_status == "ACTIVE"
        and proposal.voting_start_time <= now <= proposal.voting_end_time
    )


async def check_voting_closed(session: AsyncSession, proposal_id: str) -> bool:
    """Check if voting period has ended"""
    proposal = await get_proposal(session, proposal_id)
    if not proposal:
        return False

    if datetime.utcnow() > proposal.voting_end_time:
        if proposal.voting_status == "ACTIVE":
            await update_proposal_status(session, proposal_id, "CLOSED")
            # Determine approval
            counts = await count_votes(session, proposal_id)
            yes_percentage = (counts["yes"] / counts["total"] * 100) if counts["total"] > 0 else 0
            approval_threshold = 50.0  # From community settings

            if yes_percentage > approval_threshold:
                proposal.approval_status = "APPROVED"
            else:
                proposal.approval_status = "REJECTED"

            await session.commit()

        return True

    return False


# Learning Outcome operations
async def get_proposals_needing_outcome(session: AsyncSession) -> list[Proposal]:
    """Get CLOSED proposals where outcome hasn't been determined yet"""
    now = datetime.utcnow()
    query = (select(Proposal)
        .where(Proposal.voting_status == "CLOSED")
        .where(Proposal.outcome_determined == False)
        .where(Proposal.voting_end_time < now))
    result = await session.execute(query)
    return result.scalars().all()


async def mark_outcome_determined(
    session: AsyncSession, proposal_id: str, outcome: str, satisfaction_rating: float = None
):
    """Mark proposal outcome as determined"""
    proposal = await get_proposal(session, proposal_id)
    if proposal:
        proposal.outcome_determined = True
        proposal.outcome = outcome
        if satisfaction_rating is not None:
            proposal.community_satisfaction_rating = satisfaction_rating
        proposal.updated_at = datetime.utcnow()
        await session.commit()
        logger.info(f"Outcome determined for proposal {proposal_id}: {outcome}")


async def create_learning_outcome(
    session: AsyncSession, proposal_id: str, outcome: str, satisfaction_rating: float = None
) -> LearningOutcome:
    """Create learning outcome audit record"""
    import uuid
    lo = LearningOutcome(
        outcome_id=f"lo_{str(uuid.uuid4())[:8]}",
        proposal_id=proposal_id,
        outcome=outcome,
        success_metrics={},
        predicted_metrics={},
        community_satisfaction_rating=satisfaction_rating,
        recorded_at=datetime.utcnow(),
        learned_at=None
    )
    session.add(lo)
    await session.commit()
    logger.info(f"Learning outcome recorded: {lo.outcome_id}")
    return lo


# Database cleanup
async def close_db():
    """Close database connection"""
    if engine:
        await engine.dispose()
        logger.info("Database connection closed")
