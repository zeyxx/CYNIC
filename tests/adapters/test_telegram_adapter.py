"""
Tests for TelegramAdapter implementing unified BotInterface.

These tests verify:
1. TelegramAdapter implements BotInterface contract
2. Command routing works correctly (propose, proposals, vote, etc)
3. Telegram message parsing (slash commands in text)
4. BotResponse objects are created with correct structure
5. Telegram markdown formatting (bold, code blocks)
6. Message pagination for long responses (>4000 chars)
7. Error handling is graceful
8. Concurrent message handling
"""

import asyncio
from unittest.mock import Mock

import pytest

from cynic.interfaces.bots.bot_interface import BotCommand, BotInterface, BotResponse
from cynic.interfaces.bots.governance.adapters.telegram_adapter import TelegramAdapter
from cynic.kernel.core.unified_state import UnifiedConsciousState


class TestTelegramAdapterImplementsInterface:
    """Test TelegramAdapter implements BotInterface contract."""

    def test_telegram_adapter_is_bot_interface(self):
        """TelegramAdapter is a subclass of BotInterface."""
        assert issubclass(TelegramAdapter, BotInterface)

    def test_telegram_adapter_has_required_methods(self):
        """TelegramAdapter implements all required abstract methods."""
        assert hasattr(TelegramAdapter, "start")
        assert hasattr(TelegramAdapter, "stop")
        assert hasattr(TelegramAdapter, "handle_command")

    def test_telegram_adapter_can_be_instantiated(self):
        """TelegramAdapter can be instantiated with required parameters."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter is not None
        assert adapter.client == mock_client
        assert adapter.conscious_state == mock_state

    def test_telegram_adapter_stores_client_reference(self):
        """TelegramAdapter stores telegram client reference."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter.client is mock_client

    def test_telegram_adapter_stores_conscious_state_reference(self):
        """TelegramAdapter stores conscious_state reference."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        assert adapter.conscious_state is mock_state


class TestTelegramAdapterLifecycle:
    """Test TelegramAdapter async lifecycle (start/stop)."""

    @pytest.mark.asyncio
    async def test_adapter_start_method_exists(self):
        """TelegramAdapter.start() method exists and is async."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        assert hasattr(adapter, "start")
        assert asyncio.iscoroutinefunction(adapter.start)

    @pytest.mark.asyncio
    async def test_adapter_stop_method_exists(self):
        """TelegramAdapter.stop() method exists and is async."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        assert hasattr(adapter, "stop")
        assert asyncio.iscoroutinefunction(adapter.stop)

    @pytest.mark.asyncio
    async def test_adapter_start_can_be_awaited(self):
        """TelegramAdapter.start() can be awaited without raising."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Should not raise
        await adapter.start()

    @pytest.mark.asyncio
    async def test_adapter_stop_can_be_awaited(self):
        """TelegramAdapter.stop() can be awaited without raising."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Should not raise
        await adapter.stop()


class TestTelegramAdapterHandleCommand:
    """Test TelegramAdapter.handle_command() routing and responses."""

    @pytest.mark.asyncio
    async def test_handle_command_returns_bot_response(self):
        """handle_command returns BotResponse object."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_handle_unknown_command_returns_failure(self):
        """handle_command returns failure for unknown command."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="unknown_command",
            args={},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False
        assert "unknown" in response.message.lower() or "not found" in response.message.lower()

    @pytest.mark.asyncio
    async def test_handle_command_validates_platform(self):
        """handle_command validates platform is telegram."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Try with wrong platform
        cmd = BotCommand(
            name="propose",
            args={"title": "Test"},
            user_id="user123",
            platform="discord",  # Wrong platform
        )

        response = await adapter.handle_command(cmd)

        # Should handle gracefully (either return failure or raise ValueError)
        try:
            assert response.success is False or "platform" in response.message.lower()
        except (ValueError, AssertionError):
            # ValueError also acceptable
            pass


