"""Tests for CLI Framework."""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from io import StringIO
import sys

from cynic.kernel.observability.cli.app import CliApp


@pytest.mark.asyncio
async def test_cli_app_initializes():
    """CliApp can be created and initializes with running flag."""
    app = CliApp()
    assert app is not None
    assert hasattr(app, '_running')
    assert app._running is True


@pytest.mark.asyncio
async def test_cli_menu_structure():
    """CliApp returns correct menu items with keys and labels."""
    app = CliApp()
    menu_items = app.get_menu_items()

    assert isinstance(menu_items, list)
    assert len(menu_items) == 10  # 9 menu items + 1 exit

    # Check that all items are tuples of (key, label)
    for item in menu_items:
        assert isinstance(item, tuple)
        assert len(item) == 2
        key, label = item
        assert isinstance(key, str)
        assert isinstance(label, str)

    # Check specific menu items
    keys = [item[0] for item in menu_items]
    labels = [item[1] for item in menu_items]

    # Menu structure as per requirements
    assert '1' in keys
    assert '2' in keys
    assert '3' in keys
    assert '4' in keys
    assert '5' in keys
    assert '6' in keys
    assert '7' in keys
    assert '8' in keys
    assert '9' in keys
    assert '0' in keys  # EXIT

    # Check that labels contain key descriptions
    labels_str = ' '.join(labels)
    assert 'OBSERVE' in labels_str
    assert 'CYNIC' in labels_str or 'MIND' in labels_str
    assert 'STATE' in labels_str or 'YOUR' in labels_str
    assert 'MACHINE' in labels_str
    assert 'SYMBIOSIS' in labels_str or 'ALIGNMENT' in labels_str
    assert 'TALK' in labels_str or 'CHAT' in labels_str
    assert 'HISTORY' in labels_str or 'DECISIONS' in labels_str
    assert 'FEEDBACK' in labels_str
    assert 'ACTUATE' in labels_str or 'ACTIONS' in labels_str
    assert 'EXIT' in labels_str


@pytest.mark.asyncio
async def test_cli_show_menu_displays_menu(capsys):
    """show_menu() displays menu items to stdout."""
    app = CliApp()

    with patch('builtins.input', return_value='0'):
        await app.show_menu()

    captured = capsys.readouterr()
    output = captured.out

    # Menu should contain menu items
    assert 'OBSERVE' in output
    assert 'CYNIC' in output or 'MIND' in output
    assert 'EXIT' in output


@pytest.mark.asyncio
async def test_cli_handle_menu_choice_1_observe():
    """handle_menu_choice('1') calls show_observe()."""
    app = CliApp()

    with patch.object(app, 'show_observe', new_callable=AsyncMock) as mock_observe:
        await app.handle_menu_choice('1')
        mock_observe.assert_called_once()


@pytest.mark.asyncio
async def test_cli_handle_menu_choice_0_exit():
    """handle_menu_choice('0') sets running to False (exit)."""
    app = CliApp()
    assert app._running is True

    await app.handle_menu_choice('0')

    assert app._running is False


@pytest.mark.asyncio
async def test_cli_handle_menu_choice_invalid():
    """handle_menu_choice() handles invalid choices gracefully."""
    app = CliApp()
    app._running = True

    # Should not raise an exception
    await app.handle_menu_choice('invalid')
    await app.handle_menu_choice('99')

    # App should still be running
    assert app._running is True


@pytest.mark.asyncio
async def test_cli_show_observe_returns_without_error():
    """show_observe() executes without raising an exception."""
    app = CliApp()

    # Should not raise
    await app.show_observe()


@pytest.mark.asyncio
async def test_cli_run_can_start_and_stop():
    """run() starts the main loop and respects _running flag."""
    app = CliApp()

    # Mock the async methods that would be called during run
    with patch.object(app, 'show_menu', new_callable=AsyncMock) as mock_show_menu:
        with patch.object(app, 'handle_menu_choice', new_callable=AsyncMock) as mock_handle:
            # Simulate user pressing exit on first menu display
            async def stop_after_first_call(*args, **kwargs):
                app._running = False

            mock_show_menu.side_effect = stop_after_first_call

            # run() should complete when _running becomes False
            await app.run()

            # Should have called show_menu at least once
            assert mock_show_menu.called


@pytest.mark.asyncio
async def test_cli_get_menu_items_stable():
    """get_menu_items() returns consistent menu structure on repeated calls."""
    app = CliApp()

    items1 = app.get_menu_items()
    items2 = app.get_menu_items()

    assert items1 == items2
    assert len(items1) == len(items2)

    # Keys should be in consistent order
    keys1 = [item[0] for item in items1]
    keys2 = [item[0] for item in items2]
    assert keys1 == keys2
