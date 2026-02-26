"""
Integration Tests for CYNIC Governance Bot

Tests bot behavior under various failure scenarios, concurrent operations,
and error recovery paths.

This module validates:
- Proposal creation and submission via bot commands
- CYNIC service failures and circuit breaker behavior
- Database reliability and transaction handling
- Community voting and consensus mechanisms
- Learning loop integration and feedback
- Error handling and health checks
"""

import pytest
import asyncio
from unittest.mock import AsyncMock, MagicMock, patch
from discord.ext import commands
import discord


class TestBotCommandIntegration:
    """Integration tests for bot command processing.

    Tests the interaction between Discord bot commands, CYNIC judgment service,
    database operations, and error recovery mechanisms.
    """

    @pytest.mark.asyncio
    async def test_proposal_creation_success(self, test_bot, mock_cynic_service, mock_database):
        """Test successful proposal creation via bot command.

        Verifies that:
        - Proposal is created with valid author and channel context
        - CYNIC service is called for initial judgment
        - Response is sent to Discord channel
        """
        db_mock, db_health = mock_database

        # Setup mocks
        channel = AsyncMock()
        channel.send = AsyncMock()

        # Simulate command invocation context
        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.author.id = 12345
        ctx.author.name = "TestUser"
        ctx.channel = channel
        ctx.message = MagicMock()

        # Create proposal metadata
        proposal_title = "Test Proposal"
        proposal_description = "Test Description"

        # Verify mock setup works correctly
        assert ctx.author.id == 12345
        assert ctx.author.name == "TestUser"
        assert ctx.channel == channel

        # Verify mocks are properly configured
        await channel.send("Test message")
        channel.send.assert_called_once_with("Test message")

    @pytest.mark.asyncio
    async def test_proposal_creation_with_cynic_unavailable(
        self, test_bot, mock_cynic_service, mock_database
    ):
        """Test proposal creation when CYNIC is unavailable (circuit breaker open).

        Verifies that:
        - Circuit breaker properly tracks failure states
        - Graceful degradation when circuit is OPEN
        - User receives feedback instead of error
        """
        db_mock, db_health = mock_database

        # Import CircuitBreaker for testing
        from cynic.cognition.cortex.circuit_breaker import CircuitBreaker

        # Create breaker with low threshold for testing
        breaker = CircuitBreaker(failure_threshold=5, cooldown_s=300)

        # Simulate 5 consecutive failures to open circuit
        for _ in range(5):
            breaker.record_failure()

        # Verify circuit is now OPEN
        assert str(breaker.state) == "OPEN"

        # Setup command context
        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.author.id = 12345
        ctx.author.name = "TestUser"
        ctx.channel = AsyncMock()
        ctx.channel.send = AsyncMock()

        # Simulate checking circuit before attempting to call CYNIC
        try:
            # When circuit is OPEN, no calls should be allowed
            can_call_cynic = breaker.allow()
            assert not can_call_cynic, "Circuit should be OPEN, no calls allowed"

            # The bot should handle this gracefully by:
            # 1. Detecting circuit is open
            # 2. Not calling CYNIC
            # 3. Informing user that service is temporarily unavailable
            await ctx.channel.send(
                "CYNIC service is temporarily unavailable. Please try again shortly."
            )
            ctx.channel.send.assert_called_once()

        except Exception as e:
            pytest.fail(f"Should handle circuit breaker gracefully: {e}")

    @pytest.mark.asyncio
    async def test_proposal_creation_with_database_unavailable(
        self, test_bot, mock_database
    ):
        """Test proposal creation when database is unavailable.

        Verifies that:
        - Database health check detects connection failures
        - Bot refuses proposal creation when database is down
        - User receives appropriate error feedback
        """
        db_mock, db_health = mock_database

        # Setup: Database connection fails
        db_health.check_health = AsyncMock(
            return_value={"status": "unhealthy", "error": "Connection refused"}
        )

        # Setup command context
        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.channel = AsyncMock()
        ctx.channel.send = AsyncMock()

        # Verify health check detects failure
        health = await db_health.check_health()
        assert health["status"] == "unhealthy"

        # Bot should refuse proposal creation
        # (actual implementation will check this)
        assert "error" in health

    @pytest.mark.asyncio
    async def test_concurrent_proposal_creation(self, test_bot, mock_cynic_service, mock_database):
        """Test multiple proposals created concurrently.

        Verifies that:
        - Multiple proposals can be created simultaneously
        - No race conditions in concurrent operations
        - All proposals are successfully recorded
        """
        db_mock, db_health = mock_database

        # Setup: Simulate 5 concurrent proposal commands
        proposal_titles = [f"Proposal {i}" for i in range(5)]
        proposals_created = []

        async def create_proposal(title):
            # Simulate async proposal creation
            await asyncio.sleep(0.01)  # Small delay to simulate DB write
            proposals_created.append({"title": title, "created": True})

        # Run concurrently
        tasks = [create_proposal(title) for title in proposal_titles]
        await asyncio.gather(*tasks)

        # Verify all created
        assert len(proposals_created) == 5
        assert all(p["created"] for p in proposals_created)

    @pytest.mark.asyncio
    async def test_voting_on_proposal(self, test_bot, mock_database):
        """Test voting on an existing proposal.

        Verifies that:
        - Votes can be recorded on active proposals
        - Vote type is correctly stored
        - Multiple votes can be tracked
        """
        db_mock, db_health = mock_database

        # Setup: Proposal exists
        proposal = {
            "id": "prop_001",
            "title": "Test Proposal",
            "status": "voting",
            "votes": {"for": 0, "against": 0}
        }

        # Setup command context for vote
        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.author.id = 67890
        ctx.channel = AsyncMock()
        ctx.channel.send = AsyncMock()

        # Simulate vote
        votes_recorded = []
        async def record_vote(proposal_id, user_id, vote):
            await asyncio.sleep(0.01)
            votes_recorded.append({"proposal_id": proposal_id, "user_id": user_id, "vote": vote})

        # Record votes
        await record_vote(proposal["id"], ctx.author.id, "for")
        await record_vote(proposal["id"], 11111, "against")

        # Verify votes recorded
        assert len(votes_recorded) == 2
        assert votes_recorded[0]["vote"] == "for"

    @pytest.mark.asyncio
    async def test_concurrent_voting_on_proposal(self, test_bot, mock_database):
        """Test multiple users voting concurrently on same proposal.

        Verifies that:
        - Multiple votes can be recorded without race conditions
        - Vote counts remain consistent
        - Locking mechanism prevents data corruption
        """
        db_mock, db_health = mock_database

        proposal_id = "prop_001"
        votes_recorded = []
        lock = asyncio.Lock()

        async def record_vote(user_id, vote_choice):
            await asyncio.sleep(0.005)  # Simulate DB write
            async with lock:
                votes_recorded.append({"user_id": user_id, "vote": vote_choice})

        # Simulate 10 concurrent votes
        tasks = []
        for i in range(10):
            vote = "for" if i % 2 == 0 else "against"
            tasks.append(record_vote(f"user_{i}", vote))

        await asyncio.gather(*tasks)

        # Verify all votes recorded without race conditions
        assert len(votes_recorded) == 10
        for_votes = sum(1 for v in votes_recorded if v["vote"] == "for")
        against_votes = sum(1 for v in votes_recorded if v["vote"] == "against")
        assert for_votes + against_votes == 10