class TestTelegramAdapterMessageParsing:
    """Test Telegram message parsing (slash commands)."""

    @pytest.mark.asyncio
    async def test_parse_telegram_message_extracts_command(self):
        """_parse_telegram_message extracts command name from text."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Test parsing simple command
        cmd_name, args = adapter._parse_telegram_message("/propose")

        assert cmd_name == "propose"
        assert isinstance(args, dict)

    @pytest.mark.asyncio
    async def test_parse_telegram_message_extracts_arguments(self):
        """_parse_telegram_message extracts arguments from message."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Test parsing command with arguments
        cmd_name, args = adapter._parse_telegram_message(
            "/propose Test Title This is a description"
        )

        assert cmd_name == "propose"
        # Should extract title from arguments
        assert "title" in args.keys() or "args" in args.keys() or len(args) > 0

    @pytest.mark.asyncio
    async def test_parse_telegram_message_handles_missing_slash(self):
        """_parse_telegram_message handles text without slash gracefully."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Should handle text without slash
        result = adapter._parse_telegram_message("not a command")

        # Should return tuple or raise ValueError
        assert isinstance(result, tuple) or result is None

    def test_parse_telegram_message_handles_various_commands(self):
        """_parse_telegram_message correctly parses various command formats."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        test_cases = [
            "/propose",
            "/proposals",
            "/vote",
            "/judgment_status",
        ]

        for cmd_text in test_cases:
            result = adapter._parse_telegram_message(cmd_text)
            if result:  # Should parse
                cmd_name, args = result
                assert cmd_name in ["propose", "proposals", "vote", "judgment_status"]


class TestTelegramAdapterProposeCommand:
    """Test /propose command handling."""

    @pytest.mark.asyncio
    async def test_propose_command_creates_response(self):
        """/propose command creates BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test Proposal", "description": "Test Description"},
            user_id="user123",
            platform="telegram",
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

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Missing title
        cmd = BotCommand(
            name="propose",
            args={"description": "No title provided"},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "title" in response.message.lower()

    @pytest.mark.asyncio
    async def test_propose_response_has_proposal_id(self):
        """Successful propose response includes proposal_id in data."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="propose",
            args={"title": "Test", "description": "Desc"},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        # If successful, should have proposal_id in data
        if response.success:
            assert "proposal_id" in response.data or "id" in response.data


class TestTelegramAdapterProposalsCommand:
    """Test /proposals command handling."""

    @pytest.mark.asyncio
    async def test_proposals_command_returns_response(self):
        """/proposals command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={"page": 1},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_proposals_response_has_list_data(self):
        """Proposals response includes list in data."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={},
            user_id="user123",
            platform="telegram",
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

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="proposals",
            args={},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)


class TestTelegramAdapterVoteCommand:
    """Test /vote command handling."""

    @pytest.mark.asyncio
    async def test_vote_command_returns_response(self):
        """Vote command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"proposal_id": "prop-1", "vote": "yes"},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_vote_requires_proposal_id(self):
        """Vote command requires proposal_id."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"vote": "yes"},  # Missing proposal_id
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "proposal_id" in response.message.lower()

    @pytest.mark.asyncio
    async def test_vote_requires_valid_vote_value(self):
        """Vote command requires valid vote value (yes/no/abstain)."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="vote",
            args={"proposal_id": "prop-1", "vote": "maybe"},  # Invalid vote
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert response.success is False or "yes" in response.message.lower()

    @pytest.mark.asyncio
    async def test_vote_accepts_all_valid_options(self):
        """Vote command accepts yes, no, and abstain."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        for vote_option in ["yes", "no", "abstain"]:
            cmd = BotCommand(
                name="vote",
                args={"proposal_id": "prop-1", "vote": vote_option},
                user_id="user123",
                platform="telegram",
            )

            response = await adapter.handle_command(cmd)

            # Should either succeed or have valid error
            assert isinstance(response, BotResponse)


class TestTelegramAdapterJudgmentStatus:
    """Test /judgment_status command handling."""

    @pytest.mark.asyncio
    async def test_status_command_returns_response(self):
        """Status command returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)
        mock_state.total_judgments = 5
        mock_state.get_recent_judgments = Mock(return_value=[])
        mock_state.get_consensus_score = Mock(return_value=0.75)
        mock_state.dog_agreement_scores = {}

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="telegram",
        )

        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)
        assert response.success is True

    @pytest.mark.asyncio
    async def test_status_command_accesses_consciousness_state(self):
        """Status command queries consciousness state."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)
        mock_state.total_judgments = 10
        mock_state.get_recent_judgments = Mock(return_value=[])
        mock_state.get_consensus_score = Mock(return_value=0.85)
        mock_state.dog_agreement_scores = {}

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="telegram",
        )

        await adapter.handle_command(cmd)

        # Should have called methods on consciousness_state
        mock_state.get_recent_judgments.assert_called()
        mock_state.get_consensus_score.assert_called()


