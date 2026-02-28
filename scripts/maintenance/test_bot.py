#!/usr/bin/env python
"""
Test the Discord bot for syntax and import errors.
"""

import os
import sys
import asyncio

# Mock the Discord token before importing bot
os.environ["DISCORD_TOKEN"] = "test_token_for_syntax_check"

try:
    import cynic.discord.bot as bot_module
    # Get the actual module from sys.modules
    import sys
    bot_mod = sys.modules['cynic.discord.bot']
    print("[OK] Bot module imported successfully")
except Exception as e:
    print(f"[FAIL] Failed to import bot: {e}")
    sys.exit(1)

# Test that key functions and commands exist
try:
    assert hasattr(bot_module, 'tree'), "tree commands not found"
    assert len(bot_module.tree.get_commands()) >= 5, f"Expected 5+ commands, got {len(bot_module.tree.get_commands())}"
    assert 'cleanup' in bot_mod.__dict__, "cleanup function not found in module"
    assert 'main' in bot_mod.__dict__, "main function not found in module"
    assert 'check_cynic_health' in bot_mod.__dict__, "health check task not found in module"
    print("[OK] All required functions and commands exist")
except AssertionError as e:
    print(f"[FAIL] {e}")
    sys.exit(1)

# Test that the bot object has the right attributes
try:
    # bot_module is the actual Bot object (due to how it's defined)
    assert hasattr(bot_module, 'cynic_session'), "cynic_session attribute not found"
    assert hasattr(bot_module, 'cynic_ready'), "cynic_ready attribute not found"
    assert bot_module.cynic_session is None, "Session should be None before on_ready"
    assert bot_module.cynic_ready is False, "Should not be ready initially"
    print("[OK] Bot state initialized correctly")
except AssertionError as e:
    print(f"[FAIL] {e}")
    sys.exit(1)

# Test cleanup function exists and is async
try:
    cleanup_fn = bot_mod.__dict__['cleanup']
    assert asyncio.iscoroutinefunction(cleanup_fn), "cleanup should be async"
    print("[OK] Cleanup is async function")
except (KeyError, AssertionError) as e:
    print(f"[FAIL] {e}")
    sys.exit(1)

print("\n[PASS] All tests passed! Bot is syntax and structure correct.")
print("\nTo run the bot:")
print("  1. Create a .env file with: DISCORD_TOKEN=your_token_here")
print("  2. Make sure CYNIC API is running at http://localhost:8765")
print("  3. Run: python cynic/discord/bot.py")
