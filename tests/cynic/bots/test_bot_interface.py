"""
Tests for BotInterface contract and unified bot architecture.

These tests verify:
1. BotCommand is immutable (frozen dataclass)
2. BotResponse is immutable (frozen dataclass)
3. BotInterface is abstract and cannot be instantiated
4. All bot implementations must implement BotInterface contract
5. Platform validation works correctly
6. Async lifecycle (start/stop) works
7. Command handling returns proper responses
"""

import time
from abc import ABC

import pytest

from cynic.interfaces.bots.bot_interface import BotCommand, BotInterface, BotResponse


class TestBotCommand:
    """Test BotCommand dataclass structure and immutability."""

    def test_bot_command_structure(self):
        """BotCommand has all required fields."""
        cmd = BotCommand(
            name="propose",
            args={"title": "Test Proposal"},
            user_id="user123",
            platform="discord",
        )
        assert cmd.name == "propose"
        assert cmd.args == {"title": "Test Proposal"}
        assert cmd.user_id == "user123"
        assert cmd.platform == "discord"
        assert cmd.guild_id is None
        assert isinstance(cmd.timestamp, float)

    def test_bot_command_with_optional_fields(self):
        """BotCommand can include optional guild_id and custom timestamp."""
        timestamp = time.time()
        cmd = BotCommand(
            name="vote",
            args={"proposal_id": "prop-1", "vote": "yes"},
            user_id="user456",
            platform="telegram",
            guild_id="group-789",
            timestamp=timestamp,
        )
        assert cmd.name == "vote"
        assert cmd.guild_id == "group-789"
        assert cmd.timestamp == timestamp

    def test_bot_command_immutable(self):
        """BotCommand is frozen (immutable)."""
        cmd = BotCommand(
            name="status",
            args={},
            user_id="user123",
            platform="cli",
        )

        # Should not be able to modify attributes
        with pytest.raises(
            (AttributeError, TypeError, dataclasses.FrozenInstanceError)
        ):
            cmd.name = "modified"

    def test_bot_command_timestamp_default(self):
        """BotCommand generates timestamp if not provided."""
        before = time.time()
        cmd = BotCommand(
            name="test",
            args={},
            user_id="user123",
            platform="web",
        )
        after = time.time()

        # Timestamp should be between before and after
        assert before <= cmd.timestamp <= after

    def test_bot_command_args_default_empty_dict(self):
        """BotCommand args defaults to empty dict if not provided."""
        # This tests the behavior when args is not explicitly provided
        # Note: args is required in the dataclass, so this test just verifies
        # that it can be an empty dict
        cmd = BotCommand(
            name="test",
            args={},
            user_id="user123",
            platform="discord",
        )
        assert cmd.args == {}


class TestBotResponse:
    """Test BotResponse dataclass structure and immutability."""

    def test_bot_response_structure(self):
        """BotResponse has all required fields."""
        resp = BotResponse(success=True, message="Command executed")
        assert resp.success is True
        assert resp.message == "Command executed"
        assert resp.data == {}
        assert resp.ephemeral is False
        assert resp.error is None

    def test_bot_response_with_data(self):
        """BotResponse can include structured data."""
        resp = BotResponse(
            success=True,
            message="Proposal created",
            data={"proposal_id": "prop-1", "status": "pending"},
        )
        assert resp.data == {"proposal_id": "prop-1", "status": "pending"}

    def test_bot_response_ephemeral(self):
        """BotResponse can be ephemeral (only visible to requester)."""
        resp = BotResponse(
            success=False,
            message="Invalid input",
            ephemeral=True,
        )
        assert resp.ephemeral is True

    def test_bot_response_with_error(self):
        """BotResponse can include error message when success=False."""
        resp = BotResponse(
            success=False,
            message="Command failed",
            error="User does not have permission",
        )
        assert resp.success is False
        assert resp.error == "User does not have permission"

    def test_bot_response_immutable(self):
        """BotResponse is frozen (immutable)."""
        resp = BotResponse(success=True, message="Test")

        # Should not be able to modify attributes
        with pytest.raises(
            (AttributeError, TypeError, dataclasses.FrozenInstanceError)
        ):
            resp.success = False

    def test_bot_response_defaults(self):
        """BotResponse has sensible defaults."""
        resp = BotResponse(success=True, message="OK")
        assert resp.data == {}
        assert resp.ephemeral is False
        assert resp.error is None


