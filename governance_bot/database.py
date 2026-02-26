"""
Database operations for Governance Bot

Provides:
- Connection pooling (max 5 concurrent connections)
- Transaction management (atomicity)
- Data consistency verification
- Backup/restore functionality
- Database health checks
"""

import logging
import shutil
import asyncio
from contextlib import asynccontextmanager
from datetime import datetime, timedelta
from pathlib import Path
from sqlalchemy import create_engine, select, event, inspect
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession, async_sessionmaker

from models import Base, Community, Proposal, Vote, EScore, CommunityUser, LearningOutcome
from config import DATABASE_URL

logger = logging.getLogger(__name__)

# Async database setup
engine = None
async_session = None
db_file_path = None


class DatabaseHealthCheck:
    """Database health monitoring"""

    def __init__(self):
        self.last_check_time = None
        self.last_check_status = None
        self.error_count = 0

    async def check_health(self) -> dict:
        """Perform database health check"""
        try:
            async with session_context() as session:
                # Test basic connectivity
                result = await session.execute(select(1))
                assert result.scalar() == 1

                # Check table integrity
                inspector = inspect(engine.sync_engine)
                tables = inspector.get_table_names()
                expected_tables = {"community", "proposal", "vote", "e_score", "community_user", "learning_outcome"}
                missing_tables = expected_tables - set(tables)

                self.last_check_time = datetime.utcnow()
                self.last_check_status = "HEALTHY"
                self.error_count = 0

                return {
                    "status": "HEALTHY",
                    "timestamp": self.last_check_time,
                    "tables_found": len(tables),
                    "tables_missing": list(missing_tables),
                    "connectivity": "OK"
                }

        except Exception as e:
            logger.error(f"Database health check failed: {e}")
            self.last_check_status = "UNHEALTHY"
            self.error_count += 1
            self.last_check_time = datetime.utcnow()

            return {
                "status": "UNHEALTHY",
                "timestamp": self.last_check_time,
                "error": str(e),
                "error_count": self.error_count
            }

    def get_status(self) -> str:
        """Get human-readable status"""
        if self.last_check_status is None:
            return "NOT_CHECKED"
        if self.error_count > 2:
            return "CRITICAL"
        return self.last_check_status


db_health_check = DatabaseHealthCheck()


async def backup_database(backup_dir: str = "./backups"):
    """Create timestamped backup of database"""
    if not db_file_path or not db_file_path.exists():
        logger.warning("Cannot backup: database file not found")
        return None

    try:
        backup_path = Path(backup_dir)
        backup_path.mkdir(exist_ok=True)

        timestamp = datetime.utcnow().strftime("%Y%m%d_%H%M%S")
        backup_file = backup_path / f"governance_db_{timestamp}.sqlite"

        shutil.copy2(db_file_path, backup_file)
        logger.info(f"Database backed up to {backup_file}")
        return backup_file

    except Exception as e:
        logger.error(f"Backup failed: {e}")
        return None


async def restore_database(backup_file: Path) -> bool:
    """Restore database from backup"""
    if not db_file_path:
        logger.error("Cannot restore: database file path not set")
        return False

    try:
        if not backup_file.exists():
            logger.error(f"Backup file not found: {backup_file}")
            return False

        # Close current connection
        await close_db()

        # Restore from backup
        shutil.copy2(backup_file, db_file_path)
        logger.info(f"Database restored from {backup_file}")

        # Reinitialize
        await init_db()
        return True

    except Exception as e:
        logger.error(f"Restore failed: {e}")
        return False


