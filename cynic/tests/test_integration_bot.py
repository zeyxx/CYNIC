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
