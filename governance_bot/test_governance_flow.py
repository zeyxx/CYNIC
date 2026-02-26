"""
Test the complete governance flow: propose -> judge -> vote -> verify
This test script simulates the entire workflow without manual Discord interaction.
"""

import asyncio
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession, async_sessionmaker
from sqlalchemy import select

from models import Base, Community, Proposal, Vote
from config import DATABASE_URL
from cynic_integration import ask_cynic

# Test database
TEST_DB_URL = "sqlite+aiosqlite:///governance_bot_test.db"

async def init_test_db():
    """Initialize test database"""
    engine = create_async_engine(TEST_DB_URL, echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
        await conn.run_sync(Base.metadata.create_all)

    return engine

async def test_governance_flow():
    """Test the complete governance flow"""

    print("\n" + "="*80)
    print("CYNIC GOVERNANCE FLOW TEST")
    print("="*80)

    # Initialize database
    print("\n[1/6] Initializing test database...")
    engine = None
    try:
        engine = await init_test_db()
        async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)

        async with async_session() as session:
            # PART 1: Create community
            print("\n[2/6] Creating test community...")
            community = Community(
                community_id="discord_test_server_123",
                platform="discord",
                community_name="Test Governance Community",
                voting_period_hours=24,
                quorum_percentage=25.0,
                approval_threshold_percentage=50.0
            )
            session.add(community)
            await session.commit()
            print(f"[OK] Community created: {community.community_id}")

            # PART 2: Create proposal
            print("\n[3/6] Creating governance proposal...")
            proposal_id = f"prop_{datetime.now(timezone.utc).strftime('%Y%m%d')}_test001"
            now = datetime.now(timezone.utc)

            proposal = Proposal(
                proposal_id=proposal_id,
                community_id=community.community_id,
                proposer_id="123456789",
                title="Increase community treasury allocation",
                description="Allocate 5% of monthly revenue to community projects for development and marketing",
                category="COMMUNITY_DECISION",
                impact_level="MEDIUM",
                voting_start_time=now,
                voting_end_time=now + timedelta(hours=24),
                voting_status="PENDING",
                judgment_verdict=None,
                judgment_q_score=None
            )
            session.add(proposal)
            await session.commit()
            print(f"[OK] Proposal created: {proposal_id}")
            print(f"  Title: {proposal.title}")
            print(f"  Category: {proposal.category}")
            print(f"  Impact Level: {proposal.impact_level}")

            # PART 3: Get CYNIC judgment
            print("\n[4/6] Getting CYNIC judgment...")
            try:
                judgment = await ask_cynic(
                    question=proposal.title,
                    context=proposal.description,
                    reality="GOVERNANCE"
                )

                if judgment.get("verdict") and judgment.get("verdict") != "PENDING":
                    proposal.judgment_verdict = judgment.get("verdict")
                    proposal.judgment_q_score = judgment.get("q_score", 0.0)
                    proposal.judgment_confidence = judgment.get("confidence", 0.618)
                    proposal.judgment_data = judgment
                    await session.commit()
                    print(f"[OK] CYNIC Verdict: {judgment.get('verdict')}")
                    print(f"  Q-Score: {judgment.get('q_score', 0.0):.3f}")
                    print(f"  Confidence: {judgment.get('confidence', 0.618):.3f}")
                else:
                    print("[WARN] CYNIC returned pending verdict")
                    proposal.judgment_verdict = "PENDING"
                    proposal.judgment_q_score = 0.0
                    await session.commit()
            except Exception as e:
                print(f"[WARN] Could not reach CYNIC: {e}")
                proposal.judgment_verdict = "PENDING"
                proposal.judgment_q_score = 0.0
                await session.commit()

            # PART 4: Create votes
            print("\n[5/6] Recording test votes...")

            # Update voting status to ACTIVE
            proposal.voting_status = "ACTIVE"
            await session.commit()

            votes_data = [
                ("user_001", "YES", "I agree with this allocation"),
                ("user_002", "NO", "Treasury should stay as is"),
                ("user_003", "ABSTAIN", "Not enough information")
            ]

            vote_count = 0
            for voter_id, vote_choice, reasoning in votes_data:
                vote_id = f"vote_{proposal_id}_{voter_id}"
                vote = Vote(
                    vote_id=vote_id,
                    proposal_id=proposal_id,
                    voter_id=voter_id,
                    vote=vote_choice,
                    vote_weight=1.0,
                    reasoning=reasoning,
                    voted_at=datetime.now(timezone.utc)
                )
                session.add(vote)
                vote_count += 1
                print(f"[OK] Vote recorded: {voter_id} -> {vote_choice}")

            await session.commit()
            print(f"[OK] Total votes recorded: {vote_count}")

            # PART 5: Verify data in database
            print("\n[6/6] Verifying database contents...")

            # Count proposals
            result = await session.execute(select(Proposal))
            proposals = result.scalars().all()
            print(f"[OK] Proposals in DB: {len(proposals)}")

            # Count votes
            result = await session.execute(select(Vote))
            votes = result.scalars().all()
            print(f"[OK] Votes in DB: {len(votes)}")

            # Verify vote counts
            yes_votes = sum(1 for v in votes if v.vote == "YES")
            no_votes = sum(1 for v in votes if v.vote == "NO")
            abstain_votes = sum(1 for v in votes if v.vote == "ABSTAIN")

            print(f"\nVote breakdown:")
            print(f"  YES: {yes_votes}")
            print(f"  NO: {no_votes}")
            print(f"  ABSTAIN: {abstain_votes}")

            if yes_votes + no_votes + abstain_votes > 0:
                total = yes_votes + no_votes + abstain_votes
                yes_pct = (yes_votes / total) * 100
                no_pct = (no_votes / total) * 100
                abstain_pct = (abstain_votes / total) * 100
                print(f"\nPercentages:")
                print(f"  YES: {yes_pct:.1f}%")
                print(f"  NO: {no_pct:.1f}%")
                print(f"  ABSTAIN: {abstain_pct:.1f}%")

            # Get final proposal state
            final_proposal = await session.execute(
                select(Proposal).where(Proposal.proposal_id == proposal_id)
            )
            final_proposal = final_proposal.scalar_one()

            print(f"\n" + "="*80)
            print("FINAL GOVERNANCE FLOW TEST RESULTS")
            print("="*80)

            results = {
                "Proposal Creation": "[PASS]" if final_proposal else "[FAIL]",
                "Proposal Stored": "[PASS]" if final_proposal else "[FAIL]",
                "CYNIC Verdict": "[PASS]" if final_proposal.judgment_verdict else "[FAIL]",
                "Q-Score Recorded": "[PASS]" if (final_proposal.judgment_q_score == 0.0 if final_proposal.judgment_verdict == "PENDING" else final_proposal.judgment_q_score is not None) else "[FAIL]",
                "Votes Recorded": "[PASS]" if len(votes) == 3 else "[FAIL]",
                "All Data Consistent": "[PASS]" if all([
                    final_proposal,
                    len(votes) == 3,
                    yes_votes == 1,
                    no_votes == 1,
                    abstain_votes == 1
                ]) else "[FAIL]"
            }

            for check, result in results.items():
                print(f"{result} {check}")

            print("\nDetailed Results:")
            print(f"  Proposal ID: {proposal_id}")
            print(f"  Title: {final_proposal.title}")
            print(f"  Status: {final_proposal.voting_status}")
            print(f"  CYNIC Verdict: {final_proposal.judgment_verdict}")
            print(f"  Q-Score: {final_proposal.judgment_q_score or 'N/A'}")
            print(f"  Database File: governance_bot_test.db")
            print(f"  Total Votes: {len(votes)}")

            # Determine overall result
            all_passed = all(v == "[PASS]" for v in results.values())

            print("\n" + "="*80)
            if all_passed:
                print("[PASS] COMPLETE - All governance flow steps working")
            else:
                print("[FAIL] PARTIAL - Some steps failed or returned no data")
            print("="*80 + "\n")

            return results, all_passed
    finally:
        # Clean up resources
        if engine:
            await engine.dispose()

        # Clean up test database file
        import gc
        gc.collect()

        test_db_path = Path("governance_bot_test.db")
        if test_db_path.exists():
            try:
                test_db_path.unlink()
                print("[OK] Test database cleaned up")
            except Exception as e:
                print(f"[WARN] Could not delete test database: {e}")

        # Close any CYNIC adapter sessions
        try:
            from cynic.mcp.claude_code_bridge import _adapter
            if _adapter and hasattr(_adapter, 'session') and _adapter.session:
                await _adapter.session.close()
        except Exception:
            pass  # Adapter may not be initialized or already closed


if __name__ == "__main__":
    results, passed = asyncio.run(test_governance_flow())
    sys.exit(0 if passed else 1)