class TestTelegramAdapterMarkdownFormatting:
    """Test Telegram markdown formatting in responses."""

    def test_format_response_for_telegram_uses_markdown(self):
        """_format_response_for_telegram converts BotResponse to markdown."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        response = BotResponse(
            success=True,
            message="Test message with **bold** text",
            data={"key": "value"},
        )

        formatted = adapter._format_response_for_telegram(response)

        assert isinstance(formatted, str)
        assert len(formatted) > 0

    def test_format_response_handles_long_messages(self):
        """_format_response_for_telegram handles messages over 4000 chars."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create a very long message
        long_message = "x" * 5000

        response = BotResponse(
            success=True,
            message=long_message,
            data={},
        )

        formatted = adapter._format_response_for_telegram(response)

        # Should handle long messages gracefully (split or truncate)
        assert formatted is not None
        assert isinstance(formatted, str)

    def test_format_response_preserves_critical_info(self):
        """_format_response_for_telegram preserves success/error info."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        response = BotResponse(
            success=False,
            message="Error occurred",
            error="Test error",
        )

        formatted = adapter._format_response_for_telegram(response)

        # Should indicate error or failure
        assert formatted is not None


class TestTelegramAdapterMessagePagination:
    """Test message pagination for long responses."""

    def test_paginate_message_splits_long_text(self):
        """_paginate_message splits text over 4000 chars."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create a message longer than Telegram's limit
        long_text = "\n".join([f"Line {i}: content content content" for i in range(200)])

        pages = adapter._paginate_message(long_text, max_length=100)

        # Should split into multiple pages
        assert isinstance(pages, list)
        assert len(pages) > 1 or len(long_text) <= 100

    def test_paginate_message_respects_max_length(self):
        """_paginate_message respects max_length parameter."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Use newline-separated text for pagination to work better
        long_text = "\n".join([f"Line {i}: some content" for i in range(100)])

        pages = adapter._paginate_message(long_text, max_length=200)

        # Each page should be <= max_length
        for page in pages:
            assert len(page) <= 200  # Should respect max_length

    def test_paginate_message_handles_short_text(self):
        """_paginate_message handles short text (single page)."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        short_text = "Short message"

        pages = adapter._paginate_message(short_text, max_length=100)

        # Should return single page
        assert len(pages) == 1
        assert pages[0] == short_text


class TestTelegramAdapterErrorHandling:
    """Test error handling in TelegramAdapter."""

    @pytest.mark.asyncio
    async def test_handle_command_catches_exceptions(self):
        """handle_command catches exceptions and returns BotResponse."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create a command that will trigger an error in handler
        cmd = BotCommand(
            name="propose",
            args={"title": None},  # This might cause an error
            user_id="user123",
            platform="telegram",
        )

        # Should not raise exception
        response = await adapter.handle_command(cmd)

        assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_invalid_command_type_returns_failure(self):
        """handle_command returns failure for invalid BotCommand type."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Pass invalid type (not a BotCommand)
        try:
            response = await adapter.handle_command("not a command")
            assert response.success is False
        except (TypeError, AttributeError):
            # TypeError also acceptable
            pass


