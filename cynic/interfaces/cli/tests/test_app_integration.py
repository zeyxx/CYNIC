"""Tests for CLI App integration with dialogue, history, and feedback systems."""

from unittest.mock import AsyncMock, patch

import pytest

from cynic.kernel.observability.cli.app import CliApp


@pytest.mark.asyncio
async def test_cli_app_has_talk_option():
    """CLI app includes TALK menu option."""
    app = CliApp()
    menu_items = app.get_menu_items()

    # Check that option 6 (TALK) exists in menu
    menu_keys = [key for key, label in menu_items]
    assert '6' in menu_keys

    # Check that TALK label is in menu
    talk_items = [label for key, label in menu_items if key == '6']
    assert len(talk_items) > 0
    assert 'TALK' in talk_items[0].upper()


@pytest.mark.asyncio
async def test_cli_app_has_history_option():
    """CLI app includes HISTORY menu option."""
    app = CliApp()
    menu_items = app.get_menu_items()

    # Check that option 7 (HISTORY) exists in menu
    menu_keys = [key for key, label in menu_items]
    assert '7' in menu_keys

    # Check that HISTORY label is in menu
    history_items = [label for key, label in menu_items if key == '7']
    assert len(history_items) > 0
    assert 'HISTORY' in history_items[0].upper()


@pytest.mark.asyncio
async def test_cli_app_has_feedback_option():
    """CLI app includes FEEDBACK menu option."""
    app = CliApp()
    menu_items = app.get_menu_items()

    # Check that option 8 (FEEDBACK) exists in menu
    menu_keys = [key for key, label in menu_items]
    assert '8' in menu_keys

    # Check that FEEDBACK label is in menu
    feedback_items = [label for key, label in menu_items if key == '8']
    assert len(feedback_items) > 0
    assert 'FEEDBACK' in feedback_items[0].upper()


@pytest.mark.asyncio
async def test_talk_mode_option_routed():
    """TALK option (6) is routed to dialogue handling."""
    app = CliApp()

    # Mock the dialogue handling method
    app.handle_talk_option = AsyncMock(return_value=None)

    # Simulate choosing option 6
    await app.handle_menu_choice('6')

    # Check that handle_talk_option was called
    app.handle_talk_option.assert_called_once()


@pytest.mark.asyncio
async def test_history_option_routing():
    """HISTORY option (7) is handled."""
    app = CliApp()

    # Mock show_history method
    app.show_history = AsyncMock(return_value=None)

    # Simulate choosing option 7
    await app.handle_menu_choice('7')

    # The handler should exist or history output should be shown
    # We're just checking it doesn't crash


@pytest.mark.asyncio
async def test_feedback_option_routing():
    """FEEDBACK option (8) is handled."""
    app = CliApp()

    # Mock show_feedback method
    app.show_feedback = AsyncMock(return_value=None)

    # Simulate choosing option 8
    await app.handle_menu_choice('8')

    # The handler should exist or feedback output should be shown
    # We're just checking it doesn't crash


@pytest.mark.asyncio
async def test_handle_talk_option_initializes_dialogue():
    """handle_talk_option initializes dialogue mode correctly."""
    app = CliApp()

    # Verify the method exists
    assert hasattr(app, 'handle_talk_option')

    # Mock the dialogue mode
    with patch('cynic.kernel.observability.cli.app.DialogueMode') as mock_dialogue:
        mock_instance = AsyncMock()
        mock_instance.initialize = AsyncMock(return_value=None)
        mock_instance.get_greeting = AsyncMock(return_value="Welcome!")
        mock_instance.close = AsyncMock(return_value=None)
        mock_dialogue.return_value = mock_instance

        # Simulate the talk option with empty input to exit
        with patch('builtins.input', side_effect=['exit']):
            await app.handle_talk_option()

        # Check that dialogue mode was initialized
        mock_instance.initialize.assert_called()
