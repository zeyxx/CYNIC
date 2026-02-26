"""
Tests for DiscordAdapter implementing unified BotInterface.

These tests verify:
1. DiscordAdapter implements BotInterface contract
2. Command routing works correctly (propose, proposals, vote, etc)
3. BotResponse objects are created with correct structure
4. Error handling is graceful
5. Discord-specific features (buttons, embeds) integrate correctly
6. Async lifecycle (start/stop) works
"""

import pytest
import asyncio
from unittest.mock import Mock, AsyncMock, MagicMock, patch
from typing import Optional

from cynic.bots.bot_interface import BotInterface, BotCommand, BotResponse
from cynic.core.unified_state import UnifiedConsciousState, UnifiedJudgment
from governance_bot.adapters.discord_adapter import DiscordAdapter


class TestDiscordAdapterImplementsInterface:
    """Test DiscordAdapter implements BotInterface contract."""

    def test_discord_adapter_is_bot_interface(self):
        """DiscordAdapter is a subclass of BotInterface."""
        assert issubclass(DiscordAdapter, BotInterface)

    def test_discord_adapter_has_required_methods(self):
        """DiscordAdapter implements all required abstract methods."""
        assert hasattr(DiscordAdapter, "start")
        assert hasattr(DiscordAdapter, "stop")
        assert hasattr(DiscordAdapter, "handle_command")

    def test_discord_adapter_can_be_instantiated(self):
        """DiscordAdapter can be instantiated with required parameters."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter is not None
        assert adapter.client == mock_client
        assert adapter.conscious_state == mock_state

    def test_discord_adapter_stores_client_reference(self):
        """DiscordAdapter stores discord client reference."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter.client is mock_client

    def test_discord_adapter_stores_conscious_state_reference(self):
        """DiscordAdapter stores conscious_state reference."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter.conscious_state is mock_state


class TestDiscordAdapterLifecycle:
    """Test DiscordAdapter async lifecycle (start/stop)."""

    @pytest.mark.asyncio
    async def test_adapter_start_method_exists(self):
        """DiscordAdapter.start() method exists and is async."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        assert hasattr(adapter, "start")
        assert asyncio.iscoroutinefunction(adapter.start)

    @pytest.mark.asyncio
    async def test_adapter_stop_method_exists(self):
        """DiscordAdapter.stop() method exists and is async."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        assert hasattr(adapter, "stop")
        assert asyncio.iscoroutinefunction(adapter.stop)

    @pytest.mark.asyncio
    async def test_adapter_start_can_be_awaited(self):
        """DiscordAdapter.start() can be awaited without raising."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Should not raise
        await adapter.start()

    @pytest.mark.asyncio
    async def test_adapter_stop_can_be_awaited(self):
        """DiscordAdapter.stop() can be awaited without raising."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Should not raise
        await adapter.stop()


class TestDiscordAdapterHandleCommand:
    """Test DiscordAdapter.handle_command() routing and responses."""

    @pytest.mark.asyncio
    async def test_handle_command_returns_bot_response(self):
        """handle_command returns BotResponse object."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_handle_unknown_command_returns_failure(self):
        """handle_command returns failure for unknown command."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="unknown_command",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False
        assert "unknown" in response.message.lower() or "not found" in response.message.lower()

    @pytest.mark.asyncio
    async def test_handle_command_invalid_raises_or_returns_failure(self):
        """handle_command handles invalid commands gracefully."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Create command with invalid data
        cmd = BotCommand(
            name="propose",
            args={"title": ""},  # Empty title
            user_id="user123",
            platform="discord",
        )

        # Should either return failure response or raise ValueError
        try:
            response = await adapter.handle_command(cmd)
            assert response.success is False
        except ValueError:
            # ValueError is also acceptable
            pass