async def verify_data_consistency() -> dict:
    """Verify data consistency after crash/recovery"""
    consistency_issues = []

    try:
        async with session_context() as session:
            # Check for orphaned votes (votes for non-existent proposals)
            votes = await session.execute(select(Vote))
            for vote in votes.scalars().all():
                proposal = await get_proposal(session, vote.proposal_id)
                if not proposal:
                    consistency_issues.append(f"Orphaned vote: {vote.vote_id}")

            # Check for proposals with invalid vote counts
            proposals = await session.execute(select(Proposal))
            for proposal in proposals.scalars().all():
                counts = await count_votes(session, proposal.proposal_id)
                total = counts["yes"] + counts["no"] + counts["abstain"]

                expected_total = (proposal.yes_votes or 0) + (proposal.no_votes or 0) + (proposal.abstain_votes or 0)
                if abs(total - expected_total) > 0.01:  # Allow for float rounding
                    consistency_issues.append(
                        f"Vote count mismatch in {proposal.proposal_id}: "
                        f"actual={total}, stored={expected_total}"
                    )

            # Check for proposals with invalid timestamps
            now = datetime.utcnow()
            for proposal in proposals.scalars().all():
                if proposal.voting_start_time > now and proposal.voting_status == "CLOSED":
                    consistency_issues.append(
                        f"Temporal inconsistency in {proposal.proposal_id}: "
                        f"scheduled for future but already closed"
                    )

    except Exception as e:
        logger.error(f"Consistency check failed: {e}")
        return {
            "status": "CHECK_FAILED",
            "error": str(e)
        }

    return {
        "status": "OK" if not consistency_issues else "ISSUES_FOUND",
        "issue_count": len(consistency_issues),
        "issues": consistency_issues
    }


async def init_db(db_url: str = DATABASE_URL):
    """Initialize database with connection pooling"""
    global engine, async_session, db_file_path

    # Convert sqlite to aiosqlite for async support
    if db_url.startswith("sqlite"):
        db_url = db_url.replace("sqlite", "sqlite+aiosqlite", 1)
        # Extract file path for backup/restore
        db_file_path = Path(db_url.replace("sqlite+aiosqlite:///", ""))

    # Create engine with connection pooling
    # pool_size: max 5 concurrent connections
    # max_overflow: additional connections allowed when pool exhausted
    # pool_recycle: recycle connections after 3600 seconds (1 hour)
    engine = create_async_engine(
        db_url,
        echo=False,
        pool_size=5,
        max_overflow=10,
        pool_recycle=3600,
        pool_pre_ping=True,  # Test connections before using
    )

    async_session = async_sessionmaker(
        engine,
        class_=AsyncSession,
        expire_on_commit=False,
        autoflush=False,  # Explicit flush for better transaction control
    )

    # Create tables
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    logger.info(f"Database initialized: {db_url}")
    logger.info("Connection pool configured: pool_size=5, max_overflow=10")

    # Perform health check on startup
    health = await db_health_check.check_health()
    logger.info(f"Database health check: {health['status']}")

    # Verify data consistency
    consistency = await verify_data_consistency()
    if consistency['status'] == "ISSUES_FOUND":
        logger.warning(f"Data consistency issues found: {consistency['issue_count']}")
        for issue in consistency['issues'][:5]:  # Log first 5
            logger.warning(f"  - {issue}")
    else:
        logger.info("Data consistency verified")


async def get_session() -> AsyncSession:
    """Get async database session"""
    return async_session()


@asynccontextmanager
async def session_context(auto_commit: bool = True):
    """
    Context manager for automatic session cleanup with transaction support.

    Args:
        auto_commit: If True, automatically commit on successful completion.
                     If False, caller must explicitly commit/rollback.

    Usage:
        # Auto-commit (default)
        async with session_context() as session:
            await create_proposal(session, data)  # Automatically committed

        # Manual transaction control
        async with session_context(auto_commit=False) as session:
            try:
                await create_proposal(session, data)
                await session.commit()
            except Exception as e:
                await session.rollback()
                raise
    """
    session = await get_session()
    try:
        yield session
        if auto_commit:
            await session.commit()
    except Exception as e:
        await session.rollback()
        logger.error(f"Transaction rolled back due to error: {e}")
        raise
    finally:
        await session.close()


@asynccontextmanager
async def transaction_context():
    """
    Context manager for explicit transaction control.
    Requires manual commit/rollback.

    Usage:
        async with transaction_context() as session:
            try:
                await session.execute(...)
                await session.commit()
            except:
                await session.rollback()
                raise
    """
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
async def close_db(create_backup: bool = True):
    """
    Close database connection and optionally create backup.

    Args:
        create_backup: If True, create timestamped backup before closing.
    """
    try:
        if create_backup:
            await backup_database()

        if engine:
            await engine.dispose()
            logger.info("Database connection closed")
    except Exception as e:
        logger.error(f"Error closing database: {e}")