class TestBotInterface:
    """Test BotInterface abstract contract."""

    def test_bot_interface_is_abstract(self):
        """BotInterface cannot be instantiated directly."""
        with pytest.raises(TypeError):
            BotInterface()

    def test_bot_interface_is_abc(self):
        """BotInterface is an ABC."""
        assert isinstance(BotInterface, type)
        assert issubclass(BotInterface, ABC)

    def test_bot_interface_requires_start_method(self):
        """Any subclass must implement start() method."""

        class IncompleteBot(BotInterface):
            async def stop(self) -> None:
                pass

            async def handle_command(self, command: BotCommand) -> BotResponse:
                pass

        with pytest.raises(TypeError):
            IncompleteBot()

    def test_bot_interface_requires_stop_method(self):
        """Any subclass must implement stop() method."""

        class IncompleteBot(BotInterface):
            async def start(self) -> None:
                pass

            async def handle_command(self, command: BotCommand) -> BotResponse:
                pass

        with pytest.raises(TypeError):
            IncompleteBot()

    def test_bot_interface_requires_handle_command_method(self):
        """Any subclass must implement handle_command() method."""

        class IncompleteBot(BotInterface):
            async def start(self) -> None:
                pass

            async def stop(self) -> None:
                pass

        with pytest.raises(TypeError):
            IncompleteBot()


class TestBotImplementation:
    """Test minimal bot implementation for contract verification."""

    class MinimalBot(BotInterface):
        """Minimal bot implementation for testing."""

        def __init__(self):
            self.started = False
            self.stopped = False

        async def start(self) -> None:
            self.started = True

        async def stop(self) -> None:
            self.stopped = True

        async def handle_command(self, command: BotCommand) -> BotResponse:
            return BotResponse(
                success=True,
                message=f"Command {command.name} handled",
            )

    @pytest.mark.asyncio
    async def test_bot_implementation_contract(self):
        """Minimal bot implementation satisfies contract."""
        bot = self.MinimalBot()

        # Verify it's a BotInterface
        assert isinstance(bot, BotInterface)

    @pytest.mark.asyncio
    async def test_bot_start_stop_lifecycle(self):
        """Bot start/stop lifecycle works."""
        bot = self.MinimalBot()

        assert bot.started is False
        assert bot.stopped is False

        await bot.start()
        assert bot.started is True

        await bot.stop()
        assert bot.stopped is True

    @pytest.mark.asyncio
    async def test_bot_handle_command_returns_response(self):
        """Bot handle_command returns BotResponse."""
        bot = self.MinimalBot()

        cmd = BotCommand(
            name="test",
            args={},
            user_id="user123",
            platform="discord",
        )

        response = await bot.handle_command(cmd)
        assert isinstance(response, BotResponse)
        assert response.success is True
        assert "test" in response.message


class TestPlatformValidation:
    """Test platform parameter validation."""

    def test_platform_must_be_valid(self):
        """BotCommand validates platform is in allowed set."""
        valid_platforms = {"discord", "telegram", "cli", "web"}

        for platform in valid_platforms:
            cmd = BotCommand(
                name="test",
                args={},
                user_id="user123",
                platform=platform,
            )
            assert cmd.platform == platform

    def test_invalid_platform_raises_error(self):
        """BotCommand raises error for invalid platform."""
        with pytest.raises(ValueError):
            BotCommand(
                name="test",
                args={},
                user_id="user123",
                platform="invalid_platform",
            )

    def test_platform_case_sensitive(self):
        """Platform names are case-sensitive."""
        with pytest.raises(ValueError):
            BotCommand(
                name="test",
                args={},
                user_id="user123",
                platform="Discord",  # Should be "discord"
            )


class TestBotResponseSuccess:
    """Test BotResponse success validation."""

    def test_response_success_must_be_bool(self):
        """BotResponse.success must be boolean."""
        # Should succeed with bool
        resp = BotResponse(success=True, message="Test")
        assert resp.success is True

        resp = BotResponse(success=False, message="Test")
        assert resp.success is False

    def test_response_success_cannot_be_none(self):
        """BotResponse.success cannot be None."""
        with pytest.raises((TypeError, ValueError)):
            BotResponse(success=None, message="Test")


# Import dataclasses for exception checking
import dataclasses
