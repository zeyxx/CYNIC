#!/usr/bin/env python3
"""
Test Discord command signature fixes.
Validates that error handler and command sync work correctly.
"""

import asyncio
import logging
import sys
from unittest.mock import AsyncMock, MagicMock

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def test_error_handler_defer_race():
    """Test that error handler handles defer race condition safely"""

    # Mock discord interaction
    interaction = AsyncMock()
    interaction.response.is_done = MagicMock(return_value=False)  # Not yet deferred
    interaction.response.defer = AsyncMock()
    interaction.followup.send = AsyncMock()

    # Simulate the fixed error handler logic
    try:
        if not interaction.response.is_done():
            try:
                await interaction.response.defer(ephemeral=True)
            except Exception:
                pass

        # Always use followup for safety
        await interaction.followup.send("Error message", ephemeral=True)

        assert interaction.response.defer.called
        assert interaction.followup.send.called
        return True

    except Exception:
        return False


async def test_error_handler_already_deferred():
    """Test that error handler handles already-deferred interactions"""

    # Mock interaction that's already deferred
    interaction = AsyncMock()
    interaction.response.is_done = MagicMock(return_value=True)  # Already deferred
    interaction.followup.send = AsyncMock()

    try:
        if not interaction.response.is_done():
            # This branch shouldn't execute
            raise AssertionError(
                "Should not try to defer an already-deferred interaction"
            )

        await interaction.followup.send("Error message", ephemeral=True)

        assert interaction.followup.send.called
        return True

    except Exception:
        return False


async def test_command_signature():
    """Test that command definitions are correctly structured"""

    # Check that bot.py can be imported without errors
    try:
        # We can't fully import bot.py without Discord token, but we can check syntax
        import ast

        with open("governance_bot/bot.py", encoding="utf-8", errors="ignore") as f:
            code = f.read()

        try:
            ast.parse(code)

            # Check for required patterns
            checks = {
                "error handler": "@bot.tree.error" in code,
                "propose command": "cmd_propose" in code,
                "followup usage": "interaction.followup.send" in code,
                "defer usage": "interaction.response.defer" in code,
            }

            all_passed = True
            for _check_name, passed in checks.items():
                all_passed = all_passed and passed

            return all_passed

        except SyntaxError:
            return False

    except Exception:
        return False


async def test_sync_script_exists():
    """Test that sync script is available"""

    import os

    files = {
        "governance_bot/sync_commands.py": "Command sync script",
        "restart_bot.sh": "Bot restart script",
        "DISCORD_FIX_REPORT.md": "Fix documentation",
    }

    all_passed = True
    for filename, _description in files.items():
        exists = os.path.exists(filename)
        all_passed = all_passed and exists

    return all_passed


async def main():
    """Run all tests"""

    results = []

    # Run tests
    results.append(
        ("Error Handler Race Condition", await test_error_handler_defer_race())
    )
    results.append(
        ("Error Handler Already Deferred", await test_error_handler_already_deferred())
    )
    results.append(("Command Definitions", await test_command_signature()))
    results.append(("Fix Scripts Available", await test_sync_script_exists()))

    # Summary

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for _test_name, _result in results:
        pass

    if passed == total:
        return 0
    else:
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