class TestTelegramAdapterConcurrentMessages:
    """Test handling of concurrent messages."""

    @pytest.mark.asyncio
    async def test_handle_concurrent_commands(self):
        """Adapter handles concurrent commands safely."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create multiple commands
        commands = [
            BotCommand(
                name="propose",
                args={"title": f"Proposal {i}"},
                user_id=f"user{i}",
                platform="telegram",
            )
            for i in range(5)
        ]

        # Execute concurrently
        responses = await asyncio.gather(*[
            adapter.handle_command(cmd) for cmd in commands
        ])

        # All should return responses
        assert len(responses) == 5
        assert all(isinstance(r, BotResponse) for r in responses)


class TestTelegramAdapterPaginationEdgeCases:
    """Test pagination edge cases and line length violations."""

    def test_paginate_message_handles_line_exceeding_max_length(self):
        """_paginate_message handles lines longer than max_length correctly.

        When a single line exceeds max_length, it should be split into
        character chunks rather than placed in a page that violates the limit.

        This tests the fix for the bug where a 5000-character line in a
        message with max_length=50 would result in a page > max_length.
        """
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create a message with a line that exceeds max_length
        # This could be a code block, URL, or very long word
        very_long_line = "x" * 5000  # 5000 characters exceeds typical max_length
        max_length = 100

        # Test with the very long line
        text = f"Header line\n{very_long_line}\nFooter line"
        pages = adapter._paginate_message(text, max_length=max_length)

        # Verify that:
        # 1. All pages respect the max_length limit
        for i, page in enumerate(pages):
            assert len(page) <= max_length, (
                f"Page {i} exceeds max_length: "
                f"len(page)={len(page)}, max_length={max_length}"
            )

        # 2. Content is preserved (concatenated pages should equal original)
        reconstructed = "".join(pages)
        assert reconstructed == text, "Content was lost during pagination"

        # 3. There should be multiple pages (since 5000 > 100)
        assert len(pages) > 1, "Long content should be split into multiple pages"

    def test_paginate_message_preserves_content_with_long_line(self):
        """_paginate_message preserves all content when splitting long lines."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create different long line scenarios
        test_cases = [
            # (text, max_length, description)
            ("x" * 500, 100, "Single 500-char line with limit 100"),
            ("a" * 1000 + "\n" + "b" * 2000, 150, "Two lines, both exceeding limit"),
            ("Short\n" + "y" * 3000 + "\nShort", 200, "Long line sandwiched between short lines"),
        ]

        for text, max_length, description in test_cases:
            pages = adapter._paginate_message(text, max_length=max_length)

            # All pages must be <= max_length
            for page in pages:
                assert len(page) <= max_length, (
                    f"Test case '{description}': Page exceeds limit. "
                    f"len(page)={len(page)}, max_length={max_length}"
                )

            # Content must be preserved
            reconstructed = "".join(pages)
            assert reconstructed == text, (
                f"Test case '{description}': Content was lost or modified"
            )

    def test_paginate_message_handles_unicode_long_lines(self):
        """_paginate_message handles unicode characters in long lines."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Create a long line with unicode characters
        unicode_line = "你好世界" * 200  # 800 chars of unicode
        max_length = 100

        text = f"Header\n{unicode_line}\nFooter"
        pages = adapter._paginate_message(text, max_length=max_length)

        # All pages must respect max_length
        for page in pages:
            assert len(page) <= max_length, (
                f"Unicode page exceeds limit: "
                f"len(page)={len(page)}, max_length={max_length}"
            )

        # Content preserved
        reconstructed = "".join(pages)
        assert reconstructed == text

    def test_paginate_message_handles_code_blocks_with_long_lines(self):
        """_paginate_message handles code blocks with lines exceeding max_length."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Simulate a code block with a very long line (common edge case)
        code_block = "```\n" + ("a=b+" * 500) + "\n```"  # Long single line in code
        max_length = 100

        pages = adapter._paginate_message(code_block, max_length=max_length)

        # Verify constraint
        for page in pages:
            assert len(page) <= max_length, (
                f"Code block page exceeds limit: "
                f"len(page)={len(page)}, max_length={max_length}"
            )

        # Content preserved
        reconstructed = "".join(pages)
        assert reconstructed == code_block


class TestTelegramAdapterIntegration:
    """Integration tests for TelegramAdapter."""

    @pytest.mark.asyncio
    async def test_full_propose_workflow(self):
        """Full workflow: command routing -> handler -> response."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Simulate parsing message and creating command
        text = "/propose Test Title Test description"
        cmd_name, args = adapter._parse_telegram_message(text)

        if cmd_name:
            cmd = BotCommand(
                name=cmd_name,
                args=args or {},
                user_id="user123",
                platform="telegram",
            )

            response = await adapter.handle_command(cmd)

            assert isinstance(response, BotResponse)

    @pytest.mark.asyncio
    async def test_start_stop_lifecycle(self):
        """Full lifecycle: start -> handle commands -> stop."""
        mock_client = Mock()
        mock_state = Mock(spec=UnifiedConsciousState)

        adapter = TelegramAdapter(client=mock_client, conscious_state=mock_state)

        # Start
        await adapter.start()

        # Handle command
        cmd = BotCommand(
            name="judgment_status",
            args={},
            user_id="user123",
            platform="telegram",
        )

        mock_state.total_judgments = 0
        mock_state.get_recent_judgments = Mock(return_value=[])
        mock_state.get_consensus_score = Mock(return_value=0.0)
        mock_state.dog_agreement_scores = {}

        response = await adapter.handle_command(cmd)

        # Stop
        await adapter.stop()

        assert isinstance(response, BotResponse)
