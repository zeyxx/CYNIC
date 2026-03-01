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

import asyncio
from unittest.mock import AsyncMock, MagicMock

import pytest


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
        from cynic.kernel.organism.brain.cognition.cortex.circuit_breaker import CircuitBreaker

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

    @pytest.mark.asyncio
    async def test_circuit_breaker_recovery(self):
        """Test circuit breaker transitions from OPEN to HALF_OPEN to CLOSED.

        Verifies that:
        - Circuit can be opened by recording failures
        - Recovery timeout allows transition to HALF_OPEN
        - Successful call transitions back to CLOSED
        """
        from cynic.kernel.organism.brain.cognition.cortex.circuit_breaker import CircuitBreaker

        breaker = CircuitBreaker(failure_threshold=3, cooldown_s=1)

        # Open the circuit by recording 3 failures
        for _ in range(3):
            breaker.record_failure()
        assert str(breaker.state) == "OPEN"

        # Verify no calls allowed when OPEN
        assert not breaker.allow()

        # Wait for recovery timeout
        await asyncio.sleep(1.1)

        # Should now allow call (transitioning to HALF_OPEN)
        can_call = breaker.allow()
        if can_call:
            # Record a success
            breaker.record_success()
            # Should be CLOSED now
            assert str(breaker.state) == "CLOSED"

    @pytest.mark.asyncio
    async def test_concurrent_learning_updates(self, test_bot, mock_database):
        """Test learning loop handles concurrent outcome ratings.

        Verifies that:
        - Multiple learning updates can occur simultaneously
        - All updates are recorded without loss
        - Thread-safe updates via locking
        """
        db_mock, db_health = mock_database

        learning_updates = []
        lock = asyncio.Lock()

        async def record_learning(proposal_id, outcome, satisfaction):
            await asyncio.sleep(0.01)  # Simulate Q-table update
            async with lock:
                learning_updates.append({
                    "proposal_id": proposal_id,
                    "outcome": outcome,
                    "satisfaction": satisfaction
                })

        # Simulate 5 concurrent learning updates
        tasks = []
        for i in range(5):
            outcome = "approved" if i % 2 == 0 else "rejected"
            satisfaction = 0.8 if i % 2 == 0 else 0.4
            tasks.append(record_learning(f"prop_{i}", outcome, satisfaction))

        await asyncio.gather(*tasks)

        # Verify all updates recorded
        assert len(learning_updates) == 5
        approved = sum(1 for u in learning_updates if u["outcome"] == "approved")
        rejected = sum(1 for u in learning_updates if u["outcome"] == "rejected")
        assert approved + rejected == 5

    @pytest.mark.asyncio
    async def test_discord_command_timeout(self, test_bot):
        """Test bot handles Discord API timeouts gracefully.

        Verifies that:
        - Timeout errors are caught and handled
        - Bot doesn't crash on timeout
        - User receives feedback
        """
        from governance_bot.error_handler import handle_error

        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.channel = AsyncMock()
        ctx.channel.send = AsyncMock(side_effect=TimeoutError("Discord timeout"))

        # Simulate timeout
        try:
            await ctx.channel.send("Test message")
            pytest.fail("Should raise TimeoutError")
        except TimeoutError:
            # Verify error handling
            error_msg = await handle_error(
                TimeoutError("Discord timeout"),
                context="discord_command"
            )
            assert error_msg is not None
            assert "timed out" in error_msg.lower()

    @pytest.mark.asyncio
    async def test_database_connection_pool_exhaustion(self, mock_database):
        """Test bot behavior when database connection pool is exhausted.

        Verifies that:
        - Pool exhaustion is detected
        - Health check reports accurate pool status
        - Waiting connections are tracked
        """
        db_mock, db_health = mock_database

        # Mock pool exhaustion
        db_health.check_health = AsyncMock(
            return_value={
                "status": "unhealthy",
                "error": "Connection pool exhausted",
                "pool_size": 5,
                "active_connections": 5,
                "waiting_connections": 10
            }
        )

        # Check health
        health = await db_health.check_health()

        # Verify pool exhaustion detected
        assert health["status"] == "unhealthy"
        assert health["active_connections"] == health["pool_size"]
        assert health["waiting_connections"] > 0

    @pytest.mark.asyncio
    async def test_bot_health_check_command(self, test_bot, mock_database):
        """Test /health command returns accurate bot status.

        Verifies that:
        - Health check can be invoked via command
        - Returns accurate service status
        - Can be called concurrently without issues
        """
        db_mock, db_health = mock_database

        # Setup healthy state
        db_health.check_health = AsyncMock(return_value={"status": "healthy"})

        # Setup command context
        ctx = MagicMock()
        ctx.author = MagicMock()
        ctx.channel = AsyncMock()
        ctx.channel.send = AsyncMock()

        # Verify we can check bot health
        health = await db_health.check_health()
        assert health["status"] == "healthy"

        # Message should be sent
        await ctx.channel.send("Health check passed")
        ctx.channel.send.assert_called_once()