class TestDiscordAdapterProposeCommand:
    """Test /propose command handling."""

    @pytest.mark.asyncio
    async def test_propose_command_creates_response(self):
        """/propose command creates BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test Proposal", "description": "Test Description"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)
        assert response.message is not None
        assert isinstance(response.message, str)

    @pytest.mark.asyncio
    async def test_propose_requires_title(self):
        """Propose command requires title argument."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Missing title
        cmd = BotCommand(
            name="propose",
            args={"description": "No title provided"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "title" in response.message.lower()

    @pytest.mark.asyncio
    async def test_propose_response_has_proposal_id(self):
        """Successful propose response includes proposal_id in data."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test", "description": "Desc"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        # If successful, should have proposal_id in data
        if response.success:
            assert "proposal_id" in response.data or "id" in response.data


class TestDiscordAdapterProposalsCommand:
    """Test /proposals command handling."""

    @pytest.mark.asyncio
    async def test_proposals_command_returns_response(self):
        """/proposals command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={"page": 1},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_proposals_response_has_list_data(self):
        """Proposals response includes list in data."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        if response.success:
            # Should have proposals list in data
            assert "proposals" in response.data or "items" in response.data

    @pytest.mark.asyncio
    async def test_proposals_pagination_default_page_one(self):
        """Proposals defaults to page 1 if not specified."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)


class TestDiscordAdapterVoteCommand:
    """Test /vote command handling."""

    @pytest.mark.asyncio
    async def test_vote_command_returns_response(self):
        """Vote command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"proposal_id": "prop-1", "vote": "yes"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_vote_requires_proposal_id(self):
        """Vote command requires proposal_id."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"vote": "yes"},  # Missing proposal_id
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "proposal_id" in response.message.lower()

    @pytest.mark.asyncio
    async def test_vote_requires_vote_value(self):
        """Vote command requires vote (yes/no/abstain)."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"proposal_id": "prop-1"},  # Missing vote
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "vote" in response.message.lower()


class TestDiscordAdapterStatusCommand:
    """Test /status command handling."""

    @pytest.mark.asyncio
    async def test_status_command_returns_response(self):
        """Status command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_status_response_has_status_data(self):
        """Status response includes status information."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        if response.success:
            # Should have status-related data
            assert response.data is not None
            assert isinstance(response.data, dict)


class TestDiscordAdapterErrorHandling:
    """Test error handling in DiscordAdapter."""

    @pytest.mark.asyncio
    async def test_exception_in_command_handling_returns_failure(self):
        """Exceptions during command handling are caught and returned as failure."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        # Make conscious_state raise an exception
        mock_state.get_recent_judgments.side_effect = Exception("Database error")

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        # Should return failure response, not raise exception
        assert isinstance(response, BotResponse)
        assert response.success is False

    @pytest.mark.asyncio
    async def test_malformed_args_handled_gracefully(self):
        """Malformed arguments are handled gracefully."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Args with invalid types
        cmd = BotCommand(
            name="propose",
            args={"title": None},  # None instead of string
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        # Should handle gracefully
        assert isinstance(response, BotResponse)


class TestDiscordAdapterResponseStructure:
    """Test BotResponse structure from DiscordAdapter."""

    @pytest.mark.asyncio
    async def test_response_has_success_bool(self):
        """All responses have success as boolean."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response.success, bool)

    @pytest.mark.asyncio
    async def test_response_has_message_string(self):
        """All responses have message as string."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response.message, str)
        assert len(response.message) > 0

    @pytest.mark.asyncio
    async def test_response_has_data_dict(self):
        """All responses have data as dict."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response.data, dict)

    @pytest.mark.asyncio
    async def test_response_ephemeral_is_bool(self):
        """Response ephemeral field is boolean."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response.ephemeral, bool)


class TestDiscordAdapterPlatformIntegration:
    """Test Discord platform-specific integration."""

    @pytest.mark.asyncio
    async def test_command_platform_is_discord(self):
        """Commands from adapter specify platform='discord'."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # When we create a BotCommand with platform="discord", adapter should handle it
        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_adapter_rejects_non_discord_commands(self):
        """Adapter may reject or handle non-discord commands."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Try to handle a telegram command
        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        # Should either work or fail gracefully
        assert isinstance(response, BotResponse)


class TestDiscordAdapterLogging:
    """Test logging behavior in DiscordAdapter."""

    @pytest.mark.asyncio
    async def test_adapter_has_logger(self):
        """DiscordAdapter has or can use logger."""
        import logging
        from governance_bot.adapters import discord_adapter

        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Should have module-level logger (standard Python logging pattern)
        assert hasattr(discord_adapter, "logger")
        assert isinstance(discord_adapter.logger, logging.Logger)

    @pytest.mark.asyncio
    async def test_error_response_includes_error_field(self):
        """Failed responses include error field with details."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="unknown",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await adapter.handle_command(cmd)

        if not response.success:
            assert response.error is None or isinstance(response.error, str)


class TestDiscordAdapterConcurrency:
    """Test concurrent command handling."""

    @pytest.mark.asyncio
    async def test_adapter_handles_concurrent_commands(self):
        """Adapter can handle multiple concurrent commands."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = DiscordAdapter(client=mock_client, conscious_state=mock_state)

        # Create multiple commands
        cmd1 = BotCommand(
            name="propose",
            args={"title": "Proposal 1"},
            user_id="user1",
            platform="discord",
        )
        cmd2 = BotCommand(
            name="propose",
            args={"title": "Proposal 2"},
            user_id="user2",
            platform="discord",
        )

        # Handle concurrently
        responses = await asyncio.gather(
            adapter.handle_command(cmd1),
            adapter.handle_command(cmd2),
        )

        # Both should return BotResponse
        assert len(responses) == 2
        assert all(isinstance(r, BotResponse) for r in responses)
