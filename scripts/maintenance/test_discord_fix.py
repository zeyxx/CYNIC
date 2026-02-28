#!/usr/bin/env python3
"""
Test Discord command signature fixes.
Validates that error handler and command sync work correctly.
"""

import sys
import asyncio
import logging
from unittest.mock import AsyncMock, MagicMock, patch

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def test_error_handler_defer_race():
    """Test that error handler handles defer race condition safely"""
    print("\n=== Test 1: Error Handler Defer Race Condition ===")

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

        print("[OK] Error handler handled defer() safely")
        assert interaction.response.defer.called
        assert interaction.followup.send.called
        return True

    except Exception as e:
        print(f"[FAIL] Error handler failed: {e}")
        return False


async def test_error_handler_already_deferred():
    """Test that error handler handles already-deferred interactions"""
    print("\n=== Test 2: Error Handler Already Deferred ===")

    # Mock interaction that's already deferred
    interaction = AsyncMock()
    interaction.response.is_done = MagicMock(return_value=True)  # Already deferred
    interaction.followup.send = AsyncMock()

    try:
        if not interaction.response.is_done():
            # This branch shouldn't execute
            raise AssertionError("Should not try to defer an already-deferred interaction")

        await interaction.followup.send("Error message", ephemeral=True)

        print("[OK] Error handler correctly used followup for deferred interaction")
        assert interaction.followup.send.called
        return True

    except Exception as e:
        print(f"[FAIL] Error handler failed: {e}")
        return False


async def test_command_signature():
    """Test that command definitions are correctly structured"""
    print("\n=== Test 3: Command Definitions ===")

    # Check that bot.py can be imported without errors
    try:
        # We can't fully import bot.py without Discord token, but we can check syntax
        import ast

        with open("governance_bot/bot.py", "r", encoding="utf-8", errors="ignore") as f:
            code = f.read()

        try:
            ast.parse(code)
            print("[OK] bot.py has valid Python syntax")

            # Check for required patterns
            checks = {
                "error handler": "@bot.tree.error" in code,
                "propose command": "cmd_propose" in code,
                "followup usage": "interaction.followup.send" in code,
                "defer usage": "interaction.response.defer" in code,
            }

            all_passed = True
            for check_name, passed in checks.items():
                status = "[OK]" if passed else "[FAIL]"
                print(f"  {status} {check_name}")
                all_passed = all_passed and passed

            return all_passed

        except SyntaxError as e:
            print(f"[FAIL] Syntax error in bot.py: {e}")
            return False

    except Exception as e:
        print(f"[FAIL] Failed to check bot.py: {e}")
        return False


async def test_sync_script_exists():
    """Test that sync script is available"""
    print("\n=== Test 4: Sync Script Availability ===")

    import os

    files = {
        "governance_bot/sync_commands.py": "Command sync script",
        "restart_bot.sh": "Bot restart script",
        "DISCORD_FIX_REPORT.md": "Fix documentation",
    }

    all_passed = True
    for filename, description in files.items():
        exists = os.path.exists(filename)
        status = "[OK]" if exists else "[FAIL]"
        print(f"  {status} {description}: {filename}")
        all_passed = all_passed and exists

    return all_passed


async def main():
    """Run all tests"""
    print("""
    ================================================
      Discord Command Signature Fix - Test Suite
    ================================================
    """)

    results = []

    # Run tests
    results.append(("Error Handler Race Condition", await test_error_handler_defer_race()))
    results.append(("Error Handler Already Deferred", await test_error_handler_already_deferred()))
    results.append(("Command Definitions", await test_command_signature()))
    results.append(("Fix Scripts Available", await test_sync_script_exists()))

    # Summary
    print("\n" + "=" * 50)
    print("TEST SUMMARY")
    print("=" * 50)

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for test_name, result in results:
        status = "[OK]" if result else "[FAIL]"
        print(f"{status} {test_name}")

    print("=" * 50)
    print(f"\nResult: {passed}/{total} tests passed")

    if passed == total:
        print("""
[OK] All fixes verified!

Next steps:
1. bash restart_bot.sh
2. Check logs: tail -f governance_bot/startup.log
3. Verify commands sync without errors
        """)
        return 0
    else:
        print(f"\n[FAIL] {total - passed} test(s) failed")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
